use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    composition::{Composition, EdgeKind, Step, StepKind},
    condition::Condition,
    cost::CostEstimate,
    effect::Effect,
    operator::Operator,
    proposition::Proposition,
    term::Term,
};

/// Structural validation result for a composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationResult {
    /// True when no validation errors were found.
    pub valid: bool,
    /// Structural errors that prevent execution.
    pub errors: Vec<ValidationError>,
    /// Non-blocking quality warnings.
    pub warnings: Vec<ValidationWarning>,
}

/// Structural validation errors for compositions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationError {
    /// An edge references a missing step.
    DanglingEdge {
        /// Edge source.
        from: String,
        /// Edge target.
        to: String,
        /// Missing endpoint identity.
        missing: String,
    },
    /// A step identity appears more than once.
    DuplicateStepId(String),
    /// The composition contains a cycle.
    CycleDetected {
        /// Steps involved in the detected cycle.
        involved_steps: Vec<String>,
    },
    /// A data flow edge references an artifact the source does not produce.
    ArtifactSourceMismatch {
        /// Source step.
        edge_from: String,
        /// Target step.
        edge_to: String,
        /// Missing artifact type.
        artifact_type: String,
    },
    /// A conditional edge references a goal step as its source.
    InvalidGuardOnGoalStep {
        /// Source step.
        edge_from: String,
    },
    /// A variable remains where substitution should have resolved it.
    UnboundVariable {
        /// Step containing the variable.
        step_id: String,
        /// Variable name.
        variable: String,
    },
}

/// Structural validation warnings for compositions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationWarning {
    /// A step after the first has no incoming edges.
    DisconnectedStep(String),
    /// An operator effect is not consumed downstream.
    UnusedEffects {
        /// Step with unused effects.
        step_id: String,
    },
    /// Aggregated cost exceeds an informational threshold.
    HighCost {
        /// Aggregated estimate.
        estimated: CostEstimate,
    },
}

/// Validate composition structure without resolving capabilities or checking goal achievement.
pub fn validate(composition: &Composition) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let step_map = build_step_map(&composition.steps, &mut errors);

    check_edges(composition, &step_map, &mut errors);
    check_cycles(composition, &step_map, &mut errors);
    check_unbound_variables(&composition.steps, &mut errors);
    check_disconnected_steps(composition, &step_map, &mut warnings);
    check_unused_effects(composition, &mut warnings);

    let estimated = CostEstimate::aggregate(composition);
    if estimated.time_ms > 300_000
        || estimated.money_microdollars > 1_000_000
        || estimated.provider_calls > 100
    {
        warnings.push(ValidationWarning::HighCost { estimated });
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

fn build_step_map<'a>(
    steps: &'a [Step],
    errors: &mut Vec<ValidationError>,
) -> BTreeMap<&'a str, &'a Step> {
    let mut step_map = BTreeMap::new();
    let mut duplicates = BTreeSet::new();

    for step in steps {
        if step_map.insert(step.step_id.as_str(), step).is_some() {
            duplicates.insert(step.step_id.clone());
        }
    }

    for duplicate in duplicates {
        errors.push(ValidationError::DuplicateStepId(duplicate));
    }

    step_map
}

fn check_edges(
    composition: &Composition,
    step_map: &BTreeMap<&str, &Step>,
    errors: &mut Vec<ValidationError>,
) {
    for edge in &composition.edges {
        if !step_map.contains_key(edge.from.as_str()) {
            errors.push(ValidationError::DanglingEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
                missing: edge.from.clone(),
            });
        }
        if !step_map.contains_key(edge.to.as_str()) {
            errors.push(ValidationError::DanglingEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
                missing: edge.to.clone(),
            });
        }

        let Some(source) = step_map.get(edge.from.as_str()) else {
            continue;
        };

        match &edge.kind {
            EdgeKind::DataFlow { artifact_type } => {
                if !source_produces_artifact(source, artifact_type) {
                    errors.push(ValidationError::ArtifactSourceMismatch {
                        edge_from: edge.from.clone(),
                        edge_to: edge.to.clone(),
                        artifact_type: artifact_type.clone(),
                    });
                }
            }
            EdgeKind::Conditional { .. } if matches!(source.kind, StepKind::Goal(_)) => {
                errors.push(ValidationError::InvalidGuardOnGoalStep {
                    edge_from: edge.from.clone(),
                });
            }
            EdgeKind::Ordering | EdgeKind::Conditional { .. } => {}
        }
    }
}

fn source_produces_artifact(step: &Step, artifact_type: &str) -> bool {
    let StepKind::Op(operator) = &step.kind else {
        return false;
    };

    operator
        .resolution
        .requires_outputs
        .iter()
        .any(|slot| slot.artifact_type_id == artifact_type)
        || operator.effects.iter().any(|effect| {
            matches!(
                effect,
                Effect::Assert(Proposition::Exists {
                    artifact_type: crate::term::Term::ArtifactType(candidate),
                    ..
                }) if candidate == artifact_type
            )
        })
}

fn check_cycles(
    composition: &Composition,
    step_map: &BTreeMap<&str, &Step>,
    errors: &mut Vec<ValidationError>,
) {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack = Vec::new();

    for step_id in step_map.keys() {
        if let Some(cycle) = visit_for_cycle(
            step_id,
            composition,
            &mut visiting,
            &mut visited,
            &mut stack,
        ) {
            errors.push(ValidationError::CycleDetected {
                involved_steps: cycle,
            });
            return;
        }
    }
}

fn visit_for_cycle(
    step_id: &str,
    composition: &Composition,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    if visited.contains(step_id) {
        return None;
    }
    if visiting.contains(step_id) {
        let start = stack
            .iter()
            .position(|candidate| candidate == step_id)
            .unwrap_or(0);
        return Some(stack[start..].to_vec());
    }

    visiting.insert(step_id.to_string());
    stack.push(step_id.to_string());

    for edge in composition.edges.iter().filter(|edge| edge.from == step_id) {
        if let Some(cycle) = visit_for_cycle(&edge.to, composition, visiting, visited, stack) {
            return Some(cycle);
        }
    }

    stack.pop();
    visiting.remove(step_id);
    visited.insert(step_id.to_string());
    None
}

fn check_unbound_variables(steps: &[Step], errors: &mut Vec<ValidationError>) {
    for step in steps {
        match &step.kind {
            StepKind::Op(operator) => check_operator_variables(&step.step_id, operator, errors),
            StepKind::Goal(proposition) => {
                if let Some(variable) = proposition_variable_issue(proposition) {
                    errors.push(ValidationError::UnboundVariable {
                        step_id: step.step_id.clone(),
                        variable,
                    });
                }
            }
        }
    }
}

fn check_operator_variables(step_id: &str, operator: &Operator, errors: &mut Vec<ValidationError>) {
    for proposition in &operator.preconditions {
        if let Some(variable) = proposition_variable_issue(proposition) {
            errors.push(ValidationError::UnboundVariable {
                step_id: step_id.to_string(),
                variable,
            });
        }
    }
    for effect in &operator.effects {
        if let Some(variable) = effect_variable_issue(effect) {
            errors.push(ValidationError::UnboundVariable {
                step_id: step_id.to_string(),
                variable,
            });
        }
    }
}

fn effect_variable_issue(effect: &Effect) -> Option<String> {
    match effect {
        Effect::Assert(proposition) | Effect::Retract(proposition) => {
            proposition_variable_issue(proposition)
        }
        Effect::Update {
            subject,
            dimension,
            value,
        } => term_variable_issue(subject)
            .or_else(|| term_variable_issue(dimension))
            .or_else(|| term_variable_issue(value)),
    }
}

fn proposition_variable_issue(proposition: &Proposition) -> Option<String> {
    match proposition {
        Proposition::Holds {
            subject,
            dimension,
            condition,
        } => term_variable_issue(subject)
            .or_else(|| term_variable_issue(dimension))
            .or_else(|| condition_variable_issue(condition)),
        Proposition::Exists {
            scope,
            artifact_type,
        } => term_variable_issue(scope).or_else(|| term_variable_issue(artifact_type)),
        Proposition::Accessible { scope } => term_variable_issue(scope),
        Proposition::Related { src, relation, dst } => term_variable_issue(src)
            .or_else(|| term_variable_issue(relation))
            .or_else(|| term_variable_issue(dst)),
        Proposition::All(children) | Proposition::Any(children) => {
            children.iter().find_map(proposition_variable_issue)
        }
        Proposition::Not(child) => proposition_variable_issue(child),
    }
}

fn condition_variable_issue(condition: &Condition) -> Option<String> {
    match condition {
        Condition::Above(term)
        | Condition::Below(term)
        | Condition::Equals(term)
        | Condition::Within(term)
        | Condition::Exceeds(term) => term_variable_issue(term),
        Condition::In(terms) => terms.iter().find_map(term_variable_issue),
        Condition::Present | Condition::Absent => None,
    }
}

fn term_variable_issue(term: &Term) -> Option<String> {
    match term {
        Term::Variable(variable) => Some(variable.clone()),
        _ => None,
    }
}

fn check_disconnected_steps(
    composition: &Composition,
    step_map: &BTreeMap<&str, &Step>,
    warnings: &mut Vec<ValidationWarning>,
) {
    let Some(first) = composition.steps.first() else {
        return;
    };
    let with_incoming: BTreeSet<&str> = composition
        .edges
        .iter()
        .map(|edge| edge.to.as_str())
        .collect();

    for step_id in step_map.keys() {
        if *step_id != first.step_id && !with_incoming.contains(step_id) {
            warnings.push(ValidationWarning::DisconnectedStep((*step_id).to_string()));
        }
    }
}

fn check_unused_effects(composition: &Composition, warnings: &mut Vec<ValidationWarning>) {
    let goal_terms: Vec<&Proposition> = composition
        .steps
        .iter()
        .filter_map(|step| match &step.kind {
            StepKind::Goal(proposition) => Some(proposition),
            StepKind::Op(_) => None,
        })
        .collect();

    for step in &composition.steps {
        let StepKind::Op(operator) = &step.kind else {
            continue;
        };
        if operator.effects.is_empty() {
            continue;
        }
        let contributes = operator.effects.iter().any(|effect| {
            composition
                .edges
                .iter()
                .any(|edge| edge.from == step.step_id)
                || goal_terms
                    .iter()
                    .any(|goal| effect_mentions_goal(effect, goal))
        });
        if !contributes {
            warnings.push(ValidationWarning::UnusedEffects {
                step_id: step.step_id.clone(),
            });
        }
    }
}

fn effect_mentions_goal(effect: &Effect, goal: &Proposition) -> bool {
    match effect {
        Effect::Assert(proposition) | Effect::Retract(proposition) => proposition == goal,
        Effect::Update {
            subject, dimension, ..
        } => matches!(
            goal,
            Proposition::Holds {
                subject: goal_subject,
                dimension: goal_dimension,
                ..
            } if goal_subject == subject && goal_dimension == dimension
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        composition::{Composition, Edge, EdgeKind, Step},
        cost::CostEstimate,
        operator::{Resolution, SlotConstraint},
        term::{Literal, Term},
    };

    fn subject() -> Term {
        Term::Object(meld_events::DomainObjectRef::new("domain", "node", "a").unwrap())
    }

    fn operator_step(step_id: &str, output: &str) -> Step {
        Step {
            step_id: step_id.into(),
            kind: StepKind::Op(Operator {
                operator_id: step_id.into(),
                preconditions: vec![],
                effects: vec![Effect::Update {
                    subject: subject(),
                    dimension: Term::Dimension("confidence".into()),
                    value: Term::Literal(Literal::Number(0.9)),
                }],
                cost: CostEstimate::zero(),
                resolution: Resolution {
                    requires_inputs: vec![],
                    requires_outputs: vec![SlotConstraint {
                        artifact_type_id: output.into(),
                        required: true,
                    }],
                    scope_kind: None,
                    tags: vec![],
                    specific: None,
                },
            }),
        }
    }

    #[test]
    fn valid_composition_passes() {
        let composition = Composition {
            steps: vec![operator_step("a", "summary"), operator_step("b", "report")],
            edges: vec![Edge {
                from: "a".into(),
                to: "b".into(),
                kind: EdgeKind::DataFlow {
                    artifact_type: "summary".into(),
                },
            }],
        };

        assert!(validate(&composition).valid);
    }

    #[test]
    fn detects_dangling_edges_cycles_and_mismatches() {
        let composition = Composition {
            steps: vec![operator_step("a", "summary")],
            edges: vec![Edge {
                from: "a".into(),
                to: "missing".into(),
                kind: EdgeKind::DataFlow {
                    artifact_type: "other".into(),
                },
            }],
        };
        let result = validate(&composition);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| matches!(error, ValidationError::DanglingEdge { .. })));
        assert!(result
            .errors
            .iter()
            .any(|error| matches!(error, ValidationError::ArtifactSourceMismatch { .. })));

        let cycle = Composition {
            steps: vec![operator_step("a", "summary"), operator_step("b", "report")],
            edges: vec![
                Edge {
                    from: "a".into(),
                    to: "b".into(),
                    kind: EdgeKind::Ordering,
                },
                Edge {
                    from: "b".into(),
                    to: "a".into(),
                    kind: EdgeKind::Ordering,
                },
            ],
        };
        assert!(validate(&cycle)
            .errors
            .iter()
            .any(|error| matches!(error, ValidationError::CycleDetected { .. })));
    }

    #[test]
    fn allows_derived_terms_in_compositions() {
        let composition = Composition {
            steps: vec![Step {
                step_id: "subgoal".into(),
                kind: StepKind::Goal(Proposition::Exists {
                    scope: Term::Derived {
                        source_step: "plan".into(),
                        field_path: "items[0].scope".into(),
                    },
                    artifact_type: Term::ArtifactType("report".into()),
                }),
            }],
            edges: vec![],
        };

        assert!(validate(&composition).valid);
    }
}
