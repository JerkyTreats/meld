//! Variable substitution over compositions.
//!
//! Owner: planning language.
//! Inputs: composition templates and unification bindings.
//! Outputs: instantiated compositions or unbound-variable diagnostics.

use serde::{Deserialize, Serialize};

use crate::{
    composition::{Composition, Edge, EdgeKind, Step, StepKind},
    condition::Condition,
    effect::Effect,
    operator::Operator,
    proposition::Proposition,
    term::Term,
    unify::Bindings,
};

/// Error returned when substitution leaves variables unbound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubstitutionError {
    /// All unbound variables found during substitution.
    pub unbound_variables: Vec<UnboundVariable>,
}

/// A variable missing from a binding set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnboundVariable {
    /// Step where the unbound variable was found.
    pub step_id: String,
    /// Variable name.
    pub variable: String,
}

/// Replace all variable terms in a composition using the provided bindings.
pub fn substitute(
    composition: &Composition,
    bindings: &Bindings,
) -> Result<Composition, SubstitutionError> {
    let mut errors = Vec::new();
    let steps = composition
        .steps
        .iter()
        .map(|step| substitute_step(step, bindings, &mut errors))
        .collect();
    let edges = composition
        .edges
        .iter()
        .map(|edge| substitute_edge(edge, bindings, &mut errors))
        .collect();

    if errors.is_empty() {
        Ok(Composition { steps, edges })
    } else {
        Err(SubstitutionError {
            unbound_variables: errors,
        })
    }
}

fn substitute_step(step: &Step, bindings: &Bindings, errors: &mut Vec<UnboundVariable>) -> Step {
    let kind = match &step.kind {
        StepKind::Op(operator) => StepKind::Op(substitute_operator(
            operator,
            &step.step_id,
            bindings,
            errors,
        )),
        StepKind::Goal(proposition) => StepKind::Goal(substitute_proposition(
            proposition,
            &step.step_id,
            bindings,
            errors,
        )),
    };

    Step {
        step_id: step.step_id.clone(),
        kind,
    }
}

fn substitute_operator(
    operator: &Operator,
    step_id: &str,
    bindings: &Bindings,
    errors: &mut Vec<UnboundVariable>,
) -> Operator {
    Operator {
        operator_id: operator.operator_id.clone(),
        preconditions: operator
            .preconditions
            .iter()
            .map(|proposition| substitute_proposition(proposition, step_id, bindings, errors))
            .collect(),
        effects: operator
            .effects
            .iter()
            .map(|effect| substitute_effect(effect, step_id, bindings, errors))
            .collect(),
        cost: operator.cost.clone(),
        resolution: operator.resolution.clone(),
    }
}

fn substitute_effect(
    effect: &Effect,
    step_id: &str,
    bindings: &Bindings,
    errors: &mut Vec<UnboundVariable>,
) -> Effect {
    match effect {
        Effect::Assert(proposition) => Effect::Assert(substitute_proposition(
            proposition,
            step_id,
            bindings,
            errors,
        )),
        Effect::Retract(proposition) => Effect::Retract(substitute_proposition(
            proposition,
            step_id,
            bindings,
            errors,
        )),
        Effect::Update {
            subject,
            dimension,
            value,
        } => Effect::Update {
            subject: substitute_term(subject, step_id, bindings, errors),
            dimension: substitute_term(dimension, step_id, bindings, errors),
            value: substitute_term(value, step_id, bindings, errors),
        },
    }
}

fn substitute_proposition(
    proposition: &Proposition,
    step_id: &str,
    bindings: &Bindings,
    errors: &mut Vec<UnboundVariable>,
) -> Proposition {
    match proposition {
        Proposition::Holds {
            subject,
            dimension,
            condition,
        } => Proposition::Holds {
            subject: substitute_term(subject, step_id, bindings, errors),
            dimension: substitute_term(dimension, step_id, bindings, errors),
            condition: substitute_condition(condition, step_id, bindings, errors),
        },
        Proposition::Exists {
            scope,
            artifact_type,
        } => Proposition::Exists {
            scope: substitute_term(scope, step_id, bindings, errors),
            artifact_type: substitute_term(artifact_type, step_id, bindings, errors),
        },
        Proposition::Accessible { scope } => Proposition::Accessible {
            scope: substitute_term(scope, step_id, bindings, errors),
        },
        Proposition::Related { src, relation, dst } => Proposition::Related {
            src: substitute_term(src, step_id, bindings, errors),
            relation: substitute_term(relation, step_id, bindings, errors),
            dst: substitute_term(dst, step_id, bindings, errors),
        },
        Proposition::All(children) => Proposition::All(
            children
                .iter()
                .map(|child| substitute_proposition(child, step_id, bindings, errors))
                .collect(),
        ),
        Proposition::Any(children) => Proposition::Any(
            children
                .iter()
                .map(|child| substitute_proposition(child, step_id, bindings, errors))
                .collect(),
        ),
        Proposition::Not(child) => Proposition::Not(Box::new(substitute_proposition(
            child, step_id, bindings, errors,
        ))),
    }
}

fn substitute_condition(
    condition: &Condition,
    step_id: &str,
    bindings: &Bindings,
    errors: &mut Vec<UnboundVariable>,
) -> Condition {
    match condition {
        Condition::Above(term) => {
            Condition::Above(substitute_term(term, step_id, bindings, errors))
        }
        Condition::Below(term) => {
            Condition::Below(substitute_term(term, step_id, bindings, errors))
        }
        Condition::Equals(term) => {
            Condition::Equals(substitute_term(term, step_id, bindings, errors))
        }
        Condition::In(terms) => Condition::In(
            terms
                .iter()
                .map(|term| substitute_term(term, step_id, bindings, errors))
                .collect(),
        ),
        Condition::Within(term) => {
            Condition::Within(substitute_term(term, step_id, bindings, errors))
        }
        Condition::Exceeds(term) => {
            Condition::Exceeds(substitute_term(term, step_id, bindings, errors))
        }
        Condition::Present => Condition::Present,
        Condition::Absent => Condition::Absent,
    }
}

fn substitute_edge(edge: &Edge, bindings: &Bindings, errors: &mut Vec<UnboundVariable>) -> Edge {
    let kind = match &edge.kind {
        EdgeKind::Ordering => EdgeKind::Ordering,
        EdgeKind::DataFlow { artifact_type } => EdgeKind::DataFlow {
            artifact_type: artifact_type.clone(),
        },
        EdgeKind::Conditional { field_path, guard } => EdgeKind::Conditional {
            field_path: field_path.clone(),
            guard: substitute_condition(guard, &edge.from, bindings, errors),
        },
    };

    Edge {
        from: edge.from.clone(),
        to: edge.to.clone(),
        kind,
    }
}

fn substitute_term(
    term: &Term,
    step_id: &str,
    bindings: &Bindings,
    errors: &mut Vec<UnboundVariable>,
) -> Term {
    match term {
        Term::Variable(variable) => match bindings.get(variable) {
            Some(bound) => bound.clone(),
            None => {
                errors.push(UnboundVariable {
                    step_id: step_id.to_string(),
                    variable: variable.clone(),
                });
                term.clone()
            }
        },
        _ => term.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        composition::{Composition, Step},
        cost::CostEstimate,
        operator::Resolution,
        term::Literal,
    };

    fn composition() -> Composition {
        Composition {
            steps: vec![Step {
                step_id: "write".into(),
                kind: StepKind::Op(Operator {
                    operator_id: "write".into(),
                    preconditions: vec![Proposition::Accessible {
                        scope: Term::Variable("?node".into()),
                    }],
                    effects: vec![Effect::Update {
                        subject: Term::Variable("?node".into()),
                        dimension: Term::Dimension("confidence".into()),
                        value: Term::Literal(Literal::Number(0.9)),
                    }],
                    cost: CostEstimate::zero(),
                    resolution: Resolution {
                        requires_inputs: vec![],
                        requires_outputs: vec![],
                        scope_kind: None,
                        tags: vec![],
                        specific: None,
                    },
                }),
            }],
            edges: vec![],
        }
    }

    #[test]
    fn substitutes_all_variables() {
        let node = Term::Object(meld_events::DomainObjectRef::new("domain", "node", "a").unwrap());
        let bindings = Bindings::empty()
            .bind("?node".into(), node.clone())
            .unwrap();
        let substituted = substitute(&composition(), &bindings).unwrap();

        let StepKind::Op(operator) = &substituted.steps[0].kind else {
            panic!("expected operator step");
        };
        assert_eq!(
            operator.preconditions[0],
            Proposition::Accessible { scope: node }
        );
    }

    #[test]
    fn reports_unbound_variables() {
        let error = substitute(&composition(), &Bindings::empty()).unwrap_err();
        assert_eq!(error.unbound_variables[0].step_id, "write");
        assert_eq!(error.unbound_variables[0].variable, "?node");
    }
}
