use meld::telemetry::ProgressRuntime;
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
    assert!(all_events.windows(2).all(|pair| pair[1].seq == pair[0].seq + 1));

    let session_one_events = runtime.store().read_events(&session_one).unwrap();
    let session_two_events = runtime.store().read_events(&session_two).unwrap();

    assert_eq!(
        session_one_events.iter().map(|event| event.seq).collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(
        session_two_events.iter().map(|event| event.seq).collect::<Vec<_>>(),
        vec![2, 4]
    );
}

#[test]
fn legacy_events_remain_readable() {
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
}
