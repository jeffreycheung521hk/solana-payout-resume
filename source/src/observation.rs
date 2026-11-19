//! Chain facts gathered for one resume decision.
//!
//! Gathering them is the caller's job. These types only hold the answers; they never issue
//! queries, which keeps the decision a pure function.

use crate::record::Signature;
use std::collections::HashMap;

/// How a landed transaction executed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    /// The transaction ran without error.
    Succeeded,
    /// The transaction landed but failed; the fee was still charged.
    Failed {
        /// The error the cluster reported, as text.
        error: String,
    },
}

/// What `getSignatureStatuses` reports for a signature that has landed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureStatus {
    /// Slot the transaction landed in.
    pub slot: u64,
    /// Whether the block is finalized; false while the transaction is only processed.
    pub finalized: bool,
    /// How the transaction executed.
    pub outcome: ExecutionOutcome,
}

/// The result of looking up one signature with `searchTransactionHistory` enabled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryLookup {
    /// The signature has a status.
    Found(SignatureStatus),
    /// The node returned null. On its own this does not prove the transaction never landed.
    NotFound,
    /// The query failed or the node cannot search its history.
    Unavailable,
}

/// Chain facts for one decision about one payout record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainObservation {
    /// Current block height at finalized commitment.
    pub finalized_block_height: u64,
    /// Lowest slot the node still keeps, as `getFirstAvailableBlock` returns it.
    pub first_available_slot: u64,
    lookups: HashMap<Signature, HistoryLookup>,
}

impl ChainObservation {
    /// Starts an observation with no signature lookups yet.
    pub fn new(finalized_block_height: u64, first_available_slot: u64) -> Self {
        Self {
            finalized_block_height,
            first_available_slot,
            lookups: HashMap::new(),
        }
    }

    /// Adds the lookup result for one signature.
    pub fn with_lookup(mut self, signature: Signature, lookup: HistoryLookup) -> Self {
        self.lookups.insert(signature, lookup);
        self
    }

    /// The lookup result for this signature, if one was gathered.
    pub fn history_lookup(&self, signature: &Signature) -> Option<&HistoryLookup> {
        self.lookups.get(signature)
    }
}
