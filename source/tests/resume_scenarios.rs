//! Interrupted payouts resumed by a fresh session that knows only the record and the chain.

use solana_payout_resume::export::reconcile_csv;
use solana_payout_resume::simulation::contrast::{resign_when_unseen, trust_null_history};
use solana_payout_resume::simulation::{
    BroadcastFate, Interruption, SessionEnd, SimulatedChain, run_session, run_session_with,
};
use solana_payout_resume::{
    Address, EscalationReason, ExecutionOutcome, MAX_ATTEMPTS, NextAction, PayoutRecord,
    PayoutRecordStore, Signature,
};
use std::path::PathBuf;

const PAYOUT: &str = "INV-001";

struct Fixture {
    dir: PathBuf,
    store: PayoutRecordStore,
    chain: SimulatedChain,
}

impl Fixture {
    fn new(name: &str, fates: Vec<BroadcastFate>) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "solana-payout-resume-scenario-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let store = PayoutRecordStore::open(&dir).unwrap();
        let record = PayoutRecord::new(
            PAYOUT,
            Address([1; 32]),
            Address([2; 32]),
            Address([3; 32]),
            120_000_000,
        );
        store.save(&record).unwrap();
        let mut chain = SimulatedChain::new();
        chain.script(fates);
        Self { dir, store, chain }
    }

    fn session(&mut self, interruption: Interruption) -> SessionEnd {
        run_session(&self.store, &mut self.chain, PAYOUT, interruption).unwrap()
    }

    fn record(&self) -> PayoutRecord {
        self.store.load(PAYOUT).unwrap()
    }

    fn paid(&self) -> usize {
        self.chain.successful_transfers(PAYOUT)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn done_with(record: &PayoutRecord, index: usize) -> SessionEnd {
    SessionEnd::Settled(NextAction::Done {
        signature: record.attempts[index].signature,
    })
}

fn escalation(end: SessionEnd) -> EscalationReason {
    match end {
        SessionEnd::Settled(NextAction::Escalate { reason }) => reason,
        other => panic!("expected an escalation, got {other:?}"),
    }
}

#[test]
fn killed_after_broadcast_resumes_without_paying_twice() {
    let mut f = Fixture::new("after-broadcast", vec![BroadcastFate::lands_after(30)]);
    assert_eq!(
        f.session(Interruption::AfterBroadcast),
        SessionEnd::Interrupted
    );
    f.chain.advance(5);
    let end = f.session(Interruption::None);
    assert_eq!(end, done_with(&f.record(), 0));
    f.chain.advance(300);
    assert_eq!(f.paid(), 1);
    assert_eq!(f.record().attempts.len(), 1);
}

#[test]
fn resigning_before_expiry_pays_twice() {
    let fates = vec![
        BroadcastFate::lands_after(30),
        BroadcastFate::lands_after(30),
    ];
    let mut f = Fixture::new("resign-when-unseen", fates);
    assert_eq!(
        f.session(Interruption::AfterBroadcast),
        SessionEnd::Interrupted
    );
    f.chain.advance(5);
    let end = run_session_with(
        &f.store,
        &mut f.chain,
        PAYOUT,
        Interruption::None,
        resign_when_unseen,
    )
    .unwrap();
    assert!(matches!(end, SessionEnd::Settled(NextAction::Done { .. })));
    f.chain.advance(300);
    assert_eq!(f.paid(), 2, "the faulty policy is expected to pay twice");
}

#[test]
fn killed_between_write_and_broadcast_sends_on_resume() {
    let mut f = Fixture::new("after-write", vec![BroadcastFate::lands_after(20)]);
    assert_eq!(f.session(Interruption::AfterWrite), SessionEnd::Interrupted);
    assert_eq!(f.chain.broadcast_count(), 0);
    assert_eq!(
        f.record().attempts.len(),
        1,
        "the attempt is on disk before any broadcast"
    );
    f.chain.advance(5);
    assert_eq!(f.session(Interruption::None), done_with(&f.record(), 0));
    assert_eq!(f.paid(), 1);
}

#[test]
fn dropped_attempt_is_replaced_only_after_it_expires() {
    let fates = vec![BroadcastFate::Dropped, BroadcastFate::lands_after(10)];
    let mut f = Fixture::new("dropped", fates);
    assert_eq!(
        f.session(Interruption::AfterBroadcast),
        SessionEnd::Interrupted
    );
    let first = f.record().attempts[0].clone();
    f.chain.advance(400);
    assert!(f.chain.block_height() > first.last_valid_block_height);
    assert_eq!(f.session(Interruption::None), done_with(&f.record(), 1));
    let record = f.record();
    assert_eq!(record.attempts.len(), 2);
    assert!(record.attempts[1].blockhash_fetch_slot > first.last_valid_block_height);
    assert_eq!(f.paid(), 1);
}

#[test]
fn pruned_history_escalates_instead_of_signing_again() {
    let mut f = Fixture::new("pruned", vec![BroadcastFate::lands_after(10)]);
    assert_eq!(
        f.session(Interruption::AfterBroadcast),
        SessionEnd::Interrupted
    );
    f.chain.advance(400);
    let now = f.chain.slot();
    f.chain.prune_history_before(now);
    let reason = escalation(f.session(Interruption::None));
    assert!(
        matches!(reason, EscalationReason::HistoryDoesNotCover { .. }),
        "{reason:?}"
    );
    assert_eq!(f.paid(), 1);
    assert_eq!(f.record().attempts.len(), 1);
}

#[test]
fn trusting_null_from_pruned_history_pays_twice() {
    let fates = vec![
        BroadcastFate::lands_after(10),
        BroadcastFate::lands_after(10),
    ];
    let mut f = Fixture::new("trust-null", fates);
    assert_eq!(
        f.session(Interruption::AfterBroadcast),
        SessionEnd::Interrupted
    );
    f.chain.advance(400);
    let now = f.chain.slot();
    f.chain.prune_history_before(now);
    let end = run_session_with(
        &f.store,
        &mut f.chain,
        PAYOUT,
        Interruption::None,
        trust_null_history,
    )
    .unwrap();
    assert!(matches!(end, SessionEnd::Settled(NextAction::Done { .. })));
    assert_eq!(f.paid(), 2, "the faulty policy is expected to pay twice");
}

#[test]
fn every_attempt_dropped_stops_at_the_limit() {
    let mut f = Fixture::new("all-dropped", Vec::new());
    let reason = escalation(f.session(Interruption::None));
    assert_eq!(
        reason,
        EscalationReason::AttemptLimitReached {
            attempts: MAX_ATTEMPTS
        }
    );
    assert_eq!(f.record().attempts.len(), MAX_ATTEMPTS);
    assert_eq!(f.paid(), 0);
}

#[test]
fn landed_failure_goes_to_a_person() {
    let failed = BroadcastFate::Lands {
        after_blocks: 10,
        outcome: ExecutionOutcome::Failed {
            error: "insufficient funds".into(),
        },
    };
    let mut f = Fixture::new("failed", vec![failed]);
    let reason = escalation(f.session(Interruption::None));
    assert!(
        matches!(reason, EscalationReason::LandedButFailed { .. }),
        "{reason:?}"
    );
    assert_eq!(f.record().attempts.len(), 1);
    assert_eq!(f.paid(), 0);
}

#[test]
fn unavailable_history_goes_to_a_person() {
    let mut f = Fixture::new("unavailable", vec![BroadcastFate::lands_after(10)]);
    assert_eq!(
        f.session(Interruption::AfterBroadcast),
        SessionEnd::Interrupted
    );
    f.chain.set_history_available(false);
    let reason = escalation(f.session(Interruption::None));
    assert!(
        matches!(reason, EscalationReason::HistoryUnavailable { .. }),
        "{reason:?}"
    );
}

#[test]
fn export_lists_every_attempt_with_the_done_one_first() {
    let fates = vec![BroadcastFate::Dropped, BroadcastFate::lands_after(10)];
    let mut f = Fixture::new("export", fates);
    // Rebroadcasting a dropped attempt gives it another chance to land, so the session is
    // killed and the first attempt left to expire; only then does a second attempt exist.
    assert_eq!(
        f.session(Interruption::AfterBroadcast),
        SessionEnd::Interrupted
    );
    f.chain.advance(400);
    let end = f.session(Interruption::None);
    let SessionEnd::Settled(action) = end else {
        panic!("expected the session to settle");
    };
    let record = f.record();
    let csv = reconcile_csv([(&record, &action)]).unwrap();
    let lines: Vec<_> = csv.lines().collect();
    assert_eq!(lines.len(), 3, "header plus one row per attempt");
    let signature = |line: &str| Signature::from_base58(line.split(',').nth(1).unwrap()).unwrap();
    assert_eq!(
        signature(lines[1]),
        record.attempts[1].signature,
        "the attempt that paid comes first"
    );
    assert_eq!(signature(lines[2]), record.attempts[0].signature);
    assert!(lines[1].starts_with("INV-001,"));
}
