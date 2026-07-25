//! Durable consumer cursor contract for domain event consumers.
//!
//! Owner: events. Events owns durable cursor mutation and its monotonic
//! advancement rule. The consuming domain owns its consumer identity and
//! decides when to request advancement — only after its own domain state is
//! durable. This contract is the frozen Gate B seam for the world-model
//! evidence consumer; the graph reducer's projection-local cursor remains a
//! separate, earlier mechanism.
//!
//! This module carries no implementation. The evidence-ingestion workstream
//! supplies the first store-backed implementor.

use serde::{Deserialize, Serialize};

use crate::events::identity::LedgerIdentity;

/// Durable position of one named consumer against one ledger identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerCursorState {
    /// Ledger the cursor is bound to. A cursor never migrates silently
    /// across ledger identities.
    pub ledger_id: LedgerIdentity,
    /// Highest sequence the consumer has durably absorbed. Replay resumes
    /// strictly after this sequence.
    pub after_seq: u64,
}

/// Failure surface for durable cursor reads and advancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerCursorError {
    /// Human-readable failure description.
    pub message: String,
    /// Whether the caller may retry the same request unchanged.
    pub retryable: bool,
}

/// Durable cursor mutation port for named event consumers.
///
/// Advancement is monotonic: a request to advance behind the durable
/// position is a no-op that returns the current state, never an error.
/// Implementors must persist the new position before returning so that a
/// reopen between consumer commit and cursor commit replays idempotently.
pub trait DurableConsumerCursor {
    /// Ledger identity this cursor authority is bound to.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Current durable position for the consumer, if one exists.
    fn consumer_cursor(
        &self,
        consumer_id: &str,
    ) -> Result<Option<ConsumerCursorState>, ConsumerCursorError>;

    /// Advance the durable position to `after_seq`, monotonically.
    fn advance_consumer_cursor(
        &self,
        consumer_id: &str,
        after_seq: u64,
    ) -> Result<ConsumerCursorState, ConsumerCursorError>;
}
