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
    if let Some(stripped) = path.strip_prefix("~/") {
        let home =
            std::env::var("HOME").map_err(|_| ApiError::ConfigError("HOME not set".to_string()))?;
        return Ok(PathBuf::from(home).join(stripped));
    }
    if let Some(stripped) = path.strip_prefix("./") {
        let current_dir = std::env::current_dir().map_err(|e| {
            ApiError::ConfigError(format!("Failed to get current directory: {}", e))
        })?;
        return Ok(current_dir.join(stripped));
    }
    Ok(base_dir.join(path))
}

/// Prompt file cache with modification time tracking
pub struct PromptCache {
    cache: HashMap<PathBuf, CachedPrompt>,
}

struct CachedPrompt {
    content: String,
    modified: SystemTime,
    len: u64,
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
        let len = metadata.len();
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
        if let Some(cached) = self.cache.get(path) {
            if cached.modified == mtime && cached.len == len && cached.content == content {
                return Ok(cached.content.clone());
            }
        }
        self.cache.insert(
            path.to_path_buf(),
            CachedPrompt {
                content: content.clone(),
                modified: mtime,
                len,
            },
        );
        Ok(content)
    }
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CachedPrompt, PromptCache};
    use tempfile::TempDir;

    #[test]
    fn load_prompt_reloads_when_metadata_matches_stale_cache_entry() {
        let temp_dir = TempDir::new().unwrap();
        let prompt_file = temp_dir.path().join("prompt.md");
        std::fs::write(&prompt_file, "prompt v2").unwrap();
        let metadata = std::fs::metadata(&prompt_file).unwrap();
        let mut cache = PromptCache::new();
        cache.cache.insert(
            prompt_file.clone(),
            CachedPrompt {
                content: "prompt v1".to_string(),
                modified: metadata.modified().unwrap(),
                len: metadata.len(),
            },
        );

        let content = cache.load_prompt(&prompt_file).unwrap();

        assert_eq!(content, "prompt v2");
    }
}
