use std::fs;
use std::path::Path;

use crate::branches::contracts::{BranchCatalog, BranchCatalogEntry};
use crate::error::{ApiError, StorageError};

pub fn load(path: &Path) -> Result<BranchCatalog, ApiError> {
    if !path.exists() {
        return Ok(BranchCatalog::default());
    }
    let content = fs::read_to_string(path).map_err(StorageError::IoError)?;
    let catalog = serde_json::from_str(&content)
        .map_err(|err| ApiError::ConfigError(format!("Failed to parse branch catalog: {}", err)))?;
    Ok(catalog)
}

pub fn save(path: &Path, catalog: &BranchCatalog) -> Result<(), ApiError> {
    let parent = path.parent().ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Branch catalog path missing parent: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(StorageError::IoError)?;
    let serialized = serde_json::to_string_pretty(catalog).map_err(|err| {
        ApiError::ConfigError(format!("Failed to serialize branch catalog: {}", err))
    })?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serialized).map_err(StorageError::IoError)?;
    fs::rename(&temp_path, path).map_err(StorageError::IoError)?;
    Ok(())
}

pub fn upsert_branch(catalog: &mut BranchCatalog, entry: BranchCatalogEntry) {
    if let Some(existing) = catalog
        .branches
        .iter_mut()
        .find(|branch| branch.branch_id == entry.branch_id)
    {
        *existing = entry;
    } else {
        catalog.branches.push(entry);
    }
    catalog
        .branches
        .sort_by(|left, right| left.canonical_locator.cmp(&right.canonical_locator));
}

#[cfg(test)]
mod tests {
    use super::{load, save, upsert_branch};
    use crate::branches::contracts::{
        BranchAttachmentStatus, BranchCatalog, BranchCatalogEntry, BranchInspectionStatus,
        BranchKind, BranchMigrationStatus,
    };

    #[test]
    fn branch_catalog_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("branch_catalog.json");
        let mut branch_catalog = BranchCatalog::default();
        upsert_branch(
            &mut branch_catalog,
            BranchCatalogEntry {
                branch_id: "branch-1".to_string(),
                branch_kind: BranchKind::WorkspaceFs,
                canonical_locator: "/tmp/workspace".to_string(),
                data_home_path: "/tmp/data".to_string(),
                store_path: Some("/tmp/data/store".to_string()),
                attachment_status: BranchAttachmentStatus::Active,
                inspection_status: BranchInspectionStatus::Registered,
                migration_status: BranchMigrationStatus::Succeeded,
                last_seen_at: Some("2026-04-15T00:00:00Z".to_string()),
                last_inspected_at: Some("2026-04-15T00:00:00Z".to_string()),
                last_migration_at: Some("2026-04-15T00:00:00Z".to_string()),
            },
        );

        save(&path, &branch_catalog).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded, branch_catalog);
    }
}
