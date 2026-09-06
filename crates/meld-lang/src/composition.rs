//! Planning composition graph contracts.
//!
//! Owner: planning language.
//! Inputs: operator instances, nested goal steps, and dependency declarations.
//! Outputs: serializable step graphs consumed by validation and execution.

use serde::{Deserialize, Serialize};

use crate::{condition::Condition, operator::Operator, proposition::Proposition, term::Term};

/// Directed graph of planning steps and typed dependencies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    /// Steps in the composition graph.
    pub steps: Vec<Step>,
    /// Dependencies between steps.
    pub edges: Vec<Edge>,
}

/// One immutable input carried by a complete Task to an exact consumer slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskInput {
    /// Consumer step in the Task composition.
    pub step_id: String,
    /// Capability input slot selected by the producer.
    pub slot_id: String,
    /// Owner-defined artifact schema identity.
    pub artifact_type_id: String,
    /// Exact artifact schema version.
    pub schema_version: u32,
    /// Frozen owner input, interpreted only by the Capability implementation.
    pub content: serde_json::Value,
}

impl TaskInput {
    /// Validate the structural input address without interpreting its domain payload.
    pub fn validate(&self) -> Result<(), String> {
        if [&self.step_id, &self.slot_id, &self.artifact_type_id]
            .iter()
            .any(|value| value.trim().is_empty())
            || self.schema_version == 0
        {
            return Err(
                "Task input requires a consumer, slot, artifact type, and schema version".into(),
            );
        }
        Ok(())
    }
}

/// A step in a composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// Identity within this composition.
    pub step_id: String,
    /// The work represented by this step.
    pub kind: StepKind,
}

/// Work represented by a composition step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepKind {
    /// A capability-backed operator to resolve and dispatch.
    Op(Operator),
    /// A sub-goal to solve recursively.
    Goal(Proposition),
}

/// A dependency edge between two steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Source step identity.
    pub from: String,
    /// Target step identity.
    pub to: String,
    /// Dependency semantics.
    pub kind: EdgeKind,
}

/// The kind of dependency carried by an edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Pure ordering dependency.
    Ordering,
    /// Artifact type data flow dependency.
    DataFlow {
        /// Artifact type carried by this edge.
        artifact_type: Term,
    },
    /// Conditional activation based on a source output field.
    Conditional {
        /// Output field path to inspect.
        field_path: String,
        /// Condition that activates the target.
        guard: Condition,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cost::CostEstimate, operator::Resolution};

    #[test]
    fn composition_shapes_construct() {
        let composition = Composition {
            steps: vec![Step {
                step_id: "root".into(),
                kind: StepKind::Op(Operator {
                    operator_id: "root".into(),
                    preconditions: vec![],
                    effects: vec![],
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
        };

        assert_eq!(composition.steps.len(), 1);
    }
}
