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
    /// Get the XDG config directory path (~/.config/merkle/config.toml)
    #[cfg(test)]
    pub(crate) fn xdg_config_path() -> Option<PathBuf> {
        std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("merkle")
                .join("config.toml")
        })
    }

    /// Load configuration from files and environment.
    pub fn load(workspace_root: &Path) -> Result<MerkleConfig, ConfigError> {
        MergeService::load(workspace_root)
    }

    /// Load configuration from a specific file.
    pub fn load_from_file(path: &Path) -> Result<MerkleConfig, ConfigError> {
        MergeService::load_from_file(path)
    }

    /// Create default configuration.
    pub fn default() -> MerkleConfig {
        MerkleConfig::default()
    }
}
