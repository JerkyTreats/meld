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
use crate::world_state::graph::contracts::{
    BoundedTraversalRequest, TraversalCut, TraversalCutRequest, TraversalResult,
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

    /// Select owner revisions after catching up to the canonical Event authority.
    pub fn cut(&self, request: &TraversalCutRequest) -> Result<TraversalCut, StorageError> {
        self.with_traversal_query(|query| query.cut(request))
    }

    /// Traverse only publications admitted by the supplied owner revision cut.
    pub fn traverse(
        &self,
        cut: &TraversalCut,
        request: &BoundedTraversalRequest,
    ) -> Result<TraversalResult, StorageError> {
        self.with_traversal_query(|query| query.traverse(cut, request))
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
