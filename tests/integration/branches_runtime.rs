use meld::cli::{BranchesCommands, RunContext};
use meld::config::xdg;
use meld::branches::{BranchCatalog, BranchManifest, BranchesStatusOutput};
use tempfile::TempDir;

use crate::integration::with_xdg_data_home;

#[test]
fn startup_registers_active_branch_and_writes_ledger() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let _context = RunContext::new(workspace.path().to_path_buf(), None).unwrap();

        let data_home = xdg::workspace_data_dir(workspace.path()).unwrap();
        let manifest_path = data_home.join("branch_manifest.json");
        let ledger_path = data_home.join("branch_migration_ledger.jsonl");
        let catalog_path = xdg::data_home()
            .unwrap()
            .join("meld")
            .join("branch_catalog.json");

        assert!(manifest_path.exists(), "branch manifest should exist");
        assert!(ledger_path.exists(), "branch migration ledger should exist");
        assert!(catalog_path.exists(), "branch catalog should exist");

        let manifest: BranchManifest =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let catalog: BranchCatalog =
            serde_json::from_str(&std::fs::read_to_string(&catalog_path).unwrap()).unwrap();

        assert_eq!(
            manifest.canonical_locator,
            workspace.path().canonicalize().unwrap().to_string_lossy()
        );
        assert!(manifest.last_successful_step_id.is_some());
        assert_eq!(catalog.branches.len(), 1);
        assert_eq!(catalog.branches[0].canonical_locator, manifest.canonical_locator);

        let ledger = std::fs::read_to_string(&ledger_path).unwrap();
        assert!(ledger.contains("\"step_id\":\"write_branch_manifest\""));
        assert!(ledger.contains("\"step_id\":\"refresh_catalog_entry\""));
        assert!(ledger.contains("\"step_id\":\"mark_derived_version\""));
    });
}

#[test]
fn branches_status_lists_registered_branches() {
    let test_dir = TempDir::new().unwrap();
    let workspace_a = TempDir::new().unwrap();
    let workspace_b = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let _context_a = RunContext::new(workspace_a.path().to_path_buf(), None).unwrap();
        let _context_b = RunContext::new(workspace_b.path().to_path_buf(), None).unwrap();

        let output = meld::branches::tooling::handle_cli_command(&BranchesCommands::Status {
            format: "json".to_string(),
        })
        .unwrap();
        let parsed: BranchesStatusOutput = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed.branches.len(), 2);
        assert!(parsed.branches.iter().any(|branch| {
            branch.canonical_locator == workspace_a.path().canonicalize().unwrap().to_string_lossy()
        }));
        assert!(
            parsed
                .branches
                .iter()
                .all(|branch| !branch.migration_status.is_empty())
        );
    });
}

#[test]
fn branches_attach_registers_dormant_workspace() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let output = meld::branches::tooling::handle_cli_command(&BranchesCommands::Attach {
            path: workspace.path().to_path_buf(),
            format: "json".to_string(),
        })
        .unwrap();
        let parsed: BranchesStatusOutput = serde_json::from_str(&output).unwrap();
        let attached = parsed
            .branches
            .iter()
            .find(|branch| {
                branch.canonical_locator == workspace.path().canonicalize().unwrap().to_string_lossy()
            })
            .unwrap();

        assert_eq!(attached.attachment_status, "dormant");
        assert!(attached.store_path.is_some());
    });
}

#[test]
fn branches_discover_registers_candidates_and_skips_tmp() {
    let test_dir = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let meld_home = xdg::data_home().unwrap().join("meld");
        let real = meld_home.join("home").join("user").join("ws_dormant");
        let tmp = meld_home.join("tmp").join("scratch");
        std::fs::create_dir_all(real.join("store")).unwrap();
        std::fs::create_dir_all(real.join("frames")).unwrap();
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        std::fs::create_dir_all(tmp.join("frames")).unwrap();

        let output = meld::branches::tooling::handle_cli_command(&BranchesCommands::Discover {
            format: "json".to_string(),
        })
        .unwrap();
        let parsed: BranchesStatusOutput = serde_json::from_str(&output).unwrap();

        assert!(
            parsed
                .branches
                .iter()
                .any(|branch| branch.canonical_locator == "/home/user/ws_dormant")
        );
        assert!(
            parsed
                .branches
                .iter()
                .all(|branch| !branch.canonical_locator.contains("/tmp/"))
        );
    });
}

#[test]
fn branches_migrate_updates_registered_branch_status() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        meld::branches::tooling::handle_cli_command(&BranchesCommands::Attach {
            path: workspace.path().to_path_buf(),
            format: "json".to_string(),
        })
        .unwrap();

        let output = meld::branches::tooling::handle_cli_command(&BranchesCommands::Migrate {
            format: "json".to_string(),
        })
        .unwrap();
        let parsed: BranchesStatusOutput = serde_json::from_str(&output).unwrap();
        let migrated = parsed
            .branches
            .iter()
            .find(|branch| {
                branch.canonical_locator == workspace.path().canonicalize().unwrap().to_string_lossy()
            })
            .unwrap();

        assert_eq!(migrated.migration_status, "not_needed");
        assert!(migrated.last_migration_at.is_some());
    });
}
