use std::fs;
use std::path::{Path, PathBuf};

use crate::branches::contracts::{BranchKind, ResolvedBranch};
use crate::config::xdg;
use crate::error::ApiError;
use crate::tree::path::{canonicalize_path, normalize_path_string};

const BRANCH_MANIFEST_FILE: &str = "branch_manifest.json";
const BRANCH_MIGRATION_LEDGER_FILE: &str = "branch_migration_ledger.jsonl";

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
            manifest_path: data_home_path.join(BRANCH_MANIFEST_FILE),
            ledger_path: data_home_path.join(BRANCH_MIGRATION_LEDGER_FILE),
            canonical_locator,
            data_home_path,
        })
    }
}

pub fn resolve_active_branch(workspace_root: &Path) -> Result<ResolvedBranch, ApiError> {
    WorkspaceBranchAdapter.resolve_active_branch(workspace_root)
}

pub fn global_catalog_path() -> Result<std::path::PathBuf, ApiError> {
    let Some(data_home) = xdg::data_home() else {
        return Err(ApiError::ConfigError(
            "Could not determine XDG data home directory".to_string(),
        ));
    };
    Ok(data_home.join("meld").join("branch_catalog.json"))
}

pub fn branch_store_path(data_home_path: &Path) -> PathBuf {
    data_home_path.join("store")
}

pub fn discover_branch_data_homes() -> Result<Vec<PathBuf>, ApiError> {
    let Some(data_home) = xdg::data_home() else {
        return Err(ApiError::ConfigError(
            "Could not determine XDG data home directory".to_string(),
        ));
    };
    discover_branch_data_homes_under(&data_home.join("meld"))
}

pub fn recover_workspace_path(data_home_path: &Path) -> Result<PathBuf, ApiError> {
    let Some(data_home) = xdg::data_home() else {
        return Err(ApiError::ConfigError(
            "Could not determine XDG data home directory".to_string(),
        ));
    };
    recover_workspace_path_from_meld_home(&data_home.join("meld"), data_home_path)
}

fn discover_branch_data_homes_under(meld_home: &Path) -> Result<Vec<PathBuf>, ApiError> {
    if !meld_home.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    let mut stack = vec![meld_home.to_path_buf()];
    while let Some(current) = stack.pop() {
        if is_temp_candidate(meld_home, &current) {
            continue;
        }
        if is_branch_candidate(&current) {
            out.push(current);
            continue;
        }
        for entry in fs::read_dir(&current).map_err(crate::error::StorageError::IoError)? {
            let entry = entry.map_err(crate::error::StorageError::IoError)?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn recover_workspace_path_from_meld_home(
    meld_home: &Path,
    data_home_path: &Path,
) -> Result<PathBuf, ApiError> {
    let relative = data_home_path.strip_prefix(meld_home).map_err(|_| {
        ApiError::ConfigError(format!(
            "Branch data home is not under meld data home: {}",
            data_home_path.display()
        ))
    })?;
    let mut workspace_path = PathBuf::from("/");
    for component in relative.components() {
        workspace_path.push(component.as_os_str());
    }
    Ok(workspace_path)
}

fn is_branch_candidate(path: &Path) -> bool {
    path.join(BRANCH_MANIFEST_FILE).exists()
        || (path.join("store").is_dir()
            && (path.join("frames").is_dir()
                || path.join("workflow").is_dir()
                || path.join("head_index.bin").exists()))
}

fn is_temp_candidate(meld_home: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(meld_home) else {
        return false;
    };
    relative
        .components()
        .any(|component| component.as_os_str() == "tmp")
}

#[cfg(test)]
mod tests {
    use super::{
        branch_store_path, discover_branch_data_homes_under, recover_workspace_path_from_meld_home,
        resolve_active_branch,
    };
    use crate::branches::BranchKind;
    use std::path::{Path, PathBuf};

    #[test]
    fn active_branch_resolution_produces_workspace_fs_kind() {
        let temp = tempfile::tempdir().unwrap();
        let resolved_branch = resolve_active_branch(temp.path()).unwrap();
        assert_eq!(resolved_branch.branch_kind, BranchKind::WorkspaceFs);
        assert_eq!(
            resolved_branch.canonical_locator,
            temp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn branch_store_path_defaults_under_data_home() {
        assert_eq!(
            branch_store_path(Path::new("/tmp/meld/home/user/ws")),
            PathBuf::from("/tmp/meld/home/user/ws/store")
        );
    }

    #[test]
    fn discover_branch_data_homes_excludes_tmp_candidates() {
        let temp = tempfile::tempdir().unwrap();
        let meld_home = temp.path().join("meld");
        let real = meld_home.join("home").join("user").join("ws_a");
        let tmp = meld_home.join("tmp").join("scratch");
        std::fs::create_dir_all(real.join("store")).unwrap();
        std::fs::create_dir_all(real.join("frames")).unwrap();
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        std::fs::create_dir_all(tmp.join("frames")).unwrap();

        let found = discover_branch_data_homes_under(&meld_home).unwrap();
        assert_eq!(found, vec![real]);
    }

    #[test]
    fn recover_workspace_path_from_meld_home_rebuilds_locator() {
        let meld_home = PathBuf::from("/tmp/xdg/meld");
        let candidate = meld_home.join("home").join("user").join("ws_a");
        assert_eq!(
            recover_workspace_path_from_meld_home(&meld_home, &candidate).unwrap(),
            PathBuf::from("/home/user/ws_a")
        );
    }
}
