//! StorageConfig and resolve_paths for workspace storage.

use crate::config::xdg;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    /// Product runtime storage root. Relative paths resolve under the workspace root.
    #[serde(default)]
    pub product_root: Option<PathBuf>,
}

impl StorageConfig {
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
        if let Some(product_root) = &self.product_root {
            if product_root.is_absolute() {
                return Ok(product_root.clone());
            }
            return Ok(workspace_root.join(product_root));
        }

        Ok(xdg::workspace_data_dir(workspace_root)?.join("runtime"))
    }
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

    #[test]
    fn absent_product_root_resolves_to_xdg_runtime_root() {
        let workspace = tempfile::tempdir().unwrap();
        let config = StorageConfig::default();

        let root = config.resolve_product_root(workspace.path()).unwrap();
        let expected = xdg::workspace_data_dir(workspace.path())
            .unwrap()
            .join("runtime");

        assert_eq!(root, expected);
    }

    #[test]
    fn relative_product_root_resolves_under_workspace() {
        let workspace = tempfile::tempdir().unwrap();
        let config = StorageConfig {
            product_root: Some(PathBuf::from(".custom/runtime")),
            ..StorageConfig::default()
        };

        let root = config.resolve_product_root(workspace.path()).unwrap();

        assert_eq!(root, workspace.path().join(".custom/runtime"));
    }

    #[test]
    fn absolute_product_root_remains_absolute() {
        let workspace = tempfile::tempdir().unwrap();
        let absolute = workspace.path().join("absolute-runtime");
        let config = StorageConfig {
            product_root: Some(absolute.clone()),
            ..StorageConfig::default()
        };

        let root = config.resolve_product_root(workspace.path()).unwrap();

        assert_eq!(root, absolute);
    }

    #[test]
    fn existing_resolve_paths_behavior_is_unchanged() {
        let workspace = tempfile::tempdir().unwrap();
        let config = StorageConfig::default();

        let (store, frames, artifacts) = config.resolve_paths(workspace.path()).unwrap();
        let data_dir = xdg::workspace_data_dir(workspace.path()).unwrap();

        assert_eq!(store, data_dir.join("store"));
        assert_eq!(frames, data_dir.join("frames"));
        assert_eq!(artifacts, data_dir.join("artifacts"));
    }
}
