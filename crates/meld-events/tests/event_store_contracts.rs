use meld_events::events::store::EventStore;
use meld_events::{DomainObjectRef, EventEnvelope, EventRelation};
use serde_json::json;

fn open_store() -> (EventStore, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    (EventStore::new(db).unwrap(), temp_dir)
}

#[test]
fn idempotent_append_reuses_record_sequence() {
    let (store, _temp_dir) = open_store();

    let envelope = EventEnvelope::new(
        "2026-04-26T16:00:00Z".to_string(),
        "session-a".to_string(),
        "execution.task.completed",
        json!({"task_run": "run-a"}),
    )
    .with_record_id("record-a");

    let first_seq = store.append_envelope_idempotent(envelope.clone()).unwrap();
    let second_seq = store.append_envelope_idempotent(envelope).unwrap();

    assert_eq!(first_seq, second_seq);
    assert_eq!(store.read_events("session-a").unwrap().len(), 1);
}

#[test]
fn envelope_preserves_graph_objects_and_relations() {
    let (store, _temp_dir) = open_store();
    let task = DomainObjectRef::new("execution", "task_run", "run-a").unwrap();
    let artifact = DomainObjectRef::new("execution", "artifact", "artifact-a").unwrap();
    let relation = EventRelation::new("produced", task.clone(), artifact.clone()).unwrap();

    store
        .append_envelope(
            EventEnvelope::new_domain(
                "2026-04-26T16:01:00Z".to_string(),
                "session-a",
                "execution",
                "workflow-a",
                "execution.artifact.available",
                Some("sha256:abc".to_string()),
                json!({"artifact": "artifact-a"}),
            )
            .with_graph(vec![task, artifact], vec![relation]),
        )
        .unwrap();

    let events = store.read_events("session-a").unwrap();
    assert_eq!(events[0].domain_id, "execution");
    assert_eq!(events[0].stream_id, "workflow-a");
    assert_eq!(events[0].objects.len(), 2);
    assert_eq!(events[0].relations[0].relation_type, "produced");
}
