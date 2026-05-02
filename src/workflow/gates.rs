use crate::workflow::profile::WorkflowGate;
use crate::workflow::record_contracts::GateOutcome;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvaluationResult {
    pub outcome: GateOutcome,
    pub reasons: Vec<String>,
}

impl GateEvaluationResult {
    pub fn is_pass(&self) -> bool {
        self.outcome == GateOutcome::Pass
    }
}

pub fn evaluate_gate(
    gate: &WorkflowGate,
    output: &str,
    input_values: Option<&HashMap<String, String>>,
) -> GateEvaluationResult {
    let result = meld_execution::workflow::gates::evaluate_gate(gate, output, input_values);
    GateEvaluationResult {
        outcome: match result.outcome {
            meld_execution::workflow::GateOutcome::Pass => GateOutcome::Pass,
            meld_execution::workflow::GateOutcome::Fail => GateOutcome::Fail,
        },
        reasons: result.reasons,
    }
}
