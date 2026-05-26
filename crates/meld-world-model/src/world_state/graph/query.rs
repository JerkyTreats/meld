//! Read-only graph traversal query facade.
//!
//! This module exposes current anchors, anchor history, provenance, object
//! facts, and bounded walks without exposing sled tree layout. Belief
//! normalization consumes this facade rather than reaching into graph storage.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::world_state::graph::store::TraversalStore;
//! use meld_world_model::TraversalQuery;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let store = TraversalStore::new(sled::open(temp.path()).unwrap()).unwrap();
//! let query = TraversalQuery::new(&store);
//! let object = meld_world_model::events::DomainObjectRef::new(
//!     "workspace_fs",
//!     "node",
//!     "node-a",
//! )
//! .unwrap();
//! assert!(query.current_anchors_for_subject(&object).unwrap().is_empty());
//! ```

use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::contracts::{
    AnchorProvenanceRecord, AnchorSelectionRecord, GraphWalkResult, GraphWalkSpec,
    TraversalDirection, TraversalFactRecord,
};
use crate::world_state::graph::store::TraversalStore;

/// Read facade over graph traversal storage.
pub struct TraversalQuery<'a> {
    store: &'a TraversalStore,
}

impl<'a> TraversalQuery<'a> {
    /// Create a traversal query facade over an existing store.
    pub fn new(store: &'a TraversalStore) -> Self {
        Self { store }
    }

    /// Read the current anchor for a logical anchor reference.
    pub fn current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.store.current_anchor(anchor_ref)
    }

    /// Read the current anchor for one subject and perspective.
    pub fn current_anchor_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.store
            .current_anchor_for_subject(subject, perspective_kind, perspective_id)
    }

    /// Read every current anchor for one subject.
    pub fn current_anchors_for_subject(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.store.current_anchors_for_subject(subject)
    }

    /// Read anchor history for one logical anchor reference.
    pub fn anchor_history(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.store.anchor_history(anchor_ref)
    }

    /// Read adjacent objects through relation indexes.
    pub fn neighbors(
        &self,
        object: &DomainObjectRef,
        direction: TraversalDirection,
        relation_types: Option<&[String]>,
        current_only: bool,
    ) -> Result<Vec<DomainObjectRef>, StorageError> {
        self.store
            .neighbors(object, direction, relation_types, current_only)
    }

    /// Run a bounded graph walk from one object.
    pub fn walk(
        &self,
        start: &DomainObjectRef,
        spec: &GraphWalkSpec,
    ) -> Result<GraphWalkResult, StorageError> {
        self.store.walk(start, spec)
    }

    /// Read graph-readable facts for one object after a sequence cursor.
    pub fn facts_for_object(
        &self,
        object: &DomainObjectRef,
        after_seq: u64,
    ) -> Result<Vec<TraversalFactRecord>, StorageError> {
        self.store.facts_for_object(object, after_seq)
    }

    /// Read compact provenance for one anchor.
    pub fn provenance_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError> {
        self.store.anchor_provenance(anchor_id)
    }

    /// Read an anchor record when callers need generic graph lifecycle state.
    pub fn supersession_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        Ok(self.store.get_anchor(anchor_id)?.filter(|anchor| {
            anchor.ended_at_seq.is_some()
                || anchor.ended_by_anchor_id.is_some()
                || anchor.ended_by_fact_id.is_some()
        }))
    }

    /// Read the current workspace snapshot anchor for a source object.
    pub fn current_snapshot_for_source(
        &self,
        source: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(source, "snapshot", "current")
    }

    /// Read the current frame head anchor for a node and frame type.
    pub fn current_frame_head(
        &self,
        node: &DomainObjectRef,
        frame_type: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(node, "frame_type", frame_type)
    }

    /// Read current frame head anchors for a node.
    pub fn current_frame_heads_for_node(
        &self,
        node: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        Ok(self
            .current_anchors_for_subject(node)?
            .into_iter()
            .filter(|anchor| anchor.perspective.perspective_kind == "frame_type")
            .collect())
    }

    /// Count current frame head anchors for a frame type.
    pub fn current_frame_head_count_by_type(
        &self,
        frame_type: &str,
    ) -> Result<usize, StorageError> {
        self.store
            .current_anchor_count_by_perspective("frame_type", frame_type)
    }

    /// Read the current artifact anchor for a task run and artifact type.
    pub fn current_artifact_for_task_run(
        &self,
        task_run: &DomainObjectRef,
        artifact_type_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(task_run, "artifact_type", artifact_type_id)
    }
}
