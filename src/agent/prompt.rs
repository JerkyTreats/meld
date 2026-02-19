//! Prompt path resolution and loading policy owned by the agent domain.

use crate::error::ApiError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Resolve prompt file path with support for absolute, tilde, and relative paths
///
/// Path resolution priority:
/// 1. Absolute path (if starts with `/`)
/// 2. Tilde expansion (if starts with `~/`)
/// 3. Relative to current directory (if starts with `./`)
/// 4. Relative to base_dir (XDG config directory)
pub fn resolve_prompt_path(path: &str, base_dir: &Path) -> Result<PathBuf, ApiError> {
    if path.starts_with('/') {
        return Ok(PathBuf::from(path));
    }
    if path.starts_with("~/") {
        let home =
            std::env::var("HOME").map_err(|_| ApiError::ConfigError("HOME not set".to_string()))?;
        return Ok(PathBuf::from(home).join(&path[2..]));
    }
    if path.starts_with("./") {
        let current_dir = std::env::current_dir().map_err(|e| {
            ApiError::ConfigError(format!("Failed to get current directory: {}", e))
        })?;
        return Ok(current_dir.join(&path[2..]));
    }
    Ok(base_dir.join(path))
}

/// Prompt file cache with modification time tracking
pub struct PromptCache {
    cache: HashMap<PathBuf, (String, SystemTime)>,
}

impl PromptCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Load prompt file content with caching
    pub fn load_prompt(&mut self, path: &Path) -> Result<String, ApiError> {
        let metadata = std::fs::metadata(path).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to read prompt file {}: {}",
                path.display(),
                e
            ))
        })?;
        let mtime = metadata.modified().map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to get modification time for {}: {}",
                path.display(),
                e
            ))
        })?;
        if let Some((cached_content, cached_mtime)) = self.cache.get(path) {
            if *cached_mtime == mtime {
                return Ok(cached_content.clone());
            }
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to read prompt file {}: {}",
                path.display(),
                e
            ))
        })?;
        if content.trim().is_empty() {
            return Err(ApiError::ConfigError(format!(
                "Prompt file {} is empty",
                path.display()
            )));
        }
        self.cache
            .insert(path.to_path_buf(), (content.clone(), mtime));
        Ok(content)
    }
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new()
    }
}
