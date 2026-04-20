use meld::control::projection::ExecutionProjection;
use meld::task::ExecutionTaskEventData;
use meld::telemetry::ProgressRuntime;
use serde_json::json;

fn emit_task_event(
    runtime: &ProgressRuntime,
    session_id: &str,
    event_type: &str,
    data: &ExecutionTaskEventData,
) {
    runtime
        .emit_domain_event(
            session_id,
            "execution",
            &data.task_run_id,
            event_type,
            None,
            json!(data),
        )
        .unwrap();
}

fn task_event_data(task_id: &str, task_run_id: &str) -> ExecutionTaskEventData {
    ExecutionTaskEventData {
        task_id: task_id.to_string(),
        task_run_id: task_run_id.to_string(),
        capability_instance_id: None,
        invocation_id: None,
        target_node_id: None,
        artifact_id: None,
        artifact_type_id: None,
        attempt_index: None,
        ready_count: None,
        running_count: None,
        blocked_reason: None,
        error: None,
    }
}

#[test]
fn replay_rebuilds_execution_projection() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();
    let session_id = runtime
        .start_command_session("projection".to_string())
        .unwrap();

    let task_one = task_event_data("task_one", "run_one");
    emit_task_event(&runtime, &session_id, "execution.task.requested", &task_one);
    emit_task_event(&runtime, &session_id, "execution.task.started", &task_one);
    let mut task_one_artifact = task_one.clone();
    task_one_artifact.artifact_id = Some("artifact_one".to_string());
    task_one_artifact.artifact_type_id = Some("resolved_node_ref".to_string());
    emit_task_event(
        &runtime,
        &session_id,
        "execution.task.artifact_emitted",
        &task_one_artifact,
    );
    emit_task_event(&runtime, &session_id, "execution.task.succeeded", &task_one);

    let mut task_two = task_event_data("task_two", "run_two");
    emit_task_event(&runtime, &session_id, "execution.task.requested", &task_two);
    task_two.blocked_reason = Some("missing_input".to_string());
    emit_task_event(&runtime, &session_id, "execution.task.blocked", &task_two);
    task_two.error = Some("provider failed".to_string());
    emit_task_event(&runtime, &session_id, "execution.task.failed", &task_two);

    let projection = ExecutionProjection::replay_from_store(runtime.store(), 0).unwrap();

    assert!(projection.active_tasks.is_empty());
    assert!(projection.blocked_tasks.is_empty());
    assert!(projection.completed_tasks.contains("run_one"));
    assert!(projection.failed_tasks.contains("run_two"));
    assert_eq!(
        projection
            .artifacts_by_task_run
            .get("run_one")
            .cloned()
            .unwrap_or_default(),
        ["artifact_one".to_string()].into_iter().collect()
    );
}

#[test]
fn projection_matches_live_execution() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();
    let session_id = runtime
        .start_command_session("projection".to_string())
        .unwrap();
    let task = task_event_data("task_one", "run_one");

    emit_task_event(&runtime, &session_id, "execution.task.requested", &task);
    emit_task_event(&runtime, &session_id, "execution.task.started", &task);
    let mut artifact_event = task.clone();
    artifact_event.artifact_id = Some("artifact_one".to_string());
    artifact_event.artifact_type_id = Some("resolved_node_ref".to_string());
    emit_task_event(
        &runtime,
        &session_id,
        "execution.task.artifact_emitted",
        &artifact_event,
    );
    emit_task_event(&runtime, &session_id, "execution.task.succeeded", &task);

    let mut live_projection = ExecutionProjection::default();
    let mut last_seq = 0;
    loop {
        let batch = runtime.store().read_all_events_after(last_seq).unwrap();
        if batch.is_empty() {
            break;
        }
        for event in &batch {
            last_seq = event.seq;
            live_projection.apply(event).unwrap();
        }
    }

    let replay_projection = ExecutionProjection::replay_from_store(runtime.store(), 0).unwrap();
    assert_eq!(live_projection, replay_projection);
}
