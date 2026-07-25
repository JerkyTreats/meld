use meld::branches::{BranchCatalog, BranchManifest, BranchesStatusOutput};
use meld::cli::{BranchesCommands, Commands, RunContext};
use meld::config::xdg;
use meld::events::binding::ProductEventBinding;
use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_events::{EventEnvelope, LegacyEventMigrationSource};
use serde_json::json;
use std::process::Command;
use tempfile::TempDir;

use crate::integration::with_xdg_data_home;

#[test]
fn startup_registers_active_branch_and_writes_ledger() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let context = RunContext::new(workspace.path().to_path_buf(), None).unwrap();
        // Startup no longer performs hidden graph catch-up; the derived
        // version step is recorded by the explicit catch-up path.
        context.catch_up_graph_projection().unwrap();

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
        assert_eq!(
            catalog.branches[0].canonical_locator,
            manifest.canonical_locator
        );

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
        assert!(parsed
            .branches
            .iter()
            .all(|branch| !branch.migration_status.is_empty()));
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
                branch.canonical_locator
                    == workspace.path().canonicalize().unwrap().to_string_lossy()
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

        assert!(parsed
            .branches
            .iter()
            .any(|branch| branch.canonical_locator == "/home/user/ws_dormant"));
        assert!(parsed
            .branches
            .iter()
            .all(|branch| !branch.canonical_locator.contains("/tmp/")));
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
                branch.canonical_locator
                    == workspace.path().canonicalize().unwrap().to_string_lossy()
            })
            .unwrap();

        assert_eq!(migrated.migration_status, "not_needed");
        assert!(migrated.last_migration_at.is_some());
    });
}

#[test]
fn dormant_branch_migrations_keep_separate_product_authorities() {
    let test_dir = TempDir::new().unwrap();
    let workspace_a = TempDir::new().unwrap();
    let workspace_b = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        for (workspace, record_id) in [(&workspace_a, "branch-a"), (&workspace_b, "branch-b")] {
            let data_home = xdg::workspace_data_dir(workspace.path()).unwrap();
            let store_path = data_home.join("store");
            std::fs::create_dir_all(&store_path).unwrap();
            let store = EventStore::new(sled::open(&store_path).unwrap()).unwrap();
            store
                .append_envelope(
                    EventEnvelope::new_domain(
                        "2026-07-12T00:00:00Z".to_string(),
                        record_id,
                        "workspace_fs",
                        record_id,
                        "workspace.branch.observed",
                        None,
                        json!({ "branch": record_id }),
                    )
                    .with_record_id(record_id),
                )
                .unwrap();
            store.flush().unwrap();
            drop(store);
            meld::branches::tooling::handle_cli_command(&BranchesCommands::Attach {
                path: workspace.path().to_path_buf(),
                format: "json".to_string(),
            })
            .unwrap();
        }

        meld::branches::tooling::handle_cli_command(&BranchesCommands::Migrate {
            format: "json".to_string(),
        })
        .unwrap();

        let binding = |workspace: &TempDir| {
            let path = xdg::workspace_data_dir(workspace.path())
                .unwrap()
                .join("event_authority.json");
            serde_json::from_slice::<ProductEventBinding>(&std::fs::read(path).unwrap()).unwrap()
        };
        let binding_a = binding(&workspace_a);
        let binding_b = binding(&workspace_b);
        assert_ne!(binding_a.branch_id, binding_b.branch_id);
        assert_ne!(binding_a.ledger_identity, binding_b.ledger_identity);
        assert_ne!(binding_a.ledger_path, binding_b.ledger_path);
    });
}

#[test]
fn dormant_branch_migration_uses_its_configured_legacy_store() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let config_dir = workspace.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.toml"),
            r#"
[system.storage]
store_path = "custom-events"
frames_path = "custom-frames"
artifacts_path = "custom-artifacts"
"#,
        )
        .unwrap();
        let custom_store = workspace.path().join("custom-events");
        let store = EventStore::new(sled::open(&custom_store).unwrap()).unwrap();
        store
            .append_envelope(
                EventEnvelope::new_domain(
                    "2026-07-12T00:00:00Z".to_string(),
                    "custom-branch",
                    "workspace_fs",
                    "custom-branch",
                    "workspace.branch.observed",
                    None,
                    json!({ "branch": "custom-branch" }),
                )
                .with_record_id("custom-branch"),
            )
            .unwrap();
        store.flush().unwrap();
        drop(store);

        meld::branches::tooling::handle_cli_command(&BranchesCommands::Attach {
            path: workspace.path().to_path_buf(),
            format: "json".to_string(),
        })
        .unwrap();
        meld::branches::tooling::handle_cli_command(&BranchesCommands::Migrate {
            format: "json".to_string(),
        })
        .unwrap();

        let binding_path = xdg::workspace_data_dir(workspace.path())
            .unwrap()
            .join("event_authority.json");
        let binding: ProductEventBinding =
            serde_json::from_slice(&std::fs::read(binding_path).unwrap()).unwrap();
        assert_eq!(
            binding.source.as_ref().unwrap().ledger_path,
            custom_store.canonicalize().unwrap()
        );
        let source = LegacyEventMigrationSource::open(&custom_store).unwrap();
        let marker = source.cutover_marker().unwrap().unwrap();
        assert_eq!(marker.target_ledger_id, binding.ledger_identity);
    });
}

#[test]
fn active_branch_graph_status_reuses_the_open_product_projection() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let context = RunContext::new(workspace.path().to_path_buf(), None).unwrap();
        context.execute(&Commands::Scan { force: true }).unwrap();
        // Command routing no longer catches the projection up implicitly;
        // advance it through the explicit path before reading status.
        context.catch_up_graph_projection().unwrap();

        let output = context
            .execute(&Commands::Branches {
                command: BranchesCommands::GraphStatus {
                    scope: "active".to_string(),
                    branch_ids: Vec::new(),
                    format: "json".to_string(),
                },
            })
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["branches"][0]["read_status"], "ready");
        assert!(parsed["branches"][0]["last_reduced_seq"].is_u64());
        assert!(parsed["branches"][0]["store_path"]
            .as_str()
            .unwrap()
            .ends_with("world_model.sled"));
    });
}

#[test]
fn binary_active_graph_query_routes_through_run_context() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let bin = env!("CARGO_BIN_EXE_meld");
        let scan = Command::new(bin)
            .args([
                "--workspace",
                workspace.path().to_str().unwrap(),
                "scan",
                "--force",
            ])
            .output()
            .unwrap();
        assert!(
            scan.status.success(),
            "{}",
            String::from_utf8_lossy(&scan.stderr)
        );

        let output = Command::new(bin)
            .args([
                "--workspace",
                workspace.path().to_str().unwrap(),
                "branches",
                "graph-status",
                "--scope",
                "active",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(parsed["branches"][0]["read_status"], "ready");
    });
}
