use std::time::Duration;

use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

/// A concrete literal value in the shared language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    /// A boolean value.
    Bool(bool),
    /// A string value.
    Text(String),
    /// A floating point numeric value.
    Number(f64),
    /// A span of time.
    Duration(Duration),
}

/// The atomic unit of reference in propositions, effects, and compositions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Term {
    /// A reference to a domain object.
    Object(DomainObjectRef),
    /// A reference to a belief dimension.
    Dimension(String),
    /// A reference to an artifact type.
    ArtifactType(String),
    /// A concrete value.
    Literal(Literal),
    /// An unbound pattern variable.
    Variable(String),
    /// A value derived from a step output at runtime.
    Derived {
        /// The step that produces the output.
        source_step: String,
        /// The field path inside the step output.
        field_path: String,
    },
}

impl Term {
    /// Returns true when the term can appear in a ground world state.
    pub fn is_ground(&self) -> bool {
        !matches!(self, Term::Variable(_) | Term::Derived { .. })
    }

    /// Returns the first variable or derived marker that prevents grounding.
    pub fn grounding_issue(&self) -> Option<String> {
        match self {
            Term::Variable(variable) => Some(variable.clone()),
            Term::Derived {
                source_step,
                field_path,
            } => Some(format!("{source_step}.{field_path}")),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_ref() -> DomainObjectRef {
        DomainObjectRef::new("domain", "node", "a").unwrap()
    }

    #[test]
    fn terms_round_trip_through_json() {
        let terms = vec![
            Term::Object(object_ref()),
            Term::Dimension("confidence".into()),
            Term::ArtifactType("report".into()),
            Term::Literal(Literal::Bool(true)),
            Term::Literal(Literal::Text("ready".into())),
            Term::Literal(Literal::Number(0.7)),
            Term::Literal(Literal::Duration(Duration::from_secs(5))),
            Term::Variable("?node".into()),
            Term::Derived {
                source_step: "observe".into(),
                field_path: "scope".into(),
            },
        ];

        for term in terms {
            let encoded = serde_json::to_string(&term).unwrap();
            let decoded: Term = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, term);
        }
    }

    #[test]
    fn terms_distinguish_variants() {
        assert_ne!(Term::Dimension("x".into()), Term::ArtifactType("x".into()));
        assert_ne!(
            Term::Literal(Literal::Bool(true)),
            Term::Variable("true".into())
        );
        assert_eq!(Term::Object(object_ref()), Term::Object(object_ref()));
    }
}
