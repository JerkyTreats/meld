use meld::cli::{BranchesCommands, RunContext};
use meld::config::xdg;
use meld::roots::{RootCatalog, RootManifest, RootsStatusOutput};
use tempfile::TempDir;

use crate::integration::with_xdg_data_home;

#[test]
fn startup_registers_active_root_and_writes_ledger() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let _context = RunContext::new(workspace.path().to_path_buf(), None).unwrap();

        let data_home = xdg::workspace_data_dir(workspace.path()).unwrap();
        let manifest_path = data_home.join("root_manifest.json");
        let ledger_path = data_home.join("migration_ledger.jsonl");
        let catalog_path = xdg::data_home()
            .unwrap()
            .join("meld")
            .join("root_catalog.json");

        assert!(manifest_path.exists(), "root manifest should exist");
        assert!(ledger_path.exists(), "migration ledger should exist");
        assert!(catalog_path.exists(), "root catalog should exist");

        let manifest: RootManifest =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let catalog: RootCatalog =
            serde_json::from_str(&std::fs::read_to_string(&catalog_path).unwrap()).unwrap();

        assert_eq!(
            manifest.workspace_path,
            workspace.path().canonicalize().unwrap().to_string_lossy()
        );
        assert!(manifest.last_successful_step_id.is_some());
        assert_eq!(catalog.roots.len(), 1);
        assert_eq!(catalog.roots[0].workspace_path, manifest.workspace_path);

        let ledger = std::fs::read_to_string(&ledger_path).unwrap();
        assert!(ledger.contains("\"step_id\":\"write_root_manifest\""));
        assert!(ledger.contains("\"step_id\":\"refresh_catalog_entry\""));
        assert!(ledger.contains("\"step_id\":\"mark_derived_version\""));
    });
}

#[test]
fn roots_status_lists_registered_roots() {
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
        let parsed: RootsStatusOutput = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed.roots.len(), 2);
        assert!(parsed.roots.iter().any(|root| {
            root.workspace_path == workspace_a.path().canonicalize().unwrap().to_string_lossy()
        }));
        assert!(
            parsed
                .roots
                .iter()
                .all(|root| !root.migration_status.is_empty())
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
        let parsed: RootsStatusOutput = serde_json::from_str(&output).unwrap();
        let attached = parsed
            .roots
            .iter()
            .find(|branch| {
                branch.workspace_path == workspace.path().canonicalize().unwrap().to_string_lossy()
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
        let parsed: RootsStatusOutput = serde_json::from_str(&output).unwrap();

        assert!(
            parsed
                .roots
                .iter()
                .any(|branch| branch.workspace_path == "/home/user/ws_dormant")
        );
        assert!(
            parsed
                .roots
                .iter()
                .all(|branch| !branch.workspace_path.contains("/tmp/"))
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
        let parsed: RootsStatusOutput = serde_json::from_str(&output).unwrap();
        let migrated = parsed
            .roots
            .iter()
            .find(|branch| {
                branch.workspace_path == workspace.path().canonicalize().unwrap().to_string_lossy()
            })
            .unwrap();

        assert_eq!(migrated.migration_status, "not_needed");
        assert!(migrated.last_migration_at.is_some());
    });
}
