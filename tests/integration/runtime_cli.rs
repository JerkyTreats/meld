use meld::cli::{Commands, RunContext, RuntimeCommands};
use meld::config::ConfigLoader;
use meld::error::ApiError;
use meld::events::binding::resolve_product_event_authority;
use meld::runtime::assembly::ProductRuntimeAssembly;
use meld::runtime::contracts::{RuntimeLaunchStatus, RuntimeStatusReader};
use meld::runtime::storage::ProductStorageLayout;
use meld::runtime::supervisor::{RuntimeId, SupervisorReportStore};
use meld_events::{AppendMode, DomainObjectRef, EventEnvelope};
use serde_json::json;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

use super::parity_fixture::DeterministicDocsProvider;
use super::{create_test_agent, with_xdg_env};

#[test]
fn runtime_status_reports_desired_runtimes_without_supervisor_store() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let output = run_context
            .execute(&runtime_status_json(Vec::new()))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();

        assert!(parsed["instance"].is_null());
        assert!(parsed["supervisor_store_path"]
            .as_str()
            .unwrap()
            .ends_with("supervisor.sled"));
        let event_append = runtime_row(&parsed, "event.append");
        assert_eq!(event_append["desired_enabled"], true);
        assert_eq!(event_append["factory_available"], true);
        assert_eq!(event_append["handle_started"], false);
        let task_dispatch = runtime_row(&parsed, "execution.task_dispatch");
        assert_eq!(task_dispatch["desired_enabled"], false);
        assert_eq!(task_dispatch["factory_available"], true);
    });
}

#[test]
fn runtime_run_duration_starts_and_stops_cleanly() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();

        let output = run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-test-a"),
                1,
                Some(1),
                "on-heartbeat-expiry",
            ))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let started = parsed["started_runtime_count"].as_u64().unwrap();

        assert!(started > 0);
        assert_eq!(parsed["stopped_runtime_count"].as_u64().unwrap(), started);
        drop(run_context);
        let assembly = open_bound_assembly(&workspace_root);
        let runtime_id = RuntimeId::new("event.append").unwrap();
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
    });
}

#[test]
fn runtime_run_ticks_graph_replay_handle() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let assembly = open_bound_assembly(&workspace_root);
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        let appended = assembly
            .event_authority()
            .append_capability()
            .append_durable(
                EventEnvelope::new_domain(
                    "2026-06-22T00:00:00Z".to_string(),
                    "session-a",
                    "workspace_fs",
                    "workspace-a",
                    "workspace.node.observed",
                    None,
                    json!({ "node": "node-a" }),
                )
                .with_graph(vec![subject], Vec::new())
                .with_record_id("workspace-node-a"),
                AppendMode::Plain,
            )
            .unwrap();
        drop(assembly);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-test-graph"),
                1,
                Some(10),
                "on-heartbeat-expiry",
            ))
            .unwrap();
        drop(run_context);

        let assembly = open_bound_assembly(&workspace_root);
        let cursor = assembly
            .ports()
            .graph_cursor()
            .current()
            .unwrap()
            .expect("runtime graph replay should report its durable cursor");
        assert_eq!(
            cursor.ledger_id,
            assembly.ports().event_replay().ledger_identity()
        );
        assert_eq!(cursor.name, "world_state.graph.reducer");
        assert!(
            cursor.reported_seq >= appended.seq,
            "graph cursor {} did not cover appended event {}",
            cursor.reported_seq,
            appended.seq
        );
    });
}

fn open_bound_assembly(workspace_root: &std::path::Path) -> ProductRuntimeAssembly {
    let config = ConfigLoader::load(workspace_root).unwrap();
    let branch = meld::branches::locator::resolve_active_branch(workspace_root).unwrap();
    let product_root = config
        .system
        .storage
        .resolve_product_root(workspace_root)
        .unwrap();
    let layout = ProductStorageLayout::from_root(product_root);
    let (legacy_store, _, _) = config.system.storage.resolve_paths(workspace_root).unwrap();
    let resolved =
        resolve_product_event_authority(&branch, &layout.ledger_db, &legacy_store).unwrap();
    ProductRuntimeAssembly::load_for_workspace_with_authority(
        workspace_root,
        &config,
        resolved.authority,
    )
    .unwrap()
}

#[test]
fn runtime_status_after_run_reports_stopped_instance() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();
        run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-test-b"),
                1,
                Some(1),
                "on-heartbeat-expiry",
            ))
            .unwrap();

        let output = run_context
            .execute(&runtime_status_json(Vec::new()))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["instance"]["status"], "stopped");
        assert!(parsed["runtimes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|runtime| {
                runtime["health_status"] == "stopped" || runtime["active_lease_id"].is_null()
            }));
    });
}

#[test]
fn runtime_status_filters_runtime_ids() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let output = run_context
            .execute(&runtime_status_json(vec!["event.append".to_string()]))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let runtimes = parsed["runtimes"].as_array().unwrap();

        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0]["runtime_id"], "event.append");
    });
}

#[test]
fn runtime_status_rejects_unknown_runtime_filter() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let result = run_context.execute(&runtime_status_json(vec!["future.missing".to_string()]));

        assert_config_error_contains(result, "unknown runtime id filter");
    });
}

#[test]
fn runtime_run_rejects_zero_tick_ms() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let result = run_context.execute(&runtime_run_json(
            Some("runtime-cli-test-c"),
            0,
            Some(1),
            "on-heartbeat-expiry",
        ));

        assert_config_error_contains(result, "tick-ms must be greater than 0");
    });
}

#[test]
fn runtime_run_rejects_unknown_restart_policy() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let result = run_context.execute(&runtime_run_json(
            Some("runtime-cli-test-d"),
            1,
            Some(1),
            "always",
        ));

        assert_config_error_contains(result, "unknown restart policy");
    });
}

#[test]
fn runtime_run_accounts_distinguish_work_from_quiescence() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let assembly = open_bound_assembly(&workspace_root);
        // One committed graph event makes the first maintenance pass a
        // working pass; later passes are truthfully quiescent.
        assembly
            .event_authority()
            .append_capability()
            .append_durable(
                EventEnvelope::new_domain(
                    "2026-07-25T00:00:00Z".to_string(),
                    "session-account",
                    "workspace_fs",
                    "workspace-a",
                    "workspace.node.observed",
                    None,
                    json!({ "node": "node-account" }),
                )
                .with_graph(
                    vec![DomainObjectRef::new("workspace_fs", "node", "node-account").unwrap()],
                    Vec::new(),
                )
                .with_record_id("workspace-node-account"),
                AppendMode::Plain,
            )
            .unwrap();

        let mut sink = Vec::new();
        let output = meld::runtime::tooling::handle_cli_command_with_account_writer(
            &assembly,
            &RuntimeCommands::Run {
                instance_id: Some("runtime-cli-account".to_string()),
                tick_ms: 1,
                duration_ms: Some(60),
                format: "json".to_string(),
                restart_policy: "on-heartbeat-expiry".to_string(),
                restart_attempt_limit: 3,
                restart_backoff_ms: 0,
            },
            &mut sink,
        )
        .unwrap();
        // The returned run result stays pure JSON; account lines stream
        // through the writer instead.
        serde_json::from_str::<Value>(&output).unwrap();

        let lines = String::from_utf8(sink).unwrap();
        let accounts: Vec<Value> = lines
            .lines()
            .map(|line| serde_json::from_str(line).expect("each account line is one JSON object"))
            .collect();
        assert!(!accounts.is_empty());
        for account in &accounts {
            assert_eq!(account["type"], "runtime_tick_account");
            assert_eq!(account["instance_id"], "runtime-cli-account");
        }
        let working_pass = accounts.iter().find(|account| {
            account["quiescent"] == false
                && account["actors"].as_array().unwrap().iter().any(|actor| {
                    actor["runtime_id"] == "world_model.graph_replay"
                        && actor["items_committed"].as_u64().unwrap() >= 1
                        && actor["lifecycle"] == "active_working"
                })
        });
        assert!(
            working_pass.is_some(),
            "no working pass observed in accounts: {lines}"
        );
        let quiescent_pass = accounts.iter().find(|account| {
            account["quiescent"] == true
                && account["actors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|actor| actor["outcome"] == "no_work")
        });
        assert!(
            quiescent_pass.is_some(),
            "no quiescent pass observed in accounts: {lines}"
        );
    });
}

#[test]
fn runtime_run_publishes_durable_lifecycle_snapshots() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        let output = run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-snapshots"),
                1,
                Some(5),
                "on-heartbeat-expiry",
            ))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        drop(run_context);

        // The clean-stop envelope is durable and readable through the
        // frozen RuntimeStatusReader contract after the process would have
        // exited.
        let assembly = open_bound_assembly(&workspace_root);
        let reader = SupervisorReportStore::open(assembly.supervisor_store()).unwrap();
        let latest = reader
            .read_latest_snapshot()
            .unwrap()
            .expect("shutdown snapshot must be durable");
        assert_eq!(latest.writer.launch_status, RuntimeLaunchStatus::Stopped);
        assert_eq!(
            latest.writer.instance_id.as_deref(),
            Some("runtime-cli-snapshots")
        );
        let shutdown = latest.snapshot.shutdown.expect("shutdown summary");
        assert_eq!(
            shutdown.shutdown_id.as_deref(),
            parsed["shutdown_id"].as_str()
        );
        assert_eq!(shutdown.status, "stopped");
        assert!(latest
            .snapshot
            .instance
            .as_ref()
            .is_some_and(|instance| instance.instance_id == "runtime-cli-snapshots"));
    });
}

#[test]
fn stewardship_boot_composes_production_dispatch_routes() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();
        let workspace_root = workspace(&temp_dir);
        std::fs::create_dir_all(workspace_root.join("docs")).unwrap();
        std::fs::write(workspace_root.join("docs/guide.txt"), "guide\n").unwrap();
        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        let provider = DeterministicDocsProvider::spawn(&workspace_root);
        write_stewardship_config(&workspace_root, provider.endpoint());

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();
        let product = run_context.product_runtime();
        // The stewardship composition carries the route seed but stays a
        // truthful unresolved binding until the foreground run composes the
        // production routes.
        assert!(product.dispatch_route_seed().is_some());
        assert!(!product.dispatch_routes_bound());
        assert!(!product
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .has_semantic_body());

        run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-dispatch"),
                1,
                Some(1),
                "on-heartbeat-expiry",
            ))
            .unwrap();

        // A configured product boot resolves the dispatch actor.
        assert!(product.dispatch_routes_bound());
        assert!(product
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .has_semantic_body());

        let capability_runtime = product.capability_runtime().unwrap();
        for capability_type in [
            "docs.inspect_scope",
            "docs.draft_patch_set",
            "docs.validate_patch_set",
            "docs.publish_patch_set",
            "docs.assess_published_scope",
        ] {
            assert!(capability_runtime.catalog.contains(capability_type, 1));
            assert!(capability_runtime
                .registry
                .get(capability_type, 1)
                .is_some());
        }
        assert!(!capability_runtime.catalog.contains("merkle_traversal", 1));

        drop(run_context);
        provider.shutdown();
    });
}

fn write_stewardship_config(workspace_root: &Path, endpoint: &str) {
    let config_dir = workspace_root.join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    let target_root = workspace_root.canonicalize().unwrap();
    let config = format!(
        r#"[providers.steward-provider]
provider_name = "steward-provider"
provider_type = "local"
model = "test-model"
endpoint = "{endpoint}"

[stewardship.docs_freshness]
expression = "docs_freshness"
target_root = "{target_root}"
subject = "docs"
agent_id = "docs-writer"
provider_id = "steward-provider"

[stewardship.docs_freshness.theory]
belief_family_id = "docs_freshness"
evidence_mapping_id = "docs_freshness_outcome_interpretation_v1"
curation_rule_id = "docs_freshness"
"#,
        target_root = target_root.display()
    );
    std::fs::write(config_dir.join("config.toml"), config).unwrap();
}

fn workspace(temp_dir: &TempDir) -> std::path::PathBuf {
    let workspace_root = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();
    workspace_root
}

fn runtime_status_json(runtime_ids: Vec<String>) -> Commands {
    Commands::Runtime {
        command: RuntimeCommands::Status {
            format: "json".to_string(),
            runtime_ids,
        },
    }
}

fn runtime_run_json(
    instance_id: Option<&str>,
    tick_ms: u64,
    duration_ms: Option<u64>,
    restart_policy: &str,
) -> Commands {
    Commands::Runtime {
        command: RuntimeCommands::Run {
            instance_id: instance_id.map(str::to_string),
            tick_ms,
            duration_ms,
            format: "json".to_string(),
            restart_policy: restart_policy.to_string(),
            restart_attempt_limit: 3,
            restart_backoff_ms: 0,
        },
    }
}

fn runtime_row<'a>(parsed: &'a Value, runtime_id: &str) -> &'a Value {
    parsed["runtimes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|runtime| runtime["runtime_id"] == runtime_id)
        .unwrap()
}

fn assert_config_error_contains(result: Result<String, ApiError>, expected: &str) {
    match result {
        Err(ApiError::ConfigError(message)) => {
            assert!(message.contains("Runtime command failed:"));
            assert!(message.contains(expected), "{message}");
        }
        other => panic!("expected config error, got {other:?}"),
    }
}
