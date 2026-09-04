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

    let result = search(&request.search);
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
    // Close required artifacts backward from each consumer. Existing world
    // artifacts terminate a branch without manufacturing a producer step.
    let consumer = selected[consumer_index].1.clone();
    let scope = goal_scope(&request.problem.goal.target);
    for input in consumer
        .resolution
        .requires_inputs
        .iter()
        .filter(|slot| slot.required)
    {
        if existing_artifact(
            &request.problem.planner_cut.world_model_view.world_state,
            scope.as_ref(),
            &input.artifact_type,
        ) {
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
    let task_id = stable_id(
        "strategy-task-v1",
        &(
            &request.problem.goal.goal_id,
            &request.problem.planner_cut.cut_id,
            &composition,
            &bindings,
            &contract_ids,
            &rule.evidence_route.outcome_contract_id,
            &return_milestone,
        ),
    );
    let return_milestone = PlanMilestoneRequirement::ExecutionTerminal {
        task_id: task_id.clone(),
    };
    let task = StrategyTask {
        task_id: task_id.clone(),
        composition: composition.clone(),
        bindings: bindings.clone(),
        capability_contract_ids: contract_ids.clone(),
        expected_outcome_contract_id: rule.evidence_route.outcome_contract_id.clone(),
        authority_requirements,
        idempotency_key: format!("task::{task_id}"),
        return_milestone: Some(return_milestone),
    };
    let epistemic_operations = request
        .problem
        .curation_operations
        .first()
        .map(|operation| {
            let product_id = stable_id(
                "strategy-epistemic-product-v1",
                &(&request.problem.goal.goal_id, &operation.operation_id),
            );
            StrategyEpistemicOperation {
                idempotency_key: format!("curation::{product_id}"),
                product_id,
                operation: operation.clone(),
                authority_requirements: vec![operation.authority.agent_id.clone()],
            }
        })
        .into_iter()
        .collect::<Vec<_>>();
    let dependencies = epistemic_operations
        .first()
        .map(|operation| StrategyPlanDependency {
            dependency_id: stable_id(
                "strategy-dependency-v1",
                &(
                    &operation.product_id,
                    &task_id,
                    &operation.operation.operation_id,
                ),
            ),
            producer_product_id: operation.product_id.clone(),
            consumer_product_id: task_id.clone(),
            required_milestone: PlanMilestoneRequirement::CurationTerminal {
                operation_id: operation.operation.operation_id.clone(),
            },
        })
        .into_iter()
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
        evidence_route: rule.evidence_route.clone(),
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

fn ground_proposition(
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

fn existing_artifact(
    world_state: &meld_lang::WorldState,
    scope: Option<&Term>,
    artifact_type: &Term,
) -> bool {
    scope.is_some_and(|scope| {
        world_state.satisfies(&Proposition::Exists {
            scope: scope.clone(),
            artifact_type: artifact_type.clone(),
        })
    })
}

fn goal_scope(goal: &Proposition) -> Option<Term> {
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
