//! Read-only planner projection query facade.

use crate::belief::{BeliefKey, BeliefQuery, BeliefView, BranchScope};
use crate::events::{DomainObjectRef, LedgerCursor, LedgerIdentity};
use crate::planner::contracts::{
    PlannerAssemblyOutcome, PlannerAssemblyPolicy, PlannerAssemblyRequest,
    PlannerCurrentAssemblyRequest, PlannerCut, PlannerDecisionContext,
    PlannerFieldProjectionConfig, PlannerGraphScope, PlannerProjectionContext,
    PlannerProjectionError, PlannerProjectionInput, PlannerRefusal, PlannerRefusalGround,
    PlannerSourceKind, PlannerSourcePosition, WorldModelView,
};
use crate::world_state::graph::contracts::{
    traversal_cut_identity, traversal_result_identity, BoundedTraversalRequest,
    OwnerCurrentnessPolicy, OwnerPublicationScope, TraversalBounds, TraversalCut,
    TraversalCutStatus, TraversalDirection, TraversalOwnerRequirement, TraversalResult,
    TraversalTruncation,
};
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

    /// Assemble one complete current cut or return typed refusal grounds.
    pub fn assemble_current(
        &self,
        request: PlannerCurrentAssemblyRequest,
    ) -> PlannerAssemblyOutcome {
        let context_id = request.context.context_id.clone();
        let refuse = |detail: String| {
            PlannerAssemblyOutcome::Refused(PlannerRefusal {
                request_context_id: context_id.clone(),
                grounds: vec![PlannerRefusalGround::InvalidInput { detail }],
            })
        };
        let cut = match self.traversal_query.cut(&request.traversal_cut_request) {
            Ok(cut) => cut,
            Err(error) => return refuse(error.to_string()),
        };
        let traversal_result = match self
            .traversal_query
            .traverse(&cut, &request.traversal_request)
        {
            Ok(result) => result,
            Err(error) => return refuse(error.to_string()),
        };
        let current = match self
            .belief_query
            .current_revision_and_view(&request.belief_key)
        {
            Ok(current) => current,
            Err(error) => return refuse(error.to_string()),
        };
        let belief_view = current.as_ref().map(|(_, view)| view.clone());
        let field_config = belief_view
            .as_ref()
            .map(PlannerFieldProjectionConfig::from_belief_view)
            .unwrap_or_default();
        let anchors = match self
            .traversal_query
            .current_anchors_for_subject(&request.context.subject)
        {
            Ok(anchors) => anchors,
            Err(error) => return refuse(error.to_string()),
        };
        let mut source_positions = request.source_positions;
        source_positions.push(PlannerSourcePosition {
            kind: PlannerSourceKind::Graph,
            owner_id: "world_state.graph".to_string(),
            source_id: cut.cut_id.clone(),
            revision_id: cut.cut_id.clone(),
            content_hash: cut.cut_id.clone(),
            scope_id: request.context.scope_id.clone(),
            branch_id: request.context.branch_id.clone(),
            perspective_id: request.context.perspective_id.clone(),
            authority_scope_id: request.context.authority_scope_id.clone(),
            invalidated_by_revision_id: None,
        });
        if let Some((revision, _)) = current {
            source_positions.push(PlannerSourcePosition {
                kind: PlannerSourceKind::Belief,
                owner_id: "world_model.belief".to_string(),
                source_id: request.belief_key.index_key(),
                revision_id: revision.revision_id.clone(),
                content_hash: revision.revision_id,
                scope_id: request.context.scope_id.clone(),
                branch_id: request.context.branch_id.clone(),
                perspective_id: request.context.perspective_id.clone(),
                authority_scope_id: request.context.authority_scope_id.clone(),
                invalidated_by_revision_id: None,
            });
        }
        crate::planner::PlannerCut::assemble(PlannerAssemblyRequest {
            context: request.context.clone(),
            policy: request.policy,
            traversal_cut: cut,
            traversal_request: request.traversal_request,
            traversal_result,
            source_positions,
            view_input: PlannerProjectionInput {
                context: PlannerProjectionContext {
                    subject: request.context.subject,
                    perspective: request.belief_key.perspective,
                    branch_scope: request.belief_key.branch_scope,
                    projection_version: crate::planner::contracts::PLANNER_PROJECTION_VERSION
                        .to_string(),
                },
                belief_view,
                graph_scope: Some(PlannerGraphScope {
                    accessible: request.unanchored_belief || !anchors.is_empty(),
                    anchor_ids: anchors
                        .iter()
                        .map(|anchor| anchor.anchor_id.clone())
                        .collect(),
                    source_fact_ids: anchors
                        .iter()
                        .flat_map(|anchor| anchor.source_fact_ids.clone())
                        .collect(),
                }),
                field_config,
            },
        })
    }

    /// Legacy reader that returns only the view from one assembled Planner cut.
    ///
    /// New reasoning callers must retain the cut returned by
    /// [`Self::compatibility_cut_current_world_state`].
    pub fn project_current_world_state(
        &self,
        subject: &DomainObjectRef,
        dimension_id: &str,
        perspective: Option<PerspectiveKey>,
        branch_scope: Option<BranchScope>,
    ) -> Result<WorldModelView, PlannerProjectionError> {
        Ok(self
            .compatibility_cut_current_world_state(
                subject,
                dimension_id,
                perspective,
                branch_scope,
            )?
            .world_model_view)
    }

    /// Assemble the legacy current-subject projection beneath one canonical cut.
    ///
    /// This remains only for the Execution projection contract. Remove it when
    /// that contract consumes `PlannerCut` directly.
    pub fn compatibility_cut_current_world_state(
        &self,
        subject: &DomainObjectRef,
        dimension_id: &str,
        perspective: Option<PerspectiveKey>,
        branch_scope: Option<BranchScope>,
    ) -> Result<PlannerCut, PlannerProjectionError> {
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
        let belief_revision_id = belief_view
            .as_ref()
            .and_then(|view| view.current_revision_id.clone());
        self.assemble_compatibility_projection_cut(context, belief_view, belief_revision_id, false)
    }

    /// Legacy reader that returns only the exact-key view from a Planner cut.
    ///
    /// Unlike the subject-scan path, this reads the belief view addressed by
    /// the exact key, so the projection's revision and theory lineage are the
    /// identities the belief store holds for that key — never a same-subject
    /// neighbor selected by dimension match.
    pub fn project_world_state_for_key(
        &self,
        key: &BeliefKey,
    ) -> Result<WorldModelView, PlannerProjectionError> {
        Ok(self.compatibility_cut_for_key(key, false)?.world_model_view)
    }

    /// Assemble an exact-key legacy projection beneath one canonical cut.
    ///
    /// This remains for exact-key compatibility readers. Remove it after all
    /// such readers accept `PlannerCut` directly.
    pub fn compatibility_cut_for_key(
        &self,
        key: &BeliefKey,
        unanchored: bool,
    ) -> Result<PlannerCut, PlannerProjectionError> {
        let context = PlannerProjectionContext {
            subject: key.subject.clone(),
            perspective: key.perspective.clone(),
            branch_scope: key.branch_scope.clone(),
            projection_version: crate::planner::contracts::PLANNER_PROJECTION_VERSION.to_string(),
        };

        let current = self.belief_query.current_revision_and_view(key)?;
        let belief_revision_id = current
            .as_ref()
            .map(|(revision, _)| revision.revision_id.clone());
        let belief_view = current.map(|(_, view)| view);
        self.assemble_compatibility_projection_cut(
            context,
            belief_view,
            belief_revision_id,
            unanchored,
        )
    }

    /// Legacy reader for the cut-derived view of an unanchored belief key.
    ///
    /// The configured subject is the stewardship expression's maintained
    /// scope, validated at binding time, so scope accessibility is declared
    /// rather than derived from current anchors — the planner-side half of
    /// the family's anchor declaration. Anchors still contribute provenance
    /// when they exist.
    pub fn project_world_state_for_unanchored_key(
        &self,
        key: &BeliefKey,
    ) -> Result<WorldModelView, PlannerProjectionError> {
        Ok(self.compatibility_cut_for_key(key, true)?.world_model_view)
    }

    fn assemble_compatibility_projection_cut(
        &self,
        context: PlannerProjectionContext,
        belief_view: Option<BeliefView>,
        belief_revision_id: Option<String>,
        unanchored: bool,
    ) -> Result<PlannerCut, PlannerProjectionError> {
        let anchors = self
            .traversal_query
            .current_anchors_for_subject(&context.subject)?;
        let field_config = belief_view
            .as_ref()
            .map(PlannerFieldProjectionConfig::from_belief_view)
            .unwrap_or_default();
        let scope_id = format!("compatibility-projection::{}", context.subject.index_key());
        let branch_id = context.branch_scope.branch_id.clone();
        let perspective_id = context.perspective.perspective_id.clone();
        let scope = OwnerPublicationScope {
            scope_id: scope_id.clone(),
            branch_id: Some(branch_id.clone()),
            perspective_id: Some(perspective_id.clone()),
            valid_at: None,
        };
        let ledger_id: LedgerIdentity = "00000000-0000-0000-0000-000000000000"
            .parse()
            .expect("static compatibility ledger identity is valid");
        let position = anchors
            .iter()
            .map(|anchor| anchor.selected_at_seq)
            .max()
            .unwrap_or_default();
        let cursor = LedgerCursor {
            ledger_id,
            after_seq: position,
        };
        let mut traversal_cut = TraversalCut {
            cut_id: String::new(),
            owners: vec![TraversalOwnerRequirement {
                owner_id: "world_model.compatibility_projection".to_string(),
                scope: scope.clone(),
                required: false,
            }],
            receipts: Vec::new(),
            scope,
            currentness: OwnerCurrentnessPolicy::LatestComplete,
            event_position: cursor,
            graph_position: cursor,
            status: TraversalCutStatus::Complete,
            issues: Vec::new(),
        };
        traversal_cut.cut_id = traversal_cut_identity(&traversal_cut)?;
        let traversal_request = BoundedTraversalRequest {
            roots: vec![context.subject.clone()],
            direction: TraversalDirection::Incoming,
            relation_types: None,
            bounds: TraversalBounds {
                max_depth: 1,
                max_objects: 1,
                max_occurrences: 1,
                max_paths: 1,
            },
        };
        let traversal_result = TraversalResult {
            result_id: traversal_result_identity(&traversal_cut.cut_id, &traversal_request)?,
            cut_id: traversal_cut.cut_id.clone(),
            objects: Vec::new(),
            occurrences: Vec::new(),
            paths: Vec::new(),
            receipts: Vec::new(),
            frontier: Vec::new(),
            truncation: TraversalTruncation::default(),
        };
        let decision_context = PlannerDecisionContext {
            context_id: format!("compatibility-context::{}", context.subject.index_key()),
            agent_id: "legacy_execution_planning".to_string(),
            goal_id: format!("compatibility-goal::{}", context.subject.index_key()),
            subject: context.subject.clone(),
            scope_id,
            branch_id: branch_id.clone(),
            perspective_id: perspective_id.clone(),
            authority_scope_id: "legacy_execution_projection".to_string(),
            activation_generation: "compatibility-v1".to_string(),
        };
        let graph_revision_id = traversal_cut.cut_id.clone();
        let mut required_sources = vec![PlannerSourceKind::Graph];
        let mut source_positions = vec![PlannerSourcePosition {
            kind: PlannerSourceKind::Graph,
            owner_id: "world_state.graph".to_string(),
            source_id: graph_revision_id.clone(),
            revision_id: graph_revision_id.clone(),
            content_hash: graph_revision_id,
            scope_id: decision_context.scope_id.clone(),
            branch_id: branch_id.clone(),
            perspective_id: perspective_id.clone(),
            authority_scope_id: decision_context.authority_scope_id.clone(),
            invalidated_by_revision_id: None,
        }];
        if let Some(revision_id) = belief_revision_id {
            required_sources.push(PlannerSourceKind::Belief);
            source_positions.push(PlannerSourcePosition {
                kind: PlannerSourceKind::Belief,
                owner_id: "world_model.belief".to_string(),
                source_id: revision_id.clone(),
                revision_id: revision_id.clone(),
                content_hash: revision_id,
                scope_id: decision_context.scope_id.clone(),
                branch_id: branch_id.clone(),
                perspective_id: perspective_id.clone(),
                authority_scope_id: decision_context.authority_scope_id.clone(),
                invalidated_by_revision_id: None,
            });
        }
        let outcome = PlannerCut::assemble(PlannerAssemblyRequest {
            context: decision_context,
            policy: PlannerAssemblyPolicy {
                policy_revision_id: "legacy-execution-projection-v1".to_string(),
                required_sources,
                explicitly_not_required: vec![
                    PlannerSourceKind::Causation,
                    PlannerSourceKind::Regime,
                ],
            },
            traversal_cut,
            traversal_request,
            traversal_result,
            source_positions,
            view_input: PlannerProjectionInput {
                context,
                belief_view,
                graph_scope: Some(PlannerGraphScope {
                    accessible: unanchored || !anchors.is_empty(),
                    anchor_ids: anchors
                        .iter()
                        .map(|anchor| anchor.anchor_id.clone())
                        .collect(),
                    source_fact_ids: anchors
                        .iter()
                        .flat_map(|anchor| anchor.source_fact_ids.clone())
                        .collect(),
                }),
                field_config,
            },
        });
        match outcome {
            PlannerAssemblyOutcome::Complete(cut) => Ok(*cut),
            PlannerAssemblyOutcome::Refused(refusal) => {
                Err(crate::error::StorageError::InvalidPath(format!(
                    "compatibility projection cut was refused: {:?}",
                    refusal.grounds
                ))
                .into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::BeliefStore;
    use crate::world_state::graph::store::TraversalStore;

    #[test]
    fn legacy_projection_is_the_view_inside_a_valid_planner_cut() {
        let temp = tempfile::tempdir().unwrap();
        let belief = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        let graph = TraversalStore::new(sled::open(temp.path().join("graph")).unwrap()).unwrap();
        let query = PlannerQuery::new(BeliefQuery::new(&belief), TraversalQuery::new(&graph));
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();

        let cut = query
            .compatibility_cut_current_world_state(&subject, "docs_freshness", None, None)
            .unwrap();
        let view = query
            .project_current_world_state(&subject, "docs_freshness", None, None)
            .unwrap();

        cut.validate().unwrap();
        assert_eq!(view, cut.world_model_view);
    }
}
