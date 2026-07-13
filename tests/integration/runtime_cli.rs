use meld::cli::{Commands, RunContext, RuntimeCommands};
use meld::config::ConfigLoader;
use meld::error::ApiError;
use meld::events::binding::resolve_product_event_authority;
use meld::runtime::assembly::ProductRuntimeAssembly;
use meld::runtime::storage::ProductStorageLayout;
use meld::runtime::supervisor::RuntimeId;
use meld_events::{AppendMode, DomainObjectRef, EventEnvelope};
use serde_json::json;
use serde_json::Value;
use tempfile::TempDir;

use super::with_xdg_env;

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
        assert_eq!(event_append["desired_enabled"], false);
        assert_eq!(event_append["factory_available"], true);
        assert_eq!(event_append["role_class"], "passive_service");
        assert_eq!(event_append["implementation_state"], "concrete");
        assert_eq!(event_append["handle_started"], false);
        let graph_replay = runtime_row(&parsed, "world_model.graph_replay");
        assert_eq!(graph_replay["desired_enabled"], true);
        assert_eq!(graph_replay["factory_available"], true);
        assert_eq!(graph_replay["role_class"], "actor");
        let task_dispatch = runtime_row(&parsed, "execution.task_dispatch");
        assert_eq!(task_dispatch["desired_enabled"], false);
        assert_eq!(task_dispatch["factory_available"], true);
        assert_eq!(task_dispatch["implementation_state"], "inert");
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
            .execute(&runtime_status_json(vec![
                "world_model.graph.replay".to_string()
            ]))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let runtimes = parsed["runtimes"].as_array().unwrap();

        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0]["runtime_id"], "world_model.graph_replay");
    });
}

#[test]
fn runtime_status_text_distinguishes_role_and_implementation() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let output = run_context
            .execute(&runtime_status(
                "text",
                vec!["execution.task_dispatch".to_string()],
            ))
            .unwrap();

        assert!(output.contains("execution.task_dispatch disabled"));
        assert!(output.contains("role=actor"));
        assert!(output.contains("implementation=inert"));
        assert!(output.contains("factory=available"));
        assert!(!output.contains("execution.task_dispatch disabled unavailable"));
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

fn workspace(temp_dir: &TempDir) -> std::path::PathBuf {
    let workspace_root = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();
    workspace_root
}

fn runtime_status_json(runtime_ids: Vec<String>) -> Commands {
    runtime_status("json", runtime_ids)
}

fn runtime_status(format: &str, runtime_ids: Vec<String>) -> Commands {
    Commands::Runtime {
        command: RuntimeCommands::Status {
            format: format.to_string(),
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
