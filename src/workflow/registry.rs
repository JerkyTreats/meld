//! Workflow profile registry with source priority resolution.

use crate::config::WorkflowConfig;
use crate::error::ApiError;
use crate::workflow::builtin::{builtin_profiles, builtin_prompt_text};
use crate::workflow::profile::{PromptRefKind, WorkflowProfile};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowProfileSourceLayer {
    Workspace,
    User,
    Builtin,
}

impl WorkflowProfileSourceLayer {
    fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::User => "user",
            Self::Builtin => "builtin",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegisteredWorkflowProfile {
    pub profile: WorkflowProfile,
    pub source_layer: WorkflowProfileSourceLayer,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowRegistry {
    profiles: HashMap<String, RegisteredWorkflowProfile>,
}

impl WorkflowRegistry {
    pub fn load(workspace_root: &Path, config: &WorkflowConfig) -> Result<Self, ApiError> {
        let mut registry = Self::default();

        let workspace_dir = config.resolve_workspace_profile_dir(workspace_root);
        let user_dir = config.resolve_user_profile_dir()?;

        registry.load_layer_from_directory(
            workspace_root,
            WorkflowProfileSourceLayer::Workspace,
            &workspace_dir,
        )?;
        registry.load_layer_from_directory(
            workspace_root,
            WorkflowProfileSourceLayer::User,
            &user_dir,
        )?;

        if config.enable_builtin_profiles {
            registry.load_builtin_layer(workspace_root)?;
        }

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

    fn load_layer_from_directory(
        &mut self,
        workspace_root: &Path,
        source_layer: WorkflowProfileSourceLayer,
        directory: &Path,
    ) -> Result<(), ApiError> {
        if !directory.exists() {
            return Ok(());
        }
        if !directory.is_dir() {
            return Err(ApiError::ConfigError(format!(
                "Workflow profile source '{}' is not a directory: {}",
                source_layer.as_str(),
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
                    "Duplicate workflow_id '{}' in {} source layer",
                    profile.workflow_id,
                    source_layer.as_str()
                )));
            }

            profile.validate()?;
            validate_prompt_refs(&profile, workspace_root, Some(&profile_path), source_layer)?;

            if self.contains(&profile.workflow_id) {
                continue;
            }

            self.profiles.insert(
                profile.workflow_id.clone(),
                RegisteredWorkflowProfile {
                    profile,
                    source_layer,
                    source_path: Some(profile_path),
                },
            );
        }

        Ok(())
    }

    fn load_builtin_layer(&mut self, workspace_root: &Path) -> Result<(), ApiError> {
        let mut seen = HashSet::new();
        for profile in builtin_profiles() {
            if !seen.insert(profile.workflow_id.clone()) {
                return Err(ApiError::ConfigError(format!(
                    "Duplicate workflow_id '{}' in builtin source layer",
                    profile.workflow_id
                )));
            }

            profile.validate()?;
            validate_prompt_refs(
                &profile,
                workspace_root,
                None,
                WorkflowProfileSourceLayer::Builtin,
            )?;

            if self.contains(&profile.workflow_id) {
                continue;
            }

            self.profiles.insert(
                profile.workflow_id.clone(),
                RegisteredWorkflowProfile {
                    profile,
                    source_layer: WorkflowProfileSourceLayer::Builtin,
                    source_path: None,
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
        if path
            .components()
            .any(|component| component.as_os_str() == "prompts")
        {
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
    workspace_root: &Path,
    source_path: Option<&Path>,
    source_layer: WorkflowProfileSourceLayer,
) -> Result<(), ApiError> {
    for turn in &profile.turns {
        match PromptRefKind::parse(&turn.prompt_ref) {
            PromptRefKind::ArtifactId(_) => {}
            PromptRefKind::Builtin(ref builtin_key) => {
                if builtin_prompt_text(builtin_key).is_none() {
                    return Err(ApiError::ConfigError(format!(
                        "Workflow profile '{}' turn '{}' references unknown builtin prompt_ref '{}'",
                        profile.workflow_id, turn.turn_id, turn.prompt_ref
                    )));
                }
            }
            PromptRefKind::FilePath(ref prompt_path) => {
                let resolved = resolve_prompt_path(prompt_path, workspace_root, source_path);
                if resolved.is_none() {
                    return Err(ApiError::ConfigError(format!(
                        "Workflow profile '{}' turn '{}' has unresolved prompt_ref '{}' in {} layer",
                        profile.workflow_id,
                        turn.turn_id,
                        turn.prompt_ref,
                        source_layer.as_str()
                    )));
                }
            }
        }
    }

    Ok(())
}

fn resolve_prompt_path(
    prompt_ref: &str,
    workspace_root: &Path,
    source_path: Option<&Path>,
) -> Option<PathBuf> {
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

    let workspace_relative = workspace_root.join(&candidate);
    if workspace_relative.exists() {
        return Some(workspace_relative);
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
    fn load_includes_builtin_docs_writer_profile_by_default() {
        let temp = TempDir::new().unwrap();
        let config = WorkflowConfig::default();

        let registry = WorkflowRegistry::load(temp.path(), &config).unwrap();
        assert!(registry.contains("docs_writer_thread_v1"));

        let profile = registry.get("docs_writer_thread_v1").unwrap();
        assert_eq!(profile.source_layer, WorkflowProfileSourceLayer::Builtin);
    }

    #[test]
    fn workspace_layer_wins_over_builtin_layer_for_matching_workflow_id() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("config").join("workflows");
        let prompt_file = temp
            .path()
            .join("config")
            .join("workflows")
            .join("prompts")
            .join("custom.md");
        std::fs::create_dir_all(prompt_file.parent().unwrap()).unwrap();
        std::fs::write(&prompt_file, "test prompt").unwrap();

        write_profile(
            &workflow_dir.join("docs_writer_thread_v1.yaml"),
            "docs_writer_thread_v1",
            "config/workflows/prompts/custom.md",
        );

        let config = WorkflowConfig::default();
        let registry = WorkflowRegistry::load(temp.path(), &config).unwrap();

        let resolved = registry.get("docs_writer_thread_v1").unwrap();
        assert_eq!(resolved.source_layer, WorkflowProfileSourceLayer::Workspace);
        assert!(resolved.source_path.is_some());
    }

    #[test]
    fn load_fails_on_duplicate_workflow_id_in_same_layer() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("config").join("workflows");
        let prompt_file = temp
            .path()
            .join("config")
            .join("workflows")
            .join("prompts")
            .join("custom.md");
        std::fs::create_dir_all(prompt_file.parent().unwrap()).unwrap();
        std::fs::write(&prompt_file, "test prompt").unwrap();

        write_profile(
            &workflow_dir.join("a.yaml"),
            "workflow_a",
            "config/workflows/prompts/custom.md",
        );
        write_profile(
            &workflow_dir.join("b.yaml"),
            "workflow_a",
            "config/workflows/prompts/custom.md",
        );

        let config = WorkflowConfig::default();
        let err = WorkflowRegistry::load(temp.path(), &config).unwrap_err();
        assert!(matches!(err, ApiError::ConfigError(_)));
    }

    #[test]
    fn load_fails_when_prompt_ref_file_missing() {
        let temp = TempDir::new().unwrap();
        let workflow_dir = temp.path().join("config").join("workflows");

        write_profile(
            &workflow_dir.join("missing.yaml"),
            "workflow_missing_prompt",
            "config/workflows/prompts/does_not_exist.md",
        );

        let config = WorkflowConfig::default();
        let err = WorkflowRegistry::load(temp.path(), &config).unwrap_err();
        assert!(matches!(err, ApiError::ConfigError(_)));
    }
}
