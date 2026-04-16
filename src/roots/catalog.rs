use std::fs;
use std::path::Path;

use crate::error::{ApiError, StorageError};
use crate::roots::contracts::{BranchCatalog, BranchCatalogEntry, RootCatalog, RootCatalogEntry};

pub fn load(path: &Path) -> Result<RootCatalog, ApiError> {
    if !path.exists() {
        return Ok(RootCatalog::default());
    }
    let content = fs::read_to_string(path).map_err(StorageError::IoError)?;
    let catalog = serde_json::from_str(&content)
        .map_err(|err| ApiError::ConfigError(format!("Failed to parse root catalog: {}", err)))?;
    Ok(catalog)
}

pub fn load_branch_catalog(path: &Path) -> Result<BranchCatalog, ApiError> {
    load(path).map(Into::into)
}

pub fn save(path: &Path, catalog: &RootCatalog) -> Result<(), ApiError> {
    let parent = path.parent().ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Root catalog path missing parent: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(StorageError::IoError)?;
    let serialized = serde_json::to_string_pretty(catalog).map_err(|err| {
        ApiError::ConfigError(format!("Failed to serialize root catalog: {}", err))
    })?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serialized).map_err(StorageError::IoError)?;
    fs::rename(&temp_path, path).map_err(StorageError::IoError)?;
    Ok(())
}

pub fn save_branch_catalog(path: &Path, catalog: &BranchCatalog) -> Result<(), ApiError> {
    save(path, &catalog.clone().into())
}

pub fn upsert(catalog: &mut RootCatalog, entry: RootCatalogEntry) {
    if let Some(existing) = catalog
        .roots
        .iter_mut()
        .find(|root| root.root_id == entry.root_id)
    {
        *existing = entry;
    } else {
        catalog.roots.push(entry);
    }
    catalog
        .roots
        .sort_by(|left, right| left.workspace_path.cmp(&right.workspace_path));
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
    use super::{load, load_branch_catalog, save_branch_catalog};
    use crate::roots::contracts::{
        BranchAttachmentStatus, BranchCatalog, BranchCatalogEntry, BranchInspectionStatus,
        BranchKind, BranchMigrationStatus, RootCatalog,
    };

    #[test]
    fn branch_catalog_saves_as_root_compatible_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("root_catalog.json");
        let branch_catalog = BranchCatalog {
            branches: vec![BranchCatalogEntry {
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
            }],
        };

        save_branch_catalog(&path, &branch_catalog).unwrap();
        let root_catalog = load(&path).unwrap();
        assert_eq!(root_catalog.roots.len(), 1);
        assert_eq!(root_catalog.roots[0].root_id, "branch-1");
        assert_eq!(root_catalog.roots[0].workspace_path, "/tmp/workspace");
    }

    #[test]
    fn root_catalog_loads_into_branch_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("root_catalog.json");
        super::save(
            &path,
            &RootCatalog {
                roots: vec![crate::roots::contracts::RootCatalogEntry {
                    root_id: "root-1".to_string(),
                    workspace_path: "/tmp/workspace".to_string(),
                    data_home_path: "/tmp/data".to_string(),
                    store_path: Some("/tmp/data/store".to_string()),
                    attachment_status: crate::roots::contracts::RootAttachmentStatus::Active,
                    inspection_status: crate::roots::contracts::RootInspectionStatus::Registered,
                    migration_status: crate::roots::contracts::RootMigrationStatus::Succeeded,
                    last_seen_at: Some("2026-04-15T00:00:00Z".to_string()),
                    last_inspected_at: Some("2026-04-15T00:00:00Z".to_string()),
                    last_migration_at: Some("2026-04-15T00:00:00Z".to_string()),
                }],
            },
        )
        .unwrap();

        let branch_catalog = load_branch_catalog(&path).unwrap();
        assert_eq!(branch_catalog.branches.len(), 1);
        assert_eq!(
            branch_catalog.branches[0].branch_kind,
            BranchKind::WorkspaceFs
        );
        assert_eq!(branch_catalog.branches[0].branch_id, "root-1");
        assert_eq!(
            branch_catalog.branches[0].canonical_locator,
            "/tmp/workspace"
        );
    }
}
