use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRelation};

pub type AnchorId = String;
pub type TraversalFactId = String;
pub type ProvenanceId = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerspectiveKey {
    pub perspective_kind: String,
    pub perspective_id: String,
}

impl PerspectiveKey {
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

    pub fn index_key(&self) -> String {
        format!("{}::{}", self.perspective_kind, self.perspective_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSelectionRecord {
    pub anchor_id: AnchorId,
    pub anchor_ref: DomainObjectRef,
    pub subject: DomainObjectRef,
    pub perspective: PerspectiveKey,
    pub target: DomainObjectRef,
    pub source_fact_ids: Vec<String>,
    pub created_by_fact_id: String,
    pub selected_at_seq: u64,
    pub ended_at_seq: Option<u64>,
    pub ended_by_anchor_id: Option<AnchorId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalFactRecord {
    pub fact_id: TraversalFactId,
    pub source_spine_fact_id: String,
    pub seq: u64,
    pub event_type: String,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorProvenanceRecord {
    pub anchor_id: AnchorId,
    pub source_fact_ids: Vec<String>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphWalkSpec {
    pub direction: TraversalDirection,
    pub relation_types: Option<Vec<String>>,
    pub max_depth: usize,
    pub current_only: bool,
    pub include_facts: bool,
}

impl GraphWalkSpec {
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.max_depth == 0 {
            return Err(StorageError::InvalidPath(
                "graph walk max_depth must be at least 1".to_string(),
            ));
        }
        Ok(())
    }
}

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
