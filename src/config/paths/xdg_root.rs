//! XDG Base Directory utilities for workspace data management.

use crate::error::ApiError;
use std::path::{Path, PathBuf};

/// Get XDG data home directory
///
/// Returns `$XDG_DATA_HOME` if set, otherwise defaults to `$HOME/.local/share`
/// Follows XDG Base Directory Specification
pub fn data_home() -> Option<PathBuf> {
    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg_data_home));
    }

    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".local").join("share"))
}

/// Get the data directory for a specific workspace
///
/// Returns `$XDG_DATA_HOME/meld/<workspace_path>/`
///
/// The workspace path is canonicalized and used directly as a directory structure.
/// For example, `/home/user/projects/myproject` becomes:
/// `$XDG_DATA_HOME/meld/home/user/projects/myproject/`
///
/// This eliminates the need for any `.meld/` directory in the workspace.
pub fn workspace_data_dir(workspace_root: &Path) -> Result<PathBuf, ApiError> {
    let data_home = data_home().ok_or_else(|| {
        ApiError::ConfigError(
            "Could not determine XDG data home directory (HOME not set)".to_string(),
        )
    })?;

    // Canonicalize the workspace path to get an absolute, resolved path
    let canonical = workspace_root.canonicalize().map_err(|e| {
        ApiError::ConfigError(format!("Failed to canonicalize workspace path: {}", e))
    })?;

    // Build the data directory path by joining the canonical path components
    // Remove the leading root component (/) and use the rest as directory structure
    let mut data_dir = data_home.join("meld");

    // Iterate through path components, skipping the root
    for component in canonical.components() {
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

    Ok(data_dir)
}

/// Get XDG config home directory
///
/// Returns `$XDG_CONFIG_HOME` if set, otherwise defaults to `$HOME/.config`
/// Follows XDG Base Directory Specification
pub fn config_home() -> Result<PathBuf, ApiError> {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg_config_home));
    }

    let home = std::env::var("HOME").map_err(|_| {
        ApiError::ConfigError(
            "Could not determine XDG config home directory (HOME not set)".to_string(),
        )
    })?;

    Ok(PathBuf::from(home).join(".config"))
}

/// Get agents directory path
///
/// Returns `$XDG_CONFIG_HOME/meld/agents/`
/// Creates the directory if it doesn't exist
pub fn agents_dir() -> Result<PathBuf, ApiError> {
    let config_home = config_home()?;
    let agents_dir = config_home.join("meld").join("agents");

    if !agents_dir.exists() {
        std::fs::create_dir_all(&agents_dir).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to create agents directory {}: {}",
                agents_dir.display(),
                e
            ))
        })?;
    }

    Ok(agents_dir)
}

/// Get providers directory path
///
/// Returns `$XDG_CONFIG_HOME/meld/providers/`
/// Creates the directory if it doesn't exist
pub fn providers_dir() -> Result<PathBuf, ApiError> {
    let config_home = config_home()?;
    let providers_dir = config_home.join("meld").join("providers");

    if !providers_dir.exists() {
        std::fs::create_dir_all(&providers_dir).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to create providers directory {}: {}",
                providers_dir.display(),
                e
            ))
        })?;
    }

    Ok(providers_dir)
}

/// Get prompts directory path
///
/// Returns `$XDG_CONFIG_HOME/meld/prompts/`
/// Creates the directory if it doesn't exist
pub fn prompts_dir() -> Result<PathBuf, ApiError> {
    let config_home = config_home()?;
    let prompts_dir = config_home.join("meld").join("prompts");

    if !prompts_dir.exists() {
        std::fs::create_dir_all(&prompts_dir).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to create prompts directory {}: {}",
                prompts_dir.display(),
                e
            ))
        })?;
    }

    Ok(prompts_dir)
}
