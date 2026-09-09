//! Read-only planner projection query facade.

use crate::belief::BeliefQuery;
use crate::planner::contracts::{
    PlannerAssemblyOutcome, PlannerAssemblyRequest, PlannerCurrentAssemblyRequest,
    PlannerFieldProjectionConfig, PlannerGraphScope, PlannerProjectionContext,
    PlannerProjectionInput, PlannerRefusal, PlannerRefusalGround, PlannerSourceKind,
    PlannerSourcePosition,
};
use crate::world_state::graph::TraversalQuery;

/// Planner-safe read facade over graph and belief query surfaces.
pub struct PlannerQuery<'a> {
    curation_query: Option<crate::curation::CurationQuery<'a>>,
    belief_query: BeliefQuery<'a>,
    traversal_query: TraversalQuery<'a>,
}

impl<'a> PlannerQuery<'a> {
    /// Create a planner query facade from lower-layer read facades.
    pub fn new(belief_query: BeliefQuery<'a>, traversal_query: TraversalQuery<'a>) -> Self {
        Self {
            curation_query: None,
            belief_query,
            traversal_query,
        }
    }

    pub fn with_curation(mut self, query: crate::curation::CurationQuery<'a>) -> Self {
        self.curation_query = Some(query);
        self
    }

    /// Assemble one complete current cut or return typed refusal grounds.
    pub fn assemble_current(
        &self,
        request: PlannerCurrentAssemblyRequest,
    ) -> PlannerAssemblyOutcome {
        let observation_subject = request.context.observation_subject().clone();
        let context_id = request.context.context_id.clone();
        let refuse = |detail: String| {
            PlannerAssemblyOutcome::Refused(PlannerRefusal {
                request_context_id: context_id.clone(),
                grounds: vec![PlannerRefusalGround::InvalidInput { detail }],
            })
        };
        if request.belief_key.subject != observation_subject
            || request.belief_key.perspective.perspective_id != request.context.perspective_id
            || request.belief_key.branch_scope.branch_id != request.context.branch_id
        {
            return refuse(
                "Planner observation key differs from the selected judgment scope".into(),
            );
        }
        let mut dimensions =
            std::collections::BTreeSet::from([request.belief_key.dimension_id.clone()]);
        for selected in &request.additional_beliefs {
            if selected.key.validate().is_err()
                || selected.key.subject != observation_subject
                || selected.key.perspective != request.belief_key.perspective
                || selected.key.branch_scope != request.belief_key.branch_scope
                || !dimensions.insert(selected.key.dimension_id.clone())
            {
                return refuse("additional Belief selection differs from the judgment scope or repeats a dimension".into());
            }
        }
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
        for required in &request.required_graph_evidence {
            if !traversal_result.objects.iter().any(|object| {
                object.object_ref == required.object_ref
                    && object.state
                        == crate::world_state::graph::contracts::OwnerPublicationState::Observed
                    && required
                        .qualifications
                        .iter()
                        .all(|(key, value)| object.qualifications.get(key) == Some(value))
            }) {
                return refuse(format!(
                    "required owner evidence is not ready: {}",
                    required.object_ref.index_key()
                ));
            }
        }
        let mut current = match self
            .belief_query
            .current_revision_and_view(&request.belief_key)
        {
            Ok(current) => current,
            Err(error) => return refuse(error.to_string()),
        };
        let mut unassessed_belief = None;
        if let Some(question) = &request.policy.acquisition_question {
            if question.key != request.belief_key {
                return refuse("acquisition policy names another primary Belief question".into());
            }
            match self
                .belief_query
                .question_state(&question.key, &question.family)
            {
                Ok(crate::belief::BeliefQuestionState::NoCommittedRevision(answer)) => {
                    current = None;
                    unassessed_belief = Some(*answer);
                }
                Ok(crate::belief::BeliefQuestionState::Committed(answer)) => {
                    current = Some(*answer)
                }
                Ok(crate::belief::BeliefQuestionState::AssessmentPending) => {
                    return refuse("primary Belief question has pending assessment".into());
                }
                Ok(crate::belief::BeliefQuestionState::StaleRevision { revision_id }) => {
                    return refuse(format!(
                        "primary Belief question has a stale revision: {revision_id}"
                    ));
                }
                Err(error) => return refuse(error.to_string()),
            }
        }
        if current.is_none()
            && unassessed_belief.is_none()
            && request
                .policy
                .required_sources
                .contains(&PlannerSourceKind::Belief)
        {
            return refuse("primary Belief question has no committed revision".into());
        }
        let mut pending_derived_evidence = None;
        if let Some(required) = &request.required_derived_evidence {
            let Some(curation) = &self.curation_query else {
                return refuse("derived evidence requires its native Curation source".into());
            };
            let basis =
                match curation.current_evidence_basis(&required.curation_rule, &cut) {
                    Ok(Some(basis)) => Some(basis),
                    Ok(None) if unassessed_belief.is_some() => None,
                    Ok(None)
                        if request
                            .policy
                            .acquisition_question
                            .as_ref()
                            .is_some_and(|question| question.family == required.belief_family) =>
                    {
                        // A required observation is a prerequisite for settled
                        // knowledge, not for selecting the observation itself.
                        pending_derived_evidence = Some(required.clone());
                        None
                    }
                    Ok(None) => return refuse(
                        "Curation evidence does not match the selected source revision and rule"
                            .into(),
                    ),
                    Err(error) => return refuse(error.to_string()),
                };
            if let Some(basis) = basis {
                let Some((revision, _)) = &current else {
                    return refuse(
                        "current source judgment has no returned Belief revision".into(),
                    );
                };
                match self.belief_query.supports_current_curation(&revision.revision_id, &basis, &required.belief_family, &required.outcome_mappings) {
                Ok(true) => {},
                Ok(false) => return refuse("Belief has not consumed the selected current Curation evidence under the installed interpretation".into()),
                Err(error) => return refuse(error.to_string()),
            }
            }
        }
        let mut additional_beliefs = Vec::new();
        for selection in &request.additional_beliefs {
            let view = match self.belief_query.current_revision_and_view(&selection.key) {
                Ok(Some((_, view))) => Some(view),
                Ok(None) => None,
                Err(error) => return refuse(error.to_string()),
            };
            additional_beliefs.push(crate::planner::PlannerSelectedBeliefView {
                selection: selection.clone(),
                view,
            });
        }
        let belief_view = current.as_ref().map(|(_, view)| view.clone());
        let field_config = belief_view
            .as_ref()
            .map(PlannerFieldProjectionConfig::from_belief_view)
            .unwrap_or_default();
        let accessible = traversal_result.objects.iter().any(|object| {
            object.object_ref == observation_subject
                && object.state
                    == crate::world_state::graph::contracts::OwnerPublicationState::Observed
        });

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
        for view in belief_view.iter().chain(
            additional_beliefs
                .iter()
                .filter_map(|selected| selected.view.as_ref()),
        ) {
            let Some(revision_id) = &view.current_revision_id else {
                return refuse("Belief view has no committed revision".into());
            };
            source_positions.push(PlannerSourcePosition {
                kind: PlannerSourceKind::Belief,
                owner_id: "world_model.belief".into(),
                source_id: view.key.index_key(),
                revision_id: revision_id.clone(),
                content_hash: revision_id.clone(),
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
                unassessed_belief,
                pending_derived_evidence,
                additional_beliefs,
                context: PlannerProjectionContext {
                    subject: observation_subject.clone(),
                    perspective: request.belief_key.perspective,
                    branch_scope: request.belief_key.branch_scope,
                    projection_version: crate::planner::contracts::PLANNER_PROJECTION_VERSION
                        .to_string(),
                },
                belief_view,
                graph_scope: Some(PlannerGraphScope { accessible }),
                field_config,
            },
        })
    }
}
