//! Workflow profile schema and validation contracts.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// Workflow profile contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowProfile {
    /// Workflow profile identifier that owns this execution record.
    pub workflow_id: String,
    /// Workflow profile schema or definition version.
    pub version: u32,
    /// Human-readable workflow title.
    pub title: String,
    /// Human-readable workflow description.
    pub description: String,
    /// Thread creation and resume policy.
    pub thread_policy: WorkflowThreadPolicy,
    /// Ordered workflow turn templates in this repeated region.
    pub turns: Vec<WorkflowTurn>,
    /// Gates available to workflow turns.
    pub gates: Vec<WorkflowGate>,
    /// Artifact persistence policy for workflow output.
    pub artifact_policy: WorkflowArtifactPolicy,
    /// Failure handling policy for workflow execution.
    pub failure_policy: WorkflowFailurePolicy,
    /// Optional external thread profile identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_profile: Option<String>,
    /// Target agent identifier carried across the execution boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_agent_id: Option<String>,
    /// Optional default frame type for workflow targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_frame_type: Option<String>,
    /// Optional artifact type expected from the final workflow turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_artifact_type: Option<String>,
    /// Opt-in flag for belief-conditioned generation: belief context bundle
    /// seeding plus belief-endorsed frame selection. Off when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belief_context: Option<bool>,
}

/// Environment override for the `belief_context` workflow flag.
///
/// A recognized token wins over the profile in both directions: `1`, `true`,
/// or `on` enables the flag; `0`, `false`, or `off` disables it. Matching is
/// trimmed and case-insensitive. Any other set value is not a silent
/// disable: it logs a warning and falls back to the profile, so a
/// misspelled override cannot silently flip an experiment arm.
pub const BELIEF_CONTEXT_ENV_VAR: &str = "MELD_BELIEF_CONTEXT";

/// Resolves the `belief_context` flag for one workflow profile.
///
/// Off by default; a profile may opt in per workflow, and a recognized
/// `MELD_BELIEF_CONTEXT` token overrides the profile in either direction.
pub fn belief_context_enabled(profile: &WorkflowProfile) -> bool {
    let env_value = std::env::var(BELIEF_CONTEXT_ENV_VAR).ok();
    resolve_belief_context(env_value.as_deref(), profile)
}

/// Parses one `MELD_BELIEF_CONTEXT` override token: `Some(true)` for a
/// truthy token, `Some(false)` for a falsy token, `None` when the token is
/// unrecognized and the profile must decide.
///
/// ```rust
/// use meld_execution::workflow::profile::parse_belief_context_token;
///
/// assert_eq!(parse_belief_context_token(" TRUE "), Some(true));
/// assert_eq!(parse_belief_context_token("off"), Some(false));
/// assert_eq!(parse_belief_context_token("yes"), None);
/// ```
pub fn parse_belief_context_token(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" => Some(true),
        "0" | "false" | "off" => Some(false),
        _ => None,
    }
}

/// Resolves the `belief_context` flag from an explicit environment override
/// value and the workflow profile. An unrecognized override value warns and
/// falls back to the profile instead of disabling the flag. Pure over its
/// inputs apart from that warning, so the resolution rules are testable
/// without mutating process-global environment state;
/// [`belief_context_enabled`] supplies the live `MELD_BELIEF_CONTEXT` value.
pub fn resolve_belief_context(env_value: Option<&str>, profile: &WorkflowProfile) -> bool {
    let profile_default = profile.belief_context.unwrap_or(false);
    match env_value {
        Some(value) => match parse_belief_context_token(value) {
            Some(enabled) => enabled,
            None => {
                tracing::warn!(
                    "Unrecognized {} value '{}'; expected 1/true/on or 0/false/off, \
                     falling back to the workflow profile",
                    BELIEF_CONTEXT_ENV_VAR,
                    value
                );
                profile_default
            }
        },
        None => profile_default,
    }
}

/// Workflow thread policy contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowThreadPolicy {
    /// Declarative conditions required before starting a thread.
    #[serde(default)]
    pub start_conditions: Value,
    /// Fields used to derive thread de-duplication keys.
    #[serde(default)]
    pub dedupe_key_fields: Vec<String>,
    /// Maximum retries allowed for each workflow turn.
    pub max_turn_retries: usize,
}

/// Workflow turn contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowTurn {
    /// Workflow turn identifier within the owning workflow profile or thread.
    pub turn_id: String,
    /// Workflow turn sequence number used for deterministic ordering.
    pub seq: u32,
    /// Human-readable turn title.
    pub title: String,
    /// Prompt reference resolved before provider execution.
    pub prompt_ref: String,
    /// Input references required to render this turn.
    #[serde(default)]
    pub input_refs: Vec<String>,
    /// Declared output type produced by this workflow turn.
    pub output_type: String,
    /// Gate identifier carried across the execution boundary.
    pub gate_id: String,
    /// Maximum attempts allowed for this turn or runtime step.
    pub retry_limit: usize,
    /// Timeout for one turn attempt in milliseconds.
    pub timeout_ms: u64,
}

/// Workflow gate contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowGate {
    /// Gate identifier carried across the execution boundary.
    pub gate_id: String,
    /// Gate evaluator type selected for this gate.
    pub gate_type: String,
    /// Fields that must appear in the turn output.
    #[serde(default)]
    pub required_fields: Vec<String>,
    /// Gate-specific rule payload.
    #[serde(default)]
    pub rules: Value,
    /// True when gate failure should fail the turn.
    pub fail_on_violation: bool,
}

/// Workflow artifact policy contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowArtifactPolicy {
    /// True when workflow turn output should be persisted.
    pub store_output: bool,
    /// True when rendered prompts should be persisted.
    pub store_prompt_render: bool,
    /// True when context payloads should be persisted.
    pub store_context_payload: bool,
    /// Maximum output bytes persisted for a turn.
    pub max_output_bytes: usize,
}

/// Workflow failure policy contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowFailurePolicy {
    /// Named failure handling mode.
    pub mode: String,
    /// True when failed threads may resume from the failed turn.
    pub resume_from_failed_turn: bool,
    /// True when gate failure stops workflow execution.
    pub stop_on_gate_fail: bool,
}

/// Prompt reference kind contract used by execution runtimes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptRefKind {
    /// Prompt reference resolved from a prompt artifact identifier.
    ArtifactId(String),
    /// Prompt reference resolved from a filesystem path.
    FilePath(String),
}

impl PromptRefKind {
    /// Parses an artifact, built-in, or filesystem prompt reference.
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
    /// Validates workflow profile shape and cross references.
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

    /// Returns workflow turns in deterministic sequence order.
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

    /// Mutation closure used by profile validation table cases.
    type ProfileMutation = Box<dyn FnOnce(&mut WorkflowProfile)>;

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
            belief_context: None,
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
    fn validate_rejects_profile_level_required_fields() {
        let cases: Vec<(&str, ProfileMutation, &str)> = vec![
            (
                "empty workflow id",
                Box::new(|profile| profile.workflow_id.clear()),
                "workflow_id must not be empty",
            ),
            (
                "zero version",
                Box::new(|profile| profile.version = 0),
                "version must be greater than zero",
            ),
            (
                "empty title",
                Box::new(|profile| profile.title.clear()),
                "title must not be empty",
            ),
            (
                "empty description",
                Box::new(|profile| profile.description.clear()),
                "description must not be empty",
            ),
            (
                "zero thread retry limit",
                Box::new(|profile| profile.thread_policy.max_turn_retries = 0),
                "max_turn_retries must be greater than zero",
            ),
            (
                "zero max output bytes",
                Box::new(|profile| profile.artifact_policy.max_output_bytes = 0),
                "max_output_bytes must be greater than zero",
            ),
        ];

        for (case_name, mutate, expected) in cases {
            let mut profile = valid_profile();
            mutate(&mut profile);

            let error = match profile.validate() {
                Ok(()) => panic!("{case_name} should fail validation"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(expected),
                "{case_name} expected error containing '{expected}', got '{error}'"
            );
        }
    }

    #[test]
    fn validate_rejects_gate_and_turn_contract_violations() {
        let cases: Vec<(&str, ProfileMutation, &str)> = vec![
            (
                "duplicate gate id",
                Box::new(|profile| profile.gates.push(profile.gates[0].clone())),
                "duplicate gate_id",
            ),
            (
                "empty gate id",
                Box::new(|profile| profile.gates[0].gate_id.clear()),
                "gate_id must not be empty",
            ),
            (
                "duplicate turn id",
                Box::new(|profile| {
                    let mut turn = profile.turns[0].clone();
                    turn.seq = 2;
                    profile.turns.push(turn);
                }),
                "duplicate turn_id",
            ),
            (
                "empty turn id",
                Box::new(|profile| profile.turns[0].turn_id.clear()),
                "turn_id must not be empty",
            ),
            (
                "unknown gate reference",
                Box::new(|profile| profile.turns[0].gate_id = "missing".to_string()),
                "references unknown gate_id",
            ),
            (
                "zero turn retry limit",
                Box::new(|profile| profile.turns[0].retry_limit = 0),
                "retry_limit must be greater than zero",
            ),
            (
                "zero timeout",
                Box::new(|profile| profile.turns[0].timeout_ms = 0),
                "timeout_ms must be greater than zero",
            ),
        ];

        for (case_name, mutate, expected) in cases {
            let mut profile = valid_profile();
            mutate(&mut profile);

            let error = match profile.validate() {
                Ok(()) => panic!("{case_name} should fail validation"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(expected),
                "{case_name} expected error containing '{expected}', got '{error}'"
            );
        }
    }

    #[test]
    fn ordered_turns_sorts_by_sequence_without_mutating_profile() {
        let mut profile = valid_profile();
        let mut second = profile.turns[0].clone();
        second.turn_id = "turn-2".to_string();
        second.seq = 2;
        profile.turns[0].seq = 3;
        profile.turns.push(second);

        let ordered = profile.ordered_turns();

        assert_eq!(ordered[0].turn_id, "turn-2");
        assert_eq!(profile.turns[0].turn_id, "turn-1");
    }

    #[test]
    fn belief_context_flag_defaults_off_profile_opts_in_env_overrides() {
        let mut profile = valid_profile();
        assert!(!resolve_belief_context(None, &profile));

        profile.belief_context = Some(true);
        assert!(resolve_belief_context(None, &profile));
        assert!(!resolve_belief_context(Some("0"), &profile));
        assert!(!resolve_belief_context(Some("false"), &profile));

        profile.belief_context = None;
        assert!(resolve_belief_context(Some("1"), &profile));
        assert!(resolve_belief_context(Some(" TRUE "), &profile));
        assert!(!resolve_belief_context(Some("off"), &profile));
    }

    #[test]
    fn belief_context_unrecognized_env_value_falls_back_to_profile() {
        // Natural-but-unsupported spellings and the empty string are not
        // silent disables: the profile decides in both directions.
        let mut profile = valid_profile();
        for value in ["yes", "enabled", ""] {
            assert!(!resolve_belief_context(Some(value), &profile));
        }

        profile.belief_context = Some(true);
        for value in ["yes", "enabled", ""] {
            assert!(resolve_belief_context(Some(value), &profile));
        }
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
