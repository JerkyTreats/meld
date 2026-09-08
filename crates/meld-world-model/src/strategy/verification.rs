//! Independent soundness verification for constructed Strategy candidates.

use std::collections::{BTreeMap, BTreeSet};

use meld_lang::{evaluate, EdgeKind, EvalResult, StepKind};

use super::search::{
    candidate_evaluation, composition_contributes, plan_family_identity, plan_revision_identity,
    terminal_outcome_contract,
};
use super::{
    PlanMilestoneRequirement, PlanVerification, StrategyPlan, StrategyProblem,
    StrategyRejectionGround, StrategySuccessorPlan, StrategySuccessorRequest,
};

/// Verify candidate soundness without trusting the search traversal.
///
/// Verification re-resolves every operator against the supplied capability
/// vocabulary, rechecks current preconditions, artifact closure, settlement
/// contribution, evidence routing, and content identity. It performs no
/// repair. Method applicability cannot replace canonical operator guards.
pub fn verify_plan(problem: &StrategyProblem, candidate: &StrategyPlan) -> PlanVerification {
    verify_with_history(problem, candidate, &[])
}

fn verify_with_history(
    problem: &StrategyProblem,
    candidate: &StrategyPlan,
    history: &[super::StrategyCompletedHistoryEntry],
) -> PlanVerification {
    let mut grounds = Vec::new();
    if candidate.problem_id != problem.problem_id
        || candidate.goal_id != problem.goal.goal_id
        || candidate.planner_cut_id != problem.planner_cut.cut_id
        || candidate.plan_revision_id != plan_revision_identity(candidate)
    {
        grounds.push(StrategyRejectionGround::IdentityMismatch);
    }
    if candidate.origin == super::StrategyPlanOrigin::Satisfied {
        let confirmation_required = problem
            .theory
            .settlement_rules
            .iter()
            .find_map(|rule| {
                meld_lang::unify(&rule.goal_pattern, &problem.goal.target)
                    .map(|bindings| (rule, bindings))
            })
            .filter(|(rule, _)| rule.epistemic_placement.is_confirmation())
            .is_some_and(|(rule, bindings)| {
                super::search::ground_proposition(&rule.settlement_obligation, &bindings).is_ok_and(
                    |settlement| {
                        super::search::history_contributes(
                            &super::search::completed_task_history(history),
                            &settlement,
                        )
                    },
                )
            });
        if confirmation_required {
            let operations = super::search::epistemic_products(problem);
            if operations.is_empty()
                || !operations.iter().all(|operation| {
                    history.iter().any(|entry| {
                        matches!(&entry.product, Some(super::StrategyProduct::Epistemic(prior))
                    if prior.same_request_as(operation)
                    && prior.product_id == entry.product_id
                    && prior.return_evidence == operation.return_evidence
                    && prior.accepts_return(&entry.accepted_milestone))
                    })
                })
            {
                grounds.push(StrategyRejectionGround::InvalidEvidenceRoute);
            }
        }
        if !matches!(
            evaluate(
                &problem.planner_cut.world_model_view.world_state,
                &problem.goal.target
            ),
            EvalResult::Satisfied
        ) || !candidate.tasks.is_empty()
            || !candidate.epistemic_operations.is_empty()
            || !candidate.dependencies.is_empty()
            || !candidate.composition.steps.is_empty()
            || !candidate.composition.edges.is_empty()
            || !candidate.capability_contract_ids.is_empty()
            || candidate.evidence_route.is_some()
            || candidate.conditions != vec![problem.goal.target.clone()]
            || candidate.settlement_obligation != problem.goal.target
            || candidate.frozen_context_id != problem.planner_cut.context.context_id
            || candidate.explanation.trim().is_empty()
            || candidate.evaluation != candidate_evaluation(candidate)
        {
            grounds.push(StrategyRejectionGround::InvalidComposition);
        }
        return if grounds.is_empty() {
            PlanVerification::Valid {
                evaluation: candidate.evaluation.clone(),
            }
        } else {
            PlanVerification::Invalid { grounds }
        };
    }
    let rule = problem.theory.settlement_rules.iter().find_map(|rule| {
        meld_lang::unify(&rule.goal_pattern, &problem.goal.target).map(|bindings| (rule, bindings))
    });
    let Some((rule, bindings)) = rule else {
        return PlanVerification::Invalid {
            grounds: vec![StrategyRejectionGround::NoSettlementRule],
        };
    };
    if super::search::ground_proposition(&rule.settlement_obligation, &bindings).as_ref()
        != Ok(&candidate.settlement_obligation)
        || candidate.evidence_route.as_ref() != Some(&rule.evidence_route)
    {
        grounds.push(StrategyRejectionGround::InvalidEvidenceRoute);
    }
    if candidate.conditions != vec![problem.goal.target.clone()]
        || candidate.explanation.trim().is_empty()
        || candidate.frozen_context_id != problem.planner_cut.context.context_id
        || candidate.epistemic_operations
            != super::search::epistemic_products_with_history(problem, history)
        || !dependencies_valid(candidate, history)
    {
        grounds.push(StrategyRejectionGround::InvalidComposition);
    }
    if super::search::confirmation_is_current(problem, history)
        && !candidate.tasks.is_empty()
        && candidate.tasks.iter().all(|task| {
            super::search::completed_task_history(history)
                .iter()
                .any(|entry| {
                    matches!(&entry.product, Some(super::StrategyProduct::Task(prior))
                    if super::search::same_work(task, prior))
                })
        })
    {
        grounds.push(StrategyRejectionGround::UnchangedCompletedWork);
    }
    if let super::StrategyPlanOrigin::Method { method_id } = &candidate.origin {
        match problem
            .methods
            .iter()
            .find(|method| &method.method_id == method_id)
        {
            Some(method) => {
                let method_bindings = meld_lang::unify(&method.trigger, &problem.goal.target)
                    .and_then(|method_bindings| bindings.merge(&method_bindings));
                if method_bindings.as_ref() != Some(&candidate.bindings) {
                    grounds.push(StrategyRejectionGround::InvalidComposition);
                } else if let Err(ground) =
                    super::search::method_preconditions(problem, method, &candidate.bindings)
                {
                    grounds.push(ground);
                }
            }
            None => grounds.push(StrategyRejectionGround::InvalidComposition),
        }
    }
    let task_ids: BTreeSet<_> = candidate
        .tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect();
    let mut task_dependencies: Vec<_> = candidate
        .dependencies
        .iter()
        .filter(|dependency| {
            task_ids.contains(dependency.consumer_product_id.as_str())
                && matches!(
                    dependency.required_milestone,
                    PlanMilestoneRequirement::ExecutionTerminal { .. }
                )
        })
        .cloned()
        .collect();
    task_dependencies.sort_by(|left, right| left.dependency_id.cmp(&right.dependency_id));
    if super::search::task_ordering_dependencies(rule, &candidate.tasks, history).as_ref()
        != Ok(&task_dependencies)
    {
        grounds.push(StrategyRejectionGround::InvalidComposition);
    }
    let confirmation = candidate.tasks.is_empty();
    if confirmation {
        if !rule.epistemic_placement.is_confirmation()
            || candidate.epistemic_operations.is_empty()
            || !candidate.composition.steps.is_empty()
            || !candidate.composition.edges.is_empty()
            || candidate.bindings != meld_lang::Bindings::empty()
            || !candidate.capability_contract_ids.is_empty()
            || !candidate.epistemic_operations.iter().all(|operation| {
                let linked: Vec<_> = super::search::confirmation_history(problem, rule, history)
                    .into_iter()
                    .filter(|entry| {
                        candidate.dependencies.iter().any(|dependency| {
                            dependency.consumer_product_id == operation.product_id
                                && dependency.producer_product_id == entry.product_id
                                && dependency.required_milestone == entry.accepted_milestone
                        })
                    })
                    .collect();
                super::search::history_contributes(&linked, &candidate.settlement_obligation)
            })
        {
            grounds.push(StrategyRejectionGround::InvalidComposition);
        }
    } else {
        // The aggregate is a compatibility projection of independently complete Tasks.
        let steps: Vec<_> = candidate
            .tasks
            .iter()
            .flat_map(|task| task.composition.steps.clone())
            .collect();
        let edges: Vec<_> = candidate
            .tasks
            .iter()
            .flat_map(|task| task.composition.edges.clone())
            .collect();
        let contracts: BTreeSet<_> = candidate
            .tasks
            .iter()
            .flat_map(|task| task.capability_contract_ids.clone())
            .collect();
        if candidate.composition.steps != steps
            || candidate.composition.edges != edges
            || contracts != candidate.capability_contract_ids.iter().cloned().collect()
            || candidate.tasks.iter().any(|task| {
                task.effect_visibility
                    != if rule.epistemic_placement
                        == super::StrategyEpistemicPlacement::GraphConfirmation
                    {
                        problem.effect_visibility.clone()
                    } else {
                        None
                    }
                    || rule.epistemic_placement
                        == super::StrategyEpistemicPlacement::GraphConfirmation
                        && task
                            .effect_visibility
                            .as_ref()
                            .is_none_or(|expected| expected.validate().is_err())
                    || !meld_lang::validate(&task.composition).valid
                    || task.composition.steps.is_empty()
                    || task.return_milestone
                        != Some(PlanMilestoneRequirement::ExecutionTerminal {
                            task_id: task.task_id.clone(),
                        })
            })
        {
            grounds.push(StrategyRejectionGround::InvalidComposition);
        }
        for operation in &candidate.epistemic_operations {
            let linked = candidate.tasks.iter().all(|task| {
                candidate
                    .dependencies
                    .iter()
                    .any(|dependency| match rule.epistemic_placement {
                        super::StrategyEpistemicPlacement::Prerequisite => {
                            dependency.producer_product_id == operation.product_id
                                && dependency.consumer_product_id == task.task_id
                                && dependency.required_milestone
                                    == PlanMilestoneRequirement::CurationVisible {
                                        operation_id: operation.operation.operation_id.clone(),
                                    }
                        }
                        super::StrategyEpistemicPlacement::Confirmation
                        | super::StrategyEpistemicPlacement::GraphConfirmation => {
                            dependency.producer_product_id == task.task_id
                                && dependency.consumer_product_id == operation.product_id
                                && dependency.required_milestone == task.confirmation_milestone()
                        }
                    })
            });
            if !linked {
                grounds.push(StrategyRejectionGround::InvalidComposition);
            }
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
        evidence_outcome_supported |= candidate
            .evidence_route
            .as_ref()
            .is_some_and(|route| capability.outcome_contract_id == route.outcome_contract_id);
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
    if !confirmation && !evidence_outcome_supported {
        grounds.push(StrategyRejectionGround::InvalidEvidenceRoute);
    }
    let declared_contracts: BTreeSet<_> =
        candidate.capability_contract_ids.iter().cloned().collect();
    if used_contracts != declared_contracts {
        grounds.push(StrategyRejectionGround::IdentityMismatch);
    }
    if !confirmation
        && !composition_contributes(&candidate.composition, &candidate.settlement_obligation)
    {
        grounds.push(StrategyRejectionGround::InvalidComposition);
    }
    // Required inputs must be wired inside each Task or carried as frozen values.
    for task in &candidate.tasks {
        if task.execution_subject.as_ref() != Some(&problem.planner_cut.context.subject) {
            grounds.push(StrategyRejectionGround::IdentityMismatch);
        }
        let mut expected_inputs: Vec<_> = problem
            .task_inputs
            .iter()
            .filter(|input| {
                task.composition
                    .steps
                    .iter()
                    .any(|step| step.step_id == input.step_id)
            })
            .cloned()
            .collect();
        expected_inputs.sort_by(|left, right| {
            (&left.step_id, &left.slot_id).cmp(&(&right.step_id, &right.slot_id))
        });
        if task.source_basis_id != super::search::task_source_basis(problem)
            || task.initial_inputs != expected_inputs
            || task
                .initial_inputs
                .iter()
                .any(|input| input.validate().is_err())
            || task
                .initial_inputs
                .iter()
                .map(|input| (&input.step_id, &input.slot_id))
                .collect::<BTreeSet<_>>()
                .len()
                != task.initial_inputs.len()
        {
            grounds.push(StrategyRejectionGround::InvalidComposition);
        }
        let mut task_contracts = BTreeSet::new();
        let mut task_authority = BTreeSet::new();
        for step in &task.composition.steps {
            let StepKind::Op(operator) = &step.kind else {
                continue;
            };
            let capability = problem.capabilities.iter().find(|capability| {
                capability.operator.resolution.specific == operator.resolution.specific
            });
            if let Some(capability) = capability {
                task_contracts.insert(capability.contract_id.clone());
                if let Some(specific) = &operator.resolution.specific {
                    task_authority.insert(specific.capability_type_id.clone());
                }
                let template = meld_lang::Composition {
                    steps: vec![meld_lang::Step {
                        step_id: step.step_id.clone(),
                        kind: StepKind::Op(capability.operator.clone()),
                    }],
                    edges: Vec::new(),
                };
                if !meld_lang::substitute(&template, &task.bindings)
                    .is_ok_and(|grounded| grounded.steps[0].kind == step.kind)
                {
                    grounds.push(StrategyRejectionGround::InvalidComposition);
                }
            }
        }
        if task_contracts != task.capability_contract_ids.iter().cloned().collect()
            || task_authority != task.authority_requirements.iter().cloned().collect()
            || terminal_outcome_contract(problem, &task.composition)
                != Some(task.expected_outcome_contract_id.as_str())
        {
            grounds.push(StrategyRejectionGround::IdentityMismatch);
        }
        for step in &task.composition.steps {
            let StepKind::Op(operator) = &step.kind else {
                continue;
            };
            for input in operator
                .resolution
                .requires_inputs
                .iter()
                .filter(|slot| slot.required)
            {
                let wired = task.composition.edges.iter().any(|edge| {
                    edge.to == step.step_id
                        && matches!(
                            &edge.kind,
                            EdgeKind::DataFlow { artifact_type }
                                if artifact_type == &input.artifact_type
                        )
                });
                let supplied = task.initial_inputs.iter().any(|value| {
                    value.step_id == step.step_id
                        && input.artifact_type
                            == meld_lang::Term::ArtifactType(value.artifact_type_id.clone())
                });
                if !wired && !supplied {
                    grounds.push(StrategyRejectionGround::UnclosedArtifact {
                        artifact_type: format!("{:?}", input.artifact_type),
                    });
                }
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
    let standard = verify_with_history(
        &request.search.problem,
        &successor.plan,
        &request.completed_history,
    );
    if !matches!(standard, PlanVerification::Valid { .. }) {
        return standard;
    }
    if predecessor.plan_revision_id != plan_revision_identity(predecessor)
        || predecessor.plan_family_id != plan_family_identity(&request.search.problem)
        || predecessor.goal_id != request.search.problem.goal.goal_id
        || successor.plan.plan_family_id != predecessor.plan_family_id
        || successor.plan.predecessor_plan_revision_id.as_ref()
            != Some(&predecessor.plan_revision_id)
        || successor.plan.origin == super::StrategyPlanOrigin::Confirmation
            && predecessor.tasks.iter().any(|task| {
                !successor.plan.epistemic_operations.iter().all(|operation| {
                    successor.plan.dependencies.iter().any(|dependency| {
                        dependency.producer_product_id == task.task_id
                            && dependency.consumer_product_id == operation.product_id
                            && dependency.required_milestone == task.confirmation_milestone()
                    })
                })
            })
        || successor.completed_history != request.completed_history
        || request.completed_history.iter().any(|entry| {
            if entry.source_plan_revision_id != predecessor.plan_revision_id {
                return false;
            }
            match &entry.product {
                Some(super::StrategyProduct::Task(task)) => !predecessor.tasks.contains(task),
                Some(super::StrategyProduct::Epistemic(operation)) => {
                    !predecessor.epistemic_operations.contains(operation)
                }
                None => false,
            }
        })
    {
        return PlanVerification::Invalid {
            grounds: vec![StrategyRejectionGround::InvalidPredecessor],
        };
    }
    standard
}

pub(crate) fn dependencies_valid(
    candidate: &StrategyPlan,
    history: &[super::StrategyCompletedHistoryEntry],
) -> bool {
    let mut products = BTreeMap::new();
    for task in &candidate.tasks {
        if products
            .insert(
                task.task_id.as_str(),
                vec![
                    task.confirmation_milestone(),
                    PlanMilestoneRequirement::ExecutionTerminal {
                        task_id: task.task_id.clone(),
                    },
                ],
            )
            .is_some()
        {
            return false;
        }
    }
    for operation in &candidate.epistemic_operations {
        if products
            .insert(
                operation.product_id.as_str(),
                vec![PlanMilestoneRequirement::CurationVisible {
                    operation_id: operation.operation.operation_id.clone(),
                }],
            )
            .is_some()
        {
            return false;
        }
    }
    let mut dependency_ids = BTreeSet::new();
    for dependency in &candidate.dependencies {
        if !dependency_ids.insert(&dependency.dependency_id)
            || !products.contains_key(dependency.consumer_product_id.as_str())
            || dependency.producer_product_id == dependency.consumer_product_id
        {
            return false;
        }
        let producer = products.get(dependency.producer_product_id.as_str());
        if producer.is_some_and(|milestones| !milestones.contains(&dependency.required_milestone))
            || producer.is_none()
                && !history.iter().any(|entry| {
                    entry.product_id == dependency.producer_product_id
                        && entry.accepted_milestone == dependency.required_milestone
                        && !entry.owner_position_id.is_empty()
                })
        {
            return false;
        }
    }
    let mut pending: BTreeSet<_> = products.keys().copied().collect();
    while !pending.is_empty() {
        let ready: Vec<_> = pending
            .iter()
            .copied()
            .filter(|id| {
                !candidate.dependencies.iter().any(|dependency| {
                    dependency.consumer_product_id == *id
                        && pending.contains(dependency.producer_product_id.as_str())
                })
            })
            .collect();
        if ready.is_empty() {
            return false;
        }
        for id in ready {
            pending.remove(id);
        }
    }
    true
}
