//! Load explicitly supplied Workflow task packages; product applications belong to PDS.

use super::TaskPackageSpec;
use crate::error::ApiError;
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::fs;
use std::path::{Path, PathBuf};

/// Load the package explicitly associated with a Workflow profile directory.
/// A missing package leaves an explicit turn profile on its existing generic route.
/// It cannot select a bundled product application by Workflow name.
pub fn load_task_package_spec_for_workflow(
    registered_profile: &RegisteredWorkflowProfile,
    default_package_dir: Option<&Path>,
) -> Result<Option<TaskPackageSpec>, ApiError> {
    let Some(package_dir) = resolve_external_package_dir(registered_profile, default_package_dir)
    else {
        return Ok(None);
    };
    if !package_dir.exists() {
        return Ok(None);
    }

    let mut package_paths = collect_package_paths(&package_dir)?;
    package_paths.sort();

    for package_path in package_paths {
        let spec = load_task_package_spec_from_path(&package_path)?;
        if spec.workflow_id == registered_profile.profile.workflow_id {
            return Ok(Some(spec));
        }
    }

    Ok(None)
}

fn resolve_external_package_dir(
    registered_profile: &RegisteredWorkflowProfile,
    default_package_dir: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(source_path) = &registered_profile.source_path {
        if let Some(parent) = source_path.parent() {
            return Some(parent.join("packages"));
        }
    }

    default_package_dir.map(Path::to_path_buf)
}

fn collect_package_paths(root: &Path) -> Result<Vec<PathBuf>, ApiError> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
            paths.push(path.to_path_buf());
        }
    }

    Ok(paths)
}

fn load_task_package_spec_from_path(path: &Path) -> Result<TaskPackageSpec, ApiError> {
    let content = fs::read_to_string(path).map_err(|err| {
        ApiError::ConfigError(format!(
            "Failed to read task package {}: {}",
            path.display(),
            err
        ))
    })?;

    serde_yaml::from_str(&content).map_err(|err| {
        ApiError::ConfigError(format!(
            "Failed to parse task package {}: {}",
            path.display(),
            err
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::profile::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowProfile, WorkflowThreadPolicy,
    };
    use serde_json::json;
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

        let spec = load_task_package_spec_for_workflow(&registered_profile, None)
            .unwrap()
            .unwrap();

        assert_eq!(spec.package_id, "docs_writer_override");
    }
}
