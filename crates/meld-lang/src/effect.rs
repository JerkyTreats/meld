use serde::{Deserialize, Serialize};

use crate::{proposition::Proposition, term::Term};

/// A deterministic state change to the proposition space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Make a proposition true in the world state.
    Assert(Proposition),
    /// Remove a proposition from the world state.
    Retract(Proposition),
    /// Replace the known value for a subject and dimension.
    Update {
        /// The subject whose dimension is updated.
        subject: Term,
        /// The dimension to update.
        dimension: Term,
        /// The new canonical value.
        value: Term,
    },
}

impl Effect {
    /// Returns the first variable or derived marker that prevents grounding.
    pub fn grounding_issue(&self) -> Option<String> {
        match self {
            Effect::Assert(proposition) | Effect::Retract(proposition) => {
                proposition.grounding_issue()
            }
            Effect::Update {
                subject,
                dimension,
                value,
            } => subject
                .grounding_issue()
                .or_else(|| dimension.grounding_issue())
                .or_else(|| value.grounding_issue()),
        }
    }
}
