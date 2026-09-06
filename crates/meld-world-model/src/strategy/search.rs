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
    let mut state = SearchState::new(request);
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
            plan_revision_id: String::new(),
            plan_family_id: plan_family_identity(&request.problem),
            problem_id: request.problem.problem_id.clone(),
            goal_id: request.problem.goal.goal_id.clone(),
            planner_cut_id: request.problem.planner_cut.cut_id.clone(),
            origin: StrategyPlanOrigin::Satisfied,
            composition: Composition {
                steps: Vec::new(),
                edges: Vec::new(),
            },
            bindings: meld_lang::Bindings::empty(),
            settlement_obligation: request.problem.goal.target.clone(),
            evidence_route: None,
            capability_contract_ids: Vec::new(),
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

    let Some((rule, bindings)) = request
        .problem
        .theory
        .settlement_rules
        .iter()
        .filter_map(|rule| {
            unify(&rule.goal_pattern, &request.problem.goal.target).map(|b| (rule, b))
        })
        .next()
    else {
        state.reject(StrategyRejectionGround::NoSettlementRule);
        return state.finish(None);
    };

    let settlement = match ground_proposition(&rule.settlement_obligation, &bindings) {
        Ok(settlement) => settlement,
        Err(variable) => {
            state.reject(StrategyRejectionGround::UnboundVariable { variable });
            return state.finish(None);
        }
    };

    // Method seeds and novel construction converge before ranking so cached
    // knowledge cannot bypass the same eligibility and identity rules.
    let mut candidates = method_candidates(request, rule, &settlement, &bindings, &mut state);
    if !state.bounded {
        candidates.extend(direct_candidates(
            request,
            rule,
            &settlement,
            &bindings,
            &mut state,
        ));
    }
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
    if predecessor.plan_revision_id != plan_revision_identity(predecessor)
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

    let confirmation = confirmation_successor(request);
    let result = confirmation.unwrap_or_else(|| search(&request.search));
    let recommendation = result.recommendation.map(|mut plan| {
        plan.plan_family_id = predecessor.plan_family_id.clone();
        plan.predecessor_plan_revision_id = Some(predecessor.plan_revision_id.clone());
        plan.plan_revision_id = plan_revision_identity(&plan);
        StrategySuccessorPlan {
            plan,
            completed_history: request.completed_history.clone(),
        }
    });
    StrategySuccessorResult {
        problem_id: result.problem_id,
        completion: result.completion,
        recommendation,
        rejections: result.rejections,
        statistics: result.statistics,
    }
}

fn confirmation_successor(request: &StrategySuccessorRequest) -> Option<StrategySearchResult> {
    let problem = &request.search.problem;
    let (rule, bindings) = problem.theory.settlement_rules.iter().find_map(|rule| {
        unify(&rule.goal_pattern, &problem.goal.target).map(|bindings| (rule, bindings))
    })?;
    if !rule.epistemic_placement.is_confirmation() {
        return None;
    }
    let settlement = ground_proposition(&rule.settlement_obligation, &bindings).ok()?;
    let completed = request.completed_history.iter().find(|entry| {
        let Some(StrategyProduct::Task(task)) = &entry.product else { return false; };
        entry.product_id == task.task_id
            && confirmation_expectation_matches(problem, rule, task)
            && entry.accepted_milestone == task.confirmation_milestone()
            && task.composition.steps.iter().any(|step| matches!(&step.kind, StepKind::Op(operator)
                if operator.effects.iter().filter_map(effect_proposition).any(|effect| effect == &settlement)))
    })?;
    let epistemic_operations = epistemic_products(problem);
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
    let dependencies = epistemic_operations
        .iter()
        .map(|operation| {
            dependency(
                &completed.product_id,
                &operation.product_id,
                completed.accepted_milestone.clone(),
            )
        })
        .collect();
    let mut plan = StrategyPlan {
        plan_revision_id: String::new(),
        plan_family_id: plan_family_identity(problem),
        problem_id: problem.problem_id.clone(),
        goal_id: problem.goal.goal_id.clone(),
        planner_cut_id: problem.planner_cut.cut_id.clone(),
        origin: StrategyPlanOrigin::Confirmation,
        composition: Composition {
            steps: Vec::new(),
            edges: Vec::new(),
        },
        bindings: meld_lang::Bindings::empty(),
        settlement_obligation: settlement,
        evidence_route: Some(rule.evidence_route.clone()),
        capability_contract_ids: Vec::new(),
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

pub(crate) fn confirmation_expectation_matches(
    problem: &StrategyProblem,
    rule: &super::StrategySettlementRule,
    task: &super::StrategyTask,
) -> bool {
    match rule.epistemic_placement {
        StrategyEpistemicPlacement::GraphConfirmation => {
            task.effect_visibility == problem.effect_visibility
                && task
                    .effect_visibility
                    .as_ref()
                    .is_some_and(|expected| expected.validate().is_ok())
        }
        StrategyEpistemicPlacement::Confirmation => task.effect_visibility.is_none(),
        StrategyEpistemicPlacement::Prerequisite => false,
    }
}

pub(crate) fn epistemic_products(problem: &StrategyProblem) -> Vec<StrategyEpistemicOperation> {
    let return_evidence = problem
        .theory
        .settlement_rules
        .iter()
        .find(|rule| unify(&rule.goal_pattern, &problem.goal.target).is_some())
        .filter(|rule| rule.epistemic_placement.is_confirmation())
        .map(|rule| rule.evidence_route.clone());
    problem
        .curation_operations
        .iter()
        .map(|operation| {
            let mut product_id = stable_id(
                "strategy-epistemic-product-v1",
                &(&problem.goal.goal_id, &operation.operation_id),
            );
            if let Some(route) = &return_evidence {
                product_id = stable_id("strategy-epistemic-return-v1", &(&product_id, route));
            }
            StrategyEpistemicOperation {
                return_evidence: return_evidence.clone(),
                idempotency_key: format!("curation::{product_id}"),
                product_id,
                operation: operation.clone(),
                authority_requirements: vec![operation.authority.agent_id.clone()],
            }
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

struct SearchState<'a> {
    request: &'a StrategySearchRequest,
    rejections: Vec<StrategyRejectionGround>,
    expanded: usize,
    bounded: bool,
}

impl<'a> SearchState<'a> {
    fn new(request: &'a StrategySearchRequest) -> Self {
        Self {
            request,
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

fn direct_candidates(
    request: &StrategySearchRequest,
    rule: &StrategySettlementRule,
    settlement: &Proposition,
    goal_bindings: &meld_lang::Bindings,
    state: &mut SearchState<'_>,
) -> Vec<StrategyPlan> {
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
        if let Some(candidate) = finish_candidate(
            request,
            rule,
            settlement,
            bindings,
            composition,
            StrategyPlanOrigin::Direct,
            state,
        ) {
            candidates.push(candidate);
        }
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
    let validation = meld_lang::validate(&composition);
    if !validation.valid {
        state.reject(StrategyRejectionGround::InvalidComposition);
        return None;
    }
    if !preconditions_hold(
        &composition,
        &request.problem.planner_cut.world_model_view.world_state,
        state,
    ) {
        return None;
    }
    let mut contract_ids = Vec::new();
    let mut evidence_outcome_supported = false;
    for step in &composition.steps {
        let StepKind::Op(operator) = &step.kind else {
            state.reject(StrategyRejectionGround::InvalidComposition);
            return None;
        };
        let Some(specific) = &operator.resolution.specific else {
            state.reject(StrategyRejectionGround::InvalidComposition);
            return None;
        };
        let Some(capability) =
            request.problem.capabilities.iter().find(|candidate| {
                candidate.operator.resolution.specific.as_ref() == Some(specific)
            })
        else {
            state.reject(StrategyRejectionGround::InvalidComposition);
            return None;
        };
        evidence_outcome_supported |=
            capability.outcome_contract_id == rule.evidence_route.outcome_contract_id;
        contract_ids.push(capability.contract_id.clone());
    }
    if !evidence_outcome_supported {
        state.reject(StrategyRejectionGround::InvalidEvidenceRoute);
        return None;
    }
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
    let evaluation = evaluate_candidate(&composition);
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
            &rule.evidence_route.outcome_contract_id,
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
    let effect_visibility =
        if rule.epistemic_placement == StrategyEpistemicPlacement::GraphConfirmation {
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
        effect_visibility,
        execution_subject: Some(request.problem.planner_cut.context.subject.clone()),
        initial_inputs,
        task_id: task_id.clone(),
        composition: composition.clone(),
        bindings: bindings.clone(),
        capability_contract_ids: contract_ids.clone(),
        expected_outcome_contract_id: rule.evidence_route.outcome_contract_id.clone(),
        authority_requirements,
        idempotency_key: format!("task::{task_id}"),
        return_milestone: Some(return_milestone),
    };
    let epistemic_operations = epistemic_products(&request.problem);
    let dependencies = epistemic_operations
        .iter()
        .map(|operation| match rule.epistemic_placement {
            StrategyEpistemicPlacement::Prerequisite => dependency(
                &operation.product_id,
                &task_id,
                PlanMilestoneRequirement::CurationTerminal {
                    operation_id: operation.operation.operation_id.clone(),
                },
            ),
            StrategyEpistemicPlacement::Confirmation
            | StrategyEpistemicPlacement::GraphConfirmation => dependency(
                &task_id,
                &operation.product_id,
                task.confirmation_milestone(),
            ),
        })
        .collect();
    let plan_family_id = plan_family_identity(&request.problem);
    let mut candidate = StrategyPlan {
        plan_revision_id: String::new(),
        plan_family_id,
        problem_id: request.problem.problem_id.clone(),
        goal_id: request.problem.goal.goal_id.clone(),
        planner_cut_id: request.problem.planner_cut.cut_id.clone(),
        origin,
        composition,
        bindings,
        settlement_obligation: settlement.clone(),
        evidence_route: Some(rule.evidence_route.clone()),
        capability_contract_ids: contract_ids,
        tasks: vec![task],
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
    candidate.plan_revision_id = plan_revision_identity(&candidate);
    Some(candidate)
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

fn composition_contributes(composition: &Composition, obligation: &Proposition) -> bool {
    composition.steps.iter().any(|step| match &step.kind {
        StepKind::Op(operator) => operator
            .effects
            .iter()
            .filter_map(effect_proposition)
            .any(|effect| effect == obligation),
        StepKind::Goal(_) => false,
    })
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

pub(crate) fn plan_revision_identity(candidate: &StrategyPlan) -> String {
    let mut identity = candidate.clone();
    identity.plan_revision_id.clear();
    let bytes =
        serde_json::to_vec(&identity).expect("Strategy candidate serialization is infallible");
    format!("strategy-plan-v1::{}", blake3::hash(&bytes).to_hex())
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
    evaluate_candidate(&candidate.composition)
}
