//! Catch-up-aware query facade for the graph runtime.
//!
//! `WorldModelQueries` is the runtime-safe read surface used by application code.
//! Each query first catches the graph projection up to the event ledger, then
//! delegates to `TraversalQuery`.
//!
//! Product composition supplies a shared, port-constructed graph runtime to
//! [`WorldModelQueries::new`].

use std::sync::Arc;

use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::contracts::{
    AnchorProvenanceRecord, AnchorSelectionRecord, GraphWalkResult, GraphWalkSpec,
    TraversalDirection,
};
use crate::world_state::graph::query::TraversalQuery;
use crate::world_state::graph::runtime::GraphRuntime;

/// Graph query facade that performs runtime catch-up before reads.
#[derive(Clone)]
pub struct WorldModelQueries {
    graph_runtime: Arc<GraphRuntime>,
}

impl WorldModelQueries {
    /// Create a query facade over a shared graph runtime.
    pub fn new(graph_runtime: Arc<GraphRuntime>) -> Self {
        Self { graph_runtime }
    }

    /// Read the current anchor for a logical anchor reference.
    pub fn current_anchor(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_anchor(anchor_ref))
    }

    /// Read all current anchors for one subject.
    pub fn current_anchors_for_subject(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_anchors_for_subject(subject))
    }

    /// Read anchor history for one logical anchor reference.
    pub fn anchor_history(
        &self,
        anchor_ref: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.anchor_history(anchor_ref))
    }

    /// Read compact provenance for one anchor.
    pub fn provenance_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError> {
        self.with_traversal_query(|query| query.provenance_for_anchor(anchor_id))
    }

    /// Read the current snapshot anchor for a source object.
    pub fn current_snapshot_for_source(
        &self,
        source: &DomainObjectRef,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_snapshot_for_source(source))
    }

    /// Read the current frame head anchor for a node and frame type.
    pub fn current_frame_head(
        &self,
        node: &DomainObjectRef,
        frame_type: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_frame_head(node, frame_type))
    }

    /// Read current frame head anchors for a node.
    pub fn current_frame_heads_for_node(
        &self,
        node: &DomainObjectRef,
    ) -> Result<Vec<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| query.current_frame_heads_for_node(node))
    }

    /// Count current frame head anchors for a frame type.
    pub fn current_frame_head_count_by_type(
        &self,
        frame_type: &str,
    ) -> Result<usize, StorageError> {
        self.with_traversal_query(|query| query.current_frame_head_count_by_type(frame_type))
    }

    /// Read the current artifact anchor for a task run and artifact type.
    pub fn current_artifact_for_task_run(
        &self,
        task_run: &DomainObjectRef,
        artifact_type_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        self.with_traversal_query(|query| {
            query.current_artifact_for_task_run(task_run, artifact_type_id)
        })
    }

    /// Read neighboring objects through graph relation indexes.
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

    /// Run a bounded graph walk from one object.
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
