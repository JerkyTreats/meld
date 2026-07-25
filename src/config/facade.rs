//! ConfigLoader facade delegating to merge service.

use super::merge::service::{MergeService, WorkspaceParticipation};
use super::MerkleConfig;
use config::ConfigError;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

/// Configuration loader facade.
pub struct ConfigLoader;

impl ConfigLoader {
    /// The global config file path resolved through XDG config home.
    #[cfg(test)]
    pub(crate) fn xdg_config_path() -> Option<PathBuf> {
        super::sources::global_file::global_config_path()
    }

    /// Load configuration without any workspace-local source.
    ///
    /// Workspace config participates only through explicit selection via
    /// [`ConfigLoader::load`]; nothing is discovered from the process
    /// working directory.
    pub fn load_global() -> Result<MerkleConfig, ConfigError> {
        MergeService::load_with(WorkspaceParticipation::Absent)
    }

    /// Load configuration with the given workspace root explicitly
    /// selected as a config source.
    pub fn load(workspace_root: &Path) -> Result<MerkleConfig, ConfigError> {
        MergeService::load(workspace_root)
    }

    /// Load configuration from a specific file.
    pub fn load_from_file(path: &Path) -> Result<MerkleConfig, ConfigError> {
        MergeService::load_from_file(path)
    }

    /// Create the default configuration value.
    pub fn load_default() -> MerkleConfig {
        MerkleConfig::default()
    }
}
