use meld::control::projection::ExecutionProjection;
use meld::session::policy::PrunePolicy;
use meld::task::ExecutionTaskEventData;
use meld::telemetry::emission::emit_command_summary;
use meld::telemetry::events::ProgressEnvelope;
use meld::telemetry::routing::bus::ProgressBus;
use meld::telemetry::routing::ingestor::EventIngestor;
use meld::telemetry::sinks::store::ProgressStore;
use meld::telemetry::ProgressRuntime;
use meld::telemetry::{DomainObjectRef, EventRelation};
use serde_json::json;

#[test]
fn runtime_wide_sequence_is_monotonic() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();

    let session_one = runtime.start_command_session("scan".to_string()).unwrap();
    let session_two = runtime.start_command_session("watch".to_string()).unwrap();

    runtime
        .emit_event(&session_one, "scan_progress", json!({ "count": 1 }))
        .unwrap();
    runtime
        .emit_event(&session_two, "watch_tick", json!({ "count": 1 }))
        .unwrap();

    let all_events = runtime.store().read_all_events_after(0).unwrap();
    assert_eq!(all_events.len(), 4);
    assert!(all_events
        .windows(2)
        .all(|pair| pair[1].seq == pair[0].seq + 1));

    let session_one_events = runtime.store().read_events(&session_one).unwrap();
    let session_two_events = runtime.store().read_events(&session_two).unwrap();

    assert_eq!(
        session_one_events
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(
        session_two_events
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![2, 4]
    );
}

#[test]
fn legacy_spine_events_remain_readable_with_graph_fields() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();
    let session_id = "legacy-session";
    let legacy_tree = runtime.store().db().open_tree("obs_events").unwrap();
    let key = meld::telemetry::sinks::store::ProgressStore::encode_event_key(session_id, 1);
    let raw = r#"{"ts":"2026-02-14T12:34:56.789Z","session":"legacy-session","seq":1,"type":"session_started","data":{"command":"scan"}}"#;

    legacy_tree.insert(key.as_bytes(), raw.as_bytes()).unwrap();

    let events = runtime.store().read_events(session_id).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].session, session_id);
    assert_eq!(events[0].seq, 1);
    assert_eq!(events[0].domain_id, "telemetry");
    assert_eq!(events[0].stream_id, session_id);
    assert_eq!(events[0].content_hash, None);
    assert_eq!(events[0].event_type, "session_started");
    assert_eq!(events[0].recorded_at, events[0].ts);
    assert!(events[0].objects.is_empty());
    assert!(events[0].relations.is_empty());
}

#[test]
fn mixed_spine_events_replay_with_object_refs() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();
    let session_id = "mixed-session";
    let legacy_tree = runtime.store().db().open_tree("obs_events").unwrap();
    let legacy_key = ProgressStore::encode_event_key(session_id, 1);
    let legacy_raw = r#"{"ts":"2026-02-14T12:34:56.789Z","session":"mixed-session","seq":1,"type":"session_started","data":{"command":"scan"}}"#;
    legacy_tree
        .insert(legacy_key.as_bytes(), legacy_raw.as_bytes())
        .unwrap();

    let task_run = DomainObjectRef::new("execution", "task_run", "run_a").unwrap();
    let artifact = DomainObjectRef::new("execution", "artifact", "artifact_a").unwrap();
    let relation = EventRelation::new("produced", task_run.clone(), artifact.clone()).unwrap();

    let event = meld::telemetry::events::ProgressEvent::from_envelope(
        ProgressEnvelope::with_now_domain(
            session_id.to_string(),
            "execution".to_string(),
            "run_a".to_string(),
            "execution.task.artifact_emitted".to_string(),
            None,
            json!({
                "task_id": "task_a",
                "task_run_id": "run_a",
                "capability_instance_id": null,
                "invocation_id": null,
                "artifact_id": "artifact_a",
                "artifact_type_id": "artifact.type",
                "attempt_index": null,
                "ready_count": null,
                "running_count": null,
                "blocked_reason": null,
                "error": null
            }),
        )
        .with_graph(vec![task_run, artifact], vec![relation]),
        2,
    );
    runtime.store().append_event(&event).unwrap();

    let events = runtime.store().read_events(session_id).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].seq, 1);
    assert_eq!(events[1].seq, 2);
    assert_eq!(events[1].domain_id, "execution");
    assert_eq!(events[1].objects.len(), 2);
    assert_eq!(events[1].relations.len(), 1);
}

#[test]
fn telemetry_is_downstream_only() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();
    let session_id = runtime
        .start_command_session("summary".to_string())
        .unwrap();

    let data = ExecutionTaskEventData {
        task_id: "task_one".to_string(),
        task_run_id: "run_one".to_string(),
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
    };

    runtime
        .emit_domain_event(
            &session_id,
            "execution",
            &data.task_run_id,
            "execution.task.requested",
            None,
            json!(data),
        )
        .unwrap();

    emit_command_summary(
        &runtime,
        &session_id,
        "summary",
        None,
        true,
        10,
        Some("ok".to_string()),
        None,
        None,
        None,
    );

    let events = runtime.store().read_all_events_after(0).unwrap();
    assert!(events
        .iter()
        .any(|event| event.event_type == "command_summary"));

    let projection = ExecutionProjection::replay_from_store(runtime.store(), 0).unwrap();
    assert!(projection.active_tasks.contains("run_one"));
    assert!(projection.completed_tasks.is_empty());
    assert!(projection.last_applied_seq < events.last().unwrap().seq);
}

#[test]
fn slow_or_missing_consumer_does_not_break_append() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let store = ProgressStore::shared(db).unwrap();
    let (bus, receiver) = ProgressBus::new_pair_with_capacity(1);

    bus.emit("s1", "session_started", json!({})).unwrap();
    assert!(matches!(
        bus.emit("s1", "session_ended", json!({})),
        Err(std::sync::mpsc::TrySendError::Full(_))
    ));

    let mut ingestor = EventIngestor::new(store.clone(), receiver);
    ingestor.ingest_pending().unwrap();

    let events = store.read_all_events_after(0).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].seq, 1);

    bus.emit("s1", "session_ended", json!({})).unwrap();
    ingestor.ingest_pending().unwrap();

    let events = store.read_all_events_after(0).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].seq, 2);
}

#[test]
fn session_prune_does_not_delete_canonical_spine_history() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();

    let session_id = runtime.start_command_session("scan".to_string()).unwrap();
    runtime
        .emit_event(&session_id, "scan_progress", json!({ "count": 1 }))
        .unwrap();
    runtime
        .finish_command_session(&session_id, true, None)
        .unwrap();

    let pruned = runtime
        .prune(PrunePolicy {
            max_completed: 0,
            max_age_ms: u64::MAX,
        })
        .unwrap();
    assert_eq!(pruned, 1);

    let all_events = runtime.store().read_all_events_after(0).unwrap();
    assert_eq!(all_events.len(), 3);
    assert!(all_events.iter().any(|event| event.session == session_id));
}

#[test]
fn read_events_after_pruned_session_still_reads_canonical_history() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();

    let session_id = runtime.start_command_session("scan".to_string()).unwrap();
    runtime
        .emit_event(&session_id, "scan_progress", json!({ "count": 1 }))
        .unwrap();
    runtime
        .finish_command_session(&session_id, true, None)
        .unwrap();

    runtime
        .prune(PrunePolicy {
            max_completed: 0,
            max_age_ms: u64::MAX,
        })
        .unwrap();

    let events = runtime.store().read_events(&session_id).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_type, "session_started");
    assert_eq!(events[1].event_type, "scan_progress");
    assert_eq!(events[2].event_type, "session_ended");
}

#[test]
fn idempotent_append_reuses_existing_record_id() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();
    let session_id = runtime.start_command_session("graph".to_string()).unwrap();

    let envelope = ProgressEnvelope::with_now_domain(
        session_id.clone(),
        "world_state".to_string(),
        "graph".to_string(),
        "world_state.anchor_selected".to_string(),
        None,
        json!({
            "anchor_id": "anchor_one",
            "object_id": "object_one"
        }),
    )
    .with_record_id("world_state::anchor_selected::anchor_one");

    runtime.emit_envelope_idempotent(envelope.clone()).unwrap();
    runtime.emit_envelope_idempotent(envelope).unwrap();

    let events = runtime.store().read_all_events_after(0).unwrap();
    let selected: Vec<_> = events
        .iter()
        .filter(|event| event.event_type == "world_state.anchor_selected")
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].record_id.as_deref(),
        Some("world_state::anchor_selected::anchor_one")
    );
}

#[test]
fn legacy_records_ignore_missing_record_id() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let runtime = ProgressRuntime::new(db).unwrap();
    let session_id = "legacy-record-id";
    let legacy_tree = runtime.store().db().open_tree("obs_events").unwrap();
    let key = ProgressStore::encode_event_key(session_id, 1);
    let raw = r#"{"ts":"2026-02-14T12:34:56.789Z","session":"legacy-record-id","seq":1,"type":"session_started","data":{"command":"scan"}}"#;

    legacy_tree.insert(key.as_bytes(), raw.as_bytes()).unwrap();

    let events = runtime.store().read_events(session_id).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].record_id, None);
    assert_eq!(events[0].event_type, "session_started");
}
