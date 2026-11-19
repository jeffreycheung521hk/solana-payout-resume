//! The resume decision: a pure function of the payout record and chain facts.

use crate::observation::{ChainObservation, ExecutionOutcome, HistoryLookup, SignatureStatus};
use crate::record::{Attempt, PayoutRecord, Signature};

/// Most attempts one payout may have, counting the first.
///
/// If the absence check were wrong every time, the payout would be paid at most this many times.
pub const MAX_ATTEMPTS: usize = 3;

/// What the resuming side should do next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NextAction {
    /// The payout landed in a finalized block and succeeded.
    Done {
        /// The attempt that paid.
        signature: Signature,
    },
    /// The latest attempt is not seen landed and has not expired: send the same signed bytes
    /// again and keep waiting. The signature does not change, so this cannot pay twice.
    Rebroadcast {
        /// The attempt to resend.
        signature: Signature,
    },
    /// Sign a new attempt. Either the payout has no attempt yet, or the latest attempt is proven
    /// expired and absent from history.
    SignNewAttempt,
    /// The evidence supports none of the above; a person decides.
    Escalate {
        /// Why the payout is handed over.
        reason: EscalationReason,
    },
}

/// Why a payout is handed to a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EscalationReason {
    /// The observation has no lookup for one of the record's signatures.
    ObservationMissing {
        /// The signature without a lookup.
        signature: Signature,
    },
    /// A history lookup failed, or the node cannot search its history.
    HistoryUnavailable {
        /// The signature whose lookup failed.
        signature: Signature,
    },
    /// An earlier attempt is on chain, although it had been proven absent before the next attempt
    /// was signed.
    EarlierAttemptOnChain {
        /// The earlier attempt that was found.
        signature: Signature,
    },
    /// The latest attempt landed but failed; re-signing it unchanged would fail the same way.
    LandedButFailed {
        /// The attempt that failed.
        signature: Signature,
        /// The error the cluster reported.
        error: String,
    },
    /// The latest attempt expired while seen only as processed, never finalized.
    NotFinalizedAfterExpiry {
        /// The attempt in question.
        signature: Signature,
    },
    /// The node's history starts after the slot at which the latest attempt's blockhash was
    /// fetched, so a null lookup does not prove the attempt never landed.
    HistoryDoesNotCover {
        /// The attempt in question.
        signature: Signature,
        /// Lowest slot the node keeps.
        first_available_slot: u64,
        /// Slot at which the attempt's blockhash was fetched.
        blockhash_fetch_slot: u64,
    },
    /// The latest attempt is proven expired and absent, but the payout already has
    /// [`MAX_ATTEMPTS`] attempts.
    AttemptLimitReached {
        /// Attempts the payout has.
        attempts: usize,
    },
}

/// Decides the next action from the payout record and chain facts alone.
///
/// The rules, in order:
/// 1. No attempt yet: sign the first one. Attempts are written before they are broadcast, so
///    nothing unrecorded can be on chain.
/// 2. Every attempt needs a lookup, and every lookup must have succeeded; otherwise escalate.
/// 3. An earlier attempt found on chain contradicts the absence proof that allowed the next
///    attempt: escalate.
/// 4. The latest attempt found finalized: done if it succeeded, escalate if it failed.
/// 5. The latest attempt found but not finalized: rebroadcast while it has not expired,
///    escalate after.
/// 6. The latest attempt not found and not expired: rebroadcast.
/// 7. The latest attempt not found and expired: sign a new attempt only if the node's history
///    covers the attempt and the payout has fewer than [`MAX_ATTEMPTS`] attempts.
///
/// An attempt has expired once the finalized block height is greater than its last valid block
/// height.
pub fn next_action(record: &PayoutRecord, observation: &ChainObservation) -> NextAction {
    decide(record, observation).unwrap_or_else(|reason| NextAction::Escalate { reason })
}

enum Seen<'a> {
    Found(&'a SignatureStatus),
    NotFound,
}

fn seen<'a>(
    attempt: &Attempt,
    observation: &'a ChainObservation,
) -> Result<Seen<'a>, EscalationReason> {
    let signature = attempt.signature;
    match observation.history_lookup(&signature) {
        None => Err(EscalationReason::ObservationMissing { signature }),
        Some(HistoryLookup::Unavailable) => Err(EscalationReason::HistoryUnavailable { signature }),
        Some(HistoryLookup::NotFound) => Ok(Seen::NotFound),
        Some(HistoryLookup::Found(status)) => Ok(Seen::Found(status)),
    }
}

fn decide(
    record: &PayoutRecord,
    observation: &ChainObservation,
) -> Result<NextAction, EscalationReason> {
    let Some((latest, earlier)) = record.attempts.split_last() else {
        return Ok(NextAction::SignNewAttempt);
    };
    for attempt in earlier {
        if let Seen::Found(_) = seen(attempt, observation)? {
            return Err(EscalationReason::EarlierAttemptOnChain {
                signature: attempt.signature,
            });
        }
    }

    let signature = latest.signature;
    let expired = observation.finalized_block_height > latest.last_valid_block_height;
    match seen(latest, observation)? {
        Seen::Found(status) if status.finalized => match &status.outcome {
            ExecutionOutcome::Succeeded => Ok(NextAction::Done { signature }),
            ExecutionOutcome::Failed { error } => Err(EscalationReason::LandedButFailed {
                signature,
                error: error.clone(),
            }),
        },
        Seen::Found(_) if expired => Err(EscalationReason::NotFinalizedAfterExpiry { signature }),
        Seen::Found(_) => Ok(NextAction::Rebroadcast { signature }),
        Seen::NotFound if !expired => Ok(NextAction::Rebroadcast { signature }),
        Seen::NotFound if observation.first_available_slot > latest.blockhash_fetch_slot => {
            Err(EscalationReason::HistoryDoesNotCover {
                signature,
                first_available_slot: observation.first_available_slot,
                blockhash_fetch_slot: latest.blockhash_fetch_slot,
            })
        }
        Seen::NotFound if record.attempts.len() >= MAX_ATTEMPTS => {
            Err(EscalationReason::AttemptLimitReached {
                attempts: record.attempts.len(),
            })
        }
        Seen::NotFound => Ok(NextAction::SignNewAttempt),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Address;

    const LAST_VALID: u64 = 1_150;
    const FETCH_SLOT: u64 = 1_000;

    fn sig(n: u8) -> Signature {
        Signature([n; 64])
    }

    fn attempt(n: u8) -> Attempt {
        Attempt {
            signature: sig(n),
            signed_tx: vec![n],
            last_valid_block_height: LAST_VALID,
            blockhash_fetch_slot: FETCH_SLOT,
        }
    }

    fn record(attempts: u8) -> PayoutRecord {
        let mut record = PayoutRecord::new(
            "INV-001",
            Address([1; 32]),
            Address([2; 32]),
            Address([3; 32]),
            120_000_000,
        );
        record.attempts = (1..=attempts).map(attempt).collect();
        record
    }

    fn found(finalized: bool, outcome: ExecutionOutcome) -> HistoryLookup {
        HistoryLookup::Found(SignatureStatus {
            slot: FETCH_SLOT + 5,
            finalized,
            outcome,
        })
    }

    fn observe(
        height: u64,
        first_available: u64,
        lookups: &[(u8, HistoryLookup)],
    ) -> ChainObservation {
        lookups.iter().fold(
            ChainObservation::new(height, first_available),
            |obs, (n, lookup)| obs.with_lookup(sig(*n), lookup.clone()),
        )
    }

    fn escalated(action: NextAction) -> EscalationReason {
        match action {
            NextAction::Escalate { reason } => reason,
            other => panic!("expected an escalation, got {other:?}"),
        }
    }

    #[test]
    fn no_attempt_yet_signs_the_first() {
        assert_eq!(
            next_action(&record(0), &observe(1_000, 0, &[])),
            NextAction::SignNewAttempt
        );
    }

    #[test]
    fn finalized_success_is_done() {
        let obs = observe(1_010, 0, &[(1, found(true, ExecutionOutcome::Succeeded))]);
        assert_eq!(
            next_action(&record(1), &obs),
            NextAction::Done { signature: sig(1) }
        );
    }

    #[test]
    fn finalized_failure_escalates_with_the_error() {
        let failed = ExecutionOutcome::Failed {
            error: "insufficient funds".into(),
        };
        let obs = observe(1_010, 0, &[(1, found(true, failed))]);
        assert_eq!(
            escalated(next_action(&record(1), &obs)),
            EscalationReason::LandedButFailed {
                signature: sig(1),
                error: "insufficient funds".into()
            }
        );
    }

    #[test]
    fn processed_only_rebroadcasts_until_expiry_then_escalates() {
        let processed = found(false, ExecutionOutcome::Succeeded);
        let before = observe(LAST_VALID, 0, &[(1, processed.clone())]);
        assert_eq!(
            next_action(&record(1), &before),
            NextAction::Rebroadcast { signature: sig(1) }
        );
        let after = observe(LAST_VALID + 1, 0, &[(1, processed)]);
        assert_eq!(
            escalated(next_action(&record(1), &after)),
            EscalationReason::NotFinalizedAfterExpiry { signature: sig(1) }
        );
    }

    #[test]
    fn not_found_rebroadcasts_up_to_and_including_the_last_valid_height() {
        let obs = observe(LAST_VALID, 0, &[(1, HistoryLookup::NotFound)]);
        assert_eq!(
            next_action(&record(1), &obs),
            NextAction::Rebroadcast { signature: sig(1) }
        );
    }

    #[test]
    fn not_found_after_expiry_with_covering_history_signs_a_new_attempt() {
        let obs = observe(LAST_VALID + 1, FETCH_SLOT, &[(1, HistoryLookup::NotFound)]);
        assert_eq!(next_action(&record(1), &obs), NextAction::SignNewAttempt);
    }

    #[test]
    fn history_starting_after_the_fetch_slot_escalates() {
        let obs = observe(
            LAST_VALID + 1,
            FETCH_SLOT + 1,
            &[(1, HistoryLookup::NotFound)],
        );
        assert_eq!(
            escalated(next_action(&record(1), &obs)),
            EscalationReason::HistoryDoesNotCover {
                signature: sig(1),
                first_available_slot: FETCH_SLOT + 1,
                blockhash_fetch_slot: FETCH_SLOT,
            }
        );
    }

    #[test]
    fn attempt_limit_stops_automatic_signing() {
        let lookups = [
            (1, HistoryLookup::NotFound),
            (2, HistoryLookup::NotFound),
            (3, HistoryLookup::NotFound),
        ];
        let obs = observe(LAST_VALID + 1, 0, &lookups);
        assert_eq!(
            escalated(next_action(&record(3), &obs)),
            EscalationReason::AttemptLimitReached {
                attempts: MAX_ATTEMPTS
            }
        );
        let two = observe(LAST_VALID + 1, 0, &lookups[..2]);
        assert_eq!(next_action(&record(2), &two), NextAction::SignNewAttempt);
    }

    #[test]
    fn missing_or_failed_lookups_escalate() {
        let missing = observe(1_010, 0, &[]);
        assert_eq!(
            escalated(next_action(&record(1), &missing)),
            EscalationReason::ObservationMissing { signature: sig(1) }
        );
        let unavailable = observe(1_010, 0, &[(1, HistoryLookup::Unavailable)]);
        assert_eq!(
            escalated(next_action(&record(1), &unavailable)),
            EscalationReason::HistoryUnavailable { signature: sig(1) }
        );
    }

    #[test]
    fn an_earlier_attempt_on_chain_escalates_even_if_the_latest_succeeded() {
        let lookups = [
            (1, found(true, ExecutionOutcome::Succeeded)),
            (2, found(true, ExecutionOutcome::Succeeded)),
        ];
        let obs = observe(1_010, 0, &lookups);
        assert_eq!(
            escalated(next_action(&record(2), &obs)),
            EscalationReason::EarlierAttemptOnChain { signature: sig(1) }
        );
    }

    #[test]
    fn an_unavailable_lookup_for_an_earlier_attempt_escalates() {
        let lookups = [
            (1, HistoryLookup::Unavailable),
            (2, found(true, ExecutionOutcome::Succeeded)),
        ];
        let obs = observe(1_010, 0, &lookups);
        assert_eq!(
            escalated(next_action(&record(2), &obs)),
            EscalationReason::HistoryUnavailable { signature: sig(1) }
        );
    }
}
