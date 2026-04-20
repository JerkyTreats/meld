use std::fs;
use std::path::Path;

use crate::branches::contracts::{BranchManifest, ResolvedBranch};
use crate::error::{ApiError, StorageError};

pub const WORKSPACE_LOCATOR_VERSION: u32 = 1;
pub const AUTHORITATIVE_STATE_VERSION: u32 = 1;
pub const DERIVED_STATE_VERSION: u32 = 1;
pub const MIGRATION_RUNTIME_VERSION: u32 = 1;

pub fn load(path: &Path) -> Result<Option<BranchManifest>, ApiError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(StorageError::IoError)?;
    let manifest = serde_json::from_str(&content).map_err(|err| {
        ApiError::ConfigError(format!("Failed to parse branch manifest: {}", err))
    })?;
    Ok(Some(manifest))
}

pub fn save(path: &Path, manifest: &BranchManifest) -> Result<(), ApiError> {
    let parent = path.parent().ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Branch manifest path missing parent: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(StorageError::IoError)?;
    let serialized = serde_json::to_string_pretty(manifest).map_err(|err| {
        ApiError::ConfigError(format!("Failed to serialize branch manifest: {}", err))
    })?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serialized).map_err(StorageError::IoError)?;
    fs::rename(&temp_path, path).map_err(StorageError::IoError)?;
    Ok(())
}

pub fn new_branch_manifest(resolved: &ResolvedBranch, now: &str) -> BranchManifest {
    BranchManifest {
        branch_id: resolved.branch_id.clone(),
        branch_kind: resolved.branch_kind.clone(),
        canonical_locator: resolved.canonical_locator.to_string_lossy().to_string(),
        locator_version: WORKSPACE_LOCATOR_VERSION,
        authoritative_state_version: AUTHORITATIVE_STATE_VERSION,
        derived_state_version: DERIVED_STATE_VERSION,
        migration_runtime_version: MIGRATION_RUNTIME_VERSION,
        last_seen_at: now.to_string(),
        last_successful_plan_id: None,
        last_successful_step_id: None,
        last_reduced_seq: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{load, save};
    use crate::branches::contracts::{BranchKind, BranchManifest};

    #[test]
    fn branch_manifest_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("branch_manifest.json");
        let branch_manifest = BranchManifest {
            branch_id: "branch-1".to_string(),
            branch_kind: BranchKind::WorkspaceFs,
            canonical_locator: "/tmp/workspace".to_string(),
            locator_version: 1,
            authoritative_state_version: 1,
            derived_state_version: 1,
            migration_runtime_version: 1,
            last_seen_at: "2026-04-15T00:00:00Z".to_string(),
            last_successful_plan_id: Some("plan-1".to_string()),
            last_successful_step_id: Some("step-1".to_string()),
            last_reduced_seq: 42,
        };

        save(&path, &branch_manifest).unwrap();
        let loaded = load(&path).unwrap().unwrap();
        assert_eq!(loaded, branch_manifest);
    }
}
