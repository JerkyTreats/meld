//! StorageConfig and resolve_paths for workspace storage.

use crate::config::xdg;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_store_path() -> PathBuf {
    PathBuf::from(".merkle/store")
}

fn default_frames_path() -> PathBuf {
    PathBuf::from(".merkle/frames")
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
}

impl StorageConfig {
    /// Resolve storage paths to actual filesystem locations.
    pub fn resolve_paths(&self, workspace_root: &Path) -> Result<(PathBuf, PathBuf), ApiError> {
        let is_default_store = self.store_path == PathBuf::from(".merkle/store");
        let is_default_frames = self.frames_path == PathBuf::from(".merkle/frames");

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

        Ok((store_path, frames_path))
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            store_path: default_store_path(),
            frames_path: default_frames_path(),
        }
    }
}
