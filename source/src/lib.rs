//! Resume an interrupted SPL Token payout from a record written before broadcast.
//!
//! An agent writes every attempt to the payout's record before it broadcasts the attempt.
//! Whoever resumes the payout later, with no memory of the earlier session, decides only from
//! that record and from chain facts, through [`next_action`].
//!
//! Everything here runs offline. [`simulation`] stands in for the chain; the signatures and
//! signed bytes it makes are synthetic and are not valid Solana transactions.
//!
//! ```
//! use solana_payout_resume::{next_action, Address, ChainObservation, NextAction, PayoutRecord};
//!
//! // 120 USDC, written in the mint's smallest unit (6 decimals).
//! let record = PayoutRecord::new("INV-001", Address([1; 32]), Address([2; 32]), Address([3; 32]), 120_000_000);
//! let observation = ChainObservation::new(1_000, 0);
//!
//! // No attempt was written, so none can have been broadcast: sign the first one.
//! assert_eq!(next_action(&record, &observation), NextAction::SignNewAttempt);
//! ```
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod decision;
pub mod export;
pub mod observation;
pub mod record;
pub mod simulation;
pub mod store;

pub use decision::{EscalationReason, MAX_ATTEMPTS, NextAction, next_action};
pub use observation::{ChainObservation, ExecutionOutcome, HistoryLookup, SignatureStatus};
pub use record::{Address, Attempt, Base58Error, PayoutRecord, Signature};
pub use store::{PayoutRecordStore, StoreError};
