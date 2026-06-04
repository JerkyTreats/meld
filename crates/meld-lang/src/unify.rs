//! Proposition unification and variable bindings.
//!
//! Owner: planning language.
//! Inputs: pattern propositions and concrete propositions.
//! Outputs: deterministic variable bindings when shapes are compatible.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{condition::Condition, proposition::Proposition, term::Term};

/// Immutable variable bindings produced by unification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bindings {
    entries: BTreeMap<String, Term>,
}

impl Bindings {
    /// Create an empty binding set.
    pub fn empty() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Bind a variable to a term, returning none on conflict.
    pub fn bind(&self, variable: String, term: Term) -> Option<Bindings> {
        if let Some(existing) = self.entries.get(&variable) {
            return (existing == &term).then(|| self.clone());
        }

        let mut entries = self.entries.clone();
        entries.insert(variable, term);
        Some(Bindings { entries })
    }

    /// Return the term bound to a variable.
    pub fn get(&self, variable: &str) -> Option<&Term> {
        self.entries.get(variable)
    }

    /// Merge two binding sets, returning none on conflict.
    pub fn merge(&self, other: &Bindings) -> Option<Bindings> {
        other
            .entries
            .iter()
            .try_fold(self.clone(), |bindings, (variable, term)| {
                bindings.bind(variable.clone(), term.clone())
            })
    }

    /// Iterate over binding entries in deterministic key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Term)> {
        self.entries.iter()
    }
}

/// Attempt to unify a pattern proposition against a concrete proposition.
pub fn unify(pattern: &Proposition, concrete: &Proposition) -> Option<Bindings> {
    unify_proposition(pattern, concrete, &Bindings::empty())
}

fn unify_proposition(
    pattern: &Proposition,
    concrete: &Proposition,
    bindings: &Bindings,
) -> Option<Bindings> {
    match (pattern, concrete) {
        (
            Proposition::Holds {
                subject,
                dimension,
                condition,
            },
            Proposition::Holds {
                subject: concrete_subject,
                dimension: concrete_dimension,
                condition: concrete_condition,
            },
        ) => {
            let bindings = unify_term(subject, concrete_subject, bindings)?;
            let bindings = unify_term(dimension, concrete_dimension, &bindings)?;
            unify_condition(condition, concrete_condition, &bindings)
        }
        (
            Proposition::Exists {
                scope,
                artifact_type,
            },
            Proposition::Exists {
                scope: concrete_scope,
                artifact_type: concrete_artifact_type,
            },
        ) => {
            let bindings = unify_term(scope, concrete_scope, bindings)?;
            unify_term(artifact_type, concrete_artifact_type, &bindings)
        }
        (
            Proposition::Accessible { scope },
            Proposition::Accessible {
                scope: concrete_scope,
            },
        ) => unify_term(scope, concrete_scope, bindings),
        (
            Proposition::Related { src, relation, dst },
            Proposition::Related {
                src: concrete_src,
                relation: concrete_relation,
                dst: concrete_dst,
            },
        ) => {
            let bindings = unify_term(src, concrete_src, bindings)?;
            let bindings = unify_term(relation, concrete_relation, &bindings)?;
            unify_term(dst, concrete_dst, &bindings)
        }
        (Proposition::All(children), Proposition::All(concrete_children))
        | (Proposition::Any(children), Proposition::Any(concrete_children)) => {
            if children.len() != concrete_children.len() {
                return None;
            }
            children.iter().zip(concrete_children).try_fold(
                bindings.clone(),
                |bindings, (child, concrete_child)| {
                    unify_proposition(child, concrete_child, &bindings)
                },
            )
        }
        (Proposition::Not(child), Proposition::Not(concrete_child)) => {
            unify_proposition(child, concrete_child, bindings)
        }
        _ => None,
    }
}

fn unify_condition(
    pattern: &Condition,
    concrete: &Condition,
    bindings: &Bindings,
) -> Option<Bindings> {
    match (pattern, concrete) {
        (Condition::Above(term), Condition::Above(concrete_term))
        | (Condition::Below(term), Condition::Below(concrete_term))
        | (Condition::Equals(term), Condition::Equals(concrete_term))
        | (Condition::Within(term), Condition::Within(concrete_term))
        | (Condition::Exceeds(term), Condition::Exceeds(concrete_term)) => {
            unify_term(term, concrete_term, bindings)
        }
        (Condition::In(terms), Condition::In(concrete_terms)) => {
            if terms.len() != concrete_terms.len() {
                return None;
            }
            terms
                .iter()
                .zip(concrete_terms)
                .try_fold(bindings.clone(), |bindings, (term, concrete_term)| {
                    unify_term(term, concrete_term, &bindings)
                })
        }
        (Condition::Present, Condition::Present) | (Condition::Absent, Condition::Absent) => {
            Some(bindings.clone())
        }
        _ => None,
    }
}

fn unify_term(pattern: &Term, concrete: &Term, bindings: &Bindings) -> Option<Bindings> {
    match pattern {
        Term::Variable(variable) => bindings.bind(variable.clone(), concrete.clone()),
        _ if pattern == concrete => Some(bindings.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{condition::Condition, term::Literal};

    fn subject(id: &str) -> Term {
        Term::Object(meld_events::DomainObjectRef::new("domain", "node", id).unwrap())
    }

    #[test]
    fn unifies_variables_and_rejects_conflicts() {
        let pattern = Proposition::Related {
            src: Term::Variable("?node".into()),
            relation: Term::Dimension("same".into()),
            dst: Term::Variable("?node".into()),
        };
        let matching = Proposition::Related {
            src: subject("a"),
            relation: Term::Dimension("same".into()),
            dst: subject("a"),
        };
        let conflict = Proposition::Related {
            src: subject("a"),
            relation: Term::Dimension("same".into()),
            dst: subject("b"),
        };

        assert!(unify(&pattern, &matching).is_some());
        assert!(unify(&pattern, &conflict).is_none());
    }

    #[test]
    fn unifies_nested_shapes() {
        let pattern = Proposition::All(vec![Proposition::Holds {
            subject: Term::Variable("?node".into()),
            dimension: Term::Dimension("confidence".into()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.5))),
        }]);
        let concrete = Proposition::All(vec![Proposition::Holds {
            subject: subject("a"),
            dimension: Term::Dimension("confidence".into()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.5))),
        }]);

        assert_eq!(
            unify(&pattern, &concrete).unwrap().get("?node"),
            Some(&subject("a"))
        );
    }

    #[test]
    fn bindings_are_immutable_and_merge_conflicts() {
        let empty = Bindings::empty();
        let a = empty.bind("?node".into(), subject("a")).unwrap();
        let b = empty.bind("?node".into(), subject("b")).unwrap();

        assert!(empty.get("?node").is_none());
        assert!(a.merge(&b).is_none());
    }
}
