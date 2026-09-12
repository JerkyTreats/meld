//! Conditional method refinement over private hypothetical state.
//!
//! The resulting primitive order is explicit in the executable graph. Neither
//! projected effects nor method conclusions are published as observed knowledge.

use std::collections::{BTreeMap, BTreeSet};

use meld_lang::{
    evaluate, substitute, unify, Bindings, Composition, Edge, EdgeKind, EvalResult, Method,
    Proposition, Step, StepKind, WorldState,
};

use super::{contracts::*, search::SearchState};

#[derive(Clone)]
pub(super) struct Refined {
    pub composition: Composition,
    pub decomposition: StrategyDecomposition,
    world: WorldState,
}

impl Refined {
    fn empty(world: WorldState) -> Self {
        Self {
            composition: Composition {
                steps: vec![],
                edges: vec![],
            },
            decomposition: StrategyDecomposition {
                expansion_order: vec![],
                refinements: vec![],
                primitives: vec![],
            },
            world,
        }
    }

    fn append(&mut self, next: Self) {
        let exits: Vec<_> = self
            .composition
            .steps
            .iter()
            .filter(|step| {
                !self
                    .composition
                    .edges
                    .iter()
                    .any(|edge| edge.from == step.step_id)
            })
            .collect();
        let entries: Vec<_> = next
            .composition
            .steps
            .iter()
            .filter(|step| {
                !next
                    .composition
                    .edges
                    .iter()
                    .any(|edge| edge.to == step.step_id)
            })
            .collect();
        for before in exits {
            for after in &entries {
                self.composition.edges.push(Edge {
                    from: before.step_id.clone(),
                    to: after.step_id.clone(),
                    kind: EdgeKind::Ordering,
                });
            }
        }
        self.decomposition
            .expansion_order
            .extend(next.decomposition.expansion_order);
        self.composition.steps.extend(next.composition.steps);
        self.composition.edges.extend(next.composition.edges);
        self.decomposition
            .refinements
            .extend(next.decomposition.refinements);
        self.decomposition
            .primitives
            .extend(next.decomposition.primitives);
        self.world = next.world;
    }
}

pub(super) fn construct(
    request: &StrategySearchRequest,
    method: &Method,
    bindings: &Bindings,
    state: &mut SearchState<'_>,
) -> Vec<Refined> {
    Engine {
        request,
        replay: None,
    }
    .method(
        method,
        &request.problem.goal.target,
        bindings,
        "",
        0,
        &request.problem.planner_cut.world_model_view.world_state,
        state,
    )
}

struct Engine<'a> {
    request: &'a StrategySearchRequest,
    replay: Option<&'a StrategyDecomposition>,
}

impl Engine<'_> {
    #[allow(clippy::too_many_arguments)]
    fn method(
        &self,
        method: &Method,
        target: &Proposition,
        bindings: &Bindings,
        path: &str,
        depth: usize,
        world: &WorldState,
        budget: &mut SearchState<'_>,
    ) -> Vec<Refined> {
        if depth > self.request.bounds.max_depth {
            budget.bounded = true;
            budget.reject(StrategyRejectionGround::BoundsExceeded);
            return vec![];
        }
        let choice = StrategyRefinement::Method {
            path: path.into(),
            target: target.clone(),
            method_id: method.method_id.clone(),
            bindings: bindings.clone(),
        };
        if self
            .replay
            .is_some_and(|proof| !proof.refinements.contains(&choice))
        {
            return vec![];
        }
        if !budget.expand() {
            return vec![];
        }
        for condition in &method.preconditions {
            let Ok(condition) = super::search::ground_proposition(condition, bindings) else {
                budget.reject(StrategyRejectionGround::InvalidComposition);
                return vec![];
            };
            let ground = match evaluate(world, &condition) {
                EvalResult::Satisfied => continue,
                EvalResult::Unsatisfied { .. } => {
                    StrategyRejectionGround::UnsatisfiedMethodPrecondition {
                        method_id: method.method_id.clone(),
                    }
                }
                EvalResult::Indeterminate { .. } => {
                    StrategyRejectionGround::IndeterminateMethodPrecondition {
                        method_id: method.method_id.clone(),
                    }
                }
            };
            budget.reject(ground);
            return vec![];
        }
        let Ok(body) = substitute(&method.composition, bindings) else {
            budget.reject(StrategyRejectionGround::InvalidComposition);
            return vec![];
        };
        if !meld_lang::validate(&body).valid
            || body.steps.is_empty()
            || body
                .edges
                .iter()
                .any(|edge| matches!(edge.kind, EdgeKind::Conditional { .. }))
        {
            budget.reject(StrategyRejectionGround::InvalidComposition);
            return vec![];
        }
        // Primitive method bodies are the recursion base case. Preserve their
        // existing partial order and occurrence identities exactly.
        if body
            .steps
            .iter()
            .all(|step| matches!(step.kind, StepKind::Op(_)))
        {
            if let Err(ground) = composition_ground(world, &body) {
                budget.reject(ground);
                return vec![];
            }
            let mut projected = world.clone();
            for step in ordered_steps(&body).unwrap_or_default() {
                let StepKind::Op(operator) = &step.kind else {
                    unreachable!()
                };
                let Ok(next) = projected.apply(&operator.effects) else {
                    return vec![];
                };
                projected = next;
            }
            if !path.is_empty() && evaluate(&projected, target) != EvalResult::Satisfied {
                return vec![];
            }
            let mut composition = body;
            let rename = |id: &str| {
                if path.is_empty() {
                    id.to_string()
                } else {
                    format!("{path}/{id}")
                }
            };
            for step in &mut composition.steps {
                step.step_id = rename(&step.step_id);
            }
            for edge in &mut composition.edges {
                edge.from = rename(&edge.from);
                edge.to = rename(&edge.to);
            }
            let primitives = composition
                .steps
                .iter()
                .map(|step| StrategyPrimitiveBinding {
                    step_id: step.step_id.clone(),
                    bindings: bindings.clone(),
                })
                .collect();
            return vec![Refined {
                world: projected,
                decomposition: StrategyDecomposition {
                    expansion_order: composition
                        .steps
                        .iter()
                        .map(|step| step.step_id.clone())
                        .collect(),
                    refinements: vec![choice],
                    primitives,
                },
                composition,
            }];
        }
        let mut seed = Refined::empty(world.clone());
        seed.decomposition.refinements.push(choice);
        let mut results = self.body(&body, bindings, path, depth, BTreeMap::new(), seed, budget);
        if !path.is_empty() {
            results.retain(|result| evaluate(&result.world, target) == EvalResult::Satisfied);
        }
        results
    }

    #[allow(clippy::too_many_arguments)]
    fn body(
        &self,
        body: &Composition,
        bindings: &Bindings,
        path: &str,
        depth: usize,
        done: BTreeMap<String, Vec<String>>,
        built: Refined,
        budget: &mut SearchState<'_>,
    ) -> Vec<Refined> {
        if done.len() == body.steps.len() {
            let mut result = built;
            for edge in &body.edges {
                let (Some(from), Some(to)) = (done[&edge.from].last(), done[&edge.to].first())
                else {
                    if !matches!(edge.kind, EdgeKind::Ordering) {
                        return vec![];
                    }
                    continue;
                };
                let mapped = Edge {
                    from: from.clone(),
                    to: to.clone(),
                    kind: edge.kind.clone(),
                };
                if !result.composition.edges.contains(&mapped) {
                    result.composition.edges.push(mapped);
                }
            }
            return vec![result];
        }
        let mut ready: Vec<_> = body
            .steps
            .iter()
            .filter(|step| {
                !done.contains_key(&step.step_id)
                    && body
                        .edges
                        .iter()
                        .filter(|edge| edge.to == step.step_id)
                        .all(|edge| done.contains_key(&edge.from))
            })
            .collect();
        ready.sort_by(|a, b| a.step_id.cmp(&b.step_id));
        if let Some(proof) = self.replay {
            // Replay follows the retained traversal rather than spending a new
            // search budget enumerating other legal topological orders.
            ready.sort_by_key(|step| {
                let occurrence = if path.is_empty() {
                    step.step_id.clone()
                } else {
                    format!("{path}/{}", step.step_id)
                };
                proof
                    .expansion_order
                    .iter()
                    .position(|id| id == &occurrence)
                    .unwrap_or(usize::MAX)
            });
            ready.truncate(1);
        }
        let mut results = vec![];
        for step in ready {
            if !budget.expand() {
                break;
            }
            let child_path = if path.is_empty() {
                step.step_id.clone()
            } else {
                format!("{path}/{}", step.step_id)
            };
            let alternatives =
                match &step.kind {
                    StepKind::Op(operator) => {
                        if self.replay.is_some_and(|proof| {
                            !proof
                                .primitives
                                .iter()
                                .any(|p| p.step_id == child_path && p.bindings == *bindings)
                        }) {
                            continue;
                        }
                        if operator.preconditions.iter().any(|condition| {
                            evaluate(&built.world, condition) != EvalResult::Satisfied
                        }) {
                            budget.reject(StrategyRejectionGround::UnsatisfiedPrecondition {
                                operator_id: operator.operator_id.clone(),
                            });
                            continue;
                        }
                        let Ok(world) = built.world.apply(&operator.effects) else {
                            continue;
                        };
                        let mut leaf = Refined::empty(world);
                        leaf.composition.steps.push(Step {
                            step_id: child_path.clone(),
                            kind: step.kind.clone(),
                        });
                        leaf.decomposition
                            .primitives
                            .push(StrategyPrimitiveBinding {
                                step_id: child_path.clone(),
                                bindings: bindings.clone(),
                            });
                        vec![leaf]
                    }
                    StepKind::Goal(target) => {
                        if evaluate(&built.world, target) == EvalResult::Satisfied {
                            let mut leaf = Refined::empty(built.world.clone());
                            leaf.decomposition
                                .refinements
                                .push(StrategyRefinement::Satisfied {
                                    path: child_path.clone(),
                                    target: target.clone(),
                                });
                            vec![leaf]
                        } else {
                            let mut methods: Vec<_> = self.request.problem.methods.iter().collect();
                            methods.sort_by_key(|method| (method.preference, &method.method_id));
                            let mut alternatives = vec![];
                            for method in methods {
                                if let Some(local) = unify(&method.trigger, target) {
                                    alternatives.extend(self.method(
                                        method,
                                        target,
                                        &local,
                                        &child_path,
                                        depth + 1,
                                        &built.world,
                                        budget,
                                    ));
                                }
                                if budget.bounded {
                                    break;
                                }
                            }
                            alternatives
                        }
                    }
                };
            for next in alternatives {
                let mut next_done = done.clone();
                next_done.insert(
                    step.step_id.clone(),
                    next.composition
                        .steps
                        .iter()
                        .map(|s| s.step_id.clone())
                        .collect(),
                );
                let mut combined = built.clone();
                combined
                    .decomposition
                    .expansion_order
                    .push(child_path.clone());
                combined.append(next);
                results.extend(self.body(body, bindings, path, depth, next_done, combined, budget));
                if budget.bounded {
                    break;
                }
            }
            if budget.bounded {
                break;
            }
        }
        results
    }
}

/// Bind required artifacts to an earlier producer in the selected primitive order.
pub(super) fn wire_inputs(problem: &StrategyProblem, composition: &mut Composition) -> bool {
    let Ok(ordered) = ordered_steps(composition) else {
        return false;
    };
    let ordered: Vec<_> = ordered.into_iter().cloned().collect();
    for (index, step) in ordered.iter().enumerate() {
        let StepKind::Op(operator) = &step.kind else {
            return false;
        };
        for input in operator
            .resolution
            .requires_inputs
            .iter()
            .filter(|slot| slot.required)
        {
            if problem.task_inputs.iter().any(|value| {
                value.step_id == step.step_id
                    && input.artifact_type
                        == meld_lang::Term::ArtifactType(value.artifact_type_id.clone())
            }) {
                continue;
            }
            if composition.edges.iter().any(|edge| {
                edge.to == step.step_id
                    && edge.kind
                        == EdgeKind::DataFlow {
                            artifact_type: input.artifact_type.clone(),
                        }
            }) {
                continue;
            }
            let producer = ordered[..index].iter().rev().find(|step| matches!(&step.kind,
                StepKind::Op(op) if op.resolution.requires_outputs.iter().any(|output| output.required && output.artifact_type == input.artifact_type)));
            let Some(producer) = producer else {
                return false;
            };
            let edge = Edge {
                from: producer.step_id.clone(),
                to: step.step_id.clone(),
                kind: EdgeKind::DataFlow {
                    artifact_type: input.artifact_type.clone(),
                },
            };
            if !composition.edges.contains(&edge) {
                composition.edges.push(edge);
            }
        }
    }
    true
}

/// Replay the selected hierarchy, without selecting an alternative method.
pub(super) fn verifies(
    problem: &StrategyProblem,
    plan: &StrategyPlan,
    history: &[StrategyCompletedHistoryEntry],
) -> bool {
    let Some(proof) = &plan.decomposition else {
        return true;
    };
    let StrategyPlanOrigin::Method { method_id } = &plan.origin else {
        return false;
    };
    let Some(method) = problem
        .methods
        .iter()
        .find(|method| &method.method_id == method_id)
    else {
        return false;
    };
    let request = StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: (proof.primitives.len() + proof.refinements.len() + 1)
                .saturating_mul(32),
            max_depth: proof.refinements.len(),
        },
    };
    let mut budget = SearchState::new(&request);
    Engine {
        request: &request,
        replay: Some(proof),
    }
    .method(
        method,
        &problem.goal.target,
        &plan.bindings,
        "",
        0,
        &problem.planner_cut.world_model_view.world_state,
        &mut budget,
    )
    .into_iter()
    .any(|mut result| {
        if result.decomposition != *proof || !wire_inputs(problem, &mut result.composition) {
            return false;
        }
        let bodies = super::search::independent_components(&result.composition);
        plan.tasks
            .iter()
            .all(|task| bodies.contains(&task.composition))
            && bodies.iter().all(|body| {
                plan.tasks.iter().any(|task| &task.composition == body)
                    || super::search::completed_task_history(history)
                        .iter()
                        .any(|entry| {
                            matches!(&entry.product,
                        Some(StrategyProduct::Task(task)) if &task.composition == body)
                        })
            })
    })
}

/// Preconditions can use only effects of ordered predecessors, never unrelated work.
pub(super) fn composition_ground(
    problem: &WorldState,
    composition: &Composition,
) -> Result<(), StrategyRejectionGround> {
    let ordered = ordered_steps(composition)?;
    for step in &ordered {
        let StepKind::Op(operator) = &step.kind else {
            return Err(StrategyRejectionGround::InvalidComposition);
        };
        let mut ancestors = BTreeSet::new();
        loop {
            let before = ancestors.len();
            for edge in &composition.edges {
                if !matches!(edge.kind, EdgeKind::Conditional { .. })
                    && (edge.to == step.step_id || ancestors.contains(&edge.to))
                {
                    ancestors.insert(edge.from.clone());
                }
            }
            if before == ancestors.len() {
                break;
            }
        }
        let mut world = problem.clone();
        for prior in &ordered {
            if ancestors.contains(&prior.step_id) {
                let StepKind::Op(op) = &prior.kind else {
                    return Err(StrategyRejectionGround::InvalidComposition);
                };
                world = world
                    .apply(&op.effects)
                    .map_err(|_| StrategyRejectionGround::InvalidComposition)?;
            }
        }
        for precondition in &operator.preconditions {
            match evaluate(&world, precondition) {
                EvalResult::Satisfied => {}
                EvalResult::Unsatisfied { .. } => {
                    return Err(StrategyRejectionGround::UnsatisfiedPrecondition {
                        operator_id: operator.operator_id.clone(),
                    })
                }
                EvalResult::Indeterminate { .. } => {
                    return Err(StrategyRejectionGround::IndeterminatePrecondition {
                        operator_id: operator.operator_id.clone(),
                    })
                }
            }
        }
    }
    Ok(())
}

fn ordered_steps(composition: &Composition) -> Result<Vec<&Step>, StrategyRejectionGround> {
    let mut done = BTreeSet::new();
    let mut ordered = vec![];
    while ordered.len() < composition.steps.len() {
        let Some(step) = composition
            .steps
            .iter()
            .filter(|step| !done.contains(&step.step_id))
            .filter(|step| {
                composition
                    .edges
                    .iter()
                    .filter(|edge| edge.to == step.step_id)
                    .all(|edge| done.contains(&edge.from))
            })
            .min_by(|a, b| a.step_id.cmp(&b.step_id))
        else {
            return Err(StrategyRejectionGround::InvalidComposition);
        };
        done.insert(step.step_id.clone());
        ordered.push(step);
    }
    Ok(ordered)
}
