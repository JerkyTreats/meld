use meld_events::events::store::EventStore;
use meld_events::{
    DomainObjectRef, EventBus, EventEnvelope, EventIngestor, EventRecord, EventRuntime,
};
use proptest::prelude::*;
use serde_json::json;
use std::sync::Arc;

const SESSION_A: &str = "session-a";
const SESSION_B: &str = "session-b";
const RECORDED_AT: &str = "2026-04-26T16:00:00Z";

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn task_run() -> DomainObjectRef {
    object("execution", "task_run", "run-a")
}

fn artifact() -> DomainObjectRef {
    object("execution", "artifact", "artifact-a")
}

fn event_store() -> (tempfile::TempDir, EventStore) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    (temp_dir, EventStore::new(db).unwrap())
}

fn event_runtime() -> (tempfile::TempDir, EventRuntime) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    (temp_dir, EventRuntime::new(db).unwrap())
}

fn runtime_event(seq: u64, session: &str, event_type: &str) -> EventRecord {
    EventRecord {
        ts: RECORDED_AT.to_string(),
        recorded_at: RECORDED_AT.to_string(),
        record_id: None,
        session: session.to_string(),
        seq,
        domain_id: "telemetry".to_string(),
        stream_id: session.to_string(),
        event_type: event_type.to_string(),
        occurred_at: None,
        content_hash: None,
        objects: Vec::new(),
        relations: Vec::new(),
        data: json!({ "seq": seq }),
    }
}

fn domain_envelope(event_type: &str) -> EventEnvelope {
    EventEnvelope::new_domain(
        RECORDED_AT.to_string(),
        SESSION_A,
        "execution",
        "workflow-a",
        event_type,
        Some("sha256:abc".to_string()),
        json!({ "artifact": "artifact-a" }),
    )
}

fn related_domain_envelope() -> EventEnvelope {
    let task = task_run();
    let artifact = artifact();
    let relation =
        meld_events::EventRelation::new("produced", task.clone(), artifact.clone()).unwrap();

    domain_envelope("execution.artifact.available")
        .with_occurred_at("2026-04-26T15:59:59Z")
        .with_graph(vec![task, artifact], vec![relation])
}

#[test]
fn events_public_imports_compile() {
    let (_temp_dir, store) = event_store();
    assert!(store.read_events("missing").unwrap().is_empty());
}

#[test]
fn events_module_boundary_has_no_mod_rs() {
    assert!(!std::path::Path::new("src/events/mod.rs").exists());
}

#[test]
fn event_contracts_round_trip_and_validate() {
    let task = task_run();
    let artifact = artifact();
    let relation =
        meld_events::EventRelation::new("produced", task.clone(), artifact.clone()).unwrap();
    let envelope = related_domain_envelope().with_record_id("record-a");
    let record = EventRecord::from_envelope(envelope, 7);

    let parsed: EventRecord =
        serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();

    assert_eq!(parsed, record);
    assert!(task.validate().is_ok());
    assert!(relation.validate().is_ok());
    assert_eq!(relation.src, task);
    assert_eq!(relation.dst, artifact);
}

#[test]
fn event_contracts_reject_empty_identity_components() {
    assert!(DomainObjectRef::new("", "task_run", "run-a").is_err());
    assert!(DomainObjectRef::new("execution", " ", "run-a").is_err());
    assert!(DomainObjectRef::new("execution", "task_run", "").is_err());

    let result = meld_events::EventRelation::new("", task_run(), artifact());
    assert!(result.is_err());
}

#[test]
fn envelope_conversion_preserves_domain_metadata() {
    let record = EventRecord::from_envelope(related_domain_envelope(), 3);

    assert_eq!(record.seq, 3);
    assert_eq!(record.domain_id, "execution");
    assert_eq!(record.stream_id, "workflow-a");
    assert_eq!(record.content_hash.as_deref(), Some("sha256:abc"));
    assert_eq!(record.occurred_at.as_deref(), Some("2026-04-26T15:59:59Z"));
    assert_eq!(record.objects, vec![task_run(), artifact()]);
    assert_eq!(record.relations[0].relation_type, "produced");
}

#[test]
fn envelope_constructors_set_recorded_timestamps() {
    let empty_timestamp = EventEnvelope::new(
        String::new(),
        SESSION_A.to_string(),
        "session.started",
        json!({}),
    );
    let now_timestamp = EventEnvelope::with_now(SESSION_A, "session.started", json!({}));

    assert!(empty_timestamp.ts.is_empty());
    assert!(!empty_timestamp.recorded_at.is_empty());
    assert_ne!(empty_timestamp.recorded_at, "xyzzy");
    assert!(!now_timestamp.ts.is_empty());
    assert_eq!(now_timestamp.recorded_at, now_timestamp.ts);
    assert_ne!(now_timestamp.ts, "xyzzy");
}

#[test]
fn store_orders_session_reads_and_filters_after_cursor() {
    let (_temp_dir, store) = event_store();

    store
        .append_event(&runtime_event(3, SESSION_A, "session.ended"))
        .unwrap();
    store
        .append_event(&runtime_event(1, SESSION_A, "session.started"))
        .unwrap();
    store
        .append_event(&runtime_event(2, SESSION_B, "session.started"))
        .unwrap();

    let all_session_a = store.read_events(SESSION_A).unwrap();
    let after_first = store.read_events_after(SESSION_A, 1).unwrap();
    let all_spine = store.read_all_events_after(0).unwrap();

    assert_eq!(
        all_session_a
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(
        after_first
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![3]
    );
    assert_eq!(
        all_spine.iter().map(|event| event.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        store
            .read_all_events_after(1)
            .unwrap()
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
}

#[test]
fn manual_sequence_append_advances_allocator() {
    let (_temp_dir, store) = event_store();
    store
        .append_event(&runtime_event(9, SESSION_A, "session.started"))
        .unwrap();

    let seq = store
        .append_envelope(domain_envelope("session.continued"))
        .unwrap();
    let next_seq = store
        .append_envelope(domain_envelope("session.finished"))
        .unwrap();

    assert_eq!(seq, 10);
    assert_eq!(next_seq, 11);
}

#[test]
fn sequence_allocator_progresses_without_append_repair() {
    let (_temp_dir, store) = event_store();

    assert_eq!(store.allocate_next_seq().unwrap(), 1);
    assert_eq!(store.allocate_next_seq().unwrap(), 2);
    assert_eq!(store.allocate_next_seq().unwrap(), 3);
}

#[test]
fn idempotent_append_reuses_record_sequence_and_survives_reopen() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("events");
    let first_seq;
    {
        let store = EventStore::new(sled::open(&path).unwrap()).unwrap();
        let envelope = domain_envelope("execution.task.completed").with_record_id("record-a");

        first_seq = store.append_envelope_idempotent(envelope.clone()).unwrap();
        let second_seq = store.append_envelope_idempotent(envelope).unwrap();

        assert_eq!(first_seq, second_seq);
        assert_eq!(store.read_events(SESSION_A).unwrap().len(), 1);
        store.flush().unwrap();
    }

    let reopened = EventStore::new(sled::open(&path).unwrap()).unwrap();
    let duplicate = reopened
        .append_envelope_idempotent(
            domain_envelope("execution.task.completed").with_record_id("record-a"),
        )
        .unwrap();

    assert_eq!(duplicate, first_seq);
    assert_eq!(reopened.read_events(SESSION_A).unwrap().len(), 1);
}

#[test]
fn idempotent_presequenced_record_reuses_indexed_sequence() {
    let (_temp_dir, store) = event_store();
    let mut record = runtime_event(5, SESSION_A, "session.started");
    record.record_id = Some("record-five".to_string());

    assert_eq!(store.append_event_idempotent(&record).unwrap(), 5);

    let mut duplicate = runtime_event(99, SESSION_A, "session.duplicate");
    duplicate.record_id = Some("record-five".to_string());
    assert_eq!(store.append_event_idempotent(&duplicate).unwrap(), 5);
    assert_eq!(
        store
            .read_events(SESSION_A)
            .unwrap()
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![5]
    );
}

#[test]
fn store_reopen_preserves_domain_events() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("events");
    {
        let store = EventStore::new(sled::open(&path).unwrap()).unwrap();
        store.append_envelope(related_domain_envelope()).unwrap();
        store.flush().unwrap();
    }

    let reopened = EventStore::new(sled::open(&path).unwrap()).unwrap();
    let events = reopened.read_events(SESSION_A).unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].objects, vec![task_run(), artifact()]);
    assert_eq!(events[0].relations[0].relation_type, "produced");
}

#[test]
fn legacy_events_normalize_defaults() {
    let (_temp_dir, store) = event_store();
    let legacy = EventRecord {
        ts: RECORDED_AT.to_string(),
        recorded_at: String::new(),
        record_id: None,
        session: SESSION_A.to_string(),
        seq: 1,
        domain_id: String::new(),
        stream_id: String::new(),
        event_type: "session.started".to_string(),
        occurred_at: None,
        content_hash: None,
        objects: Vec::new(),
        relations: Vec::new(),
        data: json!({ "legacy": true }),
    };
    let legacy_tree = store.db().open_tree("obs_events").unwrap();
    let key = EventStore::encode_event_key(SESSION_A, 1);
    legacy_tree
        .insert(key.as_bytes(), serde_json::to_vec(&legacy).unwrap())
        .unwrap();

    let events = store.read_events(SESSION_A).unwrap();
    let after_equal = store.read_events_after(SESSION_A, 1).unwrap();

    assert_eq!(events[0].recorded_at, RECORDED_AT);
    assert_eq!(events[0].domain_id, "telemetry");
    assert_eq!(events[0].stream_id, SESSION_A);
    assert!(after_equal.is_empty());
}

#[test]
fn ingestor_drains_bus_into_runtime_order() {
    let (_temp_dir, store) = event_store();
    let shared = Arc::new(store);
    let (bus, rx) = EventBus::new_pair();
    let mut ingestor = EventIngestor::new(shared.clone(), rx);

    bus.emit_envelope(domain_envelope("execution.started"))
        .unwrap();
    bus.emit_envelope(domain_envelope("execution.completed"))
        .unwrap();

    assert_eq!(ingestor.ingest_pending().unwrap(), 2);
    assert_eq!(
        shared
            .read_all_events_after(0)
            .unwrap()
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn bounded_bus_reports_backpressure_without_dropping_queued_event() {
    let (_temp_dir, store) = event_store();
    let shared = Arc::new(store);
    let (bus, rx) = EventBus::new_pair_with_capacity(1);
    let mut ingestor = EventIngestor::new(shared.clone(), rx);

    bus.emit_envelope(domain_envelope("execution.started"))
        .unwrap();
    assert!(bus
        .emit_envelope(domain_envelope("execution.completed"))
        .is_err());

    assert_eq!(ingestor.ingest_pending().unwrap(), 1);
    assert_eq!(shared.read_all_events_after(0).unwrap().len(), 1);
}

#[test]
fn runtime_emit_domain_event_persists_event() {
    let (_temp_dir, runtime) = event_runtime();

    runtime
        .emit_domain_event(
            SESSION_A,
            "execution",
            "workflow-a",
            "execution.started",
            Some("sha256:abc".to_string()),
            json!({ "started": true }),
        )
        .unwrap();

    let events = runtime.store().read_events(SESSION_A).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].domain_id, "execution");
    assert_eq!(events[0].stream_id, "workflow-a");
}

#[test]
fn runtime_emit_envelope_variants_persist_events() {
    let (_temp_dir, runtime) = event_runtime();

    runtime
        .emit_envelope(domain_envelope("execution.started"))
        .unwrap();
    runtime
        .emit_envelope_idempotent(
            domain_envelope("execution.idempotent").with_record_id("runtime-record-a"),
        )
        .unwrap();
    runtime
        .emit_envelope_idempotent(
            domain_envelope("execution.idempotent").with_record_id("runtime-record-a"),
        )
        .unwrap();

    let events = runtime.store().read_events(SESSION_A).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["execution.started", "execution.idempotent"]
    );
}

#[test]
fn runtime_batch_emit_variants_persist_events() {
    let (_temp_dir, runtime) = event_runtime();

    runtime
        .emit_envelopes([
            domain_envelope("execution.started"),
            domain_envelope("execution.completed"),
        ])
        .unwrap();
    runtime
        .emit_envelopes_idempotent([
            domain_envelope("execution.batch.idempotent").with_record_id("batch-record-a"),
            domain_envelope("execution.batch.idempotent").with_record_id("batch-record-a"),
        ])
        .unwrap();

    let events = runtime.store().read_events(SESSION_A).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(
        events.iter().map(|event| event.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn runtime_best_effort_success_paths_persist_events() {
    let (_temp_dir, runtime) = event_runtime();

    runtime.emit_event_best_effort(SESSION_A, "telemetry.started", json!({}));
    runtime.emit_domain_event_best_effort(
        SESSION_A,
        "execution",
        "workflow-a",
        "execution.started",
        None,
        json!({}),
    );
    runtime.emit_envelope_best_effort(domain_envelope("execution.envelope"));
    runtime.emit_envelope_idempotent_best_effort(
        domain_envelope("execution.idempotent").with_record_id("best-effort-record-a"),
    );
    runtime.emit_envelope_idempotent_best_effort(
        domain_envelope("execution.idempotent").with_record_id("best-effort-record-a"),
    );

    let events = runtime.store().read_events(SESSION_A).unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(
        events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        vec![
            "telemetry.started",
            "execution.started",
            "execution.envelope",
            "execution.idempotent"
        ]
    );
}

proptest! {
    #[test]
    fn object_ref_index_key_preserves_components(
        domain in "[a-z][a-z0-9_]{0,24}",
        kind in "[a-z][a-z0-9_]{0,24}",
        id in "[a-zA-Z0-9][a-zA-Z0-9_.-]{0,32}",
    ) {
        let object_ref = DomainObjectRef::new(domain.clone(), kind.clone(), id.clone()).unwrap();

        prop_assert_eq!(object_ref.index_key(), format!("{domain}::{kind}::{id}"));
        prop_assert_eq!(
            serde_json::from_str::<DomainObjectRef>(
                &serde_json::to_string(&object_ref).unwrap()
            )
            .unwrap(),
            object_ref
        );
    }
}
