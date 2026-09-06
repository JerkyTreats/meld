//! Graph traversal domain for current anchors and provenance.
//!
//! The graph layer reduces runtime events into object facts, current anchors,
//! anchor history, and relation indexes. It owns graph-shaped facts only; belief
//! semantics and planner projections live in their own domains. Product
//! composition supplies identity-bearing event replay, derived-publication,
//! and cursor-reporting ports from one event authority.
//!
//! Product composition creates [`runtime::GraphRuntime`] with [`runtime::GraphRuntime::from_ports`].
//! Raw event storage construction is intentionally unavailable here.

pub mod admission;
pub mod compat;
pub mod contracts;
mod cursor;
pub mod events;
mod outbox;
pub mod ports;
pub mod projection;
pub mod query;
pub mod reducer;
pub mod runtime;
mod source_intent;
pub mod store;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod visibility;

pub use contracts::{
    AnchorEndInput, AnchorId, AnchorProvenanceRecord, AnchorSelectionInput, AnchorSelectionRecord,
    GraphWalkResult, GraphWalkSpec, PerspectiveKey, TraversalDirection, TraversalFactId,
    TraversalFactRecord, TraversalIntent,
};
pub use ports::{GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource};
pub use query::TraversalQuery;

pub(crate) mod source_replay;
