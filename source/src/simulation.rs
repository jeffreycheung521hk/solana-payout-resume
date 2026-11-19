//! An offline, deterministic stand-in for the chain, used by the tests and the example.
//!
//! The signatures and signed bytes made here are synthetic. They are not valid Solana
//! transactions and cannot be looked up on any real cluster.

use crate::decision::{EscalationReason, MAX_ATTEMPTS, NextAction, next_action};
use crate::observation::{ChainObservation, ExecutionOutcome, HistoryLookup, SignatureStatus};
use crate::record::{Attempt, PayoutRecord, Signature};
use crate::store::{PayoutRecordStore, StoreError};
use std::collections::VecDeque;

/// Blocks a blockhash stays valid after the block it belongs to.
pub const BLOCKHASH_VALIDITY_BLOCKS: u64 = 150;

/// Blocks a simulated session waits between two polls.
pub const POLL_INTERVAL_BLOCKS: u64 = 10;

/// Polls one simulated session may take before it gives up.
pub const MAX_SESSION_STEPS: usize = 1_000;

const SYNTHETIC_PREFIX: &str = "synthetic-transfer";

/// What `latest_blockhash` returns: the blockhash and the two numbers an attempt records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockhashInfo {
    /// Block height the blockhash belongs to; it stands in for the hash itself.
    pub block_height: u64,
    /// Last block height at which a transaction using this blockhash can be processed.
    pub last_valid_block_height: u64,
    /// Slot at which the blockhash was fetched.
    pub fetch_slot: u64,
}

/// What happens to a transaction when it is broadcast and is not already landed or on its way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BroadcastFate {
    /// It lands this many blocks later, if that is still within its validity window.
    Lands {
        /// Blocks until it lands; zero is treated as one.
        after_blocks: u64,
        /// How it executes.
        outcome: ExecutionOutcome,
    },
    /// It is lost before it reaches a leader.
    Dropped,
}

impl BroadcastFate {
    /// Lands and succeeds after this many blocks.
    pub fn lands_after(after_blocks: u64) -> Self {
        Self::Lands {
            after_blocks,
            outcome: ExecutionOutcome::Succeeded,
        }
    }
}

#[derive(Clone, Debug)]
struct Pending {
    signature: Signature,
    payout_id: String,
    lands_at_height: u64,
    outcome: ExecutionOutcome,
}

#[derive(Clone, Debug)]
struct Landed {
    signature: Signature,
    payout_id: String,
    slot: u64,
    outcome: ExecutionOutcome,
}

/// The simulated chain. Blocks are final as soon as they are produced.
#[derive(Clone, Debug)]
pub struct SimulatedChain {
    block_height: u64,
    slot: u64,
    first_available_slot: u64,
    history_available: bool,
    fates: VecDeque<BroadcastFate>,
    pending: Vec<Pending>,
    landed: Vec<Landed>,
    broadcasts: usize,
}

impl Default for SimulatedChain {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedChain {
    /// Starts at block height 1000 and slot 1000, keeping all history.
    pub fn new() -> Self {
        Self {
            block_height: 1_000,
            slot: 1_000,
            first_available_slot: 0,
            history_available: true,
            fates: VecDeque::new(),
            pending: Vec::new(),
            landed: Vec::new(),
            broadcasts: 0,
        }
    }

    /// Queues fates for the next broadcasts that need one, in order. Once the queue is empty,
    /// broadcasts are dropped.
    pub fn script(&mut self, fates: impl IntoIterator<Item = BroadcastFate>) {
        self.fates.extend(fates);
    }

    /// Current block height; every block is final.
    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    /// Current slot.
    pub fn slot(&self) -> u64 {
        self.slot
    }

    /// The latest blockhash, as `getLatestBlockhash` returns it at finalized commitment.
    pub fn latest_blockhash(&self) -> BlockhashInfo {
        BlockhashInfo {
            block_height: self.block_height,
            last_valid_block_height: self.block_height + BLOCKHASH_VALIDITY_BLOCKS,
            fetch_slot: self.slot,
        }
    }

    /// Receives signed bytes. Bytes that already landed or are on their way are ignored, as the
    /// cluster deduplicates by signature; bytes whose blockhash expired are rejected.
    pub fn broadcast(&mut self, signed_tx: &[u8]) {
        self.broadcasts += 1;
        let signature = synthetic_signature(signed_tx);
        let known = self
            .landed
            .iter()
            .any(|landed| landed.signature == signature)
            || self
                .pending
                .iter()
                .any(|pending| pending.signature == signature);
        if known {
            return;
        }
        let Some(transfer) = SyntheticTransfer::parse(signed_tx) else {
            return;
        };
        let last_valid = transfer.blockhash_height + BLOCKHASH_VALIDITY_BLOCKS;
        if self.block_height > last_valid {
            return;
        }
        if let Some(BroadcastFate::Lands {
            after_blocks,
            outcome,
        }) = self.fates.pop_front()
        {
            let lands_at_height = self.block_height + after_blocks.max(1);
            if lands_at_height <= last_valid {
                self.pending.push(Pending {
                    signature,
                    payout_id: transfer.payout_id,
                    lands_at_height,
                    outcome,
                });
            }
        }
    }

    /// Produces this many blocks, one per slot, landing every transaction whose time has come.
    pub fn advance(&mut self, blocks: u64) {
        for _ in 0..blocks {
            self.block_height += 1;
            self.slot += 1;
            let (height, slot) = (self.block_height, self.slot);
            let (due, waiting): (Vec<_>, Vec<_>) = self
                .pending
                .drain(..)
                .partition(|pending| pending.lands_at_height <= height);
            self.pending = waiting;
            self.landed.extend(due.into_iter().map(|pending| Landed {
                signature: pending.signature,
                payout_id: pending.payout_id,
                slot,
                outcome: pending.outcome,
            }));
        }
    }

    /// Passes slots in which no block is produced, so the slot runs ahead of the block height.
    pub fn skip_slots(&mut self, slots: u64) {
        self.slot += slots;
    }

    /// The node forgets everything before this slot, as a node that keeps only recent history
    /// does. Earlier transactions then look absent to history lookups.
    pub fn prune_history_before(&mut self, slot: u64) {
        self.first_available_slot = self.first_available_slot.max(slot);
    }

    /// Makes history lookups fail, or succeed again.
    pub fn set_history_available(&mut self, available: bool) {
        self.history_available = available;
    }

    /// Looks up every attempt of the record, as `getSignatureStatuses` with
    /// `searchTransactionHistory` and `getFirstAvailableBlock` would answer.
    pub fn observe(&self, record: &PayoutRecord) -> ChainObservation {
        record.attempts.iter().fold(
            ChainObservation::new(self.block_height, self.first_available_slot),
            |observation, attempt| {
                observation.with_lookup(attempt.signature, self.lookup(&attempt.signature))
            },
        )
    }

    fn lookup(&self, signature: &Signature) -> HistoryLookup {
        if !self.history_available {
            return HistoryLookup::Unavailable;
        }
        match self
            .landed
            .iter()
            .find(|landed| landed.signature == *signature)
        {
            Some(landed) if landed.slot >= self.first_available_slot => {
                HistoryLookup::Found(SignatureStatus {
                    slot: landed.slot,
                    finalized: true,
                    outcome: landed.outcome.clone(),
                })
            }
            _ => HistoryLookup::NotFound,
        }
    }

    /// How many transfers of this payout landed and succeeded. This is the ground truth the
    /// tests check: more than one means the payout was paid twice.
    pub fn successful_transfers(&self, payout_id: &str) -> usize {
        self.landed
            .iter()
            .filter(|landed| {
                landed.payout_id == payout_id && landed.outcome == ExecutionOutcome::Succeeded
            })
            .count()
    }

    /// How many times anything was broadcast, rebroadcasts included.
    pub fn broadcast_count(&self) -> usize {
        self.broadcasts
    }
}

/// Signs the payout's next attempt with synthetic bytes and a synthetic signature.
pub fn sign_attempt(record: &PayoutRecord, blockhash: &BlockhashInfo) -> Attempt {
    let signed_tx = format!(
        "{SYNTHETIC_PREFIX}|{}|{}|{}|{}|{}|{}|{}",
        record.payout_id,
        record.attempts.len() + 1,
        blockhash.block_height,
        record.sender_owner,
        record.recipient_owner,
        record.mint,
        record.amount_atomic
    )
    .into_bytes();
    Attempt {
        signature: synthetic_signature(&signed_tx),
        signed_tx,
        last_valid_block_height: blockhash.last_valid_block_height,
        blockhash_fetch_slot: blockhash.fetch_slot,
    }
}

struct SyntheticTransfer {
    payout_id: String,
    blockhash_height: u64,
}

impl SyntheticTransfer {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut parts = text.split('|');
        if parts.next()? != SYNTHETIC_PREFIX {
            return None;
        }
        let payout_id = parts.next()?.to_owned();
        let _attempt_number = parts.next()?;
        let blockhash_height = parts.next()?.parse().ok()?;
        Some(Self {
            payout_id,
            blockhash_height,
        })
    }
}

/// A deterministic 64-byte stand-in for a signature over the bytes. It is FNV-1a, not a
/// cryptographic signature.
fn synthetic_signature(bytes: &[u8]) -> Signature {
    let mut out = [0u8; 64];
    for (round, chunk) in (0u64..).zip(out.chunks_mut(8)) {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ round;
        for &byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        chunk.copy_from_slice(&hash.to_le_bytes());
    }
    Signature(out)
}

/// Where a simulated session stops, as if its process were killed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interruption {
    /// Not interrupted: runs until the decision is done or escalated.
    None,
    /// Stops right after a new attempt is written, before it is broadcast.
    AfterWrite,
    /// Stops right after a new attempt is broadcast.
    AfterBroadcast,
}

/// How a simulated session ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEnd {
    /// The decision reached done or escalate.
    Settled(NextAction),
    /// The session stopped at the requested interruption.
    Interrupted,
    /// The session polled [`MAX_SESSION_STEPS`] times without settling; only a faulty policy
    /// gets here.
    StepLimit,
}

/// Runs one agent session with the crate's resume decision, [`next_action`].
pub fn run_session(
    store: &PayoutRecordStore,
    chain: &mut SimulatedChain,
    payout_id: &str,
    interruption: Interruption,
) -> Result<SessionEnd, StoreError> {
    run_session_with(store, chain, payout_id, interruption, next_action)
}

/// Runs one agent session with any decision policy, so a faulty policy can be compared with
/// [`next_action`].
///
/// The session keeps nothing between polls. Each poll loads the record from the store,
/// observes the chain, decides, and acts; a new attempt is written before it is broadcast.
/// Between polls the chain advances [`POLL_INTERVAL_BLOCKS`] blocks.
pub fn run_session_with<F>(
    store: &PayoutRecordStore,
    chain: &mut SimulatedChain,
    payout_id: &str,
    interruption: Interruption,
    policy: F,
) -> Result<SessionEnd, StoreError>
where
    F: Fn(&PayoutRecord, &ChainObservation) -> NextAction,
{
    for _ in 0..MAX_SESSION_STEPS {
        let mut record = store.load(payout_id)?;
        let observation = chain.observe(&record);
        match policy(&record, &observation) {
            NextAction::SignNewAttempt => {
                let attempt = sign_attempt(&record, &chain.latest_blockhash());
                let signed_tx = attempt.signed_tx.clone();
                store.append_attempt(&mut record, attempt)?;
                if interruption == Interruption::AfterWrite {
                    return Ok(SessionEnd::Interrupted);
                }
                chain.broadcast(&signed_tx);
                if interruption == Interruption::AfterBroadcast {
                    return Ok(SessionEnd::Interrupted);
                }
            }
            NextAction::Rebroadcast { signature } => {
                if let Some(attempt) = record.attempt(&signature) {
                    chain.broadcast(&attempt.signed_tx);
                }
            }
            settled @ (NextAction::Done { .. } | NextAction::Escalate { .. }) => {
                return Ok(SessionEnd::Settled(settled));
            }
        }
        chain.advance(POLL_INTERVAL_BLOCKS);
    }
    Ok(SessionEnd::StepLimit)
}

/// Deliberately wrong policies, kept to show what the rules in [`next_action`] prevent.
pub mod contrast {
    use super::*;

    /// Signs a new attempt whenever no attempt is seen succeeded yet, without waiting for the
    /// latest attempt's blockhash to expire.
    pub fn resign_when_unseen(record: &PayoutRecord, observation: &ChainObservation) -> NextAction {
        for attempt in &record.attempts {
            if let Some(HistoryLookup::Found(status)) =
                observation.history_lookup(&attempt.signature)
            {
                if status.finalized && status.outcome == ExecutionOutcome::Succeeded {
                    return NextAction::Done {
                        signature: attempt.signature,
                    };
                }
            }
        }
        NextAction::SignNewAttempt
    }

    /// Behaves like [`next_action`] but trusts a null lookup even when the node's history does
    /// not cover the attempt.
    pub fn trust_null_history(record: &PayoutRecord, observation: &ChainObservation) -> NextAction {
        match next_action(record, observation) {
            NextAction::Escalate {
                reason: EscalationReason::HistoryDoesNotCover { .. },
            } if record.attempts.len() < MAX_ATTEMPTS => NextAction::SignNewAttempt,
            other => other,
        }
    }
}
