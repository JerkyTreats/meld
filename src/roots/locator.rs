use std::path::Path;

use crate::config::xdg;
use crate::error::ApiError;
use crate::roots::contracts::{BranchKind, ResolvedBranch, ResolvedRoot};
use crate::tree::path::{canonicalize_path, normalize_path_string};

const ROOT_MANIFEST_FILE: &str = "root_manifest.json";
const MIGRATION_LEDGER_FILE: &str = "migration_ledger.jsonl";

#[derive(Debug, Clone, Default)]
pub struct WorkspaceBranchAdapter;

impl WorkspaceBranchAdapter {
    pub fn resolve_active_branch(&self, workspace_root: &Path) -> Result<ResolvedBranch, ApiError> {
        let canonical_locator =
            canonicalize_path(workspace_root).map_err(ApiError::StorageError)?;
        let data_home_path = xdg::workspace_data_dir(&canonical_locator)?;
        let normalized_path = normalize_path_string(&canonical_locator.to_string_lossy());
        let branch_id = blake3::hash(normalized_path.as_bytes())
            .to_hex()
            .to_string();

        Ok(ResolvedBranch {
            branch_id,
            branch_kind: BranchKind::WorkspaceFs,
            manifest_path: data_home_path.join(ROOT_MANIFEST_FILE),
            ledger_path: data_home_path.join(MIGRATION_LEDGER_FILE),
            canonical_locator,
            data_home_path,
        })
    }
}

pub fn resolve_active_branch(workspace_root: &Path) -> Result<ResolvedBranch, ApiError> {
    WorkspaceBranchAdapter.resolve_active_branch(workspace_root)
}

pub fn resolve_active_root(workspace_root: &Path) -> Result<ResolvedRoot, ApiError> {
    resolve_active_branch(workspace_root).map(Into::into)
}

pub fn global_catalog_path() -> Result<std::path::PathBuf, ApiError> {
    let Some(data_home) = xdg::data_home() else {
        return Err(ApiError::ConfigError(
            "Could not determine XDG data home directory".to_string(),
        ));
    };
    Ok(data_home.join("meld").join("root_catalog.json"))
}

#[cfg(test)]
mod tests {
    use super::{resolve_active_branch, resolve_active_root};
    use crate::roots::BranchKind;

    #[test]
    fn active_root_resolution_matches_active_branch_resolution() {
        let temp = tempfile::tempdir().unwrap();
        let resolved_branch = resolve_active_branch(temp.path()).unwrap();
        let resolved_root = resolve_active_root(temp.path()).unwrap();

        assert_eq!(resolved_branch.branch_id, resolved_root.root_id);
        assert_eq!(resolved_branch.branch_kind, BranchKind::WorkspaceFs);
        assert_eq!(
            resolved_branch.canonical_locator,
            resolved_root.workspace_path
        );
        assert_eq!(resolved_branch.data_home_path, resolved_root.data_home_path);
    }
}
