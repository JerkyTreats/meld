//! Pure planner projection from public world model views.

use meld_lang::{
    condition::Condition,
    proposition::Proposition,
    term::{Literal, Term},
    WorldState,
};

use crate::planner::contracts::{
    sort_dedup, PlannerHydrationRefs, PlannerProjectionError, PlannerProjectionInput,
    PlannerProjectionOutput, PlannerProjectionWarning, PlannerSourceRef,
};

/// Project public graph and belief views into a ground world state.
pub fn project_world_state(
    input: PlannerProjectionInput,
) -> Result<PlannerProjectionOutput, PlannerProjectionError> {
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

    Ok(PlannerProjectionOutput {
        world_state: WorldState::new(propositions)?,
        projection_version: input.context.projection_version,
        source_refs,
        hydration_refs,
        warnings,
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
    if view.perspective != input.context.perspective {
        return Err(PlannerProjectionError::PerspectiveMismatch {
            expected: Box::new(input.context.perspective.clone()),
            actual: Box::new(view.perspective.clone()),
        });
    }
    if view.branch_scope != input.context.branch_scope {
        return Err(PlannerProjectionError::BranchScopeMismatch {
            expected: Box::new(input.context.branch_scope.clone()),
            actual: Box::new(view.branch_scope.clone()),
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
    if !view.confidence.is_finite() {
        return Err(PlannerProjectionError::InvalidConfidence {
            dimension_id: view.key.dimension_id.clone(),
            confidence: view.confidence,
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
        condition: Condition::Equals(Term::Literal(Literal::Number(view.confidence))),
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
