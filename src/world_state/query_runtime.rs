use std::sync::Arc;

use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::contracts::{
    AnchorProvenanceRecord, AnchorSelectionRecord, GraphWalkResult, GraphWalkSpec,
    TraversalDirection,
};
use crate::world_state::graph::query::TraversalQuery;
use crate::world_state::graph::runtime::GraphRuntime;

#[derive(Clone)]
pub struct WorldModelQueries {
    graph_runtime: Arc<GraphRuntime>,
}

impl WorldModelQueries {
    pub fn new(graph_runtime: Arc<GraphRuntime>) -> Self {
        Self { graph_runtime }
    }

    pub fn current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_anchor(anchor_ref))
    }

    pub fn current_anchors_for_subject(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_anchors_for_subject(subject))
    }

    pub fn anchor_history(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.anchor_history(anchor_ref))
    }

    pub fn provenance_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError> {
        self.with_traversal_query(|query| query.provenance_for_anchor(anchor_id))
    }

    pub fn current_snapshot_for_source(
        &self,
        source: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_snapshot_for_source(source))
    }

    pub fn current_frame_head(
        &self,
        node: &DomainObjectRef,
        frame_type: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_frame_head(node, frame_type))
    }

    pub fn current_frame_heads_for_node(
        &self,
        node: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_frame_heads_for_node(node))
    }

    pub fn current_frame_head_count_by_type(
        &self,
        frame_type: &str,
    ) -> Result<usize, StorageError> {
        self.with_traversal_query(|query| query.current_frame_head_count_by_type(frame_type))
    }

    pub fn current_artifact_for_task_run(
        &self,
        task_run: &DomainObjectRef,
        artifact_type_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| {
            query.current_artifact_for_task_run(task_run, artifact_type_id)
        })
    }

    pub fn neighbors(
        &self,
        object: &DomainObjectRef,
        direction: TraversalDirection,
        relation_types: Option<&[String]>,
        current_only: bool,
    ) -> Result<Vec<DomainObjectRef>, StorageError> {
        self.with_traversal_query(|query| {
            query.neighbors(object, direction, relation_types, current_only)
        })
    }

    pub fn walk(
        &self,
        start: &DomainObjectRef,
        spec: &GraphWalkSpec,
    ) -> Result<GraphWalkResult, StorageError> {
        self.with_traversal_query(|query| query.walk(start, spec))
    }

    fn with_traversal_query<T>(
        &self,
        f: impl FnOnce(TraversalQuery<'_>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        self.graph_runtime.catch_up()?;
        let traversal = self.graph_runtime.traversal_store();
        let query = TraversalQuery::new(traversal.as_ref());
        f(query)
    }
}
