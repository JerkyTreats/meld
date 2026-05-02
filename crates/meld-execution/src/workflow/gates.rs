use crate::workflow::profile::WorkflowGate;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    Pass,
    Fail,
}

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

    if gate.required_fields.is_empty() && gate.rules.get("required_sections_from_input").is_none() {
        return GateEvaluationResult::pass();
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
    let Ok(value) = serde_json::from_str::<Value>(input) else {
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
