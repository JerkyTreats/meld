//! Graph traversal domain for current anchors and provenance.
//!
//! The graph layer reduces runtime events into object facts, current anchors,
//! anchor history, and relation indexes. It owns graph-shaped facts only; belief
//! semantics and planner projections live in their own domains. Product
//! composition supplies identity-bearing event replay, derived-publication,
//! and cursor-reporting ports from one event authority.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::graph::runtime::GraphRuntime;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let runtime = GraphRuntime::new(sled::open(temp.path()).unwrap()).unwrap();
//! runtime.catch_up().unwrap();
//! ```

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

pub use contracts::{
    AnchorEndInput, AnchorId, AnchorProvenanceRecord, AnchorSelectionInput, AnchorSelectionRecord,
    GraphWalkResult, GraphWalkSpec, PerspectiveKey, TraversalDirection, TraversalFactId,
    TraversalFactRecord, TraversalIntent,
};
pub use ports::{GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource};
pub use query::TraversalQuery;
