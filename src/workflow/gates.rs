//! Gate evaluation contracts for workflow turns.

use crate::workflow::profile::WorkflowGate;
use crate::workflow::record_contracts::GateOutcome;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvaluationResult {
    pub outcome: GateOutcome,
    pub reasons: Vec<String>,
}

impl GateEvaluationResult {
    pub fn pass() -> Self {
        Self {
            outcome: GateOutcome::Pass,
            reasons: Vec::new(),
        }
    }

    pub fn fail(reasons: Vec<String>) -> Self {
        Self {
            outcome: GateOutcome::Fail,
            reasons,
        }
    }

    pub fn is_pass(&self) -> bool {
        self.outcome == GateOutcome::Pass
    }
}

pub fn evaluate_gate(gate: &WorkflowGate, output: &str) -> GateEvaluationResult {
    match gate.gate_type.as_str() {
        "schema_required_fields" => evaluate_schema_required_fields(gate, output),
        "required_sections" => evaluate_required_sections(gate, output),
        "no_semantic_drift" => evaluate_no_semantic_drift(gate, output),
        unknown => GateEvaluationResult::fail(vec![format!("unknown gate_type '{}'", unknown)]),
    }
}

fn evaluate_schema_required_fields(gate: &WorkflowGate, output: &str) -> GateEvaluationResult {
    let parsed = serde_json::from_str::<Value>(output).ok();
    let mut reasons = Vec::new();

    for field in &gate.required_fields {
        let has_field_in_json = parsed
            .as_ref()
            .and_then(|value| value.as_object())
            .map(|object| object.contains_key(field))
            .unwrap_or(false);
        let has_field_in_text = output.to_lowercase().contains(&field.to_lowercase());

        if !has_field_in_json && !has_field_in_text {
            reasons.push(format!("missing required field '{}'", field));
        }
    }

    if reasons.is_empty() {
        GateEvaluationResult::pass()
    } else {
        GateEvaluationResult::fail(reasons)
    }
}

fn evaluate_required_sections(gate: &WorkflowGate, output: &str) -> GateEvaluationResult {
    let lowered_output = output.to_lowercase();
    let mut reasons = Vec::new();

    for section in &gate.required_fields {
        if !lowered_output.contains(&section.to_lowercase()) {
            reasons.push(format!("missing required section '{}'", section));
        }
    }

    if reasons.is_empty() {
        GateEvaluationResult::pass()
    } else {
        GateEvaluationResult::fail(reasons)
    }
}

fn evaluate_no_semantic_drift(gate: &WorkflowGate, output: &str) -> GateEvaluationResult {
    if output.trim().is_empty() {
        return GateEvaluationResult::fail(vec!["output is empty".to_string()]);
    }

    if gate.required_fields.is_empty() {
        return GateEvaluationResult::pass();
    }

    evaluate_required_sections(gate, output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::profile::WorkflowGate;

    #[test]
    fn schema_required_fields_passes_for_json_keys() {
        let gate = WorkflowGate {
            gate_id: "g1".to_string(),
            gate_type: "schema_required_fields".to_string(),
            required_fields: vec!["claims".to_string()],
            rules: Value::Null,
            fail_on_violation: true,
        };
        let result = evaluate_gate(&gate, r#"{"claims":[]}"#);
        assert!(result.is_pass());
    }

    #[test]
    fn required_sections_fails_when_missing() {
        let gate = WorkflowGate {
            gate_id: "g2".to_string(),
            gate_type: "required_sections".to_string(),
            required_fields: vec!["scope".to_string(), "usage".to_string()],
            rules: Value::Null,
            fail_on_violation: true,
        };
        let result = evaluate_gate(&gate, "# Title\n\n## Scope");
        assert!(!result.is_pass());
        assert_eq!(result.outcome, GateOutcome::Fail);
    }
}
