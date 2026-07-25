//! Global config file source: $XDG_CONFIG_HOME/meld/config.toml, falling back to ~/.config/meld/config.toml

use crate::config::paths::xdg_root;
use config::builder::DefaultState;
use config::ConfigBuilder;
use config::ConfigError;
use config::File;
use std::path::PathBuf;
use tracing::warn;

/// Path to the global config file.
///
/// Resolved through XDG config home: `$XDG_CONFIG_HOME` wins over the
/// `$HOME/.config` fallback. Independent of the process working directory.
pub fn global_config_path() -> Option<PathBuf> {
    xdg_root::config_home()
        .ok()
        .map(|config_home| config_home.join("meld").join("config.toml"))
}

/// Add global config file source to builder if it exists.
pub fn add_to_builder(
    mut builder: ConfigBuilder<DefaultState>,
) -> Result<ConfigBuilder<DefaultState>, ConfigError> {
    if let Some(xdg_config_path) = global_config_path() {
        if xdg_config_path.exists() {
            let canonical_xdg_path = xdg_config_path
                .canonicalize()
                .unwrap_or_else(|_| xdg_config_path.clone());
            builder = builder
                .add_source(File::with_name(canonical_xdg_path.to_str().unwrap()).required(false));
        } else {
            warn!(
                config_path = %xdg_config_path.display(),
                "Global configuration file not found under XDG config home. \
                 Consider creating it for user-level defaults."
            );
        }
    }
    Ok(builder)
}
