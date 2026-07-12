//! Event-authority ports consumed by the graph projection runtime.
//!
//! The graph domain owns its replay, derived-publication, and cursor-reporting
//! requirements. Product composition supplies adapters backed by one event
//! authority without exposing event storage to this domain.

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendReceipt, EventEnvelope, EventPage, LedgerCursor, LedgerIdentity, ReplayRequest,
};

/// Identity-bearing source of bounded canonical event pages.
pub trait GraphEventReplaySource: Send + Sync {
    /// Returns the ledger whose sequence space this source replays.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Replays one bounded page after an identity-bearing cursor.
    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError>;
}

/// Durable idempotent publication sink for graph-derived events.
pub trait GraphDerivedEventSink: Send + Sync {
    /// Returns the ledger that accepts derived events.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Durably appends one graph-derived envelope idempotently.
    fn append_derived(&self, envelope: EventEnvelope)
        -> Result<AppendReceipt, EventAuthorityError>;
}

/// Durable event-authority registry reporter for the graph consumer cursor.
pub trait GraphConsumerCursorReporter: Send + Sync {
    /// Returns the ledger whose consumer registry receives reports.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Reports the graph's durable cursor monotonically.
    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError>;
}
