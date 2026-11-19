//! Runs interrupted-payout scenarios on the simulated chain and prints what happened.
//!
//! ```sh
//! cargo run --example interrupted_payout
//! ```
//!
//! Each scenario writes the payout record to a fresh directory, kills the first session where
//! the scenario says, lets the chain move on, and starts a second session that knows only the
//! record and the chain.

use solana_payout_resume::export::reconcile_csv;
use solana_payout_resume::simulation::contrast::{resign_when_unseen, trust_null_history};
use solana_payout_resume::simulation::{
    BroadcastFate, Interruption, SessionEnd, SimulatedChain, run_session_with,
};
use solana_payout_resume::{
    Address, ChainObservation, EscalationReason, ExecutionOutcome, NextAction, PayoutRecord,
    PayoutRecordStore, next_action,
};
use std::error::Error;

type Policy = fn(&PayoutRecord, &ChainObservation) -> NextAction;

struct Scenario {
    name: &'static str,
    fates: Vec<BroadcastFate>,
    interruption: Interruption,
    blocks_while_down: u64,
    prune_history: bool,
    policy: Policy,
}

struct Outcome {
    end: SessionEnd,
    record: PayoutRecord,
    paid: usize,
}

const PAYOUT: &str = "INV-001";

fn run(index: usize, scenario: &Scenario) -> Result<Outcome, Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!(
        "solana-payout-resume-example-{}-{index}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let store = PayoutRecordStore::open(&dir)?;
    store.save(&PayoutRecord::new(
        PAYOUT,
        Address([1; 32]),
        Address([2; 32]),
        Address([3; 32]),
        120_000_000,
    ))?;
    let mut chain = SimulatedChain::new();
    chain.script(scenario.fates.clone());

    let mut end = run_session_with(
        &store,
        &mut chain,
        PAYOUT,
        scenario.interruption,
        scenario.policy,
    )?;
    if end == SessionEnd::Interrupted {
        chain.advance(scenario.blocks_while_down);
        if scenario.prune_history {
            let now = chain.slot();
            chain.prune_history_before(now);
        }
        end = run_session_with(
            &store,
            &mut chain,
            PAYOUT,
            Interruption::None,
            scenario.policy,
        )?;
    }
    chain.advance(300);
    let outcome = Outcome {
        end,
        record: store.load(PAYOUT)?,
        paid: chain.successful_transfers(PAYOUT),
    };
    std::fs::remove_dir_all(&dir)?;
    Ok(outcome)
}

fn describe(end: &SessionEnd) -> String {
    match end {
        SessionEnd::Settled(NextAction::Done { .. }) => "Done".into(),
        SessionEnd::Settled(NextAction::Escalate { reason }) => {
            format!("Escalate: {}", reason_name(reason))
        }
        other => format!("{other:?}"),
    }
}

fn reason_name(reason: &EscalationReason) -> &'static str {
    match reason {
        EscalationReason::ObservationMissing { .. } => "ObservationMissing",
        EscalationReason::HistoryUnavailable { .. } => "HistoryUnavailable",
        EscalationReason::EarlierAttemptOnChain { .. } => "EarlierAttemptOnChain",
        EscalationReason::LandedButFailed { .. } => "LandedButFailed",
        EscalationReason::NotFinalizedAfterExpiry { .. } => "NotFinalizedAfterExpiry",
        EscalationReason::HistoryDoesNotCover { .. } => "HistoryDoesNotCover",
        EscalationReason::AttemptLimitReached { .. } => "AttemptLimitReached",
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let failed = BroadcastFate::Lands {
        after_blocks: 10,
        outcome: ExecutionOutcome::Failed {
            error: "insufficient funds".into(),
        },
    };
    let scenarios = [
        Scenario {
            name: "killed after broadcast, lands 30 blocks later",
            fates: vec![BroadcastFate::lands_after(30)],
            interruption: Interruption::AfterBroadcast,
            blocks_while_down: 5,
            prune_history: false,
            policy: next_action,
        },
        Scenario {
            name: "killed between write and broadcast",
            fates: vec![BroadcastFate::lands_after(20)],
            interruption: Interruption::AfterWrite,
            blocks_while_down: 5,
            prune_history: false,
            policy: next_action,
        },
        Scenario {
            name: "dropped while the session was down",
            fates: vec![BroadcastFate::Dropped, BroadcastFate::lands_after(10)],
            interruption: Interruption::AfterBroadcast,
            blocks_while_down: 400,
            prune_history: false,
            policy: next_action,
        },
        Scenario {
            name: "node history starts after the attempt",
            fates: vec![BroadcastFate::lands_after(10)],
            interruption: Interruption::AfterBroadcast,
            blocks_while_down: 400,
            prune_history: true,
            policy: next_action,
        },
        Scenario {
            name: "landed but failed",
            fates: vec![failed],
            interruption: Interruption::None,
            blocks_while_down: 0,
            prune_history: false,
            policy: next_action,
        },
        Scenario {
            name: "every attempt dropped",
            fates: Vec::new(),
            interruption: Interruption::None,
            blocks_while_down: 0,
            prune_history: false,
            policy: next_action,
        },
        Scenario {
            name: "contrast: re-sign before expiry",
            fates: vec![
                BroadcastFate::lands_after(30),
                BroadcastFate::lands_after(30),
            ],
            interruption: Interruption::AfterBroadcast,
            blocks_while_down: 5,
            prune_history: false,
            policy: resign_when_unseen,
        },
        Scenario {
            name: "contrast: trust null from pruned history",
            fates: vec![
                BroadcastFate::lands_after(10),
                BroadcastFate::lands_after(10),
            ],
            interruption: Interruption::AfterBroadcast,
            blocks_while_down: 400,
            prune_history: true,
            policy: trust_null_history,
        },
    ];

    println!(
        "{:<46} {:<34} {:>8} {:>5}",
        "scenario", "resumed session ended with", "attempts", "paid"
    );
    let mut export = None;
    for (index, scenario) in scenarios.iter().enumerate() {
        let outcome = run(index, scenario)?;
        println!(
            "{:<46} {:<34} {:>8} {:>5}",
            scenario.name,
            describe(&outcome.end),
            outcome.record.attempts.len(),
            outcome.paid
        );
        if index == 2 {
            export = Some(outcome);
        }
    }

    if let Some(Outcome {
        end: SessionEnd::Settled(action),
        record,
        ..
    }) = export
    {
        println!(
            "\nweb3testproj1 input for \"dropped while the session was down\" (synthetic signatures):"
        );
        print!("{}", reconcile_csv([(&record, &action)])?);
    }
    Ok(())
}
