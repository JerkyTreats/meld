//! Runtime operator and capability resolution contracts.
//!
//! Owner: planning language.
//! Inputs: preconditions, effects, cost estimates, and capability lookup hints.
//! Outputs: serializable operator records consumed by execution.

use serde::{Deserialize, Serialize};

use crate::{cost::CostEstimate, effect::Effect, proposition::Proposition};

/// Runtime-constructed operator contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operator {
    /// Instance identity within a composition.
    pub operator_id: String,
    /// Preconditions that must hold before dispatch.
    pub preconditions: Vec<Proposition>,
    /// Effects produced by dispatch.
    pub effects: Vec<Effect>,
    /// Estimated resource consumption.
    pub cost: CostEstimate,
    /// Capability catalog query hints.
    pub resolution: Resolution,
}

/// Query shape used by execution to resolve a capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    /// Artifact types this operator consumes.
    pub requires_inputs: Vec<SlotConstraint>,
    /// Artifact types this operator produces.
    pub requires_outputs: Vec<SlotConstraint>,
    /// Scope kind required by the capability.
    pub scope_kind: Option<String>,
    /// Freeform catalog narrowing tags.
    pub tags: Vec<String>,
    /// Optional exact capability reference.
    pub specific: Option<CapabilityRef>,
}

/// Artifact slot constraint for resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotConstraint {
    /// Artifact type identity.
    pub artifact_type_id: String,
    /// Whether the slot is required.
    pub required: bool,
}

/// Exact capability reference for resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityRef {
    /// Capability type identity.
    pub capability_type_id: String,
    /// Capability version.
    pub capability_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{condition::Condition, term::Term};

    #[test]
    fn operator_round_trips() {
        let operator = Operator {
            operator_id: "observe".into(),
            preconditions: vec![Proposition::Accessible {
                scope: Term::Variable("?scope".into()),
            }],
            effects: vec![Effect::Assert(Proposition::Exists {
                scope: Term::Variable("?scope".into()),
                artifact_type: Term::ArtifactType("summary".into()),
            })],
            cost: CostEstimate {
                time_ms: 5,
                money_microdollars: 7,
                provider_calls: 1,
            },
            resolution: Resolution {
                requires_inputs: vec![],
                requires_outputs: vec![SlotConstraint {
                    artifact_type_id: "summary".into(),
                    required: true,
                }],
                scope_kind: Some("filesystem".into()),
                tags: vec!["observe".into()],
                specific: Some(CapabilityRef {
                    capability_type_id: "observe.fs".into(),
                    capability_version: 1,
                }),
            },
        };

        let encoded = serde_json::to_string(&operator).unwrap();
        let decoded: Operator = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, operator);

        let _guard = Condition::Present;
    }
}
