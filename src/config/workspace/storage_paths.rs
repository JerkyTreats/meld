//! StorageConfig and resolve_paths for workspace storage.

use crate::config::xdg;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

fn default_store_path() -> PathBuf {
    PathBuf::from(".meld/store")
}

fn default_frames_path() -> PathBuf {
    PathBuf::from(".meld/frames")
}

fn default_artifacts_path() -> PathBuf {
    PathBuf::from(".meld/artifacts")
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to node record store (relative to workspace root)
    #[serde(default = "default_store_path")]
    pub store_path: PathBuf,

    /// Path to frame storage (relative to workspace root)
    #[serde(default = "default_frames_path")]
    pub frames_path: PathBuf,

    /// Path to prompt context artifact storage (relative to workspace root)
    #[serde(default = "default_artifacts_path")]
    pub artifacts_path: PathBuf,

    /// Product runtime storage root. Relative paths resolve under the workspace XDG data root.
    #[serde(default)]
    pub product_root: Option<PathBuf>,
}

impl StorageConfig {
    /// A runtime without a workspace binds an explicit absolute product location.
    pub fn resolve_unscoped_product_root(&self) -> Result<PathBuf, ApiError> {
        let root = self.product_root.as_ref().ok_or_else(|| {
            ApiError::ConfigError(
                "a product without a workspace requires an absolute product_root".into(),
            )
        })?;
        resolve_existing_ancestor(&normalize_absolute(root)?)
    }

    /// Resolve storage paths to actual filesystem locations.
    pub fn resolve_paths(
        &self,
        workspace_root: &Path,
    ) -> Result<(PathBuf, PathBuf, PathBuf), ApiError> {
        let is_default_store = self.store_path == Path::new(".meld/store");
        let is_default_frames = self.frames_path == Path::new(".meld/frames");
        let is_default_artifacts = self.artifacts_path == Path::new(".meld/artifacts");

        let store_path = if is_default_store {
            let data_dir = xdg::workspace_data_dir(workspace_root)?;
            data_dir.join("store")
        } else {
            workspace_root.join(&self.store_path)
        };

        let frames_path = if is_default_frames {
            let data_dir = xdg::workspace_data_dir(workspace_root)?;
            data_dir.join("frames")
        } else {
            workspace_root.join(&self.frames_path)
        };

        let artifacts_path = if is_default_artifacts {
            let data_dir = xdg::workspace_data_dir(workspace_root)?;
            data_dir.join("artifacts")
        } else {
            workspace_root.join(&self.artifacts_path)
        };

        Ok((store_path, frames_path, artifacts_path))
    }

    /// Resolve the product runtime storage root.
    pub fn resolve_product_root(&self, workspace_root: &Path) -> Result<PathBuf, ApiError> {
        let workspace = workspace_root.canonicalize().map_err(|error| {
            ApiError::ConfigError(format!(
                "Failed to canonicalize workspace path {}: {error}",
                workspace_root.display()
            ))
        })?;
        if let Some(configured) = &self.product_root {
            if configured.is_absolute() {
                let lexical = normalize_absolute(configured)?;
                if !path_is_within(&lexical, &workspace) {
                    return validate_external_product_root(lexical, &workspace, None, None);
                }
            }
        }
        let xdg_root = normalize_absolute(&xdg::workspace_data_dir(&workspace)?)?;
        let external_default = xdg_root.join("runtime");
        let resolved_xdg_root = resolve_existing_ancestor(&xdg_root)?;

        if path_is_within(&resolved_xdg_root, &workspace) {
            return Err(ApiError::ProductRootInsideWorkspace {
                old_path: xdg_root,
                new_path: resolved_xdg_root,
            });
        }

        let Some(configured) = &self.product_root else {
            return validate_external_product_root(
                external_default,
                &workspace,
                Some(&resolved_xdg_root),
                None,
            );
        };

        if configured.is_absolute() {
            let lexical = normalize_absolute(configured)?;
            if path_is_within(&lexical, &workspace) {
                return Err(ApiError::ProductRootInsideWorkspace {
                    old_path: lexical,
                    new_path: external_default,
                });
            }
            return validate_external_product_root(
                lexical,
                &workspace,
                None,
                Some(external_default),
            );
        }

        let candidate = normalize_absolute(&xdg_root.join(configured))?;
        if !path_is_within(&candidate, &xdg_root) {
            return Err(ApiError::ConfigError(format!(
                "Relative product runtime root '{}' escapes workspace XDG data root {}",
                configured.display(),
                xdg_root.display()
            )));
        }

        let legacy = normalize_absolute(&workspace.join(configured))?;
        if legacy.exists() && path_is_within(&legacy, &workspace) {
            return Err(ApiError::ProductRootInsideWorkspace {
                old_path: legacy,
                new_path: candidate,
            });
        }

        validate_external_product_root(candidate, &workspace, Some(&resolved_xdg_root), None)
    }
}

fn validate_external_product_root(
    candidate: PathBuf,
    workspace: &Path,
    required_root: Option<&Path>,
    replacement: Option<PathBuf>,
) -> Result<PathBuf, ApiError> {
    let resolved = resolve_existing_ancestor(&candidate)?;
    if path_is_within(&resolved, workspace) {
        return Err(ApiError::ProductRootInsideWorkspace {
            old_path: candidate,
            new_path: replacement.unwrap_or(resolved),
        });
    }
    if required_root.is_some_and(|root| !path_is_within(&resolved, root)) {
        return Err(ApiError::ConfigError(format!(
            "Relative product runtime root resolves outside workspace XDG data root: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, ApiError> {
    if !path.is_absolute() {
        return Err(ApiError::ConfigError(format!(
            "Product runtime path must resolve to an absolute path: {}",
            path.display()
        )));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }
    Ok(normalized)
}

fn resolve_existing_ancestor(path: &Path) -> Result<PathBuf, ApiError> {
    for ancestor in path.ancestors() {
        if ancestor.exists() {
            let resolved = ancestor.canonicalize().map_err(|error| {
                ApiError::ConfigError(format!(
                    "Failed to resolve product runtime path {}: {error}",
                    path.display()
                ))
            })?;
            let suffix = path.strip_prefix(ancestor).map_err(|error| {
                ApiError::ConfigError(format!(
                    "Failed to normalize product runtime path {}: {error}",
                    path.display()
                ))
            })?;
            return Ok(resolved.join(suffix));
        }
    }

    Err(ApiError::ConfigError(format!(
        "Product runtime path has no resolvable ancestor: {}",
        path.display()
    )))
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            store_path: default_store_path(),
            frames_path: default_frames_path(),
            artifacts_path: default_artifacts_path(),
            product_root: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::Mutex;

    static XDG_DATA_HOME_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvVarRestore {
        key: &'static str,
        original: Option<OsString>,
    }

    impl Drop for EnvVarRestore {
        fn drop(&mut self) {
            if let Some(original) = &self.original {
                std::env::set_var(self.key, original);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn with_isolated_xdg_data_home<T>(test: impl FnOnce() -> T) -> T {
        let _guard = XDG_DATA_HOME_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let data_home = tempfile::tempdir().unwrap();
        let _restore = EnvVarRestore {
            key: "XDG_DATA_HOME",
            original: std::env::var_os("XDG_DATA_HOME"),
        };

        std::env::set_var("XDG_DATA_HOME", data_home.path());

        test()
    }

    #[test]
    fn absent_product_root_resolves_to_xdg_runtime_root() {
        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let config = StorageConfig::default();

            let root = config.resolve_product_root(workspace.path()).unwrap();
            let expected = xdg::workspace_data_dir(workspace.path())
                .unwrap()
                .join("runtime");

            assert_eq!(root, expected);
        });
    }

    #[test]
    fn relative_product_root_resolves_under_workspace_xdg_data_root() {
        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let config = StorageConfig {
                product_root: Some(PathBuf::from("custom/runtime")),
                ..StorageConfig::default()
            };

            let root = config.resolve_product_root(workspace.path()).unwrap();

            assert_eq!(
                root,
                xdg::workspace_data_dir(workspace.path())
                    .unwrap()
                    .join("custom/runtime")
            );
            assert!(!root.starts_with(workspace.path()));
        });
    }

    #[test]
    fn absolute_product_root_outside_workspace_is_allowed() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let absolute = external.path().join("absolute-runtime");
        let config = StorageConfig {
            product_root: Some(absolute.clone()),
            ..StorageConfig::default()
        };

        let root = config.resolve_product_root(workspace.path()).unwrap();

        assert_eq!(root, absolute);
    }

    #[test]
    fn absolute_product_root_inside_workspace_is_rejected_before_it_exists() {
        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let absolute = workspace.path().join("runtime");
            let config = StorageConfig {
                product_root: Some(absolute.clone()),
                ..StorageConfig::default()
            };

            let error = config.resolve_product_root(workspace.path()).unwrap_err();

            match error {
                ApiError::ProductRootInsideWorkspace { old_path, new_path } => {
                    assert_eq!(old_path, absolute);
                    assert_eq!(
                        new_path,
                        xdg::workspace_data_dir(workspace.path())
                            .unwrap()
                            .join("runtime")
                    );
                }
                other => panic!("unexpected error: {other}"),
            }
        });
    }

    #[test]
    fn existing_relative_legacy_root_fails_with_old_and_new_paths() {
        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let legacy = workspace.path().join("legacy/runtime");
            std::fs::create_dir_all(&legacy).unwrap();
            let config = StorageConfig {
                product_root: Some(PathBuf::from("legacy/runtime")),
                ..StorageConfig::default()
            };

            let error = config.resolve_product_root(workspace.path()).unwrap_err();

            match error {
                ApiError::ProductRootInsideWorkspace { old_path, new_path } => {
                    assert_eq!(old_path, legacy);
                    assert_eq!(
                        new_path,
                        xdg::workspace_data_dir(workspace.path())
                            .unwrap()
                            .join("legacy/runtime")
                    );
                }
                other => panic!("unexpected error: {other}"),
            }
        });
    }

    #[test]
    fn parent_components_cannot_escape_workspace_xdg_root() {
        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let config = StorageConfig {
                product_root: Some(PathBuf::from("nested/../../escape")),
                ..StorageConfig::default()
            };

            let error = config.resolve_product_root(workspace.path()).unwrap_err();

            assert!(
                matches!(error, ApiError::ConfigError(message) if message.contains("escapes workspace XDG data root"))
            );
        });
    }

    #[cfg(unix)]
    #[test]
    fn relative_symlink_cannot_escape_workspace_xdg_root() {
        use std::os::unix::fs::symlink;

        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let external = tempfile::tempdir().unwrap();
            let xdg_root = xdg::workspace_data_dir(workspace.path()).unwrap();
            std::fs::create_dir_all(&xdg_root).unwrap();
            symlink(external.path(), xdg_root.join("escape")).unwrap();
            let config = StorageConfig {
                product_root: Some(PathBuf::from("escape/runtime")),
                ..StorageConfig::default()
            };

            let error = config.resolve_product_root(workspace.path()).unwrap_err();

            assert!(matches!(
                error,
                ApiError::ConfigError(message)
                    if message.contains("resolves outside workspace XDG data root")
            ));
        });
    }

    #[test]
    fn empty_relative_product_root_is_rejected_as_existing_legacy_workspace_root() {
        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let config = StorageConfig {
                product_root: Some(PathBuf::new()),
                ..StorageConfig::default()
            };

            let error = config.resolve_product_root(workspace.path()).unwrap_err();

            assert!(matches!(
                error,
                ApiError::ProductRootInsideWorkspace { old_path, new_path }
                    if old_path == workspace.path()
                        && new_path == xdg::workspace_data_dir(workspace.path()).unwrap()
            ));
        });
    }

    #[cfg(unix)]
    #[test]
    fn absolute_symlink_resolving_inside_workspace_is_rejected() {
        use std::os::unix::fs::symlink;

        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let external = tempfile::tempdir().unwrap();
            let link = external.path().join("workspace-link");
            symlink(workspace.path(), &link).unwrap();
            let config = StorageConfig {
                product_root: Some(link.join("runtime")),
                ..StorageConfig::default()
            };

            let error = config.resolve_product_root(workspace.path()).unwrap_err();

            assert!(matches!(
                error,
                ApiError::ProductRootInsideWorkspace { old_path, .. }
                    if old_path == link.join("runtime")
            ));
        });
    }

    #[cfg(unix)]
    #[test]
    fn absolute_path_lexically_inside_workspace_is_rejected_even_when_symlink_is_external() {
        use std::os::unix::fs::symlink;

        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let external = tempfile::tempdir().unwrap();
            let link = workspace.path().join("external-link");
            symlink(external.path(), &link).unwrap();
            let config = StorageConfig {
                product_root: Some(link.join("runtime")),
                ..StorageConfig::default()
            };

            let error = config.resolve_product_root(workspace.path()).unwrap_err();

            assert!(matches!(
                error,
                ApiError::ProductRootInsideWorkspace { old_path, .. }
                    if old_path == link.join("runtime")
            ));
        });
    }

    #[cfg(unix)]
    #[test]
    fn xdg_data_symlink_resolving_inside_workspace_is_rejected() {
        use std::os::unix::fs::symlink;

        let _guard = XDG_DATA_HOME_MUTEX
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let data_link = external.path().join("data-link");
        symlink(workspace.path(), &data_link).unwrap();
        let _restore = EnvVarRestore {
            key: "XDG_DATA_HOME",
            original: std::env::var_os("XDG_DATA_HOME"),
        };
        std::env::set_var("XDG_DATA_HOME", &data_link);

        let error = StorageConfig::default()
            .resolve_product_root(workspace.path())
            .unwrap_err();

        assert!(matches!(
            error,
            ApiError::ProductRootInsideWorkspace { old_path, .. }
                if old_path.starts_with(workspace.path())
                    || old_path.starts_with(&data_link)
        ));
    }

    #[test]
    fn existing_resolve_paths_behavior_is_unchanged() {
        with_isolated_xdg_data_home(|| {
            let workspace = tempfile::tempdir().unwrap();
            let config = StorageConfig::default();

            let (store, frames, artifacts) = config.resolve_paths(workspace.path()).unwrap();
            let data_dir = xdg::workspace_data_dir(workspace.path()).unwrap();

            assert_eq!(store, data_dir.join("store"));
            assert_eq!(frames, data_dir.join("frames"));
            assert_eq!(artifacts, data_dir.join("artifacts"));
        });
    }
}
