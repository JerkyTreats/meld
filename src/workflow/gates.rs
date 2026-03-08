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
    let normalized_output = normalize_section_token(output);
    let mut reasons = Vec::new();

    for section in &gate.required_fields {
        let normalized_section = normalize_section_token(section);
        if !normalized_output.contains(&normalized_section) {
            reasons.push(format!("missing required section '{}'", section));
        }
    }

    let forbidden_sections = gate
        .rules
        .get("forbidden_sections")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for section in forbidden_sections {
        let Some(section) = section.as_str() else {
            continue;
        };
        let normalized_section = normalize_section_token(section);
        if normalized_output.contains(&normalized_section) {
            reasons.push(format!("forbidden section '{}' present", section));
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

fn normalize_section_token(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
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

    #[test]
    fn required_sections_accepts_spacing_and_case_variants() {
        let gate = WorkflowGate {
            gate_id: "g3".to_string(),
            gate_type: "required_sections".to_string(),
            required_fields: vec![
                "api_surface".to_string(),
                "behavior_notes".to_string(),
                "related_components".to_string(),
            ],
            rules: Value::Null,
            fail_on_violation: true,
        };
        let output = r#"{
          "sections": {
            "API Surface": [],
            "Behavior Notes": [],
            "Related Components": []
          }
        }"#;

        let result = evaluate_gate(&gate, output);

        assert!(result.is_pass());
    }

    #[test]
    fn required_sections_fails_for_forbidden_sections() {
        let gate = WorkflowGate {
            gate_id: "g4".to_string(),
            gate_type: "required_sections".to_string(),
            required_fields: vec!["scope".to_string()],
            rules: serde_json::json!({
                "forbidden_sections": ["evidence_map"],
            }),
            fail_on_violation: true,
        };

        let result = evaluate_gate(&gate, r#"{"scope":"ok","evidence_map":[]}"#);

        assert!(!result.is_pass());
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("forbidden section 'evidence_map' present")));
    }
}
