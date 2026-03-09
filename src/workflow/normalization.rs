//! Workflow output normalization before gate evaluation.

use crate::workflow::profile::WorkflowGate;
use serde_json::Value;

pub fn normalize_output_for_gate(gate: &WorkflowGate, output: &str) -> String {
    match gate.gate_type.as_str() {
        "required_sections" => normalize_json_sections(gate, output),
        "no_semantic_drift" => normalize_markdown_sections(gate, output),
        _ => output.to_string(),
    }
}

fn normalize_json_sections(gate: &WorkflowGate, output: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(output) else {
        return output.to_string();
    };
    let Some(object) = value.as_object_mut() else {
        return output.to_string();
    };

    let forbidden_tokens = forbidden_section_tokens(gate);
    let keys_to_remove = object
        .iter()
        .filter_map(|(key, value)| {
            let normalized_key = normalize_token(key);
            let forbidden = forbidden_tokens
                .iter()
                .any(|token| token == &normalized_key);
            if forbidden || is_low_signal_json_value(value) {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for key in keys_to_remove {
        object.remove(&key);
    }

    serde_json::to_string(&value).unwrap_or_else(|_| output.to_string())
}

fn normalize_markdown_sections(gate: &WorkflowGate, output: &str) -> String {
    let forbidden_tokens = forbidden_section_tokens(gate);
    let sections = split_markdown_sections(output);
    if sections.is_empty() {
        return output.to_string();
    }

    let mut normalized = Vec::new();
    for section in sections {
        if section.heading_level == 0 || section.heading_level == 1 {
            normalized.push(section.render());
            continue;
        }

        let heading_token = normalize_token(&section.heading_text);
        if forbidden_tokens.iter().any(|token| token == &heading_token) {
            continue;
        }
        if is_low_signal_text(&section.body) {
            continue;
        }

        normalized.push(section.render());
    }

    normalized.join("\n\n")
}

fn forbidden_section_tokens(gate: &WorkflowGate) -> Vec<String> {
    gate.rules
        .get("forbidden_sections")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(normalize_token)
        .collect()
}

fn is_low_signal_json_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => is_low_signal_text(text),
        Value::Array(values) => values.is_empty(),
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

fn is_low_signal_text(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return true;
    }

    let normalized = normalize_token(trimmed);
    normalized.is_empty() || normalized == "insufficientcontext"
}

fn normalize_token(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownSection {
    heading_level: usize,
    heading_text: String,
    body: String,
}

impl MarkdownSection {
    fn render(&self) -> String {
        if self.heading_level == 0 {
            return self.body.clone();
        }

        let hashes = "#".repeat(self.heading_level);
        if self.body.trim().is_empty() {
            format!("{} {}", hashes, self.heading_text)
        } else {
            format!("{} {}\n{}", hashes, self.heading_text, self.body)
        }
    }
}

fn split_markdown_sections(output: &str) -> Vec<MarkdownSection> {
    let mut sections = Vec::new();
    let mut current_heading_level = 0usize;
    let mut current_heading_text = String::new();
    let mut current_body_lines = Vec::new();

    for line in output.lines() {
        if let Some((level, text)) = parse_heading(line) {
            sections.push(MarkdownSection {
                heading_level: current_heading_level,
                heading_text: current_heading_text.clone(),
                body: current_body_lines.join("\n").trim().to_string(),
            });
            current_heading_level = level;
            current_heading_text = text.to_string();
            current_body_lines.clear();
            continue;
        }

        current_body_lines.push(line.to_string());
    }

    sections.push(MarkdownSection {
        heading_level: current_heading_level,
        heading_text: current_heading_text,
        body: current_body_lines.join("\n").trim().to_string(),
    });

    sections
        .into_iter()
        .filter(|section| section.heading_level != 0 || !section.body.is_empty())
        .collect()
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if level == 0 {
        return None;
    }

    let text = trimmed[level..].trim();
    if text.is_empty() {
        return None;
    }

    Some((level, text))
}

#[cfg(test)]
mod tests {
    use super::normalize_output_for_gate;
    use crate::workflow::profile::WorkflowGate;
    use serde_json::json;

    #[test]
    fn json_normalization_removes_forbidden_and_low_signal_sections() {
        let gate = WorkflowGate {
            gate_id: "struct_gate".to_string(),
            gate_type: "required_sections".to_string(),
            required_fields: vec![],
            rules: json!({ "forbidden_sections": ["evidence_map"] }),
            fail_on_violation: true,
        };

        let normalized = normalize_output_for_gate(
            &gate,
            r#"{"scope":"ok","purpose":"Insufficient context","usage":"","evidence_map":[],"caveats":"real"}"#,
        );

        assert_eq!(normalized, r#"{"caveats":"real","scope":"ok"}"#);
    }

    #[test]
    fn markdown_normalization_removes_forbidden_and_low_signal_sections() {
        let gate = WorkflowGate {
            gate_id: "style_gate".to_string(),
            gate_type: "no_semantic_drift".to_string(),
            required_fields: vec![],
            rules: json!({ "forbidden_sections": ["evidence map"] }),
            fail_on_violation: true,
        };

        let normalized = normalize_output_for_gate(
            &gate,
            "# Title\n\n## Scope\nUseful summary\n\n## Usage\nInsufficient context\n\n## Evidence Map\nShould go away\n\n## Caveats\n\n## Related Components\nReal details",
        );

        assert!(normalized.contains("## Scope\nUseful summary"));
        assert!(normalized.contains("## Related Components\nReal details"));
        assert!(!normalized.contains("## Usage"));
        assert!(!normalized.contains("## Evidence Map"));
        assert!(!normalized.contains("## Caveats"));
    }
}
