//! Workflow profile registry backed by the user workflow directory.

use crate::config::WorkflowConfig;
use crate::error::ApiError;
use crate::workflow::profile::{PromptRefKind, WorkflowProfile};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RegisteredWorkflowProfile {
    pub profile: WorkflowProfile,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowRegistry {
    profiles: HashMap<String, RegisteredWorkflowProfile>,
}

impl WorkflowRegistry {
    pub fn load(config: &WorkflowConfig) -> Result<Self, ApiError> {
        let mut registry = Self::default();
        let user_dir = config.resolve_user_profile_dir()?;
        registry.load_directory(&user_dir)?;
        Ok(registry)
    }

    pub fn get(&self, workflow_id: &str) -> Option<&RegisteredWorkflowProfile> {
        self.profiles.get(workflow_id)
    }

    pub fn contains(&self, workflow_id: &str) -> bool {
        self.profiles.contains_key(workflow_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &RegisteredWorkflowProfile)> {
        self.profiles.iter()
    }

    fn load_directory(&mut self, directory: &Path) -> Result<(), ApiError> {
        if !directory.exists() {
            return Ok(());
        }
        if !directory.is_dir() {
            return Err(ApiError::ConfigError(format!(
                "Workflow profile directory is not a directory: {}",
                directory.display()
            )));
        }

        let mut layer_seen_ids = HashSet::new();
        let mut profile_paths = collect_workflow_profile_paths(directory)?;
        profile_paths.sort();

        for profile_path in profile_paths {
            let content = fs::read_to_string(&profile_path).map_err(|err| {
                ApiError::ConfigError(format!(
                    "Failed to read workflow profile {}: {}",
                    profile_path.display(),
                    err
                ))
            })?;
            let profile: WorkflowProfile = serde_yaml::from_str(&content).map_err(|err| {
                ApiError::ConfigError(format!(
                    "Failed to parse workflow profile {}: {}",
                    profile_path.display(),
                    err
                ))
            })?;

            if !layer_seen_ids.insert(profile.workflow_id.clone()) {
                return Err(ApiError::ConfigError(format!(
                    "Duplicate workflow_id '{}' in workflow directory {}",
                    profile.workflow_id,
                    directory.display()
                )));
            }

            profile.validate()?;
            validate_prompt_refs(&profile, Some(&profile_path))?;

            self.profiles.insert(
                profile.workflow_id.clone(),
                RegisteredWorkflowProfile {
                    profile,
                    source_path: Some(profile_path),
                },
            );
        }

        Ok(())
    }
}

fn collect_workflow_profile_paths(root: &Path) -> Result<Vec<PathBuf>, ApiError> {
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
        if path.components().any(|component| {
            let value = component.as_os_str();
            value == "prompts" || value == "packages"
        }) {
            continue;
        }

        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };

        if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
            paths.push(path.to_path_buf());
        }
    }

    Ok(paths)
}

fn validate_prompt_refs(
    profile: &WorkflowProfile,
    source_path: Option<&Path>,
) -> Result<(), ApiError> {
    for turn in &profile.turns {
        match PromptRefKind::parse(&turn.prompt_ref) {
            PromptRefKind::ArtifactId(_) => {}
            PromptRefKind::FilePath(ref prompt_path) => {
                let resolved = resolve_prompt_path(prompt_path, source_path);
                if resolved.is_none() {
                    return Err(ApiError::ConfigError(format!(
                        "Workflow profile '{}' turn '{}' has unresolved prompt_ref '{}'",
                        profile.workflow_id, turn.turn_id, turn.prompt_ref
                    )));
                }
            }
        }
    }

    Ok(())
}

fn resolve_prompt_path(prompt_ref: &str, source_path: Option<&Path>) -> Option<PathBuf> {
    let candidate = PathBuf::from(prompt_ref);
    if candidate.is_absolute() {
        if candidate.exists() {
            return Some(candidate);
        }
        return None;
    }

    if let Some(source_path) = source_path {
        if let Some(parent) = source_path.parent() {
            let parent_relative = parent.join(&candidate);
            if parent_relative.exists() {
                return Some(parent_relative);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkflowConfig;
    use tempfile::TempDir;

    fn write_profile(path: &Path, workflow_id: &str, prompt_ref: &str) {
        let content = format!(
            r#"workflow_id: {workflow_id}
version: 1
title: Test Workflow
description: Test description
thread_policy:
  start_conditions: {{}}
  dedupe_key_fields:
    - workflow_id
  max_turn_retries: 1
turns:
  - turn_id: turn_a
    seq: 1
    title: First
    prompt_ref: {prompt_ref}
    input_refs:
      - target_context
    output_type: output_a
    gate_id: gate_a
    retry_limit: 1
    timeout_ms: 60000
gates:
  - gate_id: gate_a
    gate_type: schema_required_fields
    required_fields:
      - field_a
    fail_on_violation: true
artifact_policy:
  store_output: true
  store_prompt_render: true
  store_context_payload: true
  max_output_bytes: 1024
failure_policy:
  mode: fail_fast
  resume_from_failed_turn: true
  stop_on_gate_fail: true
"#
        );

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn load_reads_profiles_from_user_workflow_directory() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("user-workflows");
        let prompt_file = workflow_dir.join("prompts").join("custom.md");
        std::fs::create_dir_all(prompt_file.parent().unwrap()).unwrap();
        std::fs::write(&prompt_file, "test prompt").unwrap();
        write_profile(
            &workflow_dir.join("docs_writer_thread_v1.yaml"),
            "docs_writer_thread_v1",
            "prompts/custom.md",
        );
        let config = WorkflowConfig {
            user_profile_dir: Some(workflow_dir.clone()),
        };

        let registry = WorkflowRegistry::load(&config).unwrap();
        assert!(registry.contains("docs_writer_thread_v1"));

        let profile = registry.get("docs_writer_thread_v1").unwrap();
        assert_eq!(
            profile.source_path.as_ref(),
            Some(&workflow_dir.join("docs_writer_thread_v1.yaml"))
        );
    }

    #[test]
    fn load_fails_on_duplicate_workflow_id_in_same_layer() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("user-workflows");
        let prompt_file = workflow_dir.join("prompts").join("custom.md");
        std::fs::create_dir_all(prompt_file.parent().unwrap()).unwrap();
        std::fs::write(&prompt_file, "test prompt").unwrap();

        write_profile(
            &workflow_dir.join("a.yaml"),
            "workflow_a",
            "prompts/custom.md",
        );
        write_profile(
            &workflow_dir.join("b.yaml"),
            "workflow_a",
            "prompts/custom.md",
        );

        let config = WorkflowConfig {
            user_profile_dir: Some(workflow_dir),
        };
        let err = WorkflowRegistry::load(&config).unwrap_err();
        assert!(matches!(err, ApiError::ConfigError(_)));
    }

    #[test]
    fn load_fails_when_prompt_ref_file_missing() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("user-workflows");

        write_profile(
            &workflow_dir.join("missing.yaml"),
            "workflow_missing_prompt",
            "prompts/does_not_exist.md",
        );

        let config = WorkflowConfig {
            user_profile_dir: Some(workflow_dir),
        };
        let err = WorkflowRegistry::load(&config).unwrap_err();
        assert!(matches!(err, ApiError::ConfigError(_)));
    }

    #[test]
    fn load_ignores_task_package_documents_under_packages_directory() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("user-workflows");
        let prompt_file = workflow_dir.join("prompts").join("custom.md");
        std::fs::create_dir_all(prompt_file.parent().unwrap()).unwrap();
        std::fs::write(&prompt_file, "test prompt").unwrap();
        write_profile(
            &workflow_dir.join("docs_writer_thread_v1.yaml"),
            "docs_writer_thread_v1",
            "prompts/custom.md",
        );
        let package_file = workflow_dir.join("packages").join("docs_writer_v2.yaml");
        std::fs::create_dir_all(package_file.parent().unwrap()).unwrap();
        std::fs::write(
            &package_file,
            "package_id: docs_writer\nworkflow_id: docs_writer_thread_v1\ntrigger:\n  accepted_targets: []\n  required_runtime_fields: []\nseed:\n  artifacts: []\nexpansions: []\n",
        )
        .unwrap();

        let registry = WorkflowRegistry::load(&WorkflowConfig {
            user_profile_dir: Some(workflow_dir),
        })
        .unwrap();

        assert!(registry.contains("docs_writer_thread_v1"));
        assert_eq!(registry.iter().count(), 1);
    }
}
