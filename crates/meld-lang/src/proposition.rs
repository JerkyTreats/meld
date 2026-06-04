//! Typed proposition language.
//!
//! Owner: planning language.
//! Inputs: subjects, dimensions, conditions, scopes, artifact types, and
//! relations expressed as terms.
//! Outputs: proposition trees used by goals, world state, validation, and
//! unification.

use serde::{Deserialize, Serialize};

use crate::{condition::Condition, term::Term};

/// A typed statement about the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Proposition {
    /// A belief dimension of a subject satisfies a condition.
    Holds {
        /// The object or scope the belief describes.
        subject: Term,
        /// The belief dimension being tested.
        dimension: Term,
        /// The comparison to evaluate.
        condition: Condition,
    },
    /// An artifact of a given type exists for a scope.
    Exists {
        /// The scope that owns the artifact.
        scope: Term,
        /// The artifact type identity.
        artifact_type: Term,
    },
    /// A scope is reachable for observation or action.
    Accessible {
        /// The reachable scope.
        scope: Term,
    },
    /// A relationship holds between two terms.
    Related {
        /// The relationship source.
        src: Term,
        /// The relationship identity.
        relation: Term,
        /// The relationship destination.
        dst: Term,
    },
    /// Every child proposition must hold.
    All(Vec<Proposition>),
    /// At least one child proposition must hold.
    Any(Vec<Proposition>),
    /// The child proposition must not hold.
    Not(Box<Proposition>),
}

impl Proposition {
    /// Returns true when the proposition contains no variables or derived terms.
    pub fn is_ground(&self) -> bool {
        self.grounding_issue().is_none()
    }

    /// Returns the first variable or derived marker that prevents grounding.
    pub fn grounding_issue(&self) -> Option<String> {
        match self {
            Proposition::Holds {
                subject,
                dimension,
                condition,
            } => subject
                .grounding_issue()
                .or_else(|| dimension.grounding_issue())
                .or_else(|| condition.grounding_issue()),
            Proposition::Exists {
                scope,
                artifact_type,
            } => scope
                .grounding_issue()
                .or_else(|| artifact_type.grounding_issue()),
            Proposition::Accessible { scope } => scope.grounding_issue(),
            Proposition::Related { src, relation, dst } => src
                .grounding_issue()
                .or_else(|| relation.grounding_issue())
                .or_else(|| dst.grounding_issue()),
            Proposition::All(children) | Proposition::Any(children) => {
                children.iter().find_map(Proposition::grounding_issue)
            }
            Proposition::Not(child) => child.grounding_issue(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Literal;

    #[test]
    fn nested_propositions_round_trip() {
        let proposition = Proposition::All(vec![
            Proposition::Accessible {
                scope: Term::Object(
                    meld_events::DomainObjectRef::new("domain", "node", "a").unwrap(),
                ),
            },
            Proposition::Not(Box::new(Proposition::Exists {
                scope: Term::Variable("?node".into()),
                artifact_type: Term::ArtifactType("report".into()),
            })),
            Proposition::Holds {
                subject: Term::Variable("?node".into()),
                dimension: Term::Dimension("confidence".into()),
                condition: Condition::In(vec![
                    Term::Literal(Literal::Text("passing".into())),
                    Term::Literal(Literal::Text("flaky".into())),
                ]),
            },
        ]);

        let encoded = serde_json::to_string(&proposition).unwrap();
        let decoded: Proposition = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, proposition);
    }

    #[test]
    fn ground_check_rejects_variables_and_derived_terms() {
        let ground = Proposition::Exists {
            scope: Term::Object(meld_events::DomainObjectRef::new("domain", "node", "a").unwrap()),
            artifact_type: Term::ArtifactType("report".into()),
        };
        assert!(ground.is_ground());

        let pattern = Proposition::Exists {
            scope: Term::Variable("?node".into()),
            artifact_type: Term::ArtifactType("report".into()),
        };
        assert!(!pattern.is_ground());

        let derived = Proposition::Accessible {
            scope: Term::Derived {
                source_step: "plan".into(),
                field_path: "items[0]".into(),
            },
        };
        assert!(!derived.is_ground());
    }
}
