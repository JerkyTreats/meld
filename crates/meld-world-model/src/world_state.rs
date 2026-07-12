//! World-state domains projected from identity-bearing event authority replay.
//!
//! The module contains two related surfaces. The graph domain is the current
//! substrate for anchors, provenance, object facts, and traversal queries. The
//! legacy claim domain remains available for compatibility and older tests.
//! Belief inference builds on graph contracts through its own domain boundary.
//!
//! Product composition builds the graph runtime from event-authority ports and
//! shares it with [`WorldModelQueries`].

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
