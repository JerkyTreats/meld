use meld_events::events::store::EventStore;
use meld_events::{DomainObjectRef, EventEnvelope, EventRecord, EventRuntime, EventWriter};
use proptest::prelude::*;
use serde_json::json;
use std::collections::BTreeMap;
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
    EventRecord::from_envelope(
        EventEnvelope::new(
            RECORDED_AT.to_string(),
            session.to_string(),
            event_type,
            json!({ "seq": seq }),
        ),
        seq,
    )
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

#[derive(Clone, Debug)]
struct GeneratedEventSpec {
    ts: String,
    session: String,
    domain_id: String,
    stream_id: String,
    event_type: String,
    content_hash: Option<String>,
    record_id: Option<String>,
    marker: u16,
}

impl GeneratedEventSpec {
    fn envelope(&self) -> EventEnvelope {
        let envelope = EventEnvelope::new_domain(
            self.ts.clone(),
            self.session.clone(),
            self.domain_id.clone(),
            self.stream_id.clone(),
            self.event_type.clone(),
            self.content_hash.clone(),
            json!({ "marker": self.marker }),
        );
        with_optional_record_id(envelope, self.record_id.clone())
    }
}

fn with_optional_record_id(envelope: EventEnvelope, record_id: Option<String>) -> EventEnvelope {
    match record_id {
        Some(record_id) => envelope.with_record_id(record_id),
        None => envelope,
    }
}

struct LegacyRecordJson {
    ts: String,
    recorded_at: String,
    session: String,
    seq: u64,
    domain_id: String,
    stream_id: String,
    event_type: String,
    data: serde_json::Value,
}

fn legacy_record_json(record: LegacyRecordJson) -> serde_json::Value {
    json!({
        "ts": record.ts,
        "recorded_at": record.recorded_at,
        "record_id": null,
        "session": record.session,
        "seq": record.seq,
        "domain_id": record.domain_id,
        "stream_id": record.stream_id,
        "type": record.event_type,
        "occurred_at": null,
        "content_hash": null,
        "objects": [],
        "relations": [],
        "data": record.data
    })
}

prop_compose! {
    fn generated_timestamp()(minute in 0u8..60, second in 0u8..60) -> String {
        format!("2026-04-26T16:{minute:02}:{second:02}Z")
    }
}

prop_compose! {
    fn generated_component()(value in "[a-z][a-z0-9_-]{0,16}") -> String {
        value
    }
}

prop_compose! {
    fn generated_event_type()(
        domain in "[a-z][a-z0-9_]{0,12}",
        action in "[a-z][a-z0-9_]{0,12}",
    ) -> String {
        format!("{domain}.{action}")
    }
}

prop_compose! {
    fn generated_content_hash()(value in "[a-f0-9]{1,24}") -> String {
        format!("sha256:{value}")
    }
}

prop_compose! {
    fn generated_event_spec()(
        ts in generated_timestamp(),
        session in generated_component(),
        domain_id in generated_component(),
        stream_id in generated_component(),
        event_type in generated_event_type(),
        content_hash in prop::option::of(generated_content_hash()),
        record_id in prop::option::of(generated_component()),
        marker in any::<u16>(),
    ) -> GeneratedEventSpec {
        GeneratedEventSpec {
            ts,
            session,
            domain_id,
            stream_id,
            event_type,
            content_hash,
            record_id,
            marker,
        }
    }
}

prop_compose! {
    fn generated_object_ref()(
        domain_id in generated_component(),
        object_kind in generated_component(),
        object_id in generated_component(),
    ) -> DomainObjectRef {
        DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
    }
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
fn event_record_serializes_current_structural_shape() {
    let record =
        EventRecord::from_envelope(related_domain_envelope().with_record_id("record-a"), 7);

    let serialized = serde_json::to_value(&record).unwrap();

    assert_eq!(serialized["seq"], 7);
    assert!(serialized.get("envelope").is_some());
    assert!(serialized.get("type").is_none());
    assert!(serialized.get("data").is_none());
    assert_eq!(serialized["envelope"]["session"], SESSION_A);
    assert_eq!(
        serialized["envelope"]["type"],
        "execution.artifact.available"
    );
    assert_eq!(serialized["envelope"]["record_id"], "record-a");
}

#[test]
fn event_record_deserializes_legacy_flattened_shape_with_current_parity() {
    let task = task_run();
    let artifact = artifact();
    let relation =
        meld_events::EventRelation::new("produced", task.clone(), artifact.clone()).unwrap();
    let envelope = related_domain_envelope().with_record_id("record-a");
    let current = json!({
        "seq": 7,
        "envelope": envelope
    });
    let legacy = json!({
        "ts": RECORDED_AT,
        "recorded_at": RECORDED_AT,
        "record_id": "record-a",
        "session": SESSION_A,
        "seq": 7,
        "domain_id": "execution",
        "stream_id": "workflow-a",
        "type": "execution.artifact.available",
        "occurred_at": "2026-04-26T15:59:59Z",
        "content_hash": "sha256:abc",
        "objects": [task, artifact],
        "relations": [relation],
        "data": { "artifact": "artifact-a" }
    });

    let current: EventRecord = serde_json::from_value(current).unwrap();
    let legacy: EventRecord = serde_json::from_value(legacy).unwrap();

    assert_eq!(legacy, current);
    assert_eq!(legacy.envelope(), current.envelope());
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
    let all_events = store.read_all_events_after(0).unwrap();

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
        all_events.iter().map(|event| event.seq).collect::<Vec<_>>(),
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
fn store_reads_all_events_after_with_limit() {
    let (_temp_dir, store) = event_store();

    store
        .append_event(&runtime_event(3, SESSION_A, "session.third"))
        .unwrap();
    store
        .append_event(&runtime_event(1, SESSION_A, "session.first"))
        .unwrap();
    store
        .append_event(&runtime_event(2, SESSION_B, "session.second"))
        .unwrap();

    let first_two = store.read_all_events_after_limit(0, 2).unwrap();
    let after_first = store.read_all_events_after_limit(1, 8).unwrap();
    let none = store.read_all_events_after_limit(0, 0).unwrap();

    assert_eq!(
        first_two.iter().map(|event| event.seq).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        after_first
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert!(none.is_empty());
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
fn appended_envelopes_receive_gapless_sequences_from_one() {
    let (_temp_dir, store) = event_store();

    assert_eq!(
        store
            .append_envelope(domain_envelope("session.started"))
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .append_envelope(domain_envelope("session.continued"))
            .unwrap(),
        2
    );
    assert_eq!(
        store
            .append_envelope(domain_envelope("session.finished"))
            .unwrap(),
        3
    );
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

// Replay below the retained lower boundary must fail with a typed gap so
// no consumer can silently skip compacted history; cursors at or above the
// boundary replay normally.
#[test]
fn replay_below_retained_boundary_returns_typed_gap() {
    let (_temp_dir, store) = event_store();
    for i in 0..5 {
        store
            .append_envelope(domain_envelope(&format!("execution.step.{i}")))
            .unwrap();
    }
    assert_eq!(store.retained_lower_boundary().unwrap(), 1);

    store.set_retained_lower_boundary(3).unwrap();
    // Lowering attempts are ignored: the boundary is a one-way promise.
    store.set_retained_lower_boundary(2).unwrap();
    assert_eq!(store.retained_lower_boundary().unwrap(), 3);

    let global = store.read_all_events_after(1).unwrap_err();
    assert!(matches!(
        global,
        meld_events::error::StorageError::RetentionGap {
            after_seq: 1,
            retained_from: 3,
        }
    ));
    assert!(store.read_all_events_after_limit(0, 2).is_err());
    assert!(store.read_events(SESSION_A).is_err());

    // A cursor exactly at the boundary edge replays without a gap.
    assert_eq!(store.read_all_events_after(2).unwrap().len(), 3);
    assert_eq!(store.read_events_after(SESSION_A, 2).unwrap().len(), 3);
}

// A genesis fact records rebuilt-from-snapshot at a basis sequence, with an
// idempotency key so re-recording the same genesis cannot duplicate.
#[test]
fn genesis_facts_record_snapshot_basis_idempotently() {
    let (_temp_dir, store) = event_store();
    let genesis = meld_events::EventEnvelope::genesis_domain(
        SESSION_A,
        "world_state",
        "graph",
        42,
        json!({ "snapshot": "snapshot-a" }),
    );

    let first = store.append_envelope_idempotent(genesis.clone()).unwrap();
    let second = store.append_envelope_idempotent(genesis).unwrap();

    assert_eq!(first, second);
    let events = store.read_all_events_after(0).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "world_state.genesis");
    assert_eq!(
        events[0].record_id.as_deref(),
        Some("genesis::world_state::graph::42")
    );
}

// Legacy sequences were per session and restart at one in every session:
// multi-session legacy stores must migrate completely, with every session's
// history preserved in its relative order under fresh global sequences.
#[test]
fn multi_session_legacy_stores_migrate_completely() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    let legacy_tree = db.open_tree("obs_events").unwrap();
    for session in ["alpha", "beta"] {
        for seq in 1u64..=3 {
            let legacy = legacy_record_json(LegacyRecordJson {
                ts: RECORDED_AT.to_string(),
                recorded_at: String::new(),
                session: session.to_string(),
                seq,
                domain_id: String::new(),
                stream_id: String::new(),
                event_type: format!("legacy.{session}.{seq}"),
                data: json!({ "seq": seq }),
            });
            let key = EventStore::encode_event_key(session, seq);
            legacy_tree
                .insert(key.as_bytes(), serde_json::to_vec(&legacy).unwrap())
                .unwrap();
        }
    }

    let store = EventStore::new(db).unwrap();
    let all = store.read_all_events_after(0).unwrap();
    assert_eq!(all.len(), 6);
    for session in ["alpha", "beta"] {
        let events = store.read_events(session).unwrap();
        assert_eq!(
            events
                .iter()
                .map(|event| event.event_type.as_str())
                .collect::<Vec<_>>(),
            vec![
                format!("legacy.{session}.1"),
                format!("legacy.{session}.2"),
                format!("legacy.{session}.3"),
            ]
        );
        assert!(events.windows(2).all(|pair| pair[0].seq < pair[1].seq));
    }
}

// A transition-era store holds ledger records and legacy rows whose
// per-session sequences collide numerically with ledger sequences; migration
// must keep both histories intact by re-sequencing the legacy rows.
#[test]
fn legacy_rows_coexist_with_ledger_history_after_migration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    {
        let store = EventStore::new(db.clone()).unwrap();
        store
            .append_envelope(domain_envelope("ledger.first"))
            .unwrap();
        store
            .append_envelope(domain_envelope("ledger.second"))
            .unwrap();
        store.flush().unwrap();
    }
    let legacy = legacy_record_json(LegacyRecordJson {
        ts: RECORDED_AT.to_string(),
        recorded_at: String::new(),
        session: "legacy-era".to_string(),
        seq: 1,
        domain_id: String::new(),
        stream_id: String::new(),
        event_type: "legacy.colliding".to_string(),
        data: json!({}),
    });
    let legacy_tree = db.open_tree("obs_events").unwrap();
    let key = EventStore::encode_event_key("legacy-era", 1);
    legacy_tree
        .insert(key.as_bytes(), serde_json::to_vec(&legacy).unwrap())
        .unwrap();

    // The first open already set the migrated flag on an empty legacy tree,
    // so this store models a transition-era db by clearing it.
    db.open_tree("obs_spine_meta")
        .unwrap()
        .remove("legacy_sessions_migrated")
        .unwrap();

    let store = EventStore::new(db).unwrap();
    let all = store.read_all_events_after(0).unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].event_type, "ledger.first");
    assert_eq!(all[1].event_type, "ledger.second");
    assert_eq!(all[2].event_type, "legacy.colliding");
    assert_eq!(all[2].seq, 3);
    assert_eq!(store.read_events("legacy-era").unwrap().len(), 1);
}

// Stores written before empty index values hold full records in the session
// index; opening slims them and reads resolve through the ledger only.
#[test]
fn full_value_session_index_rows_slim_at_open() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    let record = EventRecord::from_envelope(domain_envelope("execution.task.completed"), 1);
    let value = serde_json::to_vec(&record).unwrap();
    db.open_tree("obs_spine_events")
        .unwrap()
        .insert(format!("{:020}", 1).as_bytes(), value.clone())
        .unwrap();
    db.open_tree("obs_session_event_index")
        .unwrap()
        .insert(format!("{SESSION_A}:{:020}", 1).as_bytes(), value)
        .unwrap();
    db.flush().unwrap();

    let store = EventStore::new(db.clone()).unwrap();
    assert_eq!(store.read_events(SESSION_A).unwrap(), vec![record]);
    let (_, slimmed) = db
        .open_tree("obs_session_event_index")
        .unwrap()
        .first()
        .unwrap()
        .unwrap();
    assert!(slimmed.is_empty());
}

// Records persisted before the record index existed carry no index entry
// and no sequence metadata. Opening the store must repair both, so
// idempotent appends reuse the legacy sequence and new appends never
// collide with it.
#[test]
fn store_open_repairs_missing_record_index_and_sequence_meta() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    let record = EventRecord::from_envelope(
        domain_envelope("execution.task.completed").with_record_id("record-torn"),
        1,
    );
    db.open_tree("obs_spine_events")
        .unwrap()
        .insert(
            format!("{:020}", record.seq).as_bytes(),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    let store = EventStore::new(db).unwrap();
    let seq = store
        .append_envelope_idempotent(
            domain_envelope("execution.task.completed").with_record_id("record-torn"),
        )
        .unwrap();

    assert_eq!(seq, 1);
    assert_eq!(
        store.read_all_events_after(0).unwrap(),
        vec![record.clone()]
    );
    assert_eq!(store.read_events(SESSION_A).unwrap(), vec![record]);
    assert_eq!(
        store
            .append_envelope(domain_envelope("execution.after.repair"))
            .unwrap(),
        2
    );
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
fn store_flush_writes_pending_bytes_to_disk() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("events");
    let db = sled::Config::new()
        .path(&path)
        .flush_every_ms(None)
        .open()
        .unwrap();
    let store = EventStore::new(db.clone()).unwrap();

    for index in 0..64 {
        let mut record = runtime_event(
            index + 1,
            SESSION_A,
            &format!("execution.flush.pending.{index}"),
        );
        record.data = json!({ "payload": "event ".repeat(256) });
        store.append_event(&record).unwrap();
    }

    let before_flush = db.size_on_disk().unwrap();
    store.flush().unwrap();
    let after_flush = db.size_on_disk().unwrap();

    assert!(
        after_flush > before_flush,
        "expected flush to increase on-disk bytes from {before_flush}, got {after_flush}"
    );
}

#[test]
fn legacy_events_normalize_defaults() {
    // Legacy rows exist before the store opens; open-time migration moves
    // them into the ledger with normalized defaults.
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    let legacy = legacy_record_json(LegacyRecordJson {
        ts: RECORDED_AT.to_string(),
        recorded_at: String::new(),
        session: SESSION_A.to_string(),
        seq: 1,
        domain_id: String::new(),
        stream_id: String::new(),
        event_type: "session.started".to_string(),
        data: json!({ "legacy": true }),
    });
    let legacy_tree = db.open_tree("obs_events").unwrap();
    let key = EventStore::encode_event_key(SESSION_A, 1);
    legacy_tree
        .insert(key.as_bytes(), serde_json::to_vec(&legacy).unwrap())
        .unwrap();
    let store = EventStore::new(db).unwrap();

    let events = store.read_events(SESSION_A).unwrap();
    let after_equal = store.read_events_after(SESSION_A, 1).unwrap();

    assert_eq!(events[0].recorded_at, RECORDED_AT);
    assert_eq!(events[0].domain_id, "telemetry");
    assert_eq!(events[0].stream_id, SESSION_A);
    assert!(after_equal.is_empty());
}

#[test]
fn writer_commits_appends_in_submission_order() {
    let (_temp_dir, store) = event_store();
    let shared = Arc::new(store);
    let writer = EventWriter::spawn(shared.clone());

    let first = writer
        .append_durable(domain_envelope("execution.started"), false)
        .unwrap();
    let second = writer
        .append_durable(domain_envelope("execution.completed"), false)
        .unwrap();

    assert_eq!((first, second), (1, 2));
    assert_eq!(
        shared
            .read_all_events_after(0)
            .unwrap()
            .iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(writer.watermark().committed_seq(), 2);
}

#[test]
fn best_effort_appends_survive_writer_shutdown() {
    let (_temp_dir, store) = event_store();
    let shared = Arc::new(store);
    {
        let writer = EventWriter::spawn(shared.clone());
        writer
            .append_best_effort(domain_envelope("execution.started"), false)
            .unwrap();
        writer
            .append_best_effort(domain_envelope("execution.completed"), false)
            .unwrap();
        assert_eq!(writer.dropped_events(), 0);
    }
    assert_eq!(shared.read_all_events_after(0).unwrap().len(), 2);
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

    // Best-effort emits return before durability; the watermark is the
    // synchronization point for observing them.
    let committed = runtime
        .watermark()
        .wait_past(3, std::time::Duration::from_secs(5));
    assert!(committed >= 4);
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
    #![proptest_config(ProptestConfig::with_cases(48))]

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

    #[test]
    fn envelope_conversion_preserves_generated_metadata(
        spec in generated_event_spec(),
        src in generated_object_ref(),
        dst in generated_object_ref(),
        relation_type in generated_component(),
        occurred_at in generated_timestamp(),
        seq in 1u64..10_000,
    ) {
        let relation =
            meld_events::EventRelation::new(relation_type.clone(), src.clone(), dst.clone())
                .unwrap();
        let envelope = spec
            .envelope()
            .with_occurred_at(occurred_at.clone())
            .with_graph(vec![src.clone(), dst.clone()], vec![relation.clone()]);

        let record = EventRecord::from_envelope(envelope, seq);

        prop_assert_eq!(record.ts.as_str(), spec.ts.as_str());
        prop_assert_eq!(record.recorded_at.as_str(), spec.ts.as_str());
        prop_assert_eq!(record.record_id.as_ref(), spec.record_id.as_ref());
        prop_assert_eq!(record.session.as_str(), spec.session.as_str());
        prop_assert_eq!(record.seq, seq);
        prop_assert_eq!(record.domain_id.as_str(), spec.domain_id.as_str());
        prop_assert_eq!(record.stream_id.as_str(), spec.stream_id.as_str());
        prop_assert_eq!(record.event_type.as_str(), spec.event_type.as_str());
        prop_assert_eq!(record.occurred_at.as_deref(), Some(occurred_at.as_str()));
        prop_assert_eq!(record.content_hash.as_ref(), spec.content_hash.as_ref());
        prop_assert_eq!(record.objects.clone(), vec![src, dst]);
        prop_assert_eq!(record.relations.clone(), vec![relation]);
        prop_assert_eq!(record.data.clone(), json!({ "marker": spec.marker }));
    }

    #[test]
    fn store_assigned_sequences_match_generated_append_model(
        specs in prop::collection::vec(generated_event_spec(), 1..16),
        after_cursor in any::<u8>(),
    ) {
        let (_temp_dir, store) = event_store();
        let mut expected_all = Vec::new();
        let mut expected_by_session: BTreeMap<String, Vec<EventRecord>> = BTreeMap::new();

        for (index, spec) in specs.iter().enumerate() {
            let seq = store.append_envelope(spec.envelope()).unwrap();
            let expected_seq = index as u64 + 1;
            prop_assert_eq!(seq, expected_seq);

            let expected = EventRecord::from_envelope(spec.envelope(), expected_seq);
            expected_by_session
                .entry(spec.session.clone())
                .or_default()
                .push(expected.clone());
            expected_all.push(expected);
        }

        prop_assert_eq!(store.read_all_events_after(0).unwrap(), expected_all.clone());

        let after = u64::from(after_cursor) % (specs.len() as u64 + 2);
        let expected_after = expected_all
            .iter()
            .filter(|event| event.seq > after)
            .cloned()
            .collect::<Vec<_>>();
        prop_assert_eq!(store.read_all_events_after(after).unwrap(), expected_after);

        for (session, expected_events) in expected_by_session {
            let expected_session_after = expected_events
                .iter()
                .filter(|event| event.seq > after)
                .cloned()
                .collect::<Vec<_>>();
            prop_assert_eq!(store.read_events(&session).unwrap(), expected_events);
            prop_assert_eq!(
                store.read_events_after(&session, after).unwrap(),
                expected_session_after
            );
        }
    }

    #[test]
    fn idempotent_envelope_append_reuses_first_generated_record_id(
        mut specs in prop::collection::vec(generated_event_spec(), 1..16),
    ) {
        let (_temp_dir, store) = event_store();
        let mut expected_all = Vec::new();
        let mut first_seq_by_record_id: BTreeMap<String, u64> = BTreeMap::new();
        let mut next_seq = 1u64;

        for (index, spec) in specs.iter_mut().enumerate() {
            spec.record_id = if index % 3 == 0 {
                None
            } else {
                Some(format!("record-{}", index % 4))
            };
            let seq = store.append_envelope_idempotent(spec.envelope()).unwrap();

            match spec.record_id.clone() {
                Some(record_id) => {
                    if let Some(existing_seq) = first_seq_by_record_id.get(&record_id) {
                        prop_assert_eq!(seq, *existing_seq);
                    } else {
                        prop_assert_eq!(seq, next_seq);
                        first_seq_by_record_id.insert(record_id, seq);
                        expected_all.push(EventRecord::from_envelope(spec.envelope(), seq));
                        next_seq += 1;
                    }
                }
                None => {
                    prop_assert_eq!(seq, next_seq);
                    expected_all.push(EventRecord::from_envelope(spec.envelope(), seq));
                    next_seq += 1;
                }
            }
        }

        prop_assert_eq!(store.read_all_events_after(0).unwrap(), expected_all);
        prop_assert_eq!(
            store
                .append_envelope(domain_envelope("execution.after.idempotent"))
                .unwrap(),
            next_seq
        );
    }

    #[test]
    fn presequenced_append_advances_allocator_past_generated_sequence(
        seq in 1u64..10_000,
        session in generated_component(),
        event_type in generated_event_type(),
    ) {
        let (_temp_dir, store) = event_store();
        let record = runtime_event(seq, &session, &event_type);

        store.append_event(&record).unwrap();

        prop_assert_eq!(store.read_events(&session).unwrap(), vec![record]);
        prop_assert_eq!(
            store
                .append_envelope(domain_envelope("execution.after.manual"))
                .unwrap(),
            seq + 1
        );
    }

    #[test]
    fn legacy_records_normalize_generated_defaults(
        ts in generated_timestamp(),
        session in generated_component(),
        seq in 1u64..10_000,
        marker in any::<u16>(),
    ) {
        let temp_dir = tempfile::tempdir().unwrap();
        let db = sled::open(temp_dir.path().join("events")).unwrap();
        let legacy = legacy_record_json(LegacyRecordJson {
            ts: ts.clone(),
            recorded_at: String::new(),
            session: session.clone(),
            seq,
            domain_id: String::new(),
            stream_id: String::new(),
            event_type: "legacy.generated".to_string(),
            data: json!({ "marker": marker }),
        });
        let legacy_tree = db.open_tree("obs_events").unwrap();
        let key = EventStore::encode_event_key(&session, seq);
        legacy_tree
            .insert(key.as_bytes(), serde_json::to_vec(&legacy).unwrap())
            .unwrap();

        let store = EventStore::new(db).unwrap();
        let events = store.read_events(&session).unwrap();

        prop_assert_eq!(events.len(), 1);
        prop_assert_eq!(events[0].recorded_at.as_str(), ts.as_str());
        prop_assert_eq!(events[0].domain_id.as_str(), "telemetry");
        prop_assert_eq!(events[0].stream_id.as_str(), session.as_str());
        prop_assert_eq!(&events[0].data, &json!({ "marker": marker }));
        prop_assert!(store.read_events_after(&session, seq).unwrap().is_empty());
    }
}
