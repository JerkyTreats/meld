#![no_main]

use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use meld_events::events::store::EventStore;
use meld_events::{EventBus, EventEnvelope, EventIngestor};
use serde_json::json;

fn event_store() -> Arc<EventStore> {
    let db = sled::Config::new()
        .temporary(true)
        .open()
        .expect("temporary sled db");
    Arc::new(EventStore::new(db).expect("event store"))
}

fn envelope(index: usize, byte: u8) -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-04-26T16:00:00Z".to_string(),
        format!("session-{}", byte % 4),
        "fuzz",
        format!("stream-{}", byte % 3),
        format!("fuzz.ingestor.{}", byte % 8),
        None,
        json!({ "byte": byte, "index": index }),
    )
}

fuzz_target!(|data: &[u8]| {
    let capacity = data
        .first()
        .map(|byte| usize::from(byte % 8) + 1)
        .unwrap_or(1);
    let store = event_store();
    let (bus, rx) = EventBus::new_pair_with_capacity(capacity);
    let mut ingestor = EventIngestor::new(store.clone(), rx);
    let mut accepted = 0usize;

    for (index, byte) in data.iter().copied().skip(1).take(16).enumerate() {
        if bus.emit_envelope(envelope(index, byte)).is_ok() {
            accepted += 1;
        }
    }

    let ingested = ingestor.ingest_pending().expect("ingest pending");
    assert_eq!(ingested, accepted);
    assert_eq!(
        store.read_all_events_after(0).expect("read events").len(),
        accepted
    );
});
