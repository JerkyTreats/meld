#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_events::events::store::EventStore;
use meld_events::EventEnvelope;
use serde_json::json;

fn event_store() -> EventStore {
    let db = sled::Config::new()
        .temporary(true)
        .open()
        .expect("temporary sled db");
    EventStore::new(db).expect("event store")
}

fn envelope(index: usize, byte: u8) -> EventEnvelope {
    let envelope = EventEnvelope::new_domain(
        "2026-04-26T16:00:00Z".to_string(),
        format!("session-{}", byte % 4),
        "fuzz",
        format!("stream-{}", byte % 3),
        format!("fuzz.event.{}", byte % 8),
        Some(format!("sha256:{byte:02x}")),
        json!({ "byte": byte, "index": index }),
    );
    if byte & 1 == 1 {
        envelope.with_record_id(format!("record-{}", byte % 4))
    } else {
        envelope
    }
}

fuzz_target!(|data: &[u8]| {
    let store = event_store();

    for (index, byte) in data.iter().copied().take(16).enumerate() {
        let envelope = envelope(index, byte);
        if byte & 2 == 2 {
            let _ = store.append_envelope_idempotent(envelope);
        } else {
            let _ = store.append_envelope(envelope);
        }
    }

    let all = store.read_all_events_after(0).expect("read all events");
    let mut previous = 0u64;
    for event in &all {
        assert!(event.seq > previous);
        assert!(!event.recorded_at.is_empty());
        previous = event.seq;
    }

    for session_index in 0..4 {
        let session = format!("session-{session_index}");
        let events = store.read_events(&session).expect("read session events");
        for window in events.windows(2) {
            assert!(window[0].seq <= window[1].seq);
        }
    }
});
