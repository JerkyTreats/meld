use crate::execution::contracts::ProviderExecutionBinding;
use crate::generation::NodeId;
use crate::workflow::profile::{WorkflowProfile, WorkflowTurn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionRequest {
    pub node_id: [u8; 32],
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub path: Option<String>,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionSummary {
    pub workflow_id: String,
    pub thread_id: String,
    pub turns_completed: usize,
    pub final_frame_id: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTargetProgressEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
    pub final_frame_id: Option<String>,
    pub turns_completed: Option<usize>,
    pub reused_existing_head: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTurnProgressEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub turn_seq: u32,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub attempt: usize,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
    pub final_frame_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowForceResetProgressEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub previous_frame_id: Option<String>,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
}

/// Builds the deterministic thread id for one workflow target.
pub fn workflow_thread_id(profile: &WorkflowProfile, node_id: NodeId, frame_type: &str) -> String {
    let payload = format!(
        "{}:{}:{}",
        profile.workflow_id,
        hex::encode(node_id),
        frame_type
    );
    let digest = blake3::hash(payload.as_bytes()).to_hex().to_string();
    format!("thread-{}", &digest[..16])
}

/// Returns the frame type used by one workflow turn output.
pub fn workflow_turn_frame_type(
    requested_frame_type: &str,
    turn: &WorkflowTurn,
    prompt_link_id: &str,
    is_final_turn: bool,
) -> String {
    if is_final_turn {
        return requested_frame_type.to_string();
    }

    format!(
        "{}--workflow-turn-{}-{}",
        requested_frame_type, turn.seq, prompt_link_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::profile::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowGate, WorkflowThreadPolicy,
    };

    fn test_profile() -> WorkflowProfile {
        WorkflowProfile {
            workflow_id: "docs_writer_thread_v1".to_string(),
            version: 1,
            title: "Docs Writer".to_string(),
            description: "Test workflow".to_string(),
            thread_policy: WorkflowThreadPolicy {
                start_conditions: serde_json::Value::Null,
                dedupe_key_fields: vec!["workflow_id".to_string()],
                max_turn_retries: 1,
            },
            turns: vec![],
            gates: vec![WorkflowGate {
                gate_id: "style_gate".to_string(),
                gate_type: "no_semantic_drift".to_string(),
                required_fields: vec![],
                rules: serde_json::Value::Null,
                fail_on_violation: false,
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
                stop_on_gate_fail: false,
            },
            thread_profile: None,
            target_agent_id: None,
            target_frame_type: None,
            final_artifact_type: None,
        }
    }

    #[test]
    fn thread_id_is_deterministic_for_same_inputs() {
        let profile = test_profile();
        let node_id = [1u8; 32];

        let left = workflow_thread_id(&profile, node_id, "context-docs-writer");
        let right = workflow_thread_id(&profile, node_id, "context-docs-writer");

        assert_eq!(left, right);
        assert!(left.starts_with("thread-"));
    }

    #[test]
    fn intermediate_turns_use_distinct_frame_types() {
        let turn = WorkflowTurn {
            turn_id: "evidence_gather".to_string(),
            seq: 1,
            title: "Gather Evidence".to_string(),
            prompt_ref: "prompts/docs_writer/evidence_gather.md".to_string(),
            input_refs: vec!["target_context".to_string()],
            output_type: "evidence_map".to_string(),
            gate_id: "evidence_gate".to_string(),
            retry_limit: 1,
            timeout_ms: 60000,
        };

        let frame_type =
            workflow_turn_frame_type("context-docs-writer", &turn, "prompt-link-abc", false);

        assert_eq!(
            frame_type,
            "context-docs-writer--workflow-turn-1-prompt-link-abc"
        );
    }

    #[test]
    fn final_turn_uses_requested_frame_type() {
        let turn = WorkflowTurn {
            turn_id: "style_refine".to_string(),
            seq: 4,
            title: "Refine Style".to_string(),
            prompt_ref: "prompts/docs_writer/style_refine.md".to_string(),
            input_refs: vec!["readme_struct".to_string()],
            output_type: "readme_final".to_string(),
            gate_id: "style_gate".to_string(),
            retry_limit: 1,
            timeout_ms: 60000,
        };

        let frame_type =
            workflow_turn_frame_type("context-docs-writer", &turn, "prompt-link-ignored", true);

        assert_eq!(frame_type, "context-docs-writer");
    }
}
