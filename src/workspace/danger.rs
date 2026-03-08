//! Destructive workspace runtime state operations.

use crate::config::{xdg, ConfigLoader};
use crate::error::ApiError;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FlushTarget {
    kind: &'static str,
    path: PathBuf,
}

/// Destructive workspace state operations.
pub struct WorkspaceDangerService;

impl WorkspaceDangerService {
    /// Remove all workspace runtime state except logs.
    pub fn flush(
        workspace_root: &Path,
        config_path: Option<&Path>,
        dry_run: bool,
        yes: bool,
    ) -> Result<String, ApiError> {
        let workspace_root = workspace_root.canonicalize().map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to canonicalize workspace path '{}': {}",
                workspace_root.display(),
                e
            ))
        })?;

        if !workspace_root.is_dir() {
            return Err(ApiError::ConfigError(format!(
                "Workspace path is not a directory: {}",
                workspace_root.display()
            )));
        }

        if !dry_run && !yes {
            return Err(ApiError::ConfigError(
                "Refusing destructive flush without --yes. Use --dry-run to inspect targets first."
                    .to_string(),
            ));
        }

        let targets = Self::resolve_flush_targets(&workspace_root, config_path)?;
        let existing_targets: Vec<FlushTarget> = targets
            .into_iter()
            .filter(|target| target.path.exists())
            .collect();

        if existing_targets.is_empty() {
            return Ok(format!(
                "No workspace runtime state found for {}. Logs preserved.",
                workspace_root.display()
            ));
        }

        if dry_run {
            let mut output = format!(
                "Would remove {} runtime state paths for {}. Logs preserved.",
                existing_targets.len(),
                workspace_root.display()
            );
            for target in &existing_targets {
                output.push_str(&format!("\n- {}: {}", target.kind, target.path.display()));
            }
            return Ok(output);
        }

        for target in &existing_targets {
            Self::remove_target(target)?;
        }

        let mut output = format!(
            "Removed {} runtime state paths for {}. Logs preserved.",
            existing_targets.len(),
            workspace_root.display()
        );
        for target in &existing_targets {
            output.push_str(&format!("\n- {}: {}", target.kind, target.path.display()));
        }
        Ok(output)
    }

    fn resolve_flush_targets(
        workspace_root: &Path,
        config_path: Option<&Path>,
    ) -> Result<Vec<FlushTarget>, ApiError> {
        let config = if let Some(config_path) = config_path {
            ConfigLoader::load_from_file(config_path)?
        } else {
            ConfigLoader::load(workspace_root)?
        };

        let (store_path, frames_path, artifacts_path) =
            config.system.storage.resolve_paths(workspace_root)?;

        let mut targets = Vec::new();
        if let Ok(data_dir) = xdg::workspace_data_dir(workspace_root) {
            targets.push(FlushTarget {
                kind: "workspace_data_root",
                path: data_dir,
            });
        }

        targets.push(FlushTarget {
            kind: "workspace_fallback_data_root",
            path: Self::fallback_workspace_data_dir(workspace_root),
        });
        targets.push(FlushTarget {
            kind: "node_store",
            path: store_path,
        });
        targets.push(FlushTarget {
            kind: "frame_store",
            path: frames_path,
        });
        targets.push(FlushTarget {
            kind: "artifact_store",
            path: artifacts_path,
        });

        Self::dedupe_and_validate_targets(workspace_root, targets)
    }

    fn dedupe_and_validate_targets(
        workspace_root: &Path,
        targets: Vec<FlushTarget>,
    ) -> Result<Vec<FlushTarget>, ApiError> {
        let mut unique = Vec::new();
        let mut seen = BTreeSet::new();

        for target in targets {
            if seen.insert(target.path.clone()) {
                Self::validate_target_path(workspace_root, &target.path)?;
                unique.push(target);
            }
        }

        unique.sort_by(|left, right| {
            left.path
                .components()
                .count()
                .cmp(&right.path.components().count())
                .then_with(|| left.path.cmp(&right.path))
        });

        let mut filtered = Vec::new();
        for target in unique {
            if filtered
                .iter()
                .any(|existing: &FlushTarget| target.path.starts_with(&existing.path))
            {
                continue;
            }
            filtered.push(target);
        }

        filtered.sort();
        Ok(filtered)
    }

    fn validate_target_path(workspace_root: &Path, target: &Path) -> Result<(), ApiError> {
        if target == workspace_root {
            return Err(ApiError::ConfigError(format!(
                "Refusing to flush workspace root directly: {}",
                target.display()
            )));
        }

        if workspace_root.starts_with(target) {
            return Err(ApiError::ConfigError(format!(
                "Refusing to remove ancestor of workspace root: {}",
                target.display()
            )));
        }

        Ok(())
    }

    fn remove_target(target: &FlushTarget) -> Result<(), ApiError> {
        if !target.path.exists() {
            return Ok(());
        }

        if target.path.is_dir() {
            fs::remove_dir_all(&target.path).map_err(|e| {
                ApiError::ConfigError(format!(
                    "Failed to remove {} '{}': {}",
                    target.kind,
                    target.path.display(),
                    e
                ))
            })
        } else {
            fs::remove_file(&target.path).map_err(|e| {
                ApiError::ConfigError(format!(
                    "Failed to remove {} '{}': {}",
                    target.kind,
                    target.path.display(),
                    e
                ))
            })
        }
    }

    fn fallback_workspace_data_dir(workspace_root: &Path) -> PathBuf {
        let mut data_dir = std::env::temp_dir().join("meld");
        for component in workspace_root.components() {
            match component {
                std::path::Component::RootDir => {}
                std::path::Component::Prefix(_) => {}
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {}
                std::path::Component::Normal(name) => {
                    data_dir = data_dir.join(name);
                }
            }
        }
        data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::WorkspaceDangerService;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn flush_requires_yes_without_dry_run() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();

        let error = WorkspaceDangerService::flush(&workspace, None, false, false).unwrap_err();
        assert!(error.to_string().contains("--yes"));
    }
}
