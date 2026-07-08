//! World-state domains built from the shared event ledger.
//!
//! The module contains two related surfaces. The graph domain is the current
//! substrate for anchors, provenance, object facts, and traversal queries. The
//! legacy claim domain remains available for compatibility and older tests.
//! Belief inference builds on graph contracts through its own domain boundary.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::world_state::graph::runtime::GraphRuntime;
//! use meld_world_model::WorldModelQueries;
//! use std::sync::Arc;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let runtime = Arc::new(GraphRuntime::new(sled::open(temp.path()).unwrap()).unwrap());
//! let queries = WorldModelQueries::new(runtime);
//! assert!(queries.current_frame_head_count_by_type("analysis").unwrap() == 0);
//! ```

pub mod graph;

pub mod contracts;
pub mod events;
pub mod legacy_claims;
pub mod projection;
pub mod query;
pub mod query_runtime;
pub mod reducer;
pub mod store;

pub use contracts::{
    ClaimId, ClaimKind, ClaimRecord, EvidenceId, EvidenceRecord, ProvenanceRecord,
    SettlementStatus, WorldStateFactId,
};
pub use graph::{
    AnchorId, AnchorProvenanceRecord, AnchorSelectionRecord, GraphWalkResult, GraphWalkSpec,
    PerspectiveKey, TraversalDirection, TraversalFactId, TraversalFactRecord, TraversalQuery,
};
pub use query::WorldStateQuery;
pub use query_runtime::WorldModelQueries;
