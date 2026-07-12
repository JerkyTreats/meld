//! Behavior tests for the health surface through the public port.
//!
//! The backpressure drop path is exercised by the writer's own tests; here
//! the counter is asserted to surface as zero on a healthy ledger so the
//! wiring is proven without forcing drops.

#![cfg(feature = "test-support")]

use std::sync::Arc;

use meld_events::events::observability::EventObservabilityPort;
use meld_events::events::test_support::{
    EventCursorRegistry, EventCursorRegistryTestSupport as _, EventStore,
    EventStoreTestSupport as _, EventWriter, EventWriterTestSupport as _, LedgerObservability,
    LedgerObservabilityTestSupport as _,
};
use meld_events::{CoverageTruncation, EventEnvelope};
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
    assert_eq!(report.append_rate_coverage.tip_seq, 0);
    assert_eq!(report.append_rate_coverage.scanned_from_seq, None);
    assert_eq!(
        report.append_rate_coverage.truncation,
        CoverageTruncation::None
    );
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
    assert_eq!(report.append_rate_coverage.scanned_from_seq, Some(1));
    assert_eq!(report.append_rate_coverage.scanned_through_seq, Some(7));
    assert_eq!(
        report.append_rate_coverage.truncation,
        CoverageTruncation::None
    );
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
fn health_snapshot_clamps_a_concurrently_advanced_watermark_to_its_frozen_tip() {
    let fixture = open_fixture();
    for i in 0..2 {
        fixture
            .store
            .append_envelope(EventEnvelope::with_now(
                "session-a",
                "observed.tick",
                json!({ "i": i }),
            ))
            .unwrap();
    }
    fixture.registry.report("consumer", 1).unwrap();

    // A watermark sampled after the report's tip can already describe a
    // later commit. Use an independently advanced writer to deterministically
    // reproduce that interleaving without timing-dependent sleeps.
    let later_db = sled::Config::new().temporary(true).open().unwrap();
    let later_store = EventStore::shared(later_db).unwrap();
    let later_writer = EventWriter::spawn(Arc::clone(&later_store));
    for i in 0..3 {
        later_writer
            .append_durable(
                EventEnvelope::with_now("later", "later.tick", json!({ "i": i })),
                false,
            )
            .unwrap();
    }
    let port = LedgerObservability::new(
        Arc::clone(&fixture.store),
        later_writer.watermark(),
        fixture.registry.clone(),
        later_writer.dropped_handle(),
    );

    let report = port.health().unwrap();
    assert_eq!(report.tip_seq, 2);
    assert_eq!(report.committed_watermark, 2);
    assert_eq!(report.consumers[0].reported_seq, 1);
    assert_eq!(report.consumers[0].lag, 1);
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
    assert_eq!(report.append_rate_coverage.scanned_from_seq, Some(3));
    assert_eq!(report.append_rate_coverage.scanned_through_seq, Some(6));
    assert_eq!(
        report.append_rate_coverage.truncation,
        CoverageTruncation::Before
    );
}

#[test]
fn compatibility_observability_reuses_persisted_identity() {
    let fixture = open_fixture();
    let first = fixture.port.health().unwrap().ledger_id;
    let reopened = LedgerObservability::try_new(
        Arc::clone(&fixture.store),
        fixture.writer.watermark(),
        fixture.registry.clone(),
        fixture.writer.dropped_handle(),
    )
    .unwrap();

    assert_eq!(reopened.health().unwrap().ledger_id, first);
}

#[test]
fn compatibility_observability_rejects_corrupt_persisted_identity() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    db.open_tree("obs_spine_meta")
        .unwrap()
        .insert("ledger_identity", &[1_u8, 2, 3])
        .unwrap();
    db.flush().unwrap();
    let store = EventStore::shared(db.clone()).unwrap();
    let registry = EventCursorRegistry::open(&db).unwrap();
    let writer = EventWriter::spawn(Arc::clone(&store));

    let error =
        LedgerObservability::try_new(store, writer.watermark(), registry, writer.dropped_handle())
            .err()
            .expect("corrupt identity must fail");
    assert!(matches!(
        error,
        meld_events::error::StorageError::InvalidPath(_)
    ));
}

#[test]
fn authority_health_validates_identity_before_reporting() {
    use meld_events::{EventAuthority, EventAuthorityOpenOptions, LedgerIdentity};

    let db = sled::Config::new().temporary(true).open().unwrap();
    let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
    let observability = authority.observability_capability();
    let expected = authority.ledger_identity();

    let report = observability.health(expected).unwrap();
    assert_eq!(report.ledger_id, expected);

    let actual = LedgerIdentity::new();
    let error = observability.health(actual).unwrap_err();
    assert!(matches!(
        error,
        meld_events::error::EventAuthorityError::IdentityMismatch {
            expected: found_expected,
            actual: found_actual,
        } if found_expected == expected && found_actual == actual
    ));
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

#[test]
fn append_rate_coverage_reports_the_fixed_record_ceiling() {
    let fixture = open_fixture();
    append_domain_events(&fixture, "execution", 513);

    let report = fixture.port.health().unwrap();

    assert_eq!(report.append_rates[0].events_in_window, 512);
    assert_eq!(report.append_rate_coverage.scanned_from_seq, Some(2));
    assert_eq!(report.append_rate_coverage.scanned_through_seq, Some(513));
    assert_eq!(
        report.append_rate_coverage.truncation,
        CoverageTruncation::Before
    );
}
