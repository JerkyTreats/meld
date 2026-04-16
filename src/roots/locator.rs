use std::path::Path;

use crate::config::xdg;
use crate::error::ApiError;
use crate::roots::contracts::ResolvedRoot;
use crate::tree::path::{canonicalize_path, normalize_path_string};

const ROOT_MANIFEST_FILE: &str = "root_manifest.json";
const MIGRATION_LEDGER_FILE: &str = "migration_ledger.jsonl";

pub fn resolve_active_root(workspace_root: &Path) -> Result<ResolvedRoot, ApiError> {
    let workspace_path = canonicalize_path(workspace_root).map_err(ApiError::StorageError)?;
    let data_home_path = xdg::workspace_data_dir(&workspace_path)?;
    let normalized_path = normalize_path_string(&workspace_path.to_string_lossy());
    let root_id = blake3::hash(normalized_path.as_bytes())
        .to_hex()
        .to_string();

    Ok(ResolvedRoot {
        root_id,
        manifest_path: data_home_path.join(ROOT_MANIFEST_FILE),
        ledger_path: data_home_path.join(MIGRATION_LEDGER_FILE),
        workspace_path,
        data_home_path,
    })
}

pub fn global_catalog_path() -> Result<std::path::PathBuf, ApiError> {
    let Some(data_home) = xdg::data_home() else {
        return Err(ApiError::ConfigError(
            "Could not determine XDG data home directory".to_string(),
        ));
    };
    Ok(data_home.join("meld").join("root_catalog.json"))
}
