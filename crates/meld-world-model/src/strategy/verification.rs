//! Independent soundness verification for constructed Strategy candidates.

use std::collections::BTreeSet;

use meld_lang::{evaluate, EdgeKind, Effect, EvalResult, Proposition, StepKind};

use super::search::{candidate_evaluation, plan_family_identity, plan_revision_identity};
use super::{
    PlanMilestoneRequirement, PlanVerification, StrategyPlan, StrategyProblem,
    StrategyRejectionGround, StrategySuccessorPlan, StrategySuccessorRequest,
};

/// Verify candidate soundness without trusting the search traversal.
///
/// Verification re-resolves every operator against the supplied capability
/// vocabulary, rechecks current preconditions, artifact closure, settlement
/// contribution, evidence routing, and content identity. It performs no
/// repair and never relies on the candidate construction origin.
pub fn verify_plan(problem: &StrategyProblem, candidate: &StrategyPlan) -> PlanVerification {
    let mut grounds = Vec::new();
    if candidate.problem_id != problem.problem_id
        || candidate.goal_id != problem.goal.goal_id
        || candidate.planner_cut_id != problem.planner_cut.cut_id
        || candidate.plan_revision_id != plan_revision_identity(candidate)
    {
        grounds.push(StrategyRejectionGround::IdentityMismatch);
    }
    if !meld_lang::validate(&candidate.composition).valid {
        grounds.push(StrategyRejectionGround::InvalidComposition);
    }
    if candidate.tasks.len() != 1
        || candidate.tasks[0].composition != candidate.composition
        || candidate.tasks[0].capability_contract_ids != candidate.capability_contract_ids
        || candidate.conditions.is_empty()
        || candidate.explanation.trim().is_empty()
        || candidate.frozen_context_id != problem.planner_cut.context.context_id
    {
        grounds.push(StrategyRejectionGround::InvalidComposition);
    }
    if candidate.epistemic_operations.len() != problem.curation_operations.len().min(1) {
        grounds.push(StrategyRejectionGround::InvalidComposition);
    }
    if let Some(operation) = candidate.epistemic_operations.first() {
        let linked = candidate.dependencies.iter().any(|dependency| {
            dependency.producer_product_id == operation.product_id
                && dependency.consumer_product_id == candidate.tasks[0].task_id
                && dependency.required_milestone
                    == PlanMilestoneRequirement::CurationTerminal {
                        operation_id: operation.operation.operation_id.clone(),
                    }
        });
        if !linked || !problem.curation_operations.contains(&operation.operation) {
            grounds.push(StrategyRejectionGround::InvalidComposition);
        }
    }
    // The candidate carries an operational graph, but the problem remains
    // authoritative for which capability contracts that graph may name.
    let mut evidence_outcome_supported = false;
    let mut used_contracts = BTreeSet::new();
    for step in &candidate.composition.steps {
        let StepKind::Op(operator) = &step.kind else {
            grounds.push(StrategyRejectionGround::InvalidComposition);
            continue;
        };
        let Some(specific) = &operator.resolution.specific else {
            grounds.push(StrategyRejectionGround::InvalidComposition);
            continue;
        };
        let Some(capability) = problem
            .capabilities
            .iter()
            .find(|capability| capability.operator.resolution.specific.as_ref() == Some(specific))
        else {
            grounds.push(StrategyRejectionGround::InvalidComposition);
            continue;
        };
        if !candidate
            .capability_contract_ids
            .contains(&capability.contract_id)
        {
            grounds.push(StrategyRejectionGround::IdentityMismatch);
        }
        used_contracts.insert(capability.contract_id.clone());
        evidence_outcome_supported |=
            capability.outcome_contract_id == candidate.evidence_route.outcome_contract_id;
        for precondition in &operator.preconditions {
            match evaluate(
                &problem.planner_cut.world_model_view.world_state,
                precondition,
            ) {
                EvalResult::Satisfied => {}
                EvalResult::Unsatisfied { .. } => {
                    grounds.push(StrategyRejectionGround::UnsatisfiedPrecondition {
                        operator_id: operator.operator_id.clone(),
                    })
                }
                EvalResult::Indeterminate { .. } => {
                    grounds.push(StrategyRejectionGround::IndeterminatePrecondition {
                        operator_id: operator.operator_id.clone(),
                    })
                }
            }
        }
    }
    if !evidence_outcome_supported {
        grounds.push(StrategyRejectionGround::InvalidEvidenceRoute);
    }
    let declared_contracts: BTreeSet<_> =
        candidate.capability_contract_ids.iter().cloned().collect();
    if used_contracts != declared_contracts {
        grounds.push(StrategyRejectionGround::IdentityMismatch);
    }
    if !composition_contributes(&candidate.composition, &candidate.settlement_obligation) {
        grounds.push(StrategyRejectionGround::InvalidComposition);
    }
    // Required inputs must either be wired inside the candidate or already
    // exist in the planner projection used to construct it.
    for step in &candidate.composition.steps {
        let StepKind::Op(operator) = &step.kind else {
            continue;
        };
        for input in operator
            .resolution
            .requires_inputs
            .iter()
            .filter(|slot| slot.required)
        {
            let wired = candidate.composition.edges.iter().any(|edge| {
                edge.to == step.step_id
                    && matches!(
                        &edge.kind,
                        EdgeKind::DataFlow { artifact_type }
                            if artifact_type == &input.artifact_type
                    )
            });
            let existing =
                goal_scope(&problem.goal.target).is_some_and(|scope| {
                    problem.planner_cut.world_model_view.world_state.satisfies(
                        &Proposition::Exists {
                            scope,
                            artifact_type: input.artifact_type.clone(),
                        },
                    )
                });
            if !wired && !existing {
                grounds.push(StrategyRejectionGround::UnclosedArtifact {
                    artifact_type: format!("{:?}", input.artifact_type),
                });
            }
        }
    }
    let evaluation = candidate_evaluation(candidate);
    if candidate.evaluation != evaluation {
        grounds.push(StrategyRejectionGround::IdentityMismatch);
    }
    grounds.sort_by_key(|ground| format!("{ground:?}"));
    grounds.dedup();
    if grounds.is_empty() {
        PlanVerification::Valid { evaluation }
    } else {
        PlanVerification::Invalid { grounds }
    }
}

/// Verify a reconstructed Plan and its immutable predecessor history.
pub fn verify_successor_plan(
    request: &StrategySuccessorRequest,
    successor: &StrategySuccessorPlan,
) -> PlanVerification {
    let predecessor = request.predecessor_plan.as_ref();
    let standard = verify_plan(&request.search.problem, &successor.plan);
    if !matches!(standard, PlanVerification::Valid { .. }) {
        return standard;
    }
    if predecessor.plan_revision_id != plan_revision_identity(predecessor)
        || predecessor.plan_family_id != plan_family_identity(&request.search.problem)
        || predecessor.goal_id != request.search.problem.goal.goal_id
        || successor.plan.plan_family_id != predecessor.plan_family_id
        || successor.plan.predecessor_plan_revision_id.as_ref()
            != Some(&predecessor.plan_revision_id)
        || successor.completed_history != request.completed_history
    {
        return PlanVerification::Invalid {
            grounds: vec![StrategyRejectionGround::InvalidPredecessor],
        };
    }
    standard
}

fn composition_contributes(composition: &meld_lang::Composition, obligation: &Proposition) -> bool {
    composition.steps.iter().any(|step| match &step.kind {
        StepKind::Op(operator) => operator.effects.iter().any(
            |effect| matches!(effect, Effect::Assert(proposition) if proposition == obligation),
        ),
        StepKind::Goal(_) => false,
    })
}

fn goal_scope(goal: &Proposition) -> Option<meld_lang::Term> {
    match goal {
        Proposition::Holds { subject, .. } => Some(subject.clone()),
        Proposition::Exists { scope, .. } | Proposition::Accessible { scope } => {
            Some(scope.clone())
        }
        Proposition::Related { src, .. } => Some(src.clone()),
        Proposition::All(children) | Proposition::Any(children) => {
            children.first().and_then(goal_scope)
        }
        Proposition::Not(child) => goal_scope(child),
    }
}
