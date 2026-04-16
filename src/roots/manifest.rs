use std::fs;
use std::path::Path;

use crate::error::{ApiError, StorageError};
use crate::roots::contracts::{BranchKind, BranchManifest, ResolvedRoot, RootManifest};

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

pub fn load_branch(path: &Path) -> Result<Option<BranchManifest>, ApiError> {
    load(path).map(|manifest| manifest.map(Into::into))
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

pub fn save_branch(path: &Path, manifest: &BranchManifest) -> Result<(), ApiError> {
    save(path, &manifest.clone().into())
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

pub fn new_branch_manifest(resolved: &ResolvedRoot, now: &str) -> BranchManifest {
    BranchManifest {
        branch_id: resolved.root_id.clone(),
        branch_kind: BranchKind::WorkspaceFs,
        canonical_locator: resolved.workspace_path.to_string_lossy().to_string(),
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
    use super::{load, load_branch, save_branch};
    use crate::roots::contracts::{BranchKind, BranchManifest, RootManifest};

    #[test]
    fn branch_manifest_saves_as_root_compatible_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("root_manifest.json");
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

        save_branch(&path, &branch_manifest).unwrap();
        let root_manifest = load(&path).unwrap().unwrap();
        assert_eq!(
            root_manifest,
            RootManifest {
                root_id: "branch-1".to_string(),
                workspace_path: "/tmp/workspace".to_string(),
                workspace_locator_version: 1,
                authoritative_state_version: 1,
                derived_state_version: 1,
                migration_runtime_version: 1,
                last_seen_at: "2026-04-15T00:00:00Z".to_string(),
                last_successful_plan_id: Some("plan-1".to_string()),
                last_successful_step_id: Some("step-1".to_string()),
                last_reduced_seq: 42,
            }
        );
    }

    #[test]
    fn root_manifest_loads_into_branch_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("root_manifest.json");
        super::save(
            &path,
            &RootManifest {
                root_id: "root-1".to_string(),
                workspace_path: "/tmp/workspace".to_string(),
                workspace_locator_version: 2,
                authoritative_state_version: 3,
                derived_state_version: 4,
                migration_runtime_version: 5,
                last_seen_at: "2026-04-15T00:00:00Z".to_string(),
                last_successful_plan_id: None,
                last_successful_step_id: None,
                last_reduced_seq: 7,
            },
        )
        .unwrap();

        let branch_manifest = load_branch(&path).unwrap().unwrap();
        assert_eq!(branch_manifest.branch_kind, BranchKind::WorkspaceFs);
        assert_eq!(branch_manifest.branch_id, "root-1");
        assert_eq!(branch_manifest.canonical_locator, "/tmp/workspace");
        assert_eq!(branch_manifest.locator_version, 2);
        assert_eq!(branch_manifest.last_reduced_seq, 7);
    }
}
