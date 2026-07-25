//! ConfigLoader facade delegating to merge service.

use super::merge::service::MergeService;
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

    /// Load configuration from files and environment.
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
