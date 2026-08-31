//! Pure planner projection from public world model views.

use meld_lang::{
    condition::Condition,
    proposition::Proposition,
    term::{Literal, Term},
    WorldState,
};

use crate::planner::contracts::{
    sort_dedup, PlannerAssemblyOutcome, PlannerAssemblyRequest, PlannerCut, PlannerHydrationRefs,
    PlannerProjectionError, PlannerProjectionInput, PlannerProjectionWarning, PlannerRefusal,
    PlannerRefusalGround, PlannerSourceKind, PlannerSourceRef, WorldModelView,
};
use crate::world_state::graph::contracts::TraversalCutStatus;

impl PlannerCut {
    /// Assemble one complete immutable cut or return every blocking refusal.
    pub fn assemble(mut request: PlannerAssemblyRequest) -> PlannerAssemblyOutcome {
        request.policy.required_sources.sort();
        request.policy.required_sources.dedup();
        request.policy.explicitly_not_required.sort();
        request.policy.explicitly_not_required.dedup();
        request.source_positions.sort();
        request.source_positions.dedup();
        let mut grounds = validate_assembly_request(&request);
        let view = project_world_state(request.view_input.clone()).map_err(|error| {
            grounds.push(PlannerRefusalGround::InvalidInput {
                detail: error.to_string(),
            });
        });
        grounds.sort();
        grounds.dedup();
        if !grounds.is_empty() {
            return PlannerAssemblyOutcome::Refused(PlannerRefusal {
                request_context_id: request.context.context_id,
                grounds,
            });
        }
        let world_model_view = view.expect("validated Planner view");
        let identity = (
            &request.context,
            &request.policy,
            &request.traversal_cut,
            &request.traversal_request,
            &request.traversal_result,
            &request.source_positions,
            &world_model_view,
        );
        let bytes = serde_json::to_vec(&identity).expect("Planner cut identity is serializable");
        PlannerAssemblyOutcome::Complete(Box::new(Self {
            cut_id: format!("planner-cut-v1::{}", blake3::hash(&bytes).to_hex()),
            context: request.context,
            policy: request.policy,
            traversal_cut: request.traversal_cut,
            traversal_request: request.traversal_request,
            traversal_result: request.traversal_result,
            source_positions: request.source_positions,
            world_model_view,
        }))
    }

    /// Validate the stored body against its content identity.
    pub fn validate(&self) -> Result<(), PlannerProjectionError> {
        self.traversal_cut.validate_identity()?;
        if self.traversal_result.cut_id != self.traversal_cut.cut_id {
            return Err(PlannerProjectionError::Storage(
                crate::error::StorageError::InvalidPath(
                    "Planner cut traversal result names another cut".to_string(),
                ),
            ));
        }
        let identity = (
            &self.context,
            &self.policy,
            &self.traversal_cut,
            &self.traversal_request,
            &self.traversal_result,
            &self.source_positions,
            &self.world_model_view,
        );
        let bytes = serde_json::to_vec(&identity)?;
        let expected = format!("planner-cut-v1::{}", blake3::hash(&bytes).to_hex());
        if self.cut_id != expected {
            return Err(PlannerProjectionError::Storage(
                crate::error::StorageError::InvalidPath(
                    "Planner cut identity is not canonical".to_string(),
                ),
            ));
        }
        Ok(())
    }
}

fn validate_assembly_request(request: &PlannerAssemblyRequest) -> Vec<PlannerRefusalGround> {
    let mut grounds = Vec::new();
    if request.policy.policy_revision_id.trim().is_empty()
        || request.context.context_id.trim().is_empty()
        || request.context.agent_id.trim().is_empty()
        || request.context.goal_id.trim().is_empty()
        || request.context.activation_generation.trim().is_empty()
    {
        grounds.push(PlannerRefusalGround::InvalidInput {
            detail: "Planner policy, context, Agent, goal, and generation must be exact"
                .to_string(),
        });
    }
    for kind in &request.policy.required_sources {
        let supplied = request
            .source_positions
            .iter()
            .filter(|source| source.kind == *kind)
            .collect::<Vec<_>>();
        if supplied.is_empty() {
            grounds.push(PlannerRefusalGround::Missing { kind: *kind });
        } else if supplied.len() > 1 {
            grounds.push(PlannerRefusalGround::Duplicate { kind: *kind });
        }
    }
    for source in &request.source_positions {
        let kind = source.kind;
        if !request.policy.required_sources.contains(&kind) {
            grounds.push(PlannerRefusalGround::Unexpected { kind });
        }
        if source.invalidated_by_revision_id.is_some() {
            grounds.push(PlannerRefusalGround::Invalidated {
                kind,
                revision_id: source.revision_id.clone(),
            });
        }
        if source.scope_id != request.context.scope_id {
            grounds.push(PlannerRefusalGround::ScopeMismatch { kind });
        }
        if source.branch_id != request.context.branch_id {
            grounds.push(PlannerRefusalGround::BranchMismatch { kind });
        }
        if source.perspective_id != request.context.perspective_id {
            grounds.push(PlannerRefusalGround::PerspectiveMismatch { kind });
        }
        if source.authority_scope_id != request.context.authority_scope_id {
            grounds.push(PlannerRefusalGround::Unauthorized { kind });
        }
        if source.owner_id.trim().is_empty()
            || source.source_id.trim().is_empty()
            || source.revision_id.trim().is_empty()
            || source.content_hash.trim().is_empty()
        {
            grounds.push(PlannerRefusalGround::InvalidInput {
                detail: format!("{kind:?} source position is incomplete"),
            });
        }
    }
    for kind in &request.policy.explicitly_not_required {
        if request.policy.required_sources.contains(kind) {
            grounds.push(PlannerRefusalGround::InvalidInput {
                detail: format!("{kind:?} cannot be both required and not required"),
            });
        }
    }
    for kind in [PlannerSourceKind::Causation, PlannerSourceKind::Regime] {
        if !request.policy.required_sources.contains(&kind)
            && !request.policy.explicitly_not_required.contains(&kind)
        {
            grounds.push(PlannerRefusalGround::Missing { kind });
        }
    }
    if request.traversal_cut.status != TraversalCutStatus::Complete
        || !request.traversal_cut.issues.is_empty()
        || request.traversal_cut.validate_identity().is_err()
    {
        grounds.push(PlannerRefusalGround::IncompleteTraversal);
    }
    if request.traversal_result.cut_id != request.traversal_cut.cut_id {
        grounds.push(PlannerRefusalGround::TraversalResultMismatch);
    }
    grounds
}

/// Project public graph and belief views into a ground world state.
pub fn project_world_state(
    input: PlannerProjectionInput,
) -> Result<WorldModelView, PlannerProjectionError> {
    input.field_config.validate()?;

    let mut propositions = Vec::new();
    let mut source_refs = Vec::new();
    let mut hydration_refs = PlannerHydrationRefs::default();
    let mut warnings = Vec::new();

    if let Some(view) = input.belief_view.as_ref() {
        validate_view_context(&input, view)?;
        project_belief_view(
            &input.context.subject,
            view,
            &input.field_config,
            &mut propositions,
            &mut source_refs,
            &mut hydration_refs,
        )?;
    } else {
        warnings.push(PlannerProjectionWarning::MissingBelief {
            subject: input.context.subject.clone(),
        });
    }

    if let Some(scope) = input.graph_scope.as_ref() {
        if scope.accessible && input.field_config.emit_accessible {
            propositions.push(Proposition::Accessible {
                scope: Term::Object(input.context.subject.clone()),
            });
            source_refs.push(PlannerSourceRef::ProjectionRule {
                rule_id: "graph.accessible".to_string(),
            });
        }
        for anchor_id in &scope.anchor_ids {
            source_refs.push(PlannerSourceRef::GraphAnchor {
                anchor_id: anchor_id.clone(),
            });
            hydration_refs.graph_anchor_ids.push(anchor_id.clone());
        }
        for source_fact_id in &scope.source_fact_ids {
            source_refs.push(PlannerSourceRef::SourceFact {
                source_fact_id: source_fact_id.clone(),
            });
            hydration_refs.source_fact_ids.push(source_fact_id.clone());
        }
    } else {
        warnings.push(PlannerProjectionWarning::MissingGraphScope {
            subject: input.context.subject.clone(),
        });
    }

    sort_propositions(&mut propositions)?;
    propositions.dedup();
    sort_dedup(&mut source_refs);
    hydration_refs.sort_and_dedup();
    warnings.sort();
    warnings.dedup();

    // Theory lineage rides the belief view unchanged; projection never
    // manufactures or rewrites revision identity.
    let theory_revision = input
        .belief_view
        .as_ref()
        .and_then(|view| view.theory_revision.clone());

    Ok(WorldModelView {
        world_state: WorldState::new(propositions)?,
        projection_version: input.context.projection_version,
        source_refs,
        hydration_refs,
        warnings,
        theory_revision,
    })
}

fn validate_view_context(
    input: &PlannerProjectionInput,
    view: &crate::belief::BeliefView,
) -> Result<(), PlannerProjectionError> {
    if view.key.subject != input.context.subject {
        return Err(PlannerProjectionError::SubjectMismatch {
            expected: Box::new(input.context.subject.clone()),
            actual: Box::new(view.key.subject.clone()),
        });
    }
    if view.key.perspective != input.context.perspective {
        return Err(PlannerProjectionError::PerspectiveMismatch {
            expected: Box::new(input.context.perspective.clone()),
            actual: Box::new(view.key.perspective.clone()),
        });
    }
    if view.key.branch_scope != input.context.branch_scope {
        return Err(PlannerProjectionError::BranchScopeMismatch {
            expected: Box::new(input.context.branch_scope.clone()),
            actual: Box::new(view.key.branch_scope.clone()),
        });
    }
    Ok(())
}

fn project_belief_view(
    subject: &crate::events::DomainObjectRef,
    view: &crate::belief::BeliefView,
    field_config: &crate::planner::contracts::PlannerFieldProjectionConfig,
    propositions: &mut Vec<Proposition>,
    source_refs: &mut Vec<PlannerSourceRef>,
    hydration_refs: &mut PlannerHydrationRefs,
) -> Result<(), PlannerProjectionError> {
    let confidence = view.planner_projection.confidence;
    if !confidence.is_finite() {
        return Err(PlannerProjectionError::InvalidConfidence {
            dimension_id: view.key.dimension_id.clone(),
            confidence,
        });
    }

    let subject = Term::Object(subject.clone());
    let confidence_dimension = view.key.dimension_id.clone();
    let stale_dimension = field_config.stale_dimension(&view.key.dimension_id)?;
    let observation_dimension =
        field_config.observation_needed_dimension(&view.key.dimension_id)?;
    let observation_open = view
        .observation
        .as_ref()
        .is_some_and(|observation| observation.open);

    propositions.push(Proposition::Holds {
        subject: subject.clone(),
        dimension: Term::Dimension(confidence_dimension),
        condition: Condition::Equals(Term::Literal(Literal::Number(confidence))),
    });
    propositions.push(Proposition::Holds {
        subject: subject.clone(),
        dimension: Term::Dimension(stale_dimension),
        condition: Condition::Equals(Term::Literal(Literal::Bool(view.freshness.stale))),
    });
    propositions.push(Proposition::Holds {
        subject,
        dimension: Term::Dimension(observation_dimension),
        condition: Condition::Equals(Term::Literal(Literal::Bool(observation_open))),
    });

    source_refs.push(PlannerSourceRef::ProjectionRule {
        rule_id: "belief.confidence".to_string(),
    });
    source_refs.push(PlannerSourceRef::ProjectionRule {
        rule_id: "belief.stale".to_string(),
    });
    source_refs.push(PlannerSourceRef::ProjectionRule {
        rule_id: "belief.observation_needed".to_string(),
    });

    if let Some(revision_id) = &view.current_revision_id {
        source_refs.push(PlannerSourceRef::BeliefRevision {
            revision_id: revision_id.clone(),
        });
        hydration_refs.revision_ids.push(revision_id.clone());
    }
    for evidence_id in &view.hydration.evidence_ids {
        source_refs.push(PlannerSourceRef::Evidence {
            evidence_id: evidence_id.clone(),
        });
        hydration_refs.evidence_ids.push(evidence_id.clone());
    }
    for source_fact_id in &view.hydration.source_fact_ids {
        source_refs.push(PlannerSourceRef::SourceFact {
            source_fact_id: source_fact_id.clone(),
        });
        hydration_refs.source_fact_ids.push(source_fact_id.clone());
    }
    for anchor_id in &view.hydration.graph_anchor_ids {
        source_refs.push(PlannerSourceRef::GraphAnchor {
            anchor_id: anchor_id.clone(),
        });
        hydration_refs.graph_anchor_ids.push(anchor_id.clone());
    }

    Ok(())
}

fn sort_propositions(propositions: &mut Vec<Proposition>) -> Result<(), PlannerProjectionError> {
    let mut keyed = propositions
        .drain(..)
        .map(|proposition| serde_json::to_string(&proposition).map(|key| (key, proposition)))
        .collect::<Result<Vec<_>, _>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    propositions.extend(keyed.into_iter().map(|(_, proposition)| proposition));
    Ok(())
}

#[cfg(test)]
mod cut_tests {
    use meld_events::{DomainObjectRef, LedgerCursor, LedgerIdentity};

    use crate::belief::BranchScope;
    use crate::planner::*;
    use crate::world_state::graph::contracts::*;
    use crate::world_state::graph::PerspectiveKey;

    #[test]
    fn refuses_missing_source_and_undeclared_optional_domains() {
        let mut input = request();
        input.policy.explicitly_not_required.clear();
        input
            .source_positions
            .retain(|source| source.kind != PlannerSourceKind::Belief);
        let PlannerAssemblyOutcome::Refused(refusal) = PlannerCut::assemble(input) else {
            panic!("incomplete request must refuse")
        };
        for kind in [
            PlannerSourceKind::Belief,
            PlannerSourceKind::Causation,
            PlannerSourceKind::Regime,
        ] {
            assert!(refusal
                .grounds
                .contains(&PlannerRefusalGround::Missing { kind }));
        }
    }

    #[test]
    fn cut_identity_is_invariant_to_unordered_source_inputs() {
        let forward = request();
        let mut reverse = forward.clone();
        reverse.source_positions.reverse();
        reverse.policy.required_sources.reverse();
        reverse.policy.explicitly_not_required.reverse();
        let PlannerAssemblyOutcome::Complete(forward) = PlannerCut::assemble(forward) else {
            panic!("complete request must assemble")
        };
        let PlannerAssemblyOutcome::Complete(reverse) = PlannerCut::assemble(reverse) else {
            panic!("reordered request must assemble")
        };
        assert_eq!(forward, reverse);
    }

    fn request() -> PlannerAssemblyRequest {
        let subject = DomainObjectRef::new("workspace_fs", "node", "meld").unwrap();
        let context = PlannerDecisionContext {
            context_id: "context-v1".into(),
            agent_id: "agent-v1".into(),
            goal_id: "goal-v1".into(),
            subject: subject.clone(),
            scope_id: "meld".into(),
            branch_id: "main".into(),
            perspective_id: "default".into(),
            authority_scope_id: "authority-v1".into(),
            activation_generation: "activation-v1".into(),
        };
        let kinds = vec![
            PlannerSourceKind::Graph,
            PlannerSourceKind::Belief,
            PlannerSourceKind::Directive,
            PlannerSourceKind::MaintainedCondition,
            PlannerSourceKind::CapabilityCatalog,
            PlannerSourceKind::CurationCatalog,
            PlannerSourceKind::StrategyPolicy,
        ];
        let ledger_id = LedgerIdentity::new();
        let mut cut = TraversalCut {
            cut_id: String::new(),
            owners: vec![TraversalOwnerRequirement {
                owner_id: "workspace_fs".into(),
                scope: OwnerPublicationScope {
                    scope_id: "meld".into(),
                    branch_id: Some("main".into()),
                    perspective_id: Some("default".into()),
                    valid_at: None,
                },
                required: false,
            }],
            receipts: Vec::new(),
            scope: OwnerPublicationScope {
                scope_id: "meld".into(),
                branch_id: Some("main".into()),
                perspective_id: Some("default".into()),
                valid_at: None,
            },
            currentness: OwnerCurrentnessPolicy::LatestComplete,
            event_position: LedgerCursor {
                ledger_id,
                after_seq: 7,
            },
            graph_position: LedgerCursor {
                ledger_id,
                after_seq: 7,
            },
            status: TraversalCutStatus::Complete,
            issues: Vec::new(),
        };
        cut.cut_id = traversal_cut_identity(&cut).unwrap();
        let traversal = BoundedTraversalRequest {
            roots: vec![subject.clone()],
            direction: TraversalDirection::Incoming,
            relation_types: None,
            bounds: TraversalBounds {
                max_depth: 1,
                max_objects: 2,
                max_occurrences: 2,
                max_paths: 2,
            },
        };
        PlannerAssemblyRequest {
            context: context.clone(),
            policy: PlannerAssemblyPolicy {
                policy_revision_id: "policy-v1".into(),
                required_sources: kinds.clone(),
                explicitly_not_required: vec![
                    PlannerSourceKind::Causation,
                    PlannerSourceKind::Regime,
                ],
            },
            traversal_result: TraversalResult {
                result_id: traversal_result_identity(&cut.cut_id, &traversal).unwrap(),
                cut_id: cut.cut_id.clone(),
                objects: Vec::new(),
                occurrences: Vec::new(),
                paths: Vec::new(),
                receipts: Vec::new(),
                frontier: Vec::new(),
                truncation: TraversalTruncation::default(),
            },
            traversal_cut: cut,
            traversal_request: traversal,
            source_positions: kinds
                .into_iter()
                .map(|kind| PlannerSourcePosition {
                    kind,
                    owner_id: format!("{kind:?}"),
                    source_id: format!("{kind:?}-source"),
                    revision_id: format!("{kind:?}-revision"),
                    content_hash: format!("{kind:?}-hash"),
                    scope_id: context.scope_id.clone(),
                    branch_id: context.branch_id.clone(),
                    perspective_id: context.perspective_id.clone(),
                    authority_scope_id: context.authority_scope_id.clone(),
                    invalidated_by_revision_id: None,
                })
                .collect(),
            view_input: PlannerProjectionInput {
                context: PlannerProjectionContext {
                    subject,
                    perspective: PerspectiveKey::new("frame", "default").unwrap(),
                    branch_scope: BranchScope::main(),
                    projection_version: PLANNER_PROJECTION_VERSION.into(),
                },
                belief_view: None,
                graph_scope: Some(PlannerGraphScope {
                    accessible: true,
                    anchor_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                }),
                field_config: PlannerFieldProjectionConfig::default(),
            },
        }
    }
}
