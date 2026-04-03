//! Workflow profile schema and validation contracts.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowProfile {
    pub workflow_id: String,
    pub version: u32,
    pub title: String,
    pub description: String,
    pub thread_policy: WorkflowThreadPolicy,
    pub turns: Vec<WorkflowTurn>,
    pub gates: Vec<WorkflowGate>,
    pub artifact_policy: WorkflowArtifactPolicy,
    pub failure_policy: WorkflowFailurePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_artifact_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowThreadPolicy {
    #[serde(default)]
    pub start_conditions: Value,
    #[serde(default)]
    pub dedupe_key_fields: Vec<String>,
    pub max_turn_retries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowTurn {
    pub turn_id: String,
    pub seq: u32,
    pub title: String,
    pub prompt_ref: String,
    #[serde(default)]
    pub input_refs: Vec<String>,
    pub output_type: String,
    pub gate_id: String,
    pub retry_limit: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowGate {
    pub gate_id: String,
    pub gate_type: String,
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub rules: Value,
    pub fail_on_violation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowArtifactPolicy {
    pub store_output: bool,
    pub store_prompt_render: bool,
    pub store_context_payload: bool,
    pub max_output_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowFailurePolicy {
    pub mode: String,
    pub resume_from_failed_turn: bool,
    pub stop_on_gate_fail: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptRefKind {
    ArtifactId(String),
    FilePath(String),
}

impl PromptRefKind {
    pub fn parse(value: &str) -> Self {
        if let Some(rest) = value.strip_prefix("artifact:") {
            return Self::ArtifactId(rest.to_string());
        }
        if let Some(rest) = value.strip_prefix("builtin:") {
            return Self::FilePath(format!("prompts/{}.md", rest));
        }
        Self::FilePath(value.to_string())
    }
}

impl WorkflowProfile {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.workflow_id.trim().is_empty() {
            return Err(invalid_profile(
                &self.workflow_id,
                "workflow_id must not be empty",
            ));
        }
        if self.version == 0 {
            return Err(invalid_profile(
                &self.workflow_id,
                "version must be greater than zero",
            ));
        }
        if self.title.trim().is_empty() {
            return Err(invalid_profile(
                &self.workflow_id,
                "title must not be empty",
            ));
        }
        if self.description.trim().is_empty() {
            return Err(invalid_profile(
                &self.workflow_id,
                "description must not be empty",
            ));
        }
        if self.turns.is_empty() {
            return Err(invalid_profile(
                &self.workflow_id,
                "turns must not be empty",
            ));
        }
        if self.gates.is_empty() {
            return Err(invalid_profile(
                &self.workflow_id,
                "gates must not be empty",
            ));
        }
        if self.thread_policy.max_turn_retries == 0 {
            return Err(invalid_profile(
                &self.workflow_id,
                "thread_policy.max_turn_retries must be greater than zero",
            ));
        }
        if self.artifact_policy.max_output_bytes == 0 {
            return Err(invalid_profile(
                &self.workflow_id,
                "artifact_policy.max_output_bytes must be greater than zero",
            ));
        }

        let mut gate_ids: HashSet<&str> = HashSet::new();
        for gate in &self.gates {
            if gate.gate_id.trim().is_empty() {
                return Err(invalid_profile(
                    &self.workflow_id,
                    "gate_id must not be empty",
                ));
            }
            if gate.gate_type.trim().is_empty() {
                return Err(invalid_profile(
                    &self.workflow_id,
                    "gate_type must not be empty",
                ));
            }
            if !gate_ids.insert(gate.gate_id.as_str()) {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!("duplicate gate_id '{}'", gate.gate_id),
                ));
            }
        }

        let mut turn_ids: HashSet<&str> = HashSet::new();
        let mut sequences: HashSet<u32> = HashSet::new();
        for turn in &self.turns {
            if turn.turn_id.trim().is_empty() {
                return Err(invalid_profile(
                    &self.workflow_id,
                    "turn_id must not be empty",
                ));
            }
            if !turn_ids.insert(turn.turn_id.as_str()) {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!("duplicate turn_id '{}'", turn.turn_id),
                ));
            }
            if !sequences.insert(turn.seq) {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!("duplicate turn seq '{}'", turn.seq),
                ));
            }
            if turn.title.trim().is_empty() {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!("turn '{}' has empty title", turn.turn_id),
                ));
            }
            if turn.prompt_ref.trim().is_empty() {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!("turn '{}' has empty prompt_ref", turn.turn_id),
                ));
            }
            if turn.output_type.trim().is_empty() {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!("turn '{}' has empty output_type", turn.turn_id),
                ));
            }
            if turn.retry_limit == 0 {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!(
                        "turn '{}' retry_limit must be greater than zero",
                        turn.turn_id
                    ),
                ));
            }
            if turn.timeout_ms == 0 {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!(
                        "turn '{}' timeout_ms must be greater than zero",
                        turn.turn_id
                    ),
                ));
            }
            if !gate_ids.contains(turn.gate_id.as_str()) {
                return Err(invalid_profile(
                    &self.workflow_id,
                    &format!(
                        "turn '{}' references unknown gate_id '{}'",
                        turn.turn_id, turn.gate_id
                    ),
                ));
            }
        }

        Ok(())
    }

    pub fn ordered_turns(&self) -> Vec<WorkflowTurn> {
        let mut ordered = self.turns.clone();
        ordered.sort_by_key(|turn| turn.seq);
        ordered
    }
}

fn invalid_profile(workflow_id: &str, reason: &str) -> ApiError {
    ApiError::ConfigError(format!(
        "Workflow profile '{}' is invalid: {}",
        workflow_id, reason
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_profile() -> WorkflowProfile {
        WorkflowProfile {
            workflow_id: "docs_writer_thread_v1".to_string(),
            version: 1,
            title: "Docs writer".to_string(),
            description: "Four turn workflow".to_string(),
            thread_policy: WorkflowThreadPolicy {
                start_conditions: Value::Null,
                dedupe_key_fields: vec!["workflow_id".to_string()],
                max_turn_retries: 1,
            },
            turns: vec![WorkflowTurn {
                turn_id: "turn-1".to_string(),
                seq: 1,
                title: "First turn".to_string(),
                prompt_ref: "prompts/docs_writer/evidence_gather.md".to_string(),
                input_refs: vec!["target_context".to_string()],
                output_type: "evidence_map".to_string(),
                gate_id: "gate-1".to_string(),
                retry_limit: 1,
                timeout_ms: 60000,
            }],
            gates: vec![WorkflowGate {
                gate_id: "gate-1".to_string(),
                gate_type: "schema_required_fields".to_string(),
                required_fields: vec!["claims".to_string()],
                rules: Value::Null,
                fail_on_violation: true,
            }],
            artifact_policy: WorkflowArtifactPolicy {
                store_output: true,
                store_prompt_render: true,
                store_context_payload: true,
                max_output_bytes: 1024,
            },
            failure_policy: WorkflowFailurePolicy {
                mode: "fail_fast".to_string(),
                resume_from_failed_turn: true,
                stop_on_gate_fail: true,
            },
            thread_profile: None,
            target_agent_id: None,
            target_frame_type: None,
            final_artifact_type: None,
        }
    }

    #[test]
    fn validate_accepts_valid_profile() {
        valid_profile().validate().unwrap();
    }

    #[test]
    fn validate_rejects_duplicate_turn_seq() {
        let mut profile = valid_profile();
        let mut extra = profile.turns[0].clone();
        extra.turn_id = "turn-2".to_string();
        profile.turns.push(extra);

        let err = profile.validate().unwrap_err();
        assert!(matches!(err, ApiError::ConfigError(_)));
    }

    #[test]
    fn prompt_ref_kind_parses_prefixes() {
        assert_eq!(
            PromptRefKind::parse("artifact:abcd"),
            PromptRefKind::ArtifactId("abcd".to_string())
        );
        assert_eq!(
            PromptRefKind::parse("builtin:docs_writer/evidence_gather"),
            PromptRefKind::FilePath("prompts/docs_writer/evidence_gather.md".to_string())
        );
        assert_eq!(
            PromptRefKind::parse("config/workflows/prompts/test.md"),
            PromptRefKind::FilePath("config/workflows/prompts/test.md".to_string())
        );
    }
}
