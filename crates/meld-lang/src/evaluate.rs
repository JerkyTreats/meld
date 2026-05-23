use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    condition::Condition,
    proposition::Proposition,
    term::{Literal, Term},
    world_state::WorldState,
};

/// Three-valued proposition evaluation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvalResult {
    /// The proposition holds in the world state.
    Satisfied,
    /// The proposition is known not to hold.
    Unsatisfied {
        /// Unsatisfied proposition gaps.
        gap: Vec<Proposition>,
    },
    /// The proposition references missing knowledge.
    Indeterminate {
        /// Missing terms that prevented evaluation.
        missing: Vec<Term>,
    },
}

/// Evaluate a proposition against a world state using three-valued semantics.
pub fn evaluate(state: &WorldState, proposition: &Proposition) -> EvalResult {
    match proposition {
        Proposition::Holds {
            subject,
            dimension,
            condition,
        } => evaluate_holds(state, proposition, subject, dimension, condition),
        Proposition::Exists {
            scope,
            artifact_type,
        } => {
            if state.propositions().iter().any(|candidate| {
                matches!(
                    candidate,
                    Proposition::Exists {
                        scope: candidate_scope,
                        artifact_type: candidate_artifact_type
                    } if candidate_scope == scope && candidate_artifact_type == artifact_type
                )
            }) {
                EvalResult::Satisfied
            } else {
                EvalResult::Unsatisfied {
                    gap: vec![proposition.clone()],
                }
            }
        }
        Proposition::Accessible { scope } => {
            if state.propositions().iter().any(|candidate| {
                matches!(
                    candidate,
                    Proposition::Accessible {
                        scope: candidate_scope
                    } if candidate_scope == scope
                )
            }) {
                EvalResult::Satisfied
            } else {
                EvalResult::Unsatisfied {
                    gap: vec![proposition.clone()],
                }
            }
        }
        Proposition::Related { src, relation, dst } => {
            if state.propositions().iter().any(|candidate| {
                matches!(
                    candidate,
                    Proposition::Related {
                        src: candidate_src,
                        relation: candidate_relation,
                        dst: candidate_dst
                    } if candidate_src == src && candidate_relation == relation && candidate_dst == dst
                )
            }) {
                EvalResult::Satisfied
            } else {
                EvalResult::Unsatisfied {
                    gap: vec![proposition.clone()],
                }
            }
        }
        Proposition::All(children) => evaluate_all(children, state),
        Proposition::Any(children) => evaluate_any(children, state, proposition),
        Proposition::Not(child) => match evaluate(state, child) {
            EvalResult::Satisfied => EvalResult::Unsatisfied {
                gap: vec![proposition.clone()],
            },
            EvalResult::Unsatisfied { .. } => EvalResult::Satisfied,
            EvalResult::Indeterminate { .. } => EvalResult::Unsatisfied {
                gap: vec![proposition.clone()],
            },
        },
    }
}

fn evaluate_holds(
    state: &WorldState,
    proposition: &Proposition,
    subject: &Term,
    dimension: &Term,
    condition: &Condition,
) -> EvalResult {
    let matches: Vec<&Condition> = state
        .propositions()
        .iter()
        .filter_map(|candidate| match candidate {
            Proposition::Holds {
                subject: candidate_subject,
                dimension: candidate_dimension,
                condition: candidate_condition,
            } if candidate_subject == subject && candidate_dimension == dimension => {
                Some(candidate_condition)
            }
            _ => None,
        })
        .collect();

    if matches.is_empty() {
        return match condition {
            Condition::Absent => EvalResult::Satisfied,
            _ => EvalResult::Indeterminate {
                missing: vec![dimension.clone()],
            },
        };
    }

    match condition {
        Condition::Present => return EvalResult::Satisfied,
        Condition::Absent => {
            return EvalResult::Unsatisfied {
                gap: vec![proposition.clone()],
            };
        }
        _ => {}
    }

    if matches
        .iter()
        .any(|stored_condition| condition_satisfied(stored_condition, condition))
    {
        EvalResult::Satisfied
    } else {
        EvalResult::Unsatisfied {
            gap: vec![proposition.clone()],
        }
    }
}

fn evaluate_all(children: &[Proposition], state: &WorldState) -> EvalResult {
    let mut gaps = Vec::new();
    let mut missing = Vec::new();

    for child in children {
        match evaluate(state, child) {
            EvalResult::Satisfied => {}
            EvalResult::Unsatisfied { gap } => gaps.extend(gap),
            EvalResult::Indeterminate {
                missing: child_missing,
            } => missing.extend(child_missing),
        }
    }

    if !missing.is_empty() {
        EvalResult::Indeterminate { missing }
    } else if gaps.is_empty() {
        EvalResult::Satisfied
    } else {
        EvalResult::Unsatisfied { gap: gaps }
    }
}

fn evaluate_any(
    children: &[Proposition],
    state: &WorldState,
    proposition: &Proposition,
) -> EvalResult {
    let mut saw_indeterminate = false;
    let mut missing = Vec::new();

    for child in children {
        match evaluate(state, child) {
            EvalResult::Satisfied => return EvalResult::Satisfied,
            EvalResult::Unsatisfied { .. } => {}
            EvalResult::Indeterminate {
                missing: child_missing,
            } => {
                saw_indeterminate = true;
                missing.extend(child_missing);
            }
        }
    }

    if saw_indeterminate {
        EvalResult::Indeterminate { missing }
    } else {
        EvalResult::Unsatisfied {
            gap: vec![proposition.clone()],
        }
    }
}

pub(crate) fn condition_satisfied(stored: &Condition, query: &Condition) -> bool {
    if stored == query {
        return true;
    }

    let Some(stored_value) = stored_value(stored) else {
        return false;
    };

    match query {
        Condition::Above(threshold) => compare_values(stored_value, threshold)
            .map(|ordering| ordering == std::cmp::Ordering::Greater)
            .unwrap_or(false),
        Condition::Below(threshold) | Condition::Within(threshold) => {
            compare_values(stored_value, threshold)
                .map(|ordering| ordering == std::cmp::Ordering::Less)
                .unwrap_or(false)
        }
        Condition::Equals(expected) => stored_value == expected,
        Condition::In(terms) => terms.iter().any(|term| term == stored_value),
        Condition::Exceeds(threshold) => compare_values(stored_value, threshold)
            .map(|ordering| ordering == std::cmp::Ordering::Greater)
            .unwrap_or(false),
        Condition::Present | Condition::Absent => false,
    }
}

fn stored_value(condition: &Condition) -> Option<&Term> {
    match condition {
        Condition::Equals(value) => Some(value),
        _ => None,
    }
}

fn compare_values(left: &Term, right: &Term) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (Term::Literal(Literal::Number(left)), Term::Literal(Literal::Number(right))) => {
            left.partial_cmp(right)
        }
        (Term::Literal(Literal::Duration(left)), Term::Literal(Literal::Duration(right))) => {
            compare_duration(left, right)
        }
        _ => None,
    }
}

fn compare_duration(left: &Duration, right: &Duration) -> Option<std::cmp::Ordering> {
    Some(left.cmp(right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_state::WorldState;

    fn subject() -> Term {
        Term::Object(meld_events::DomainObjectRef::new("domain", "node", "a").unwrap())
    }

    #[test]
    fn evaluates_thresholds_and_missing_dimensions() {
        let query = Proposition::Holds {
            subject: subject(),
            dimension: Term::Dimension("confidence".into()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
        };
        let state = WorldState::new(vec![Proposition::Holds {
            subject: subject(),
            dimension: Term::Dimension("confidence".into()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.3))),
        }])
        .unwrap();
        assert_eq!(
            evaluate(&state, &query),
            EvalResult::Unsatisfied {
                gap: vec![query.clone()]
            }
        );

        let empty = WorldState::empty();
        assert_eq!(
            evaluate(&empty, &query),
            EvalResult::Indeterminate {
                missing: vec![Term::Dimension("confidence".into())]
            }
        );
    }

    #[test]
    fn evaluates_compound_propositions() {
        let accessible = Proposition::Accessible { scope: subject() };
        let exists = Proposition::Exists {
            scope: subject(),
            artifact_type: Term::ArtifactType("report".into()),
        };
        let state = WorldState::new(vec![accessible.clone()]).unwrap();

        assert_eq!(evaluate(&state, &accessible), EvalResult::Satisfied);
        assert_eq!(
            evaluate(
                &state,
                &Proposition::All(vec![accessible.clone(), exists.clone()])
            ),
            EvalResult::Unsatisfied { gap: vec![exists] }
        );
        assert_eq!(
            evaluate(&state, &Proposition::Any(vec![accessible.clone()])),
            EvalResult::Satisfied
        );
        assert_eq!(
            evaluate(&state, &Proposition::Not(Box::new(accessible))),
            EvalResult::Unsatisfied {
                gap: vec![Proposition::Not(Box::new(Proposition::Accessible {
                    scope: subject()
                }))]
            }
        );
    }
}
