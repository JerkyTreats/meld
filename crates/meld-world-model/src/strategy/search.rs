//! Deterministic bounded construction for the minimal Strategy engine.

use std::collections::BTreeSet;

use meld_lang::{
    evaluate, substitute, unify, Composition, Edge, EdgeKind, Effect, EvalResult, GoalLifecycle,
    Proposition, Step, StepKind, Term,
};

use super::contracts::*;

/// Search one immutable problem using deterministic bounded traversal.
///
/// The function returns the strongest verified-shape candidate it can
/// construct within the supplied bounds. `Bounded` means traversal stopped
/// early and must not be interpreted as proof that no candidate exists.
pub fn search(request: &StrategySearchRequest) -> StrategySearchResult {
    search_with_completed(request, &[], &[])
}

fn search_with_completed(
    request: &StrategySearchRequest,
    completed: &[&StrategyTask],
    history: &[StrategyCompletedHistoryEntry],
) -> StrategySearchResult {
    let mut state = SearchState::new(request);
    state.history = history;
    if !request.problem.goal.target.is_ground()
        || !matches!(request.problem.goal.lifecycle, GoalLifecycle::Proposed)
    {
        state.reject(StrategyRejectionGround::InvalidGoal);
        return state.finish(None);
    }

    if matches!(
        evaluate(
            &request.problem.planner_cut.world_model_view.world_state,
            &request.problem.goal.target
        ),
        EvalResult::Satisfied
    ) {
        let mut plan = StrategyPlan {
            historical_identity: None,
            settlement_rule_id: String::new(),
            plan_revision_id: String::new(),
            plan_family_id: plan_family_identity(&request.problem),
            problem_id: request.problem.problem_id.clone(),
            goal_id: request.problem.goal.goal_id.clone(),
            planner_cut_id: request.problem.planner_cut.cut_id.clone(),
            origin: StrategyPlanOrigin::Satisfied,
            bindings: meld_lang::Bindings::empty(),
            settlement_obligation: request.problem.goal.target.clone(),
            evidence_route: None,
            tasks: Vec::new(),
            epistemic_operations: Vec::new(),
            dependencies: Vec::new(),
            conditions: vec![request.problem.goal.target.clone()],
            frozen_context_id: request.problem.planner_cut.context.context_id.clone(),
            explanation: "Admitted owner evidence already establishes the desired condition".into(),
            predecessor_plan_revision_id: None,
            evaluation: StrategyPlanEvaluation {
                step_count: 0,
                time_ms: 0,
                provider_calls: 0,
            },
        };
        plan.plan_revision_id = plan_revision_identity(&plan);
        return state.finish(Some(plan));
    }

    let mut rules = request
        .problem
        .theory
        .settlement_rules
        .iter()
        .filter_map(|rule| {
            unify(&rule.goal_pattern, &request.problem.goal.target).map(|bindings| (rule, bindings))
        })
        .collect::<Vec<_>>();
    rules.sort_by_key(|(rule, _)| settlement_rule_identity(rule));
    if rules.is_empty() {
        state.reject(StrategyRejectionGround::NoSettlementRule);
        return state.finish(None);
    }
    let mut candidates = Vec::new();
    for (rule, bindings) in rules {
        if request
            .problem
            .planner_cut
            .world_model_view
            .unassessed_belief
            .is_some()
            && rule.construction != StrategyConstruction::ObserveUnknown
        {
            state.reject(StrategyRejectionGround::InvalidComposition);
            continue;
        }
        let settlement = match ground_proposition(&rule.settlement_obligation, &bindings) {
            Ok(value) => value,
            Err(_) => {
                state.reject(StrategyRejectionGround::InvalidComposition);
                continue;
            }
        };
        if rule.construction == StrategyConstruction::ObserveUnknown {
            if let Some(candidate) = epistemic_candidate(request, rule, &settlement, &mut state) {
                candidates.push(candidate);
            }
            continue;
        }
        candidates.extend(method_candidates(
            request,
            rule,
            &settlement,
            &bindings,
            &mut state,
        ));
        if !state.bounded {
            candidates.extend(direct_candidates(
                request,
                rule,
                &settlement,
                &bindings,
                &mut state,
            ));
        }
        if state.bounded {
            break;
        }
    }
    candidates.retain(|candidate| {
        let repeats = !candidate.tasks.is_empty()
            && candidate
                .tasks
                .iter()
                .all(|task| completed.iter().any(|prior| same_work(task, prior)));
        if repeats {
            state.reject(StrategyRejectionGround::UnchangedCompletedWork);
        }
        !repeats
    });
    candidates.sort_by(|left, right| {
        if request.problem.evaluation_policy.prefer_fewer_steps {
            candidate_key(left).cmp(&candidate_key(right))
        } else {
            alternative_candidate_key(left).cmp(&alternative_candidate_key(right))
        }
    });
    let recommendation = candidates.into_iter().next();
    state.finish(recommendation)
}

/// Reconstruct one immutable successor from an exact predecessor and history.
///
/// This function has no runtime reads or writes. Equal frozen inputs produce
/// the same successor revision and preserve the supplied completed history.
pub fn search_successor(request: &StrategySuccessorRequest) -> StrategySuccessorResult {
    let predecessor = request.predecessor_plan.as_ref();
    let expected_family = plan_family_identity(&request.search.problem);
    if !super::history::predecessor_identity_valid(predecessor)
        || predecessor.plan_family_id != expected_family
        || predecessor.goal_id != request.search.problem.goal.goal_id
    {
        return StrategySuccessorResult {
            problem_id: request.search.problem.problem_id.clone(),
            completion: StrategySearchCompletion::Exhaustive,
            recommendation: None,
            rejections: vec![StrategyRejectionGround::InvalidPredecessor],
            statistics: StrategySearchStatistics::default(),
        };
    }

    let prior_rule = selected_rule(&request.search.problem, predecessor);
    if predecessor.historical_identity.is_some()
        && predecessor.origin != StrategyPlanOrigin::Satisfied
        && prior_rule.is_none()
    {
        return StrategySuccessorResult {
            problem_id: request.search.problem.problem_id.clone(),
            completion: StrategySearchCompletion::Exhaustive,
            recommendation: None,
            rejections: vec![StrategyRejectionGround::InvalidPredecessor],
            statistics: StrategySearchStatistics::default(),
        };
    }
    let confirmed_current = prior_rule.is_some_and(|rule| {
        confirmation_is_current(&request.search.problem, rule, &request.completed_history)
    });
    let source_changed_after_confirmation = predecessor.origin == StrategyPlanOrigin::Confirmation
        && !prior_rule
            .map(|rule| epistemic_products(&request.search.problem, rule))
            .unwrap_or_default()
            .iter()
            .all(|operation| {
                predecessor
                    .epistemic_operations
                    .iter()
                    .any(|prior| prior.same_source_as(operation))
            });
    // Changed source may require new work, but a currently satisfied condition
    // does not discharge this Goal's outstanding confirmation of prior effects.
    let confirmation = if source_changed_after_confirmation
        && !matches!(
            evaluate(
                &request
                    .search
                    .problem
                    .planner_cut
                    .world_model_view
                    .world_state,
                &request.search.problem.goal.target,
            ),
            EvalResult::Satisfied
        ) {
        None
    } else {
        confirmation_successor(request)
    };
    let completed: Vec<_> = if confirmed_current {
        completed_task_history(&request.completed_history)
            .into_iter()
            .filter_map(|entry| match &entry.product {
                Some(StrategyProduct::Task(task)) => Some(task.as_ref()),
                _ => None,
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut result = confirmation.unwrap_or_else(|| {
        search_with_completed(&request.search, &completed, &request.completed_history)
    });
    let recommendation = result.recommendation.and_then(|mut plan| {
        if let Some(rule) = selected_rule(&request.search.problem, &plan) {
            match planned_epistemic_products(
                &request.search.problem,
                rule,
                &plan.tasks,
                &request.completed_history,
            ) {
                Ok((operations, dependencies)) => {
                    plan.epistemic_operations = operations;
                    plan.dependencies = dependencies;
                }
                Err(ground) => {
                    result.rejections.push(ground);
                    return None;
                }
            }
        }
        plan.plan_family_id = predecessor.plan_family_id.clone();
        plan.predecessor_plan_revision_id = Some(predecessor.plan_revision_id.clone());
        plan.plan_revision_id = plan_revision_identity(&plan);
        Some(StrategySuccessorPlan {
            plan,
            completed_history: request.completed_history.clone(),
        })
    });
    StrategySuccessorResult {
        problem_id: result.problem_id,
        completion: result.completion,
        recommendation,
        rejections: result.rejections,
        statistics: result.statistics,
    }
}

pub(super) fn confirmation_is_current(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
    history: &[StrategyCompletedHistoryEntry],
) -> bool {
    let Ok((operations, _)) = planned_epistemic_products(problem, rule, &[], history) else {
        return false;
    };
    !operations.is_empty()
        && operations.iter().all(|operation| {
            history.iter().any(|entry| {
                matches!(&entry.product,
            Some(StrategyProduct::Epistemic(prior))
            if entry.product_id == prior.product_id
                && !entry.owner_position_id.is_empty()
                && prior.accepts_return(&entry.accepted_milestone)
                && prior.same_request_as(operation))
            })
        })
}

/// Only comparable owner-authored inputs can justify repeating a completed effect.
pub(super) fn task_source_basis(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
) -> Option<String> {
    let owners = &rule.repeat_on_changed_owners;
    if owners.is_empty() {
        return None;
    }
    let mut sources = Vec::new();
    for owner in owners {
        let receipts = problem
            .planner_cut
            .traversal_cut
            .receipts
            .iter()
            .filter(|receipt| {
                &receipt.owner_id == owner
                    && (receipt.owner_id != crate::curation::CURATION_OWNER_ID
                        || receipt.scope != problem.planner_cut.traversal_cut.scope)
            })
            .collect::<Vec<_>>();
        if receipts.is_empty() {
            return None;
        }
        for receipt in receipts {
            let basis = receipt
                .work_input_basis_id
                .as_deref()
                .filter(|basis| !basis.trim().is_empty())?;
            sources.push((&receipt.owner_id, &receipt.scope, basis));
        }
    }
    sources.sort_by_cached_key(|source| {
        serde_json::to_vec(source).expect("serializable work-input identity")
    });
    Some(stable_id("strategy-task-input-v2", &sources))
}

pub(super) fn same_work(left: &StrategyTask, right: &StrategyTask) -> bool {
    let changed_inputs = match (&left.source_basis_id, &right.source_basis_id) {
        (Some(left), Some(right)) => {
            left.starts_with("strategy-task-input-v2::")
                && right.starts_with("strategy-task-input-v2::")
                && left != right
        }
        _ => false,
    };
    same_effect(left, right) && !changed_inputs
}

pub(super) fn same_effect(left: &StrategyTask, right: &StrategyTask) -> bool {
    left.composition == right.composition
        && left.bindings == right.bindings
        && left.initial_inputs == right.initial_inputs
        && left.execution_subject == right.execution_subject
        && left.capability_contract_ids == right.capability_contract_ids
        && left.expected_outcome_contract_id == right.expected_outcome_contract_id
        && left.authority_requirements == right.authority_requirements
        && left.effect_visibility == right.effect_visibility
}

fn confirmation_successor(request: &StrategySuccessorRequest) -> Option<StrategySearchResult> {
    let problem = &request.search.problem;
    let rule = selected_rule(problem, &request.predecessor_plan)?;
    let bindings = unify(&rule.goal_pattern, &problem.goal.target)?;
    if !rule.has_evidence_returns() {
        return None;
    }
    let settlement = ground_proposition(&rule.settlement_obligation, &bindings).ok()?;
    let completed = confirmation_history(problem, rule, &request.completed_history);
    if !history_contributes(&completed, &settlement)
        || !request.predecessor_plan.tasks.iter().all(|task| {
            completed
                .iter()
                .any(|entry| entry.product_id == task.task_id)
        })
    {
        return None;
    }
    let (epistemic_operations, dependencies) =
        planned_epistemic_products(problem, rule, &[], &request.completed_history).ok()?;
    if epistemic_operations.is_empty() {
        return None;
    }
    if epistemic_operations.iter().all(|operation| {
        request.completed_history.iter().any(|entry| {
            matches!(&entry.product, Some(StrategyProduct::Epistemic(prior))
            if prior.same_request_as(operation)
            && entry.product_id == prior.product_id
                && prior.return_evidence == operation.return_evidence
                && prior.accepts_return(&entry.accepted_milestone))
        })
    }) {
        return None;
    }
    let mut plan = StrategyPlan {
        historical_identity: None,
        settlement_rule_id: settlement_rule_identity(rule),
        plan_revision_id: String::new(),
        plan_family_id: plan_family_identity(problem),
        problem_id: problem.problem_id.clone(),
        goal_id: problem.goal.goal_id.clone(),
        planner_cut_id: problem.planner_cut.cut_id.clone(),
        origin: StrategyPlanOrigin::Confirmation,
        bindings: meld_lang::Bindings::empty(),
        settlement_obligation: settlement,
        evidence_route: Some(rule.evidence_route.clone()),
        tasks: Vec::new(),
        epistemic_operations,
        dependencies,
        conditions: vec![problem.goal.target.clone()],
        frozen_context_id: problem.planner_cut.context.context_id.clone(),
        explanation: "Confirm an accepted executable effect under the new admitted cut".into(),
        predecessor_plan_revision_id: None,
        evaluation: StrategyPlanEvaluation {
            step_count: 0,
            time_ms: 0,
            provider_calls: 0,
        },
    };
    plan.plan_revision_id = plan_revision_identity(&plan);
    Some(SearchState::new(&request.search).finish(Some(plan)))
}

pub(crate) fn completed_task_history(
    history: &[StrategyCompletedHistoryEntry],
) -> Vec<&StrategyCompletedHistoryEntry> {
    let mut seen = BTreeSet::new();
    history
        .iter()
        .filter(|entry| {
            if interrupted_history_entry(entry, history) {
                return false;
            }
            matches!(&entry.product, Some(StrategyProduct::Task(task))
            if entry.product_id == task.task_id
                && !entry.owner_position_id.is_empty()
                && entry.accepted_milestone == task.confirmation_milestone())
                && seen.insert(&entry.product_id)
        })
        .collect()
}

/// A later, more specific owner return can qualify a previously recorded terminal
/// position without rewriting that immutable history as completed work.
pub(crate) fn interrupted_history_entry(
    entry: &StrategyCompletedHistoryEntry,
    history: &[StrategyCompletedHistoryEntry],
) -> bool {
    matches!(entry.accepted_milestone, PlanMilestoneRequirement::ExecutionTerminal { .. })
        && history.iter().any(|returned| {
            returned.source_plan_revision_id == entry.source_plan_revision_id
                && returned.product_id == entry.product_id
                && matches!(&returned.accepted_milestone,
                    PlanMilestoneRequirement::ExecutionInterrupted { task_id } if task_id == &entry.product_id)
        })
}

pub(crate) fn confirmation_history<'a>(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
    history: &'a [StrategyCompletedHistoryEntry],
) -> Vec<&'a StrategyCompletedHistoryEntry> {
    completed_task_history(history)
        .into_iter()
        .filter(|entry| {
            matches!(&entry.product, Some(StrategyProduct::Task(task))
            if confirmation_expectation_matches(problem, rule, task))
        })
        .collect()
}

pub(crate) fn history_contributes(
    history: &[&StrategyCompletedHistoryEntry],
    settlement: &Proposition,
) -> bool {
    let composition = Composition {
        steps: history
            .iter()
            .flat_map(|entry| match &entry.product {
                Some(StrategyProduct::Task(task)) => task.composition.steps.clone(),
                _ => Vec::new(),
            })
            .collect(),
        edges: Vec::new(),
    };
    composition_contributes(&composition, settlement)
}

pub(crate) fn confirmation_expectation_matches(
    problem: &StrategyProblem,
    rule: &super::StrategySettlementRule,
    task: &super::StrategyTask,
) -> bool {
    rule.has_evidence_returns()
        && if rule.task_requires_visibility(task) {
            task.effect_visibility == problem.effect_visibility
                && task
                    .effect_visibility
                    .as_ref()
                    .is_some_and(|expected| expected.validate().is_ok())
        } else {
            task.effect_visibility.is_none()
        }
}

/// A prerequisite's own publication changes the cut, but does not create a new obligation
/// when the new cut still selects the exact accepted result over unchanged source evidence.
pub(crate) fn epistemic_products_with_history(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
    history: &[StrategyCompletedHistoryEntry],
) -> Vec<StrategyEpistemicOperation> {
    epistemic_products(problem, rule)
        .into_iter()
        .map(|operation| {
            if operation.return_evidence.is_some() {
                return operation;
            }
            history
                .iter()
                .find_map(|entry| {
                    let Some(StrategyProduct::Epistemic(prior)) = &entry.product else {
                        return None;
                    };
                    (entry.product_id == prior.product_id
                        && prior.accepts_return(&entry.accepted_milestone)
                        && prior.operation.authority == operation.operation.authority
                        && prior.operation.request_id == operation.operation.request_id
                        && prior.same_source_as(&operation)
                        && problem
                            .planner_cut
                            .traversal_cut
                            .receipts
                            .iter()
                            .any(|receipt| {
                                receipt.owner_id == crate::curation::CURATION_OWNER_ID
                                    && receipt.scope == operation.operation.source_cut.scope
                                    && receipt.revision_id == entry.owner_position_id
                            }))
                    .then(|| prior.as_ref().clone())
                })
                .unwrap_or(operation)
        })
        .collect()
}

pub(crate) fn epistemic_products(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
) -> Vec<StrategyEpistemicOperation> {
    problem
        .curation_operations
        .iter()
        .filter_map(|operation| {
            let selection = rule.epistemic_selections.iter().find(|selection| {
                selection
                    .rule_revision
                    .as_ref()
                    .is_none_or(|reference| reference == &operation.rule_revision)
            })?;
            let return_evidence = selection
                .evidence_return
                .then(|| rule.evidence_route.clone());
            let mut product_id = stable_id(
                "strategy-epistemic-product-v1",
                &(&problem.goal.goal_id, &operation.operation_id),
            );
            if let Some(route) = &return_evidence {
                product_id = stable_id("strategy-epistemic-return-v1", &(&product_id, route));
            }
            Some(StrategyEpistemicOperation {
                return_evidence: return_evidence.clone(),
                idempotency_key: format!("curation::{product_id}"),
                product_id,
                operation: operation.clone(),
                authority_requirements: vec![operation.authority.agent_id.clone()],
            })
        })
        .collect()
}

fn dependency(
    producer: &str,
    consumer: &str,
    milestone: PlanMilestoneRequirement,
) -> StrategyPlanDependency {
    StrategyPlanDependency {
        dependency_id: match &milestone {
            PlanMilestoneRequirement::CurationTerminal { operation_id } => stable_id(
                "strategy-dependency-v1",
                &(producer, consumer, operation_id),
            ),
            _ => stable_id("strategy-dependency-v1", &(producer, consumer, &milestone)),
        },
        producer_product_id: producer.into(),
        consumer_product_id: consumer.into(),
        required_milestone: milestone,
    }
}

fn product_dependencies(
    rule: &StrategySettlementRule,
    tasks: &[StrategyTask],
    operations: &[StrategyEpistemicOperation],
    history: &[StrategyCompletedHistoryEntry],
    required_consumers: &BTreeSet<String>,
) -> Result<Vec<StrategyPlanDependency>, StrategyRejectionGround> {
    let current: Vec<_> = tasks
        .iter()
        .map(|task| StrategyProduct::Task(Box::new(task.clone())))
        .chain(
            operations
                .iter()
                .map(|operation| StrategyProduct::Epistemic(Box::new(operation.clone()))),
        )
        .collect();
    let mut producers: std::collections::BTreeMap<_, _> = current
        .iter()
        .map(|product| (product_id(product), product))
        .collect();
    for entry in history {
        if let Some(product) = &entry.product {
            let accepted = match product {
                StrategyProduct::Task(task) => {
                    task.accounts_for_return(&entry.accepted_milestone)
                        || task.confirmation_milestone() == entry.accepted_milestone
                }
                StrategyProduct::Epistemic(operation) => {
                    operation.accepts_return(&entry.accepted_milestone)
                }
            };
            if accepted
                && !entry.owner_position_id.is_empty()
                && product_id(product) == entry.product_id
            {
                producers.entry(product_id(product)).or_insert(product);
            }
        }
    }
    let mut dependencies = Vec::new();
    for constraint in &rule.product_ordering {
        let consumers: Vec<_> = current
            .iter()
            .filter(|product| {
                required_consumers.contains(product_id(product))
                    && selector_matches(&constraint.after, product)
            })
            .collect();
        if consumers.is_empty() {
            continue;
        }
        let selected_current: Vec<_> = current
            .iter()
            .filter(|product| selector_matches(&constraint.before, product))
            .collect();
        let selected: Vec<_> = if selected_current.is_empty() {
            producers
                .values()
                .copied()
                .filter(|product| selector_matches(&constraint.before, product))
                .collect()
        } else {
            selected_current
        };
        if selected.is_empty()
            && constraint.before.is_exact()
            && constraint.condition == StrategyOrderingCondition::ConsumerSelected
        {
            return Err(StrategyRejectionGround::InvalidComposition);
        }
        for before in selected {
            let milestone = match (before, constraint.milestone) {
                (StrategyProduct::Task(task), StrategyDependencyMilestone::ExecutionTerminal) => {
                    PlanMilestoneRequirement::ExecutionTerminal {
                        task_id: task.task_id.clone(),
                    }
                }
                (StrategyProduct::Task(task), StrategyDependencyMilestone::EffectVisible)
                    if task.effect_visibility.is_some() =>
                {
                    task.confirmation_milestone()
                }
                (
                    StrategyProduct::Epistemic(operation),
                    StrategyDependencyMilestone::CurationVisible,
                ) if operation.return_evidence.is_none() => {
                    PlanMilestoneRequirement::CurationVisible {
                        operation_id: operation.operation.operation_id.clone(),
                    }
                }
                _ => return Err(StrategyRejectionGround::InvalidComposition),
            };
            for after in &consumers {
                if product_id(before) == product_id(after) {
                    return Err(StrategyRejectionGround::InvalidComposition);
                }
                dependencies.push(dependency(
                    product_id(before),
                    product_id(after),
                    milestone.clone(),
                ));
            }
        }
    }
    dependencies.sort_by(|left, right| left.dependency_id.cmp(&right.dependency_id));
    dependencies.dedup();
    Ok(dependencies)
}

pub(crate) fn planned_epistemic_products(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
    tasks: &[StrategyTask],
    history: &[StrategyCompletedHistoryEntry],
) -> Result<(Vec<StrategyEpistemicOperation>, Vec<StrategyPlanDependency>), StrategyRejectionGround>
{
    let mut operations = epistemic_products_with_history(problem, rule, history);
    let mut required: BTreeSet<String> = tasks
        .iter()
        .map(|task| task.task_id.clone())
        .chain(
            operations
                .iter()
                .filter(|operation| operation.return_evidence.is_some())
                .map(|operation| operation.product_id.clone()),
        )
        .collect();
    let dependencies = loop {
        let dependencies = product_dependencies(rule, tasks, &operations, history, &required)?;
        let before = required.len();
        for edge in &dependencies {
            if required.contains(&edge.consumer_product_id) {
                required.insert(edge.producer_product_id.clone());
            }
        }
        if required.len() == before {
            break dependencies;
        }
    };
    operations.retain(|operation| required.contains(&operation.product_id));
    if operations.iter().any(|operation| {
        problem
            .unavailable_curation_operation_ids
            .contains(&operation.operation.operation_id)
    }) {
        return Err(StrategyRejectionGround::UnchangedCompletedWork);
    }
    Ok((operations, dependencies))
}

fn product_id(product: &StrategyProduct) -> &str {
    match product {
        StrategyProduct::Task(task) => &task.task_id,
        StrategyProduct::Epistemic(operation) => &operation.product_id,
    }
}
fn selector_matches(selector: &StrategyProductSelector, product: &StrategyProduct) -> bool {
    match product {
        StrategyProduct::Task(task) => selector.selects_task(task),
        StrategyProduct::Epistemic(operation) => selector.selects_epistemic(operation),
    }
}

struct SearchState<'a> {
    request: &'a StrategySearchRequest,
    history: &'a [StrategyCompletedHistoryEntry],
    rejections: Vec<StrategyRejectionGround>,
    expanded: usize,
    bounded: bool,
}

impl<'a> SearchState<'a> {
    fn new(request: &'a StrategySearchRequest) -> Self {
        Self {
            request,
            history: &[],
            rejections: Vec::new(),
            expanded: 0,
            bounded: false,
        }
    }

    fn expand(&mut self) -> bool {
        if self.expanded >= self.request.bounds.max_expansions {
            self.bounded = true;
            self.reject(StrategyRejectionGround::BoundsExceeded);
            return false;
        }
        self.expanded += 1;
        true
    }

    fn reject(&mut self, ground: StrategyRejectionGround) {
        if !self.rejections.contains(&ground) {
            self.rejections.push(ground);
        }
    }

    fn finish(self, recommendation: Option<StrategyPlan>) -> StrategySearchResult {
        StrategySearchResult {
            problem_id: self.request.problem.problem_id.clone(),
            completion: if self.bounded {
                StrategySearchCompletion::Bounded
            } else {
                StrategySearchCompletion::Exhaustive
            },
            recommendation,
            rejections: self.rejections,
            statistics: StrategySearchStatistics {
                expanded_positions: self.expanded,
            },
        }
    }
}

fn epistemic_candidate(
    request: &StrategySearchRequest,
    rule: &StrategySettlementRule,
    settlement: &Proposition,
    state: &mut SearchState<'_>,
) -> Option<StrategyPlan> {
    if !matches!(
        evaluate(
            &request.problem.planner_cut.world_model_view.world_state,
            &request.problem.goal.target
        ),
        EvalResult::Indeterminate { .. }
    ) || !state.expand()
    {
        return None;
    }
    let (operations, dependencies) =
        match planned_epistemic_products(&request.problem, rule, &[], state.history) {
            Ok(products) => products,
            Err(ground) => {
                state.reject(ground);
                return None;
            }
        };
    if operations.is_empty()
        || !operations
            .iter()
            .any(|operation| operation.return_evidence.is_some())
    {
        state.reject(StrategyRejectionGround::InvalidEvidenceRoute);
        return None;
    }
    let mut plan = StrategyPlan {
        historical_identity: None,
        settlement_rule_id: settlement_rule_identity(rule),
        plan_revision_id: String::new(),
        plan_family_id: plan_family_identity(&request.problem),
        problem_id: request.problem.problem_id.clone(),
        goal_id: request.problem.goal.goal_id.clone(),
        planner_cut_id: request.problem.planner_cut.cut_id.clone(),
        origin: StrategyPlanOrigin::Epistemic,
        bindings: meld_lang::Bindings::empty(),
        settlement_obligation: settlement.clone(),
        evidence_route: Some(rule.evidence_route.clone()),
        tasks: Vec::new(),
        epistemic_operations: operations,
        dependencies,
        conditions: vec![request.problem.goal.target.clone()],
        frozen_context_id: request.problem.planner_cut.context.context_id.clone(),
        explanation:
            "Acquire bounded evidence for unknown Goal knowledge before selecting executable work"
                .into(),
        predecessor_plan_revision_id: None,
        evaluation: StrategyPlanEvaluation {
            step_count: 0,
            time_ms: 0,
            provider_calls: 0,
        },
    };
    if !super::verification::dependencies_valid(&plan, state.history) {
        state.reject(StrategyRejectionGround::InvalidComposition);
        return None;
    }
    plan.plan_revision_id = plan_revision_identity(&plan);
    Some(plan)
}

fn method_candidates(
    request: &StrategySearchRequest,
    rule: &StrategySettlementRule,
    settlement: &Proposition,
    goal_bindings: &meld_lang::Bindings,
    state: &mut SearchState<'_>,
) -> Vec<StrategyPlan> {
    let mut methods: Vec<_> = request.problem.methods.iter().collect();
    methods.sort_by(|left, right| left.method_id.cmp(&right.method_id));
    let mut candidates = Vec::new();
    for method in methods {
        if !state.expand() {
            break;
        }
        let Some(method_bindings) = unify(&method.trigger, &request.problem.goal.target) else {
            continue;
        };
        let Some(bindings) = goal_bindings.merge(&method_bindings) else {
            continue;
        };
        if let Err(ground) = method_preconditions(&request.problem, method, &bindings) {
            state.reject(ground);
            continue;
        }
        let Ok(composition) = substitute(&method.composition, &bindings) else {
            state.reject(StrategyRejectionGround::UnboundVariable {
                variable: "method composition".to_string(),
            });
            continue;
        };
        if !preconditions_hold(
            &composition,
            &request.problem.planner_cut.world_model_view.world_state,
            state,
        ) {
            continue;
        }
        if !composition_contributes(&composition, settlement) {
            continue;
        }
        if let Some(candidate) = finish_candidate(
            request,
            rule,
            settlement,
            bindings,
            composition,
            StrategyPlanOrigin::Method {
                method_id: method.method_id.clone(),
            },
            state,
        ) {
            candidates.push(candidate);
        }
    }
    candidates
}

pub(crate) fn method_preconditions(
    problem: &super::StrategyProblem,
    method: &meld_lang::Method,
    bindings: &meld_lang::Bindings,
) -> Result<(), StrategyRejectionGround> {
    for precondition in &method.preconditions {
        let ground = ground_proposition(precondition, bindings)
            .map_err(|variable| StrategyRejectionGround::UnboundVariable { variable })?;
        match evaluate(&problem.planner_cut.world_model_view.world_state, &ground) {
            EvalResult::Satisfied => {}
            EvalResult::Unsatisfied { .. } => {
                return Err(StrategyRejectionGround::UnsatisfiedMethodPrecondition {
                    method_id: method.method_id.clone(),
                });
            }
            EvalResult::Indeterminate { .. } => {
                return Err(StrategyRejectionGround::IndeterminateMethodPrecondition {
                    method_id: method.method_id.clone(),
                });
            }
        }
    }
    Ok(())
}

fn direct_candidates(
    request: &StrategySearchRequest,
    rule: &StrategySettlementRule,
    settlement: &Proposition,
    goal_bindings: &meld_lang::Bindings,
    state: &mut SearchState<'_>,
) -> Vec<StrategyPlan> {
    direct_bodies(request, settlement, goal_bindings, state, 0)
        .into_iter()
        .filter_map(|bodies| {
            finish_bodies(
                request,
                rule,
                settlement,
                goal_bindings.clone(),
                bodies,
                StrategyPlanOrigin::Direct,
                state,
            )
        })
        .collect()
}

fn direct_bodies(
    request: &StrategySearchRequest,
    settlement: &Proposition,
    goal_bindings: &meld_lang::Bindings,
    state: &mut SearchState<'_>,
    depth: usize,
) -> Vec<Vec<(meld_lang::Bindings, Composition)>> {
    if depth > request.bounds.max_depth {
        state.bounded = true;
        return Vec::new();
    }
    let mut candidates: Vec<_> = atomic_compositions(request, settlement, goal_bindings, state)
        .into_iter()
        .map(|body| vec![body])
        .collect();
    match settlement {
        Proposition::All(children) if !children.is_empty() => {
            let mut combinations = vec![Vec::new()];
            for child in children {
                let alternatives = direct_bodies(request, child, goal_bindings, state, depth + 1);
                let mut next = Vec::new();
                for prior in &combinations {
                    for bodies in &alternatives {
                        if !state.expand() {
                            break;
                        }
                        let mut combined = prior.clone();
                        for body in bodies {
                            if !combined.contains(body) {
                                combined.push(body.clone());
                            }
                        }
                        next.push(combined);
                    }
                }
                combinations = next;
                if combinations.is_empty() {
                    break;
                }
            }
            candidates.extend(combinations);
        }
        Proposition::Any(children) => {
            for child in children {
                candidates.extend(direct_bodies(
                    request,
                    child,
                    goal_bindings,
                    state,
                    depth + 1,
                ));
            }
        }
        _ => {}
    }
    candidates
}

fn atomic_compositions(
    request: &StrategySearchRequest,
    settlement: &Proposition,
    goal_bindings: &meld_lang::Bindings,
    state: &mut SearchState<'_>,
) -> Vec<(meld_lang::Bindings, Composition)> {
    let mut capabilities: Vec<_> = request.problem.capabilities.iter().collect();
    capabilities.sort_by(|left, right| left.contract_id.cmp(&right.contract_id));
    let mut candidates = Vec::new();
    for capability in capabilities {
        if !state.expand() {
            break;
        }
        let Some(effect_bindings) = capability
            .operator
            .effects
            .iter()
            .filter_map(effect_proposition)
            .find_map(|effect| unify(effect, settlement))
        else {
            continue;
        };
        let Some(bindings) = goal_bindings.merge(&effect_bindings) else {
            continue;
        };
        let Some(root) = ground_operator(&capability.operator, &bindings, state) else {
            continue;
        };
        let mut selected = vec![(capability.contract_id.clone(), root)];
        let mut edges = Vec::new();
        let mut visiting = BTreeSet::new();
        if !close_inputs(
            request,
            0,
            &bindings,
            &mut selected,
            &mut edges,
            &mut visiting,
            state,
        ) {
            continue;
        }
        let steps = selected
            .iter()
            .map(|(_, operator)| Step {
                step_id: operator.operator_id.clone(),
                kind: StepKind::Op(operator.clone()),
            })
            .collect();
        let composition = Composition { steps, edges };
        candidates.push((bindings, composition));
    }
    candidates
}

fn close_inputs(
    request: &StrategySearchRequest,
    consumer_index: usize,
    bindings: &meld_lang::Bindings,
    selected: &mut Vec<(String, meld_lang::Operator)>,
    edges: &mut Vec<Edge>,
    visiting: &mut BTreeSet<String>,
    state: &mut SearchState<'_>,
) -> bool {
    // A complete Task carries each input or receives it from another contained step.
    let consumer = selected[consumer_index].1.clone();
    for input in consumer
        .resolution
        .requires_inputs
        .iter()
        .filter(|slot| slot.required)
    {
        if request.problem.task_inputs.iter().any(|value| {
            value.step_id == consumer.operator_id
                && value.validate().is_ok()
                && input.artifact_type == Term::ArtifactType(value.artifact_type_id.clone())
        }) {
            continue;
        }
        if let Some((_, producer)) = selected.iter().find(|(_, operator)| {
            operator
                .resolution
                .requires_outputs
                .iter()
                .any(|output| output.required && output.artifact_type == input.artifact_type)
        }) {
            if producer.operator_id == consumer.operator_id {
                state.reject(StrategyRejectionGround::UnclosedArtifact {
                    artifact_type: format!("{:?}", input.artifact_type),
                });
                return false;
            }
            let edge = Edge {
                from: producer.operator_id.clone(),
                to: consumer.operator_id.clone(),
                kind: EdgeKind::DataFlow {
                    artifact_type: input.artifact_type.clone(),
                },
            };
            if !edges.contains(&edge) {
                edges.push(edge);
            }
            continue;
        }
        let artifact_key = format!("{:?}", input.artifact_type);
        if !visiting.insert(artifact_key.clone()) {
            state.reject(StrategyRejectionGround::UnclosedArtifact {
                artifact_type: format!("{:?}", input.artifact_type),
            });
            return false;
        }
        let depth = visiting.len();
        if depth > request.bounds.max_depth {
            state.bounded = true;
            state.reject(StrategyRejectionGround::BoundsExceeded);
            return false;
        }
        let mut producers: Vec<_> = request
            .problem
            .capabilities
            .iter()
            .filter(|candidate| {
                candidate
                    .operator
                    .resolution
                    .requires_outputs
                    .iter()
                    .any(|output| output.required && output.artifact_type == input.artifact_type)
            })
            .collect();
        producers.sort_by(|left, right| left.contract_id.cmp(&right.contract_id));
        if producers.is_empty() {
            state.reject(StrategyRejectionGround::UnclosedArtifact {
                artifact_type: format!("{:?}", input.artifact_type),
            });
            return false;
        }
        let mut closed = false;
        for producer in producers {
            if !state.expand() {
                return false;
            }
            let Some(operator) = ground_operator(&producer.operator, bindings, state) else {
                continue;
            };
            let single = Composition {
                steps: vec![Step {
                    step_id: operator.operator_id.clone(),
                    kind: StepKind::Op(operator.clone()),
                }],
                edges: Vec::new(),
            };
            if !preconditions_hold(
                &single,
                &request.problem.planner_cut.world_model_view.world_state,
                state,
            ) {
                continue;
            }
            let mut trial_selected = selected.clone();
            let mut trial_edges = edges.clone();
            let mut trial_visiting = visiting.clone();
            let producer_index = trial_selected.len();
            let producer_id = operator.operator_id.clone();
            trial_selected.push((producer.contract_id.clone(), operator));
            if !close_inputs(
                request,
                producer_index,
                bindings,
                &mut trial_selected,
                &mut trial_edges,
                &mut trial_visiting,
                state,
            ) {
                continue;
            }
            trial_edges.push(Edge {
                from: producer_id,
                to: consumer.operator_id.clone(),
                kind: EdgeKind::DataFlow {
                    artifact_type: input.artifact_type.clone(),
                },
            });
            *selected = trial_selected;
            *edges = trial_edges;
            closed = true;
            break;
        }
        visiting.remove(&artifact_key);
        if !closed {
            state.reject(StrategyRejectionGround::UnclosedArtifact {
                artifact_type: format!("{:?}", input.artifact_type),
            });
            return false;
        }
    }
    true
}

fn finish_candidate(
    request: &StrategySearchRequest,
    rule: &StrategySettlementRule,
    settlement: &Proposition,
    bindings: meld_lang::Bindings,
    composition: Composition,
    origin: StrategyPlanOrigin,
    state: &mut SearchState<'_>,
) -> Option<StrategyPlan> {
    let bodies = independent_components(&composition)
        .into_iter()
        .map(|body| (bindings.clone(), body))
        .collect();
    finish_bodies(request, rule, settlement, bindings, bodies, origin, state)
}

fn finish_bodies(
    request: &StrategySearchRequest,
    rule: &StrategySettlementRule,
    settlement: &Proposition,
    bindings: meld_lang::Bindings,
    bodies: Vec<(meld_lang::Bindings, Composition)>,
    origin: StrategyPlanOrigin,
    state: &mut SearchState<'_>,
) -> Option<StrategyPlan> {
    let bindings = if bodies.len() == 1 {
        bodies[0].0.clone()
    } else {
        bindings
    };
    let mut tasks = Vec::new();
    for (local_bindings, composition) in bodies {
        if !meld_lang::validate(&composition).valid
            || !preconditions_hold(
                &composition,
                &request.problem.planner_cut.world_model_view.world_state,
                state,
            )
        {
            state.reject(StrategyRejectionGround::InvalidComposition);
            return None;
        }
        let Some(task) = complete_task(request, rule, composition, local_bindings, state) else {
            state.reject(StrategyRejectionGround::InvalidComposition);
            return None;
        };
        if !tasks
            .iter()
            .any(|prior: &StrategyTask| prior.task_id == task.task_id)
        {
            tasks.push(task);
        }
    }
    if tasks.is_empty() {
        return None;
    }
    if !tasks_contribute(&tasks, settlement) {
        return None;
    }
    let mut contract_ids: Vec<_> = tasks
        .iter()
        .flat_map(|task| task.capability_contract_ids.clone())
        .collect();
    contract_ids.sort();
    contract_ids.dedup();
    if !request.problem.capabilities.iter().any(|capability| {
        contract_ids.contains(&capability.contract_id)
            && capability.outcome_contract_id == rule.evidence_route.outcome_contract_id
    }) {
        state.reject(StrategyRejectionGround::InvalidEvidenceRoute);
        return None;
    }
    // Completed complete Tasks remain historical producers. They are not
    // reconstructed as fresh executable work merely because another local product waits.
    tasks.retain(|task| {
        !completed_task_history(state.history).iter().any(|entry|
        matches!(&entry.product, Some(StrategyProduct::Task(prior)) if same_work(task, prior)))
    });
    if tasks.is_empty() {
        state.reject(StrategyRejectionGround::UnchangedCompletedWork);
        return None;
    }
    let evaluation = evaluate_tasks(&tasks);
    let (epistemic_operations, dependencies) =
        match planned_epistemic_products(&request.problem, rule, &tasks, state.history) {
            Ok(products) => products,
            Err(ground) => {
                state.reject(ground);
                return None;
            }
        };
    let plan_family_id = plan_family_identity(&request.problem);
    let mut candidate = StrategyPlan {
        historical_identity: None,
        settlement_rule_id: settlement_rule_identity(rule),
        plan_revision_id: String::new(),
        plan_family_id,
        problem_id: request.problem.problem_id.clone(),
        goal_id: request.problem.goal.goal_id.clone(),
        planner_cut_id: request.problem.planner_cut.cut_id.clone(),
        origin,
        bindings,
        settlement_obligation: settlement.clone(),
        evidence_route: Some(rule.evidence_route.clone()),
        tasks,
        epistemic_operations,
        dependencies,
        conditions: vec![request.problem.goal.target.clone()],
        frozen_context_id: request.problem.planner_cut.context.context_id.clone(),
        explanation: format!(
            "Plan '{}' closes Goal '{}' through exact frozen products",
            request.problem.problem_id, request.problem.goal.goal_id
        ),
        predecessor_plan_revision_id: None,
        evaluation,
    };
    if !super::verification::dependencies_valid(&candidate, state.history) {
        state.reject(StrategyRejectionGround::InvalidComposition);
        return None;
    }
    candidate.plan_revision_id = plan_revision_identity(&candidate);
    Some(candidate)
}

fn complete_task(
    request: &StrategySearchRequest,
    rule: &StrategySettlementRule,
    composition: Composition,
    bindings: meld_lang::Bindings,
    state: &mut SearchState<'_>,
) -> Option<StrategyTask> {
    let outcome_contract_id =
        terminal_outcome_contract(&request.problem, &composition)?.to_string();
    let mut contract_ids: Vec<_> = composition
        .steps
        .iter()
        .map(|step| {
            let StepKind::Op(operator) = &step.kind else {
                return None;
            };
            let specific = operator.resolution.specific.as_ref()?;
            request
                .problem
                .capabilities
                .iter()
                .find(|capability| {
                    capability.operator.resolution.specific.as_ref() == Some(specific)
                })
                .map(|capability| capability.contract_id.clone())
        })
        .collect::<Option<Vec<_>>>()?;
    contract_ids.sort();
    contract_ids.dedup();
    let mut authority_requirements = composition
        .steps
        .iter()
        .filter_map(|step| match &step.kind {
            StepKind::Op(operator) => operator
                .resolution
                .specific
                .as_ref()
                .map(|specific| specific.capability_type_id.clone()),
            StepKind::Goal(_) => None,
        })
        .collect::<Vec<_>>();
    authority_requirements.sort();
    authority_requirements.dedup();
    let return_milestone = PlanMilestoneRequirement::ExecutionTerminal {
        task_id: "pending".to_string(),
    };
    let mut initial_inputs: Vec<_> = request
        .problem
        .task_inputs
        .iter()
        .filter(|input| {
            composition
                .steps
                .iter()
                .any(|step| step.step_id == input.step_id)
        })
        .cloned()
        .collect();
    initial_inputs.sort_by(|left, right| {
        (&left.step_id, &left.slot_id).cmp(&(&right.step_id, &right.slot_id))
    });
    let task_id = stable_id(
        "strategy-task-v1",
        &(
            &request.problem.goal.goal_id,
            &request.problem.planner_cut.source_basis_id(),
            &composition,
            &bindings,
            &contract_ids,
            &outcome_contract_id,
            &return_milestone,
        ),
    );
    let task_id = if initial_inputs.is_empty() {
        task_id
    } else {
        stable_id("strategy-task-inputs-v1", &(&task_id, &initial_inputs))
    };
    let task_id = stable_id(
        "strategy-task-subject-v1",
        &(&task_id, &request.problem.planner_cut.context.subject),
    );
    let effect_visibility = if rule.product_ordering.iter().any(|ordering| {
        ordering.milestone == StrategyDependencyMilestone::EffectVisible
            && match &ordering.before {
                StrategyProductSelector::AllTasks => true,
                StrategyProductSelector::Task { contract_id } => contract_ids.contains(contract_id),
                _ => false,
            }
    }) {
        let Some(expected) = request
            .problem
            .effect_visibility
            .clone()
            .filter(|expected| expected.validate().is_ok())
        else {
            state.reject(StrategyRejectionGround::InvalidEvidenceRoute);
            return None;
        };
        Some(expected)
    } else {
        None
    };
    let task_id = effect_visibility.as_ref().map_or_else(
        || task_id.clone(),
        |expected| stable_id("strategy-task-visibility-v1", &(&task_id, expected)),
    );
    let return_milestone = PlanMilestoneRequirement::ExecutionTerminal {
        task_id: task_id.clone(),
    };
    let task = StrategyTask {
        source_basis_id: task_source_basis(&request.problem, rule),
        effect_visibility,
        execution_subject: Some(request.problem.planner_cut.context.subject.clone()),
        initial_inputs,
        task_id: task_id.clone(),
        composition: composition.clone(),
        bindings: bindings.clone(),
        capability_contract_ids: contract_ids.clone(),
        expected_outcome_contract_id: outcome_contract_id,
        authority_requirements,
        idempotency_key: format!("task::{task_id}"),
        return_milestone: Some(return_milestone),
    };
    Some(task)
}

fn preconditions_hold(
    composition: &Composition,
    world_state: &meld_lang::WorldState,
    state: &mut SearchState<'_>,
) -> bool {
    for step in &composition.steps {
        let StepKind::Op(operator) = &step.kind else {
            continue;
        };
        for precondition in &operator.preconditions {
            match evaluate(world_state, precondition) {
                EvalResult::Satisfied => {}
                EvalResult::Unsatisfied { .. } => {
                    state.reject(StrategyRejectionGround::UnsatisfiedPrecondition {
                        operator_id: operator.operator_id.clone(),
                    });
                    return false;
                }
                EvalResult::Indeterminate { .. } => {
                    state.reject(StrategyRejectionGround::IndeterminatePrecondition {
                        operator_id: operator.operator_id.clone(),
                    });
                    return false;
                }
            }
        }
    }
    true
}

fn effect_proposition(effect: &Effect) -> Option<&Proposition> {
    match effect {
        Effect::Assert(proposition) => Some(proposition),
        Effect::Retract(_) | Effect::Update { .. } => None,
    }
}

pub(crate) fn composition_contributes(composition: &Composition, obligation: &Proposition) -> bool {
    fn supports(effect: &Proposition, wanted: &Proposition) -> bool {
        effect == wanted
            || matches!(effect, Proposition::All(children) if children.iter().any(|child| supports(child, wanted)))
    }
    let exact = composition.steps.iter().any(|step| match &step.kind {
        StepKind::Op(operator) => operator
            .effects
            .iter()
            .filter_map(effect_proposition)
            .any(|effect| supports(effect, obligation)),
        StepKind::Goal(_) => false,
    });
    exact
        || match obligation {
            Proposition::All(children) => {
                !children.is_empty()
                    && children
                        .iter()
                        .all(|child| composition_contributes(composition, child))
            }
            Proposition::Any(children) => children
                .iter()
                .any(|child| composition_contributes(composition, child)),
            _ => false,
        }
}

pub(crate) fn terminal_outcome_contract<'a>(
    problem: &'a StrategyProblem,
    composition: &Composition,
) -> Option<&'a str> {
    let sinks: Vec<_> = composition
        .steps
        .iter()
        .filter(|step| {
            !composition
                .edges
                .iter()
                .any(|edge| edge.from == step.step_id)
        })
        .collect();
    let [sink] = sinks.as_slice() else {
        return None;
    };
    let StepKind::Op(operator) = &sink.kind else {
        return None;
    };
    let specific = operator.resolution.specific.as_ref()?;
    problem
        .capabilities
        .iter()
        .find(|capability| capability.operator.resolution.specific.as_ref() == Some(specific))
        .map(|capability| capability.outcome_contract_id.as_str())
}

fn independent_components(composition: &Composition) -> Vec<Composition> {
    let mut remaining: BTreeSet<_> = composition
        .steps
        .iter()
        .map(|step| step.step_id.as_str())
        .collect();
    let mut components = Vec::new();
    while let Some(first) = remaining.iter().next().copied() {
        let mut members = BTreeSet::from([first]);
        loop {
            let before = members.len();
            for edge in &composition.edges {
                if members.contains(edge.from.as_str()) || members.contains(edge.to.as_str()) {
                    members.insert(edge.from.as_str());
                    members.insert(edge.to.as_str());
                }
            }
            if members.len() == before {
                break;
            }
        }
        remaining.retain(|id| !members.contains(id));
        components.push(Composition {
            steps: composition
                .steps
                .iter()
                .filter(|step| members.contains(step.step_id.as_str()))
                .cloned()
                .collect(),
            edges: composition
                .edges
                .iter()
                .filter(|edge| members.contains(edge.from.as_str()))
                .cloned()
                .collect(),
        });
    }
    components
}

fn ground_operator(
    operator: &meld_lang::Operator,
    bindings: &meld_lang::Bindings,
    state: &mut SearchState<'_>,
) -> Option<meld_lang::Operator> {
    let composition = Composition {
        steps: vec![Step {
            step_id: operator.operator_id.clone(),
            kind: StepKind::Op(operator.clone()),
        }],
        edges: Vec::new(),
    };
    match substitute(&composition, bindings) {
        Ok(composition) => match composition.steps.into_iter().next()?.kind {
            StepKind::Op(operator) => Some(operator),
            StepKind::Goal(_) => None,
        },
        Err(error) => {
            for unbound in error.unbound_variables {
                state.reject(StrategyRejectionGround::UnboundVariable {
                    variable: unbound.variable,
                });
            }
            None
        }
    }
}

pub(crate) fn ground_proposition(
    proposition: &Proposition,
    bindings: &meld_lang::Bindings,
) -> Result<Proposition, String> {
    let composition = Composition {
        steps: vec![Step {
            step_id: "settlement".to_string(),
            kind: StepKind::Goal(proposition.clone()),
        }],
        edges: Vec::new(),
    };
    substitute(&composition, bindings)
        .map_err(|error| error.unbound_variables[0].variable.clone())
        .and_then(
            |composition| match composition.steps.into_iter().next().unwrap().kind {
                StepKind::Goal(proposition) => Ok(proposition),
                StepKind::Op(_) => unreachable!(),
            },
        )
}

fn evaluate_candidate(composition: &Composition) -> StrategyPlanEvaluation {
    let mut time_ms = 0;
    let mut provider_calls = 0;
    for step in &composition.steps {
        if let StepKind::Op(operator) = &step.kind {
            time_ms += operator.cost.time_ms;
            provider_calls += operator.cost.provider_calls;
        }
    }
    StrategyPlanEvaluation {
        step_count: composition.steps.len(),
        time_ms,
        provider_calls,
    }
}

fn evaluate_tasks(tasks: &[StrategyTask]) -> StrategyPlanEvaluation {
    tasks
        .iter()
        .map(|task| evaluate_candidate(&task.composition))
        .fold(
            StrategyPlanEvaluation {
                step_count: 0,
                time_ms: 0,
                provider_calls: 0,
            },
            |mut total, local| {
                total.step_count += local.step_count;
                total.time_ms += local.time_ms;
                total.provider_calls += local.provider_calls;
                total
            },
        )
}

/// Check prospective contribution directly over the complete Task products.
pub(crate) fn tasks_and_history_contribute(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
    tasks: &[StrategyTask],
    history: &[StrategyCompletedHistoryEntry],
    obligation: &Proposition,
) -> bool {
    let mut products = tasks.to_vec();
    products.extend(
        current_completed_tasks(problem, rule, history)
            .into_iter()
            .cloned(),
    );
    tasks_contribute(&products, obligation)
}

/// Historical producers contribute only to the same subject and material inputs.
/// Their accepted owner milestone remains the authority for completion.
pub(crate) fn current_completed_tasks<'a>(
    problem: &StrategyProblem,
    rule: &StrategySettlementRule,
    history: &'a [StrategyCompletedHistoryEntry],
) -> Vec<&'a StrategyTask> {
    let basis = task_source_basis(problem, rule);
    completed_task_history(history)
        .into_iter()
        .filter_map(|entry| match &entry.product {
            Some(StrategyProduct::Task(task))
                if task.source_basis_id == basis
                    && task.execution_subject.as_ref()
                        == Some(&problem.planner_cut.context.subject) =>
            {
                Some(task.as_ref())
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn tasks_contribute(tasks: &[StrategyTask], obligation: &Proposition) -> bool {
    tasks
        .iter()
        .any(|task| composition_contributes(&task.composition, obligation))
        || match obligation {
            Proposition::All(children) => {
                !children.is_empty() && children.iter().all(|child| tasks_contribute(tasks, child))
            }
            Proposition::Any(children) => {
                children.iter().any(|child| tasks_contribute(tasks, child))
            }
            _ => false,
        }
}

fn candidate_key(candidate: &StrategyPlan) -> (usize, u64, u32, &str) {
    (
        candidate.evaluation.step_count,
        candidate.evaluation.time_ms,
        candidate.evaluation.provider_calls,
        &candidate.plan_revision_id,
    )
}

fn alternative_candidate_key(candidate: &StrategyPlan) -> (u64, u32, usize, &str) {
    (
        candidate.evaluation.time_ms,
        candidate.evaluation.provider_calls,
        candidate.evaluation.step_count,
        &candidate.plan_revision_id,
    )
}

pub(crate) fn settlement_rule_identity(rule: &StrategySettlementRule) -> String {
    stable_id("strategy-settlement-rule-v1", rule)
}

pub(crate) fn selected_rule<'a>(
    problem: &'a StrategyProblem,
    plan: &StrategyPlan,
) -> Option<&'a StrategySettlementRule> {
    if !plan.settlement_rule_id.is_empty() {
        return problem.theory.settlement_rules.iter().find(|rule| {
            settlement_rule_identity(rule) == plan.settlement_rule_id
                && unify(&rule.goal_pattern, &problem.goal.target).is_some()
        });
    }
    if plan.historical_identity.is_none() || !super::history::predecessor_identity_valid(plan) {
        return None;
    }
    // Historical Plans did not name their rule. Only a unique exact meaning
    // can be recovered; ambiguity does not authorize another selection.
    let mut matches = problem.theory.settlement_rules.iter().filter(|rule| {
        unify(&rule.goal_pattern, &problem.goal.target).is_some_and(|bindings| {
            ground_proposition(&rule.settlement_obligation, &bindings).as_ref()
                == Ok(&plan.settlement_obligation)
                && plan.evidence_route.as_ref() == Some(&rule.evidence_route)
        })
    });
    let selected = matches.next()?;
    matches.next().is_none().then_some(selected)
}

pub(crate) fn plan_revision_identity(candidate: &StrategyPlan) -> String {
    let mut identity = candidate.clone();
    identity.plan_revision_id.clear();
    let bytes =
        serde_json::to_vec(&identity).expect("Strategy candidate serialization is infallible");
    format!("strategy-plan-v2::{}", blake3::hash(&bytes).to_hex())
}

pub(crate) fn plan_family_identity(problem: &StrategyProblem) -> String {
    stable_id(
        "strategy-plan-family-v1",
        &(
            problem.goal.agent_id.as_str(),
            problem.goal.goal_id.as_str(),
        ),
    )
}

fn stable_id(namespace: &str, value: &impl serde::Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("Strategy identity is serializable");
    format!("{namespace}::{}", blake3::hash(&bytes).to_hex())
}

pub(crate) fn candidate_evaluation(candidate: &StrategyPlan) -> StrategyPlanEvaluation {
    evaluate_tasks(&candidate.tasks)
}
