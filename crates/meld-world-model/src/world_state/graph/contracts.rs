//! Public graph traversal contracts.
//!
//! These records describe facts extracted from the event spine and anchors that
//! identify the current object for a subject under a perspective. Anchors are
//! generic graph contracts, not belief or planner decisions.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::PerspectiveKey;
//!
//! let key = PerspectiveKey::new("frame_type", "analysis").unwrap();
//! assert_eq!(key.index_key(), "frame_type::analysis");
//! ```

use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRelation};

/// Durable anchor identifier.
pub type AnchorId = String;
/// Durable traversal fact identifier.
pub type TraversalFactId = String;
/// Durable provenance identifier.
pub type ProvenanceId = String;

/// Perspective namespace for current-anchor selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerspectiveKey {
    /// Perspective family such as `frame_type`.
    pub perspective_kind: String,
    /// Perspective member such as `analysis`.
    pub perspective_id: String,
}

impl PerspectiveKey {
    /// Build and validate a perspective key.
    pub fn new(
        perspective_kind: impl Into<String>,
        perspective_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        let key = Self {
            perspective_kind: perspective_kind.into(),
            perspective_id: perspective_id.into(),
        };
        key.validate()?;
        Ok(key)
    }

    /// Reject empty fields before using the key in durable indexes.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.perspective_kind.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "perspective kind must be non-empty".to_string(),
            ));
        }
        if self.perspective_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "perspective id must be non-empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Deterministic storage key for perspective indexes.
    pub fn index_key(&self) -> String {
        format!("{}::{}", self.perspective_kind, self.perspective_id)
    }
}

/// Durable record for one anchor selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSelectionRecord {
    /// Stable anchor id.
    pub anchor_id: AnchorId,
    /// Logical anchor slot being selected.
    pub anchor_ref: DomainObjectRef,
    /// Subject whose current state this anchor describes.
    pub subject: DomainObjectRef,
    /// Perspective that owns this current selection.
    pub perspective: PerspectiveKey,
    /// Current target object for the subject and perspective.
    pub target: DomainObjectRef,
    /// Source fact ids that justify this anchor.
    pub source_fact_ids: Vec<String>,
    /// Fact that created this anchor record.
    pub created_by_fact_id: String,
    /// Runtime sequence where this anchor became current.
    pub selected_at_seq: u64,
    /// Runtime sequence where this anchor stopped being current.
    pub ended_at_seq: Option<u64>,
    /// Replacement anchor id when superseded by another anchor.
    pub ended_by_anchor_id: Option<AnchorId>,
    #[serde(default)]
    /// Fact that ended this anchor when known.
    pub ended_by_fact_id: Option<String>,
}

/// Reducer input for selecting a new current anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSelectionInput {
    pub anchor_ref: DomainObjectRef,
    pub subject: DomainObjectRef,
    pub perspective: PerspectiveKey,
    pub target: DomainObjectRef,
    pub source_fact_id: String,
}

/// Reducer input for ending a current anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorEndInput {
    pub anchor_ref: DomainObjectRef,
    pub ended_at_seq: u64,
}

/// Graph mutation intent derived from a source event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TraversalIntent {
    /// Select a new current anchor.
    SelectAnchor(AnchorSelectionInput),
    /// End an existing current anchor.
    EndAnchor(AnchorEndInput),
}

/// Graph-readable fact copied from the event spine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalFactRecord {
    pub fact_id: TraversalFactId,
    pub source_spine_fact_id: String,
    pub seq: u64,
    pub event_type: String,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

/// Provenance bundle for a selected anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorProvenanceRecord {
    pub anchor_id: AnchorId,
    pub source_fact_ids: Vec<String>,
    #[serde(default)]
    pub derived_fact_ids: Vec<String>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

/// Direction used by neighbor and walk queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}

/// Bounded graph walk request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphWalkSpec {
    pub direction: TraversalDirection,
    pub relation_types: Option<Vec<String>>,
    pub max_depth: usize,
    pub current_only: bool,
    pub include_facts: bool,
}

impl GraphWalkSpec {
    /// Validate walk bounds before querying indexes.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.max_depth == 0 {
            return Err(StorageError::InvalidPath(
                "graph walk max_depth must be at least 1".to_string(),
            ));
        }
        Ok(())
    }
}

/// Objects, facts, and relations reached by a graph walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphWalkResult {
    pub visited_objects: Vec<DomainObjectRef>,
    pub visited_facts: Vec<TraversalFactRecord>,
    pub traversed_relations: Vec<EventRelation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_selection_record_round_trips() {
        let record = AnchorSelectionRecord {
            anchor_id: "anchor_a".to_string(),
            anchor_ref: DomainObjectRef::new("context", "head", "node_a::analysis").unwrap(),
            subject: DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap(),
            perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
            target: DomainObjectRef::new("context", "frame", "frame_a").unwrap(),
            source_fact_ids: vec!["spine::1".to_string()],
            created_by_fact_id: "fact_a".to_string(),
            selected_at_seq: 1,
            ended_at_seq: None,
            ended_by_anchor_id: None,
            ended_by_fact_id: None,
        };

        let serialized = serde_json::to_string(&record).unwrap();
        let parsed: AnchorSelectionRecord = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed.anchor_id, "anchor_a");
        assert_eq!(parsed.perspective.perspective_kind, "frame_type");
    }

    #[test]
    fn perspective_key_rejects_empty_fields() {
        assert!(PerspectiveKey::new("", "analysis").is_err());
        assert!(PerspectiveKey::new("frame_type", "").is_err());
    }

    #[test]
    fn graph_walk_spec_requires_positive_depth() {
        let spec = GraphWalkSpec {
            direction: TraversalDirection::Both,
            relation_types: None,
            max_depth: 0,
            current_only: true,
            include_facts: false,
        };
        assert!(spec.validate().is_err());
    }
}
