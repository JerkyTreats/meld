use crate::workflow::profile::WorkflowGate;
use crate::workflow::record_contracts::GateOutcome;
use serde_json::Value;
use std::collections::HashMap;

/// Gate evaluation result contract used by execution runtimes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvaluationResult {
    /// Gate outcome recorded by workflow execution.
    pub outcome: GateOutcome,
    /// Human readable reasons emitted by gate evaluation.
    pub reasons: Vec<String>,
}

impl GateEvaluationResult {
    /// Builds a passing gate evaluation result.
    pub fn pass() -> Self {
        Self {
            outcome: GateOutcome::Pass,
            reasons: Vec::new(),
        }
    }

    /// Builds a failing gate evaluation result with reasons.
    pub fn fail(reasons: Vec<String>) -> Self {
        Self {
            outcome: GateOutcome::Fail,
            reasons,
        }
    }

    /// Returns true when the gate evaluation passed.
    pub fn is_pass(&self) -> bool {
        self.outcome == GateOutcome::Pass
    }
}

/// Evaluates workflow output against the selected gate contract.
pub fn evaluate_gate(
    gate: &WorkflowGate,
    output: &str,
    input_values: Option<&HashMap<String, String>>,
) -> GateEvaluationResult {
    match gate.gate_type.as_str() {
        "schema_required_fields" => evaluate_schema_required_fields(gate, output),
        "required_sections" => evaluate_required_sections(gate, output, input_values),
        "no_semantic_drift" => evaluate_no_semantic_drift(gate, output, input_values),
        unknown => GateEvaluationResult::fail(vec![format!("unknown gate_type '{}'", unknown)]),
    }
}

fn evaluate_schema_required_fields(gate: &WorkflowGate, output: &str) -> GateEvaluationResult {
    let mut reasons = Vec::new();

    match decode_json_lenient(output).as_ref().and_then(Value::as_object) {
        Some(object) => {
            for field in &gate.required_fields {
                if !object.contains_key(field) {
                    reasons.push(format!("missing required field '{}'", field));
                }
            }
            for field in non_empty_array_fields(gate) {
                if let Some(Value::Array(items)) = object.get(field) {
                    if items.is_empty() {
                        reasons.push(format!("required array '{}' is empty", field));
                    }
                }
            }
        }
        None => reasons.push("output is not a decodable JSON object".to_string()),
    }

    if reasons.is_empty() {
        GateEvaluationResult::pass()
    } else {
        GateEvaluationResult::fail(reasons)
    }
}

fn non_empty_array_fields(gate: &WorkflowGate) -> Vec<&str> {
    gate.rules
        .get("non_empty_arrays")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// Decodes model output as JSON, tolerating a fenced code block or leading
/// and trailing prose around one JSON object — the same shapes the finalize
/// path accepts. Presence of a field name in surrounding prose never counts.
fn decode_json_lenient(output: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(output.trim()) {
        return Some(value);
    }
    if let Some(fenced) = extract_fenced_block(output) {
        if let Ok(value) = serde_json::from_str::<Value>(fenced.trim()) {
            return Some(value);
        }
    }
    extract_first_json_object(output)
        .and_then(|slice| serde_json::from_str::<Value>(slice).ok())
}

fn extract_fenced_block(output: &str) -> Option<&str> {
    let open = output.find("```")?;
    let after_marker = &output[open + 3..];
    let body_start = after_marker.find('\n')? + 1;
    let body = &after_marker[body_start..];
    let close = body.find("```")?;
    Some(&body[..close])
}

fn extract_first_json_object(output: &str) -> Option<&str> {
    let start = output.find('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in output[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&output[start..start + offset + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

fn evaluate_required_sections(
    gate: &WorkflowGate,
    output: &str,
    input_values: Option<&HashMap<String, String>>,
) -> GateEvaluationResult {
    let normalized_output = normalize_section_token(output);
    let mut reasons = Vec::new();

    for section in required_sections_for_gate(gate, input_values) {
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

fn evaluate_no_semantic_drift(
    gate: &WorkflowGate,
    output: &str,
    input_values: Option<&HashMap<String, String>>,
) -> GateEvaluationResult {
    if output.trim().is_empty() {
        return GateEvaluationResult::fail(vec!["output is empty".to_string()]);
    }

    // A drift gate with nothing to check is a configuration or wiring
    // failure, not a pass: a vacuous pass is indistinguishable from a real
    // one in the durable record.
    if required_sections_for_gate(gate, input_values).is_empty() {
        return GateEvaluationResult::fail(vec![
            "no required sections resolved; gate has nothing to check against".to_string(),
        ]);
    }

    evaluate_required_sections(gate, output, input_values)
}

fn required_sections_for_gate<'a>(
    gate: &'a WorkflowGate,
    input_values: Option<&HashMap<String, String>>,
) -> Vec<&'a str> {
    let mut sections: Vec<&str> = gate.required_fields.iter().map(String::as_str).collect();

    let dynamic_sections = gate
        .rules
        .get("required_sections_from_input")
        .and_then(Value::as_str)
        .and_then(|input_key| {
            input_values
                .and_then(|values| values.get(input_key))
                .map(|value| collect_required_sections_from_input(value))
        })
        .unwrap_or_default();

    for section in dynamic_sections {
        if !sections.contains(&section) {
            sections.push(section);
        }
    }

    sections
}

fn collect_required_sections_from_input(input: &str) -> Vec<&'static str> {
    let Some(value) = decode_json_lenient(input) else {
        return Vec::new();
    };
    let Some(object) = value.as_object() else {
        return Vec::new();
    };

    let mut sections = Vec::new();
    for (key, value) in object {
        let Some(section_name) = markdown_section_name(key) else {
            continue;
        };
        if !section_has_meaningful_content(key, value) {
            continue;
        }
        sections.push(section_name);
    }

    sections
}

fn markdown_section_name(key: &str) -> Option<&'static str> {
    match key {
        "scope" => Some("scope"),
        "purpose" => Some("purpose"),
        "api_surface" => Some("api surface"),
        "behavior_notes" => Some("behavior notes"),
        "usage" => Some("usage"),
        "caveats" => Some("caveats"),
        "related_components" => Some("related components"),
        _ => None,
    }
}

fn section_has_meaningful_content(key: &str, value: &Value) -> bool {
    match key {
        "api_surface" => api_surface_has_meaningful_content(value),
        _ => value_has_meaningful_content(value),
    }
}

fn api_surface_has_meaningful_content(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(api_surface_entry_has_meaningful_content),
        Value::Object(_) => api_surface_entry_has_meaningful_content(value),
        _ => value_has_meaningful_content(value),
    }
}

fn api_surface_entry_has_meaningful_content(value: &Value) -> bool {
    match value {
        Value::Object(object) => object
            .iter()
            .any(|(key, value)| key != "path" && value_has_meaningful_content(value)),
        _ => value_has_meaningful_content(value),
    }
}

fn value_has_meaningful_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(boolean) => *boolean,
        Value::Number(_) => true,
        Value::String(string) => !string.trim().is_empty(),
        Value::Array(items) => items.iter().any(value_has_meaningful_content),
        Value::Object(object) => object.values().any(value_has_meaningful_content),
    }
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
    use serde_json::json;

    fn gate(gate_type: &str, required_fields: Vec<&str>) -> WorkflowGate {
        WorkflowGate {
            gate_id: "gate-1".to_string(),
            gate_type: gate_type.to_string(),
            required_fields: required_fields
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            rules: json!({}),
            fail_on_violation: true,
        }
    }

    #[test]
    fn schema_required_fields_requires_json_and_rejects_text_presence() {
        let gate = gate("schema_required_fields", vec!["claims", "evidence"]);

        assert!(evaluate_gate(&gate, r#"{"claims":[],"evidence":[]}"#, None).is_pass());
        // Fenced model output decodes; prose mentioning field names does not.
        assert!(evaluate_gate(
            &gate,
            "```json\n{\"claims\":[],\"evidence\":[]}\n```",
            None
        )
        .is_pass());
        assert!(!evaluate_gate(&gate, "Claims\nEvidence", None).is_pass());
    }

    #[test]
    fn schema_required_fields_rejects_empty_arrays_when_rule_named() {
        let mut gate = gate("schema_required_fields", vec!["claims"]);
        gate.rules = json!({ "non_empty_arrays": ["claims"] });

        assert!(!evaluate_gate(&gate, r#"{"claims":[]}"#, None).is_pass());
        assert!(evaluate_gate(&gate, r#"{"claims":[{"claim_id":"c1"}]}"#, None).is_pass());
    }

    #[test]
    fn no_semantic_drift_fails_when_no_sections_resolve() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({ "required_sections_from_input": "readme_struct" });

        // Input key absent entirely: nothing to check must fail, not pass.
        let result = evaluate_gate(&gate, "# A README\nBody", Some(&HashMap::new()));
        assert!(!result.is_pass());
    }

    #[test]
    fn schema_required_fields_reports_missing_fields() {
        let gate = gate("schema_required_fields", vec!["claims", "evidence"]);

        let result = evaluate_gate(&gate, r#"{"claims":[]}"#, None);

        assert_eq!(result.outcome, GateOutcome::Fail);
        assert!(!result.is_pass());
        assert_eq!(result.reasons, vec!["missing required field 'evidence'"]);
    }

    #[test]
    fn required_sections_enforces_missing_and_forbidden_sections() {
        let mut gate = gate("required_sections", vec!["purpose", "usage"]);
        gate.rules = json!({
            "forbidden_sections": ["caveats"]
        });

        let result = evaluate_gate(&gate, "## Purpose\nGood\n\n## Caveats\nNo", None);

        assert_eq!(result.outcome, GateOutcome::Fail);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("missing required section 'usage'")));
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("forbidden section 'caveats' present")));
    }

    #[test]
    fn no_semantic_drift_uses_dynamic_required_sections_from_input() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "purpose": "Explain the module",
                "api_surface": [
                    { "path": "lib.rs", "summary": "Public API" }
                ],
                "caveats": ""
            })
            .to_string(),
        )]);

        let passing = evaluate_gate(
            &gate,
            "## Purpose\nExplain the module\n\n## API Surface\nPublic API",
            Some(&input_values),
        );
        let failing = evaluate_gate(&gate, "## Purpose\nExplain the module", Some(&input_values));

        assert!(passing.is_pass());
        assert_eq!(failing.outcome, GateOutcome::Fail);
        assert!(failing.reasons[0].contains("api surface"));
    }

    #[test]
    fn no_semantic_drift_requires_scope_section_from_input() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "scope": "Document the public execution mutation behavior"
            })
            .to_string(),
        )]);

        let passing = evaluate_gate(
            &gate,
            "## Scope\nDocument the public execution mutation behavior",
            Some(&input_values),
        );
        let failing = evaluate_gate(&gate, "## Purpose\nDocument behavior", Some(&input_values));

        assert!(passing.is_pass());
        assert_eq!(failing.outcome, GateOutcome::Fail);
        assert!(failing
            .reasons
            .iter()
            .any(|reason| reason.contains("scope")));
    }

    #[test]
    fn no_semantic_drift_requires_purpose_section_from_input() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "purpose": "Explain why the execution mutation exists"
            })
            .to_string(),
        )]);

        let passing = evaluate_gate(
            &gate,
            "## Purpose\nExplain why the execution mutation exists",
            Some(&input_values),
        );
        let failing = evaluate_gate(&gate, "## Scope\nDocument behavior", Some(&input_values));

        assert!(passing.is_pass());
        assert_eq!(failing.outcome, GateOutcome::Fail);
        assert!(failing
            .reasons
            .iter()
            .any(|reason| reason.contains("purpose")));
    }

    #[test]
    fn no_semantic_drift_requires_usage_and_behavior_sections_from_input() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "behavior_notes": "Preserve existing task ordering",
                "usage": "Run the workflow through the task package path"
            })
            .to_string(),
        )]);

        let passing = evaluate_gate(
            &gate,
            "## Behavior Notes\nPreserve ordering\n\n## Usage\nRun through task package",
            Some(&input_values),
        );
        let failing = evaluate_gate(&gate, "## Purpose\nDocument behavior", Some(&input_values));

        assert!(passing.is_pass());
        assert_eq!(failing.outcome, GateOutcome::Fail);
        assert!(failing
            .reasons
            .iter()
            .any(|reason| reason.contains("behavior notes")));
        assert!(failing
            .reasons
            .iter()
            .any(|reason| reason.contains("usage")));
    }

    #[test]
    fn no_semantic_drift_requires_caveats_section_from_input() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "caveats": "Callers must preserve existing serialized fields"
            })
            .to_string(),
        )]);

        let passing = evaluate_gate(
            &gate,
            "## Caveats\nCallers must preserve existing serialized fields",
            Some(&input_values),
        );
        let failing = evaluate_gate(&gate, "## Purpose\nDocument behavior", Some(&input_values));

        assert!(passing.is_pass());
        assert_eq!(failing.outcome, GateOutcome::Fail);
        assert!(failing
            .reasons
            .iter()
            .any(|reason| reason.contains("caveats")));
    }

    #[test]
    fn no_semantic_drift_requires_related_components_section_from_input() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "related_components": ["task package prepare", "workflow gates"]
            })
            .to_string(),
        )]);

        let passing = evaluate_gate(
            &gate,
            "## Related Components\nTask package prepare and workflow gates",
            Some(&input_values),
        );
        let failing = evaluate_gate(&gate, "## Purpose\nDocument behavior", Some(&input_values));

        assert!(passing.is_pass());
        assert_eq!(failing.outcome, GateOutcome::Fail);
        assert!(failing
            .reasons
            .iter()
            .any(|reason| reason.contains("related components")));
    }

    #[test]
    fn no_semantic_drift_requires_api_surface_from_object_input() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "api_surface": {
                    "path": "lib.rs",
                    "summary": "Public workflow API"
                }
            })
            .to_string(),
        )]);

        let passing = evaluate_gate(
            &gate,
            "## API Surface\nPublic workflow API",
            Some(&input_values),
        );
        let failing = evaluate_gate(&gate, "## Purpose\nDocument behavior", Some(&input_values));

        assert!(passing.is_pass());
        assert_eq!(failing.outcome, GateOutcome::Fail);
        assert!(failing
            .reasons
            .iter()
            .any(|reason| reason.contains("api surface")));
    }

    #[test]
    fn no_semantic_drift_ignores_api_surface_object_with_only_path() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "api_surface": {
                    "path": "lib.rs"
                },
                "purpose": "Document behavior"
            })
            .to_string(),
        )]);

        let result = evaluate_gate(&gate, "## Purpose\nDocument behavior", Some(&input_values));

        assert!(result.is_pass());
    }

    #[test]
    fn no_semantic_drift_ignores_api_surface_array_with_only_paths() {
        let mut gate = gate("no_semantic_drift", vec![]);
        gate.rules = json!({
            "required_sections_from_input": "target_context"
        });
        let input_values = HashMap::from([(
            "target_context".to_string(),
            json!({
                "api_surface": [
                    { "path": "lib.rs" }
                ],
                "purpose": "Document behavior"
            })
            .to_string(),
        )]);

        let result = evaluate_gate(&gate, "## Purpose\nDocument behavior", Some(&input_values));

        assert!(result.is_pass());
    }

    #[test]
    fn no_semantic_drift_rejects_empty_output() {
        let gate = gate("no_semantic_drift", vec!["purpose"]);

        let result = evaluate_gate(&gate, "  ", None);

        assert_eq!(result.outcome, GateOutcome::Fail);
        assert_eq!(result.reasons, vec!["output is empty"]);
    }
}
