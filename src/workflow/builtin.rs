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
            "Build evidence for README generation from the provided Context. Return JSON only with schema {\"claims\":[{\"claim_id\":\"c1\",\"statement\":\"...\",\"evidence_path\":\"...\",\"evidence_symbol\":\"...\",\"evidence_quote\":\"...\"}]}. Use only exact paths, identifiers, and quotes that appear in Context. Each claim must cite one concrete path and one concrete symbol or quote. Prefer subsystem purpose, public entry points, determinism rules, and failure behavior. For directory targets, cover the module anchor first and then add at least one supported claim for each major child path that contains useful evidence. Do not let one file dominate if other child paths contain supported identifiers. Omit unsupported claims.",
        ),
        "docs_writer/verification" => Some(
            "Validate each claim against the provided evidence. Return JSON only with keys verified_claims, rejected_claims, and reasons. Keep verified_claims as full claim objects. Keep rejected_claims as objects with claim_id, statement, and reason_code. Verify a claim only when the cited path is present and the cited symbol or quote supports the statement. Reject generic restatements and unsupported summaries.",
        ),
        "docs_writer/readme_struct" => Some(
            "Build a structured README draft from verified_claims only. Return JSON only with key title and any supported keys among scope, purpose, api_surface, behavior_notes, usage, caveats, and related_components. Omit any section that is empty or lacks sufficient context. Use concrete module names, paths, and identifiers. For directory targets, explain module responsibilities, determinism rules, public entry points, and operational caveats when verified. In api_surface and related_components, cover the major child paths present in verified_claims rather than collapsing the README into one dominant file. Do not include an evidence_map key.",
        ),
        "docs_writer/style_refine" => Some(
            "Refine the README for clarity and flow. Return markdown only. Preserve technical meaning and preserve only the supported sections present in the input README structure. Use heading forms # title and ## Scope, ## Purpose, ## API Surface, ## Behavior Notes, ## Usage, ## Caveats, and ## Related Components for any sections you include. Omit sections that are absent or unsupported in the input. Do not add an Evidence Map section. Do not add files, symbols, or claims that are absent from the input.",
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
                fail_on_violation: false,
            },
            WorkflowGate {
                gate_id: "verification_gate".to_string(),
                gate_type: "schema_required_fields".to_string(),
                required_fields: vec!["verified_claims".to_string()],
                rules: serde_json::Value::Null,
                fail_on_violation: false,
            },
            WorkflowGate {
                gate_id: "struct_gate".to_string(),
                gate_type: "required_sections".to_string(),
                required_fields: vec!["title".to_string()],
                rules: json!({
                    "forbidden_sections": ["evidence_map"],
                }),
                fail_on_violation: false,
            },
            WorkflowGate {
                gate_id: "style_gate".to_string(),
                gate_type: "no_semantic_drift".to_string(),
                required_fields: vec![],
                rules: json!({
                    "forbidden_sections": ["evidence map"],
                    "required_sections_from_input": "readme_struct",
                }),
                fail_on_violation: false,
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
            stop_on_gate_fail: false,
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

    #[test]
    fn builtin_docs_writer_style_gate_targets_markdown_sections() {
        let profile = builtin_profiles()
            .into_iter()
            .find(|profile| profile.workflow_id == "docs_writer_thread_v1")
            .unwrap();

        let style_gate = profile
            .gates
            .iter()
            .find(|gate| gate.gate_id == "style_gate")
            .unwrap();

        assert_eq!(style_gate.gate_type, "no_semantic_drift");
        assert!(style_gate.required_fields.is_empty());
        assert_eq!(
            style_gate.rules["required_sections_from_input"],
            serde_json::json!("readme_struct")
        );
        assert_eq!(
            style_gate.rules["forbidden_sections"],
            serde_json::json!(["evidence map"])
        );
        assert!(!style_gate.fail_on_violation);
    }

    #[test]
    fn builtin_docs_writer_struct_gate_only_requires_title() {
        let profile = builtin_profiles()
            .into_iter()
            .find(|profile| profile.workflow_id == "docs_writer_thread_v1")
            .unwrap();

        let struct_gate = profile
            .gates
            .iter()
            .find(|gate| gate.gate_id == "struct_gate")
            .unwrap();

        assert_eq!(struct_gate.gate_type, "required_sections");
        assert_eq!(struct_gate.required_fields, vec!["title".to_string()]);
        assert!(!struct_gate.fail_on_violation);
    }

    #[test]
    fn builtin_docs_writer_prompts_allow_omitting_unsupported_sections() {
        let readme_struct = builtin_prompt_text("docs_writer/readme_struct").unwrap();
        let style_refine = builtin_prompt_text("docs_writer/style_refine").unwrap();

        assert!(readme_struct.contains("any supported keys among"));
        assert!(
            readme_struct.contains("Omit any section that is empty or lacks sufficient context")
        );
        assert!(style_refine.contains(
            "preserve only the supported sections present in the input README structure"
        ));
        assert!(style_refine.contains("Omit sections that are absent or unsupported in the input"));
    }

    #[test]
    fn builtin_docs_writer_gates_are_advisory() {
        let profile = builtin_profiles()
            .into_iter()
            .find(|profile| profile.workflow_id == "docs_writer_thread_v1")
            .unwrap();

        assert!(!profile.failure_policy.stop_on_gate_fail);
        assert!(profile.gates.iter().all(|gate| !gate.fail_on_violation));
    }
}
