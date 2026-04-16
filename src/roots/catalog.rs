use std::fs;
use std::path::Path;

use crate::error::{ApiError, StorageError};
use crate::roots::contracts::{RootCatalog, RootCatalogEntry};

pub fn load(path: &Path) -> Result<RootCatalog, ApiError> {
    if !path.exists() {
        return Ok(RootCatalog::default());
    }
    let content = fs::read_to_string(path).map_err(StorageError::IoError)?;
    let catalog = serde_json::from_str(&content)
        .map_err(|err| ApiError::ConfigError(format!("Failed to parse root catalog: {}", err)))?;
    Ok(catalog)
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
