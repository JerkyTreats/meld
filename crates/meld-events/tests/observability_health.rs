//! Behavior tests for the health surface through the public port.
//!
//! The backpressure drop path is exercised by the writer's own tests; here
//! the counter is asserted to surface as zero on a healthy ledger so the
//! wiring is proven without forcing drops.

use std::sync::Arc;

use meld_events::events::observability::EventObservabilityPort;
use meld_events::events::registry::EventCursorRegistry;
use meld_events::events::store::EventStore;
use meld_events::{EventEnvelope, EventWriter, LedgerObservability};
use serde_json::json;

struct Fixture {
    _dir: tempfile::TempDir,
    store: Arc<EventStore>,
    registry: EventCursorRegistry,
    writer: EventWriter,
    port: LedgerObservability,
}

fn open_fixture() -> Fixture {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let store = EventStore::shared(db.clone()).unwrap();
    let registry = EventCursorRegistry::open(&db).unwrap();
    let writer = EventWriter::spawn(Arc::clone(&store));
    let port = LedgerObservability::new(
        Arc::clone(&store),
        writer.watermark(),
        registry.clone(),
        writer.dropped_handle(),
    );
    Fixture {
        _dir: dir,
        store,
        registry,
        writer,
        port,
    }
}

fn append_domain_events(fixture: &Fixture, domain_id: &str, count: usize) {
    for i in 0..count {
        fixture
            .writer
            .append_durable(
                EventEnvelope::with_now_domain(
                    "session-a",
                    domain_id,
                    "stream-a",
                    format!("{domain_id}.tick"),
                    None,
                    json!({ "i": i }),
                ),
                false,
            )
            .unwrap();
    }
}

#[test]
fn empty_ledger_reports_zeros_and_empty_collections() {
    let fixture = open_fixture();

    let report = fixture.port.health().unwrap();

    assert_eq!(report.tip_seq, 0);
    assert_eq!(report.committed_watermark, 0);
    // One is the store's full-history boundary, not a retained sequence.
    assert_eq!(report.retained_from, 1);
    assert_eq!(report.dropped_events, 0);
    assert!(report.consumers.is_empty());
    assert!(report.append_rates.is_empty());
}

#[test]
fn populated_ledger_reports_tip_watermark_and_sorted_rates() {
    let fixture = open_fixture();
    append_domain_events(&fixture, "workspace_fs", 2);
    append_domain_events(&fixture, "execution", 3);
    append_domain_events(&fixture, "context", 2);

    let report = fixture.port.health().unwrap();

    assert_eq!(report.tip_seq, 7);
    assert_eq!(report.committed_watermark, 7);
    assert_eq!(report.retained_from, 1);
    assert_eq!(report.dropped_events, 0);

    let rates: Vec<(&str, u64)> = report
        .append_rates
        .iter()
        .map(|rate| (rate.domain_id.as_str(), rate.events_in_window))
        .collect();
    // Descending by count, count ties broken ascending by domain id.
    assert_eq!(
        rates,
        vec![("execution", 3), ("context", 2), ("workspace_fs", 2)]
    );
    for rate in &report.append_rates {
        assert!(rate.window_seconds.is_some());
    }
}

#[test]
fn consumer_lag_is_watermark_minus_reported_cursor() {
    let fixture = open_fixture();
    append_domain_events(&fixture, "execution", 5);
    fixture.registry.report("caught_up", 5).unwrap();
    fixture.registry.report("behind", 2).unwrap();
    fixture.registry.report("ahead", 9).unwrap();

    let report = fixture.port.health().unwrap();

    assert_eq!(report.committed_watermark, 5);
    let consumers: Vec<(&str, u64, u64)> = report
        .consumers
        .iter()
        .map(|consumer| (consumer.name.as_str(), consumer.reported_seq, consumer.lag))
        .collect();
    // Snapshot order is name order; a cursor past the watermark saturates
    // to zero lag instead of wrapping.
    assert_eq!(
        consumers,
        vec![("ahead", 9, 0), ("behind", 2, 3), ("caught_up", 5, 0)]
    );
}

#[test]
fn retained_from_reflects_raised_boundary_and_window_shrinks_past_it() {
    let fixture = open_fixture();
    append_domain_events(&fixture, "execution", 6);
    fixture.store.set_retained_lower_boundary(3).unwrap();

    let report = fixture.port.health().unwrap();

    assert_eq!(report.retained_from, 3);
    assert_eq!(report.tip_seq, 6);
    // The rate window starts at retained history, so health stays
    // computable after compaction: sequences 3..=6 remain.
    assert_eq!(report.append_rates.len(), 1);
    assert_eq!(report.append_rates[0].domain_id, "execution");
    assert_eq!(report.append_rates[0].events_in_window, 4);
}

#[test]
fn unparseable_window_timestamps_omit_window_seconds() {
    let fixture = open_fixture();
    fixture
        .writer
        .append_durable(
            EventEnvelope::new_domain(
                "not-a-timestamp".to_string(),
                "session-a",
                "execution",
                "stream-a",
                "execution.tick",
                None,
                json!({}),
            ),
            false,
        )
        .unwrap();
    append_domain_events(&fixture, "execution", 1);

    let report = fixture.port.health().unwrap();

    assert_eq!(report.append_rates.len(), 1);
    assert_eq!(report.append_rates[0].events_in_window, 2);
    assert_eq!(report.append_rates[0].window_seconds, None);
}
