use std::fs;
use std::path::Path;

use crate::error::{ApiError, StorageError};
use crate::roots::contracts::{ResolvedRoot, RootManifest};

pub const WORKSPACE_LOCATOR_VERSION: u32 = 1;
pub const AUTHORITATIVE_STATE_VERSION: u32 = 1;
pub const DERIVED_STATE_VERSION: u32 = 1;
pub const MIGRATION_RUNTIME_VERSION: u32 = 1;

pub fn load(path: &Path) -> Result<Option<RootManifest>, ApiError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(StorageError::IoError)?;
    let manifest = serde_json::from_str(&content)
        .map_err(|err| ApiError::ConfigError(format!("Failed to parse root manifest: {}", err)))?;
    Ok(Some(manifest))
}

pub fn save(path: &Path, manifest: &RootManifest) -> Result<(), ApiError> {
    let parent = path.parent().ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Root manifest path missing parent: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(StorageError::IoError)?;
    let serialized = serde_json::to_string_pretty(manifest).map_err(|err| {
        ApiError::ConfigError(format!("Failed to serialize root manifest: {}", err))
    })?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serialized).map_err(StorageError::IoError)?;
    fs::rename(&temp_path, path).map_err(StorageError::IoError)?;
    Ok(())
}

pub fn new_manifest(resolved: &ResolvedRoot, now: &str) -> RootManifest {
    RootManifest {
        root_id: resolved.root_id.clone(),
        workspace_path: resolved.workspace_path.to_string_lossy().to_string(),
        workspace_locator_version: WORKSPACE_LOCATOR_VERSION,
        authoritative_state_version: AUTHORITATIVE_STATE_VERSION,
        derived_state_version: DERIVED_STATE_VERSION,
        migration_runtime_version: MIGRATION_RUNTIME_VERSION,
        last_seen_at: now.to_string(),
        last_successful_plan_id: None,
        last_successful_step_id: None,
        last_reduced_seq: 0,
    }
}
