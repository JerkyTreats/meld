//! Builtin workflow profiles and prompts.

use crate::workflow::profile::{
    WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowGate, WorkflowProfile,
    WorkflowThreadPolicy, WorkflowTurn,
};
use serde_json::json;

pub fn builtin_profiles() -> Vec<WorkflowProfile> {
    vec![docs_writer_thread_v1()]
}

pub fn builtin_prompt_text(prompt_ref: &str) -> Option<&'static str> {
    match prompt_ref {
        "docs_writer/evidence_gather" => Some(
            "You are building evidence for README generation. Return JSON with a top level claims list and evidence references for each claim.",
        ),
        "docs_writer/verification" => Some(
            "Validate each claim using provided evidence and return JSON with verified_claims rejected_claims and reasons.",
        ),
        "docs_writer/readme_struct" => Some(
            "Build a structured README draft using verified_claims only. Return JSON with required sections and no style polish.",
        ),
        "docs_writer/style_refine" => Some(
            "Refine style and flow while preserving meaning. Return markdown in readme_markdown.",
        ),
        _ => None,
    }
}

fn docs_writer_thread_v1() -> WorkflowProfile {
    WorkflowProfile {
        workflow_id: "docs_writer_thread_v1".to_string(),
        version: 1,
        title: "Docs Writer Turn Workflow".to_string(),
        description: "Four turn docs generation with deterministic gates".to_string(),
        thread_policy: WorkflowThreadPolicy {
            start_conditions: json!({
                "require_directory_target": true,
                "require_target_head": true,
            }),
            dedupe_key_fields: vec![
                "workflow_id".to_string(),
                "target_node_id".to_string(),
                "target_frame_id".to_string(),
            ],
            max_turn_retries: 1,
        },
        turns: vec![
            WorkflowTurn {
                turn_id: "evidence_gather".to_string(),
                seq: 1,
                title: "Gather Evidence".to_string(),
                prompt_ref: "builtin:docs_writer/evidence_gather".to_string(),
                input_refs: vec!["target_context".to_string()],
                output_type: "evidence_map".to_string(),
                gate_id: "evidence_gate".to_string(),
                retry_limit: 1,
                timeout_ms: 60000,
            },
            WorkflowTurn {
                turn_id: "verification".to_string(),
                seq: 2,
                title: "Verify Claims".to_string(),
                prompt_ref: "builtin:docs_writer/verification".to_string(),
                input_refs: vec!["evidence_map".to_string()],
                output_type: "verification_report".to_string(),
                gate_id: "verification_gate".to_string(),
                retry_limit: 1,
                timeout_ms: 60000,
            },
            WorkflowTurn {
                turn_id: "readme_struct".to_string(),
                seq: 3,
                title: "Build Readme Structure".to_string(),
                prompt_ref: "builtin:docs_writer/readme_struct".to_string(),
                input_refs: vec!["verification_report".to_string()],
                output_type: "readme_struct".to_string(),
                gate_id: "struct_gate".to_string(),
                retry_limit: 1,
                timeout_ms: 60000,
            },
            WorkflowTurn {
                turn_id: "style_refine".to_string(),
                seq: 4,
                title: "Refine Style".to_string(),
                prompt_ref: "builtin:docs_writer/style_refine".to_string(),
                input_refs: vec!["readme_struct".to_string()],
                output_type: "readme_final".to_string(),
                gate_id: "style_gate".to_string(),
                retry_limit: 1,
                timeout_ms: 60000,
            },
        ],
        gates: vec![
            WorkflowGate {
                gate_id: "evidence_gate".to_string(),
                gate_type: "schema_required_fields".to_string(),
                required_fields: vec!["claims".to_string()],
                rules: serde_json::Value::Null,
                fail_on_violation: true,
            },
            WorkflowGate {
                gate_id: "verification_gate".to_string(),
                gate_type: "schema_required_fields".to_string(),
                required_fields: vec!["verified_claims".to_string()],
                rules: serde_json::Value::Null,
                fail_on_violation: true,
            },
            WorkflowGate {
                gate_id: "struct_gate".to_string(),
                gate_type: "required_sections".to_string(),
                required_fields: vec![
                    "scope".to_string(),
                    "purpose".to_string(),
                    "api_surface".to_string(),
                    "behavior_notes".to_string(),
                    "usage".to_string(),
                    "caveats".to_string(),
                    "related_components".to_string(),
                ],
                rules: serde_json::Value::Null,
                fail_on_violation: true,
            },
            WorkflowGate {
                gate_id: "style_gate".to_string(),
                gate_type: "no_semantic_drift".to_string(),
                required_fields: vec!["readme_markdown".to_string()],
                rules: serde_json::Value::Null,
                fail_on_violation: true,
            },
        ],
        artifact_policy: WorkflowArtifactPolicy {
            store_output: true,
            store_prompt_render: true,
            store_context_payload: true,
            max_output_bytes: 262144,
        },
        failure_policy: WorkflowFailurePolicy {
            mode: "fail_fast".to_string(),
            resume_from_failed_turn: true,
            stop_on_gate_fail: true,
        },
        thread_profile: Some("docs_writer_default".to_string()),
        target_agent_id: Some("docs-writer".to_string()),
        target_frame_type: Some("context-docs-writer".to_string()),
        final_artifact_type: Some("readme_final".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_docs_writer_profile_validates() {
        let profile = builtin_profiles()
            .into_iter()
            .find(|profile| profile.workflow_id == "docs_writer_thread_v1")
            .unwrap();
        profile.validate().unwrap();
    }

    #[test]
    fn builtin_prompts_are_available() {
        assert!(builtin_prompt_text("docs_writer/evidence_gather").is_some());
        assert!(builtin_prompt_text("docs_writer/verification").is_some());
        assert!(builtin_prompt_text("docs_writer/readme_struct").is_some());
        assert!(builtin_prompt_text("docs_writer/style_refine").is_some());
    }
}
