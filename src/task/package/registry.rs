//! Runtime loader for task package documents with embedded fallback.

use crate::error::ApiError;
use crate::task::package::TaskPackageSpec;
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::fs;
use std::path::{Path, PathBuf};

const BUILTIN_TASK_PACKAGE_IDS: [&str; 1] = ["docs_writer"];
const BUILTIN_DOCS_WRITER_PACKAGE_SOURCE: &str =
    include_str!("../../../workflows/packages/docs_writer_v2.yaml");

/// Loads one built-in task package document by package id.
pub fn load_builtin_task_package_spec(package_id: &str) -> Result<TaskPackageSpec, ApiError> {
    let source = match package_id {
        "docs_writer" => BUILTIN_DOCS_WRITER_PACKAGE_SOURCE,
        _ => {
            return Err(ApiError::ConfigError(format!(
                "Unknown built-in task package '{}'",
                package_id
            )));
        }
    };

    serde_yaml::from_str(source).map_err(|err| {
        ApiError::ConfigError(format!(
            "Failed to parse built-in task package '{}': {}",
            package_id, err
        ))
    })
}

/// Loads the built-in task package document bound to one workflow id, if any.
pub fn load_builtin_task_package_spec_for_workflow(
    workflow_id: &str,
) -> Result<Option<TaskPackageSpec>, ApiError> {
    for package_id in BUILTIN_TASK_PACKAGE_IDS {
        let spec = load_builtin_task_package_spec(package_id)?;
        if spec.workflow_id == workflow_id {
            return Ok(Some(spec));
        }
    }

    Ok(None)
}

/// Loads the task package document bound to one workflow id, preferring external package docs.
pub fn load_task_package_spec_for_workflow(
    registered_profile: &RegisteredWorkflowProfile,
) -> Result<Option<TaskPackageSpec>, ApiError> {
    if let Some(spec) = load_external_task_package_spec_for_workflow(registered_profile)? {
        return Ok(Some(spec));
    }

    load_builtin_task_package_spec_for_workflow(&registered_profile.profile.workflow_id)
}

fn load_external_task_package_spec_for_workflow(
    registered_profile: &RegisteredWorkflowProfile,
) -> Result<Option<TaskPackageSpec>, ApiError> {
    let package_dir = resolve_external_package_dir(registered_profile)?;
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
) -> Result<PathBuf, ApiError> {
    if let Some(source_path) = &registered_profile.source_path {
        if let Some(parent) = source_path.parent() {
            return Ok(parent.join("packages"));
        }
    }

    Ok(crate::config::WorkflowConfig::default()
        .resolve_user_profile_dir()?
        .join("packages"))
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
    fn loads_docs_writer_package_spec_from_embedded_yaml() {
        let spec = load_builtin_task_package_spec("docs_writer").unwrap();

        assert_eq!(spec.package_id, "docs_writer");
        assert_eq!(spec.workflow_id, "docs_writer_thread_v1");
        assert_eq!(spec.trigger.accepted_targets.len(), 2);
        assert_eq!(spec.seed.artifacts.len(), 3);
        assert_eq!(spec.expansions.len(), 1);
    }

    #[test]
    fn loads_builtin_package_by_workflow_id() {
        let spec = load_builtin_task_package_spec_for_workflow("docs_writer_thread_v1")
            .unwrap()
            .unwrap();

        assert_eq!(spec.package_id, "docs_writer");
    }

    #[test]
    fn prefers_external_package_document_from_workflow_directory() {
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
            },
            source_path: Some(workflow_dir.join("docs_writer_thread_v1.yaml")),
        };

        let spec = load_task_package_spec_for_workflow(&registered_profile)
            .unwrap()
            .unwrap();

        assert_eq!(spec.package_id, "docs_writer_override");
    }
}
