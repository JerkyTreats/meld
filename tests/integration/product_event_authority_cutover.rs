use meld::branches::locator::resolve_active_branch;
use meld::cli::{Commands, EventCommands, RunContext, RuntimeCommands};
use meld::config::ConfigLoader;
use meld::events::binding::{ProductEventBinding, ProductEventBindingState};
use meld::session::SessionStore;
use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_events::{EventEnvelope, LedgerCursor, LegacyEventMigrationSource, ReplayRequest};
use serde_json::{json, Value};
use std::process::Command;

use super::with_xdg_env;

#[test]
fn real_cli_migrates_and_reuses_one_authority_for_event_and_runtime_routes() {
    let temp = tempfile::TempDir::new().unwrap();
    with_xdg_env(&temp, || {
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let config = ConfigLoader::load(&workspace).unwrap();
        let (legacy_path, _, _) = config.system.storage.resolve_paths(&workspace).unwrap();
        std::fs::create_dir_all(&legacy_path).unwrap();

        let legacy_store = EventStore::new(sled::open(&legacy_path).unwrap()).unwrap();
        legacy_store
            .append_envelope(
                EventEnvelope::new_domain(
                    "2026-07-12T00:00:00Z".to_string(),
                    "legacy-session",
                    "workspace_fs",
                    "workspace-a",
                    "workspace.node.observed",
                    None,
                    json!({ "node": "node-a" }),
                )
                .with_record_id("legacy-node-a"),
            )
            .unwrap();
        legacy_store.flush().unwrap();
        drop(legacy_store);
        let legacy_rows_before = legacy_event_rows(&legacy_path);

        let context = RunContext::new(workspace.clone(), None).unwrap();
        context.execute(&Commands::Scan { force: true }).unwrap();
        let status: Value = serde_json::from_str(
            &context
                .execute(&event_status_json())
                .expect("direct event status must use the product authority"),
        )
        .unwrap();
        let ledger_id = status["ledger_id"].as_str().unwrap().to_string();

        let tail: Value = serde_json::from_str(
            &context
                .execute(&event_tail_json())
                .expect("direct event tail must read migrated product history"),
        )
        .unwrap();
        assert_eq!(tail["ledger_id"], ledger_id);
        assert!(tail["records"].as_array().unwrap().iter().any(|record| {
            record["envelope"]["record_id"] == "legacy-node-a"
                && record["envelope"]["type"] == "workspace.node.observed"
        }));
        use meld_world_model::graph::contracts::*;
        let owner_record = tail["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| {
                record["envelope"]["type"] == OWNER_PUBLICATION_EVENT_TYPE
                    && record["envelope"]["domain_id"] == "workspace_fs"
            })
            .unwrap();
        let operation: OwnerPublicationOperation =
            serde_json::from_value(owner_record["envelope"]["data"].clone()).unwrap();
        let source_seq = owner_record["seq"].as_u64().unwrap();
        let cut = context
            .api()
            .world_model_queries()
            .unwrap()
            .cut(&TraversalCutRequest {
                owners: vec![TraversalOwnerRequirement {
                    owner_id: "workspace_fs".into(),
                    scope: operation.batch.scope.clone(),
                    required: true,
                    event_source: None,
                }],
                scope: operation.batch.scope.clone(),
                currentness: OwnerCurrentnessPolicy::LatestComplete,
                event_position: LedgerCursor {
                    ledger_id: ledger_id.parse().unwrap(),
                    after_seq: source_seq,
                },
            })
            .unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Complete);
        assert_eq!(cut.receipts[0].source_event.unwrap().seq, source_seq);
        assert_eq!(cut.receipts[0].revision_id, operation.batch.revision_id);

        context
            .execute(&runtime_run_json())
            .expect("runtime run must reuse the RunContext authority");
        let after_runtime: Value = serde_json::from_str(
            &context
                .execute(&event_status_json())
                .expect("event status must remain on the same authority"),
        )
        .unwrap();
        assert_eq!(after_runtime["ledger_id"], ledger_id);
        let watermark_before = context.event_watermark_capability().snapshot().unwrap();
        let graph_cursor_before = context.graph_event_cursor().unwrap();
        drop(context);

        let branch = resolve_active_branch(&workspace).unwrap();
        let binding: ProductEventBinding = serde_json::from_slice(
            &std::fs::read(branch.data_home_path.join("event_authority.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(binding.state, ProductEventBindingState::Active);
        assert_eq!(binding.branch_id, branch.branch_id);
        assert_eq!(binding.ledger_identity.to_string(), ledger_id);
        let source = LegacyEventMigrationSource::open(&legacy_path).unwrap();
        let marker = source.cutover_marker().unwrap().unwrap();
        assert_eq!(marker.target_ledger_id, binding.ledger_identity);
        drop(source);

        let compatibility_sessions = SessionStore::new(sled::open(&legacy_path).unwrap()).unwrap();
        assert!(!compatibility_sessions.list_sessions().unwrap().is_empty());
        drop(compatibility_sessions);

        let legacy_store = EventStore::new(sled::open(&legacy_path).unwrap()).unwrap();
        let error = legacy_store
            .append_envelope(EventEnvelope::with_now(
                "late-legacy",
                "legacy.write",
                json!({}),
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            meld_events::error::StorageError::MigrationConflict(_)
        ));
        drop(legacy_store);
        assert_eq!(legacy_event_rows(&legacy_path), legacy_rows_before);

        let reopened = RunContext::new(workspace, None).unwrap();
        let watermark_reopened = reopened.event_watermark_capability().snapshot().unwrap();
        assert_eq!(watermark_reopened, watermark_before);
        let graph_cursor_reopened = reopened.graph_event_cursor().unwrap();
        assert_eq!(
            graph_cursor_reopened.ledger_id,
            graph_cursor_before.ledger_id
        );
        assert!(graph_cursor_reopened.after_seq >= graph_cursor_before.after_seq);
        reopened
            .progress_runtime()
            .emit_event("reopen-proof", "reopen.proof", json!({}))
            .unwrap();
        let next = reopened
            .event_replay_capability()
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: watermark_before.ledger_id,
                    after_seq: watermark_before.tip_seq,
                },
                limit: 1,
            })
            .unwrap();
        assert_eq!(next.records[0].seq, watermark_before.tip_seq + 1);
        assert_eq!(next.ledger_id.to_string(), ledger_id);
    });
}

fn legacy_event_rows(path: &std::path::Path) -> Vec<(Vec<u8>, Vec<u8>)> {
    let db = sled::open(path).unwrap();
    let tree = db.open_tree("obs_spine_events").unwrap();
    let rows = tree
        .iter()
        .map(|row| {
            let (key, value) = row.unwrap();
            (key.to_vec(), value.to_vec())
        })
        .collect();
    drop(tree);
    drop(db);
    rows
}

#[test]
fn binary_direct_commands_preserve_one_identity_across_processes() {
    let temp = tempfile::TempDir::new().unwrap();
    with_xdg_env(&temp, || {
        let workspace = temp.path().join("workspace-binary");
        std::fs::create_dir_all(&workspace).unwrap();

        let first = run_binary_event_status(&workspace);
        let second = run_binary_event_status(&workspace);
        assert_eq!(first["ledger_id"], second["ledger_id"]);

        let runtime = Command::new(env!("CARGO_BIN_EXE_meld"))
            .args([
                "--workspace",
                workspace.to_str().unwrap(),
                "runtime",
                "run",
                "--instance-id",
                "binary-authority-cutover",
                "--tick-ms",
                "1",
                "--duration-ms",
                "1",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert!(
            runtime.status.success(),
            "{}",
            String::from_utf8_lossy(&runtime.stderr)
        );

        let after_runtime = run_binary_event_status(&workspace);
        assert_eq!(first["ledger_id"], after_runtime["ledger_id"]);

        let config = ConfigLoader::load(&workspace).unwrap();
        let (legacy_path, _, _) = config.system.storage.resolve_paths(&workspace).unwrap();
        let legacy_store = EventStore::new(sled::open(legacy_path).unwrap()).unwrap();
        let error = legacy_store
            .append_envelope(EventEnvelope::with_now(
                "old-binary",
                "old_binary.semantic_write",
                json!({}),
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            meld_events::error::StorageError::MigrationConflict(_)
        ));
    });
}

#[test]
fn real_route_rejects_a_mismatched_active_binding_without_fallback() {
    let temp = tempfile::TempDir::new().unwrap();
    with_xdg_env(&temp, || {
        let workspace = temp.path().join("workspace-mismatch");
        std::fs::create_dir_all(&workspace).unwrap();
        let context = RunContext::new(workspace.clone(), None).unwrap();
        drop(context);

        let branch = resolve_active_branch(&workspace).unwrap();
        let binding_path = branch.data_home_path.join("event_authority.json");
        let mut binding: ProductEventBinding =
            serde_json::from_slice(&std::fs::read(&binding_path).unwrap()).unwrap();
        binding.ledger_identity = meld_events::LedgerIdentity::new();
        std::fs::write(&binding_path, serde_json::to_vec_pretty(&binding).unwrap()).unwrap();

        let error = RunContext::new(workspace, None)
            .err()
            .expect("mismatched active binding must fail closed");
        assert!(error.to_string().contains("cutover marker targets ledger"));
    });
}

fn run_binary_event_status(workspace: &std::path::Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_meld"))
        .args([
            "--workspace",
            workspace.to_str().unwrap(),
            "event",
            "status",
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
    serde_json::from_slice(&output.stdout).unwrap()
}

fn event_status_json() -> Commands {
    Commands::Event {
        command: EventCommands::Status {
            format: "json".to_string(),
        },
    }
}

fn event_tail_json() -> Commands {
    Commands::Event {
        command: EventCommands::Tail {
            format: "json".to_string(),
            after: Some(0),
            limit: 128,
            follow: false,
        },
    }
}

fn runtime_run_json() -> Commands {
    Commands::Runtime {
        command: RuntimeCommands::Run {
            instance_id: Some("event-authority-cutover-test".to_string()),
            tick_ms: 1,
            duration_ms: Some(1),
            format: "json".to_string(),
            restart_policy: "on-heartbeat-expiry".to_string(),
            restart_attempt_limit: 1,
            restart_backoff_ms: 1,
        },
    }
}
