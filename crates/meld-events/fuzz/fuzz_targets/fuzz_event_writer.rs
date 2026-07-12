#![no_main]

use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use meld_events::events::test_support::{
    EventStore, EventStoreTestSupport as _, EventWriter, EventWriterTestSupport as _,
};
use meld_events::EventEnvelope;
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
        format!("fuzz.writer.{}", byte % 8),
        None,
        json!({ "byte": byte, "index": index }),
    )
}

fuzz_target!(|data: &[u8]| {
    let store = event_store();
    let writer = EventWriter::spawn(Arc::clone(&store));
    let mut durable = 0usize;
    let mut expected_min = 0usize;

    for (index, byte) in data.iter().copied().take(16).enumerate() {
        if byte % 2 == 0 {
            let seq = writer
                .append_durable(envelope(index, byte), byte % 4 == 0)
                .expect("durable append");
            assert!(seq >= 1);
            durable += 1;
        } else if writer
            .append_best_effort(envelope(index, byte), byte % 4 == 1)
            .is_ok()
        {
            expected_min += 1;
        }
    }

    let watermark = writer.watermark().committed_seq();
    assert!(watermark as usize >= durable);
    drop(writer);
    let persisted = store.read_all_events_after(0).expect("read events").len();
    assert!(persisted >= durable + expected_min);
});
