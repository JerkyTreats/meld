//! Root task package discovery adapter.

use crate::error::ApiError;
use crate::task::package::TaskPackageSpec;
use crate::workflow::registry::RegisteredWorkflowProfile;

/// Loads the task package explicitly supplied beside a Workflow profile.
pub fn load_task_package_spec_for_workflow(
    registered_profile: &RegisteredWorkflowProfile,
) -> Result<Option<TaskPackageSpec>, ApiError> {
    meld_execution::task::package::load_task_package_spec_for_workflow(
        registered_profile,
        Some(crate::task::package::default_user_task_package_dir()?.as_path()),
    )
    .map_err(ApiError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::profile::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowProfile, WorkflowThreadPolicy,
    };
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn loads_explicit_package_document_from_workflow_directory() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("workflows");
        let package_dir = workflow_dir.join("packages");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(
            package_dir.join("docs_writer_v2.yaml"),
            r#"package_id: docs_writer_override
workflow_id: docs_writer_thread_v1
trigger:
  accepted_targets:
    - path
  required_runtime_fields:
    - agent_id
seed:
  artifacts: []
expansions: []
"#,
        )
        .unwrap();
        let registered_profile = RegisteredWorkflowProfile {
            profile: WorkflowProfile {
                workflow_id: "docs_writer_thread_v1".to_string(),
                version: 1,
                title: "Docs Writer".to_string(),
                description: "Writes docs".to_string(),
                thread_policy: WorkflowThreadPolicy {
                    start_conditions: json!({}),
                    dedupe_key_fields: Vec::new(),
                    max_turn_retries: 1,
                },
                turns: Vec::new(),
                gates: Vec::new(),
                artifact_policy: WorkflowArtifactPolicy {
                    store_output: true,
                    store_prompt_render: true,
                    store_context_payload: true,
                    max_output_bytes: 1024,
                },
                failure_policy: WorkflowFailurePolicy {
                    mode: "fail_fast".to_string(),
                    resume_from_failed_turn: false,
                    stop_on_gate_fail: true,
                },
                thread_profile: None,
                target_agent_id: None,
                target_frame_type: None,
                final_artifact_type: None,
                belief_context: None,
            },
            source_path: Some(workflow_dir.join("docs_writer_thread_v1.yaml")),
        };

        let spec = load_task_package_spec_for_workflow(&registered_profile)
            .unwrap()
            .unwrap();

        assert_eq!(spec.package_id, "docs_writer_override");
    }

    #[test]
    fn resolves_default_user_package_dir() {
        let path = crate::task::package::default_user_task_package_dir().unwrap();

        assert!(path.ends_with("packages"));
    }
}
