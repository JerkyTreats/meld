use serde::{Deserialize, Serialize};

use crate::{
    condition::Condition,
    effect::Effect,
    evaluate::{condition_satisfied, evaluate, EvalResult},
    proposition::Proposition,
    term::Term,
    unify::{unify, Bindings},
};

/// A set of ground propositions representing current belief.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    propositions: Vec<Proposition>,
}

/// Error returned when a ground-only position contains a variable or derived term.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundingError {
    /// Index of the proposition or effect that failed grounding.
    pub index: usize,
    /// Variable or derived marker that prevented grounding.
    pub variable: String,
}

impl WorldState {
    /// Create a world state from ground propositions.
    pub fn new(propositions: Vec<Proposition>) -> Result<Self, GroundingError> {
        for (index, proposition) in propositions.iter().enumerate() {
            if let Some(variable) = proposition.grounding_issue() {
                return Err(GroundingError { index, variable });
            }
        }

        Ok(Self { propositions })
    }

    /// Create an empty world state.
    pub fn empty() -> Self {
        Self {
            propositions: Vec::new(),
        }
    }

    /// Returns true when this state satisfies the query.
    pub fn satisfies(&self, query: &Proposition) -> bool {
        if query.is_ground() {
            evaluate(self, query) == EvalResult::Satisfied
        } else {
            self.pattern_satisfies(query)
        }
    }

    /// Borrow all current ground propositions.
    pub fn propositions(&self) -> &[Proposition] {
        &self.propositions
    }

    /// Find propositions matching a pattern.
    pub fn query(&self, pattern: &Proposition) -> Vec<Bindings> {
        self.query_with_bindings(pattern, &Bindings::empty())
    }

    /// Return propositions in the query that are not satisfied.
    pub fn gap(&self, query: &Proposition) -> Vec<Proposition> {
        match evaluate(self, query) {
            EvalResult::Satisfied => Vec::new(),
            EvalResult::Unsatisfied { gap } => gap,
            EvalResult::Indeterminate { .. } => vec![query.clone()],
        }
    }

    /// Apply effects sequentially to produce a new world state.
    pub fn apply(&self, effects: &[Effect]) -> Result<WorldState, GroundingError> {
        for (index, effect) in effects.iter().enumerate() {
            if let Some(variable) = effect.grounding_issue() {
                return Err(GroundingError { index, variable });
            }
        }

        let mut propositions = self.propositions.clone();

        for effect in effects {
            match effect {
                Effect::Assert(proposition) => {
                    if !propositions.contains(proposition) {
                        propositions.push(proposition.clone());
                    }
                }
                Effect::Retract(proposition) => {
                    if let Some(index) = propositions
                        .iter()
                        .position(|candidate| retract_matches(candidate, proposition))
                    {
                        propositions.remove(index);
                    }
                }
                Effect::Update {
                    subject,
                    dimension,
                    value,
                } => {
                    propositions.retain(|candidate| {
                        !matches!(
                            candidate,
                            Proposition::Holds {
                                subject: candidate_subject,
                                dimension: candidate_dimension,
                                ..
                            } if candidate_subject == subject && candidate_dimension == dimension
                        )
                    });
                    propositions.push(Proposition::Holds {
                        subject: subject.clone(),
                        dimension: dimension.clone(),
                        condition: Condition::Equals(value.clone()),
                    });
                }
            }
        }

        WorldState::new(propositions)
    }

    fn pattern_satisfies(&self, query: &Proposition) -> bool {
        match query {
            Proposition::All(children) => children.iter().all(|child| self.satisfies(child)),
            Proposition::Any(children) => children.iter().any(|child| self.satisfies(child)),
            Proposition::Not(child) => !self.satisfies(child),
            _ => !self.query(query).is_empty(),
        }
    }

    fn query_with_bindings(&self, pattern: &Proposition, bindings: &Bindings) -> Vec<Bindings> {
        match pattern {
            Proposition::All(children) => {
                children.iter().fold(vec![bindings.clone()], |acc, child| {
                    acc.into_iter()
                        .flat_map(|bindings| self.query_with_bindings(child, &bindings))
                        .collect()
                })
            }
            Proposition::Any(children) => children
                .iter()
                .flat_map(|child| self.query_with_bindings(child, bindings))
                .collect(),
            Proposition::Not(child) => {
                if self.query_with_bindings(child, bindings).is_empty() {
                    vec![bindings.clone()]
                } else {
                    Vec::new()
                }
            }
            _ => self
                .propositions
                .iter()
                .filter_map(|proposition| match_world_pattern(pattern, proposition, bindings))
                .collect(),
        }
    }
}

fn retract_matches(candidate: &Proposition, target: &Proposition) -> bool {
    match (candidate, target) {
        (
            Proposition::Holds {
                subject: candidate_subject,
                dimension: candidate_dimension,
                ..
            },
            Proposition::Holds {
                subject, dimension, ..
            },
        ) => candidate_subject == subject && candidate_dimension == dimension,
        _ => candidate == target,
    }
}

fn match_world_pattern(
    pattern: &Proposition,
    proposition: &Proposition,
    bindings: &Bindings,
) -> Option<Bindings> {
    match (pattern, proposition) {
        (
            Proposition::Holds {
                subject,
                dimension,
                condition,
            },
            Proposition::Holds {
                subject: candidate_subject,
                dimension: candidate_dimension,
                condition: candidate_condition,
            },
        ) => {
            let bindings = match_term_pattern(subject, candidate_subject, bindings)?;
            let bindings = match_term_pattern(dimension, candidate_dimension, &bindings)?;
            match_condition_pattern(condition, candidate_condition, &bindings)
        }
        _ => unify(pattern, proposition).and_then(|matched| bindings.merge(&matched)),
    }
}

fn match_condition_pattern(
    pattern: &Condition,
    candidate: &Condition,
    bindings: &Bindings,
) -> Option<Bindings> {
    match (pattern, candidate) {
        (Condition::Present, _) => Some(bindings.clone()),
        (Condition::Absent, _) => None,
        (Condition::Equals(pattern_term), Condition::Equals(candidate_term)) => {
            match_term_pattern(pattern_term, candidate_term, bindings)
        }
        _ if pattern.is_ground() && condition_satisfied(candidate, pattern) => {
            Some(bindings.clone())
        }
        _ => None,
    }
}

fn match_term_pattern(pattern: &Term, candidate: &Term, bindings: &Bindings) -> Option<Bindings> {
    match pattern {
        Term::Variable(variable) => bindings.bind(variable.clone(), candidate.clone()),
        _ if pattern == candidate => Some(bindings.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::{Literal, Term};

    fn subject() -> Term {
        Term::Object(meld_events::DomainObjectRef::new("domain", "node", "a").unwrap())
    }

    #[test]
    fn rejects_non_ground_world_state() {
        let err = WorldState::new(vec![Proposition::Accessible {
            scope: Term::Variable("?node".into()),
        }])
        .unwrap_err();

        assert_eq!(err.variable, "?node");
    }

    #[test]
    fn applies_assert_retract_and_update_immutably() {
        let accessible = Proposition::Accessible { scope: subject() };
        let state = WorldState::empty();
        let asserted = state
            .apply(&[
                Effect::Assert(accessible.clone()),
                Effect::Assert(accessible.clone()),
            ])
            .unwrap();

        assert_eq!(state.propositions().len(), 0);
        assert_eq!(asserted.propositions().len(), 1);

        let updated = asserted
            .apply(&[Effect::Update {
                subject: subject(),
                dimension: Term::Dimension("confidence".into()),
                value: Term::Literal(Literal::Number(0.9)),
            }])
            .unwrap();
        assert!(updated.satisfies(&Proposition::Holds {
            subject: subject(),
            dimension: Term::Dimension("confidence".into()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
        }));

        let retracted = updated.apply(&[Effect::Retract(accessible)]).unwrap();
        assert_eq!(retracted.propositions().len(), 1);
    }

    #[test]
    fn query_matches_variables_with_evaluation_conditions() {
        let state = WorldState::new(vec![Proposition::Holds {
            subject: subject(),
            dimension: Term::Dimension("confidence".into()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.3))),
        }])
        .unwrap();

        let matches = state.query(&Proposition::Holds {
            subject: Term::Variable("?node".into()),
            dimension: Term::Dimension("confidence".into()),
            condition: Condition::Below(Term::Literal(Literal::Number(0.5))),
        });

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].get("?node"), Some(&subject()));
    }
}
