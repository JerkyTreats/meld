use crate::error::StorageError;
use crate::telemetry::DomainObjectRef;
use crate::world_state::traversal::contracts::{
    AnchorProvenanceRecord, AnchorSelectionRecord, GraphWalkResult, GraphWalkSpec,
    TraversalDirection, TraversalFactRecord,
};
use crate::world_state::traversal::store::TraversalStore;

pub struct TraversalQuery<'a> {
    store: &'a TraversalStore,
}

impl<'a> TraversalQuery<'a> {
    pub fn new(store: &'a TraversalStore) -> Self {
        Self { store }
    }

    pub fn current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.store.current_anchor(anchor_ref)
    }

    pub fn current_anchor_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.store
            .current_anchor_for_subject(subject, perspective_kind, perspective_id)
    }

    pub fn anchor_history(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.store.anchor_history(anchor_ref)
    }

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

    pub fn walk(
        &self,
        start: &DomainObjectRef,
        spec: &GraphWalkSpec,
    ) -> Result<GraphWalkResult, StorageError> {
        self.store.walk(start, spec)
    }

    pub fn facts_for_object(
        &self,
        object: &DomainObjectRef,
        after_seq: u64,
    ) -> Result<Vec<TraversalFactRecord>, StorageError> {
        self.store.facts_for_object(object, after_seq)
    }

    pub fn provenance_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError> {
        self.store.anchor_provenance(anchor_id)
    }

    pub fn current_snapshot_for_source(
        &self,
        source: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(source, "snapshot", "current")
    }

    pub fn current_frame_head(
        &self,
        node: &DomainObjectRef,
        frame_type: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(node, "frame_type", frame_type)
    }

    pub fn current_artifact_for_task_run(
        &self,
        task_run: &DomainObjectRef,
        artifact_type_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.current_anchor_for_subject(task_run, "artifact_type", artifact_type_id)
    }
}
