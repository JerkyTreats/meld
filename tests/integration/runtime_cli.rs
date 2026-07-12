use meld::cli::{Commands, RunContext, RuntimeCommands};
use meld::config::ConfigLoader;
use meld::error::ApiError;
use meld::runtime::assembly::ProductRuntimeAssembly;
use meld::runtime::supervisor::RuntimeId;
use meld_events::{DomainObjectRef, EventEnvelope};
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
        let config = ConfigLoader::load(&workspace_root).unwrap();
        let assembly =
            ProductRuntimeAssembly::load_for_workspace(&workspace_root, &config).unwrap();
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
        let config = ConfigLoader::load(&workspace_root).unwrap();
        let assembly =
            ProductRuntimeAssembly::load_for_workspace(&workspace_root, &config).unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        assembly
            .stores()
            .event_store
            .append_envelope(
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

        let assembly =
            ProductRuntimeAssembly::load_for_workspace(&workspace_root, &config).unwrap();
        assert_eq!(
            assembly.ports().graph_cursor().current().unwrap(),
            Some(meld_events::ConsumerCursorPosition {
                ledger_id: assembly.ports().event_replay().ledger_identity(),
                name: "world_state.graph.reducer".to_string(),
                reported_seq: 1,
            })
        );
    });
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
