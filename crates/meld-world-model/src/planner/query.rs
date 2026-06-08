//! Read-only planner projection query facade.

use crate::belief::{BeliefQuery, BranchScope};
use crate::events::DomainObjectRef;
use crate::planner::contracts::{
    PlannerFieldProjectionConfig, PlannerGraphScope, PlannerProjectionContext,
    PlannerProjectionError, PlannerProjectionInput, PlannerProjectionOutput,
};
use crate::planner::projection::project_world_state;
use crate::world_state::graph::{PerspectiveKey, TraversalQuery};

/// Planner-safe read facade over graph and belief query surfaces.
pub struct PlannerQuery<'a> {
    belief_query: BeliefQuery<'a>,
    traversal_query: TraversalQuery<'a>,
}

impl<'a> PlannerQuery<'a> {
    /// Create a planner query facade from lower-layer read facades.
    pub fn new(belief_query: BeliefQuery<'a>, traversal_query: TraversalQuery<'a>) -> Self {
        Self {
            belief_query,
            traversal_query,
        }
    }

    /// Project the current first-slice world state for one subject.
    pub fn project_current_world_state(
        &self,
        subject: &DomainObjectRef,
        dimension_id: &str,
        perspective: Option<PerspectiveKey>,
        branch_scope: Option<BranchScope>,
    ) -> Result<PlannerProjectionOutput, PlannerProjectionError> {
        let perspective = perspective
            .unwrap_or_else(|| PerspectiveKey::new("default", "default").expect("valid default"));
        let branch_scope = branch_scope.unwrap_or_else(BranchScope::main);
        let context = PlannerProjectionContext {
            subject: subject.clone(),
            perspective: perspective.clone(),
            branch_scope: branch_scope.clone(),
            projection_version: crate::planner::contracts::PLANNER_PROJECTION_VERSION.to_string(),
        };

        let belief_view = self
            .belief_query
            .current_views_for_subject(subject, &perspective)?
            .into_iter()
            .find(|view| {
                view.key.dimension_id == dimension_id
                    && view.key.perspective == perspective
                    && view.key.branch_scope == branch_scope
            });
        let field_config = belief_view
            .as_ref()
            .map(PlannerFieldProjectionConfig::from_belief_view)
            .unwrap_or_default();

        let anchors = self.traversal_query.current_anchors_for_subject(subject)?;
        let graph_scope = PlannerGraphScope {
            accessible: !anchors.is_empty(),
            anchor_ids: anchors
                .iter()
                .map(|anchor| anchor.anchor_id.clone())
                .collect(),
            source_fact_ids: anchors
                .iter()
                .flat_map(|anchor| anchor.source_fact_ids.clone())
                .collect(),
        };

        project_world_state(PlannerProjectionInput {
            context,
            belief_view,
            graph_scope: Some(graph_scope),
            field_config,
        })
    }
}
