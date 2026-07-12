//! Behavior tests for the session timeline and flow window surfaces.
//!
//! Everything drives the public [`EventObservabilityPort`]; the fixtures
//! append envelopes with explicit timestamps so gap and span math is
//! deterministic.

#![cfg(feature = "test-support")]

use std::sync::Arc;

use meld_events::error::StorageError;
use meld_events::events::observability::{EventObservabilityPort, FlowWindow};
use meld_events::events::test_support::{
    EventCursorRegistry, EventCursorRegistryTestSupport as _, EventStore,
    EventStoreTestSupport as _, EventWriter, EventWriterTestSupport as _, LedgerObservability,
    LedgerObservabilityTestSupport as _,
};
use meld_events::{CoverageTruncation, EventEnvelope, EventRecord};
use serde_json::json;

struct Fixture {
    _dir: tempfile::TempDir,
    _writer: EventWriter,
    store: Arc<EventStore>,
    port: LedgerObservability,
}

fn fixture() -> Fixture {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let store = EventStore::shared(db.clone()).unwrap();
    let registry = EventCursorRegistry::open(&db).unwrap();
    let writer = EventWriter::spawn(Arc::clone(&store));
    let port = LedgerObservability::new(
        Arc::clone(&store),
        writer.watermark(),
        registry,
        writer.dropped_handle(),
    );
    Fixture {
        _dir: dir,
        _writer: writer,
        store,
        port,
    }
}

fn append(store: &EventStore, ts: &str, session: &str, domain: &str, event_type: &str) -> u64 {
    store
        .append_envelope(EventEnvelope::new_domain(
            ts.to_string(),
            session,
            domain,
            format!("{domain}-stream"),
            event_type,
            None,
            json!({}),
        ))
        .unwrap()
}

#[test]
fn timeline_orders_steps_and_computes_gaps() {
    let fx = fixture();
    append(
        &fx.store,
        "2026-07-08T00:00:00Z",
        "s-a",
        "alpha",
        "alpha.start",
    );
    // A foreign session interleaves to prove the timeline partitions by
    // session rather than reading the whole ledger.
    append(
        &fx.store,
        "2026-07-08T00:00:01Z",
        "s-other",
        "beta",
        "beta.noise",
    );
    append(
        &fx.store,
        "2026-07-08T00:00:02Z",
        "s-a",
        "beta",
        "beta.step",
    );
    append(
        &fx.store,
        "2026-07-08T00:00:02.500Z",
        "s-a",
        "alpha",
        "alpha.step",
    );
    append(&fx.store, "not-a-timestamp", "s-a", "alpha", "alpha.odd");
    append(
        &fx.store,
        "2026-07-08T00:00:05Z",
        "s-a",
        "alpha",
        "alpha.end",
    );

    let report = fx.port.session("s-a").unwrap();

    assert_eq!(report.session_id, "s-a");
    assert_eq!(report.events_returned, 5);
    assert_eq!(
        report.observed_started_at.as_deref(),
        Some("2026-07-08T00:00:00Z")
    );
    assert_eq!(
        report.observed_ended_at.as_deref(),
        Some("2026-07-08T00:00:05Z")
    );
    assert_eq!(report.coverage.retained_from, 1);
    assert_eq!(report.coverage.tip_seq, 6);
    assert_eq!(report.coverage.scanned_from_seq, Some(1));
    assert_eq!(report.coverage.scanned_through_seq, Some(6));

    let seqs: Vec<u64> = report.steps.iter().map(|step| step.seq).collect();
    assert_eq!(seqs, vec![1, 3, 4, 5, 6]);

    let gaps: Vec<Option<u64>> = report.steps.iter().map(|step| step.gap_ms).collect();
    // First step has no predecessor; the unparseable timestamp voids its own
    // gap and the following step's gap.
    assert_eq!(gaps, vec![None, Some(2000), Some(500), None, None]);

    assert_eq!(report.steps[1].domain_id, "beta");
    assert_eq!(report.steps[1].event_type, "beta.step");
    assert_eq!(report.steps[1].recorded_at, "2026-07-08T00:00:02Z");
}

#[test]
fn empty_session_yields_empty_timeline_without_error() {
    let fx = fixture();
    append(
        &fx.store,
        "2026-07-08T00:00:00Z",
        "s-other",
        "alpha",
        "alpha.noise",
    );

    let report = fx.port.session("s-missing").unwrap();

    assert_eq!(report.session_id, "s-missing");
    assert_eq!(report.events_returned, 0);
    assert!(report.steps.is_empty());
    assert_eq!(report.observed_started_at, None);
    assert_eq!(report.observed_ended_at, None);
}

#[test]
fn flow_counts_and_orders_domains_and_types() {
    let fx = fixture();
    append(&fx.store, "2026-07-08T00:00:00Z", "s", "alpha", "alpha.one");
    append(&fx.store, "2026-07-08T00:00:05Z", "s", "beta", "beta.tick");
    append(&fx.store, "2026-07-08T00:00:10Z", "s", "beta", "beta.tick");
    append(
        &fx.store,
        "2026-07-08T00:00:15Z",
        "s",
        "gamma",
        "gamma.tick",
    );
    append(&fx.store, "2026-07-08T00:00:20Z", "s", "beta", "beta.tick");
    append(
        &fx.store,
        "2026-07-08T00:00:30Z",
        "s",
        "gamma",
        "gamma.tock",
    );

    let report = fx.port.flow(FlowWindow { max_events: 100 }).unwrap();

    assert_eq!(report.window_events, 6);
    assert_eq!(report.span_seconds, Some(30));

    let domains: Vec<(&str, u64, u64)> = report
        .by_domain
        .iter()
        .map(|flow| (flow.domain_id.as_str(), flow.count, flow.last_seq))
        .collect();
    assert_eq!(
        domains,
        vec![("beta", 3, 5), ("gamma", 2, 6), ("alpha", 1, 1)]
    );

    let types: Vec<(&str, u64)> = report
        .by_type
        .iter()
        .map(|flow| (flow.event_type.as_str(), flow.count))
        .collect();
    // Descending by count, count ties ascending by type name.
    assert_eq!(
        types,
        vec![
            ("beta.tick", 3),
            ("alpha.one", 1),
            ("gamma.tick", 1),
            ("gamma.tock", 1)
        ]
    );
    assert!(report.silent_domains.is_empty());
}

#[test]
fn flow_window_covers_only_trailing_events() {
    let fx = fixture();
    append(&fx.store, "2026-07-08T00:00:00Z", "s", "alpha", "alpha.one");
    append(&fx.store, "2026-07-08T00:00:05Z", "s", "beta", "beta.tick");
    append(&fx.store, "2026-07-08T00:00:20Z", "s", "beta", "beta.tick");
    append(
        &fx.store,
        "2026-07-08T00:00:30Z",
        "s",
        "gamma",
        "gamma.tock",
    );

    let report = fx.port.flow(FlowWindow { max_events: 2 }).unwrap();

    assert_eq!(report.window_events, 2);
    assert_eq!(report.span_seconds, Some(10));

    let domains: Vec<(&str, u64, u64)> = report
        .by_domain
        .iter()
        .map(|flow| (flow.domain_id.as_str(), flow.count, flow.last_seq))
        .collect();
    // Count tie between beta and gamma resolves ascending by domain id.
    assert_eq!(domains, vec![("beta", 1, 3), ("gamma", 1, 4)]);

    // Alpha emitted before the window start and nothing inside it.
    let silent: Vec<(&str, u64, &str)> = report
        .silent_domains
        .iter()
        .map(|silent| {
            (
                silent.domain_id.as_str(),
                silent.last_seq,
                silent.last_recorded_at.as_str(),
            )
        })
        .collect();
    assert_eq!(silent, vec![("alpha", 1, "2026-07-08T00:00:00Z")]);
    assert_eq!(report.coverage.scanned_from_seq, Some(3));
    assert_eq!(report.coverage.scanned_through_seq, Some(4));
    assert_eq!(report.coverage.truncation, CoverageTruncation::Before);
    assert_eq!(report.silent_domain_coverage.scanned_from_seq, Some(1));
    assert_eq!(report.silent_domain_coverage.scanned_through_seq, Some(2));
    assert_eq!(
        report.silent_domain_coverage.truncation,
        CoverageTruncation::After
    );
}

#[test]
fn flow_window_counts_records_in_sparse_sequence_space() {
    let fx = fixture();
    for (seq, domain) in [(1, "early"), (100, "middle"), (10_000, "late")] {
        let envelope = EventEnvelope::new_domain(
            "2026-07-08T00:00:00Z".to_string(),
            "s",
            domain,
            format!("{domain}-stream"),
            format!("{domain}.tick"),
            None,
            json!({}),
        );
        fx.store
            .append_event(&EventRecord { seq, envelope })
            .unwrap();
    }

    let report = fx.port.flow(FlowWindow { max_events: 2 }).unwrap();

    assert_eq!(report.window_events, 2);
    assert_eq!(
        report
            .by_domain
            .iter()
            .map(|flow| (flow.domain_id.as_str(), flow.last_seq))
            .collect::<Vec<_>>(),
        vec![("late", 10_000), ("middle", 100)]
    );
}

#[test]
fn silent_domain_reports_its_last_event_before_the_window() {
    let fx = fixture();
    append(
        &fx.store,
        "2026-07-08T00:00:00Z",
        "s",
        "early",
        "early.tick",
    );
    append(
        &fx.store,
        "2026-07-08T00:00:01Z",
        "s",
        "early",
        "early.tick",
    );
    for i in 0..5 {
        append(
            &fx.store,
            &format!("2026-07-08T00:01:0{i}Z"),
            "s",
            "busy",
            "busy.tick",
        );
    }

    let report = fx.port.flow(FlowWindow { max_events: 5 }).unwrap();

    assert_eq!(report.window_events, 5);
    assert_eq!(report.by_domain.len(), 1);
    assert_eq!(report.by_domain[0].domain_id, "busy");

    assert_eq!(report.silent_domains.len(), 1);
    assert_eq!(report.silent_domains[0].domain_id, "early");
    // The census reports the domain's newest event before the window.
    assert_eq!(report.silent_domains[0].last_seq, 2);
    assert_eq!(
        report.silent_domains[0].last_recorded_at,
        "2026-07-08T00:00:01Z"
    );
}

#[test]
fn flow_window_clamps_to_the_retention_boundary() {
    let fx = fixture();
    for i in 0..10 {
        append(
            &fx.store,
            &format!("2026-07-08T00:00:0{i}Z"),
            "s",
            "alpha",
            "alpha.tick",
        );
    }
    fx.store.set_retained_lower_boundary(6).unwrap();

    // The window asks for far more than retained history; the read must
    // degrade to everything retained instead of tripping a retention gap.
    let report = fx.port.flow(FlowWindow { max_events: 100 }).unwrap();

    assert_eq!(report.window_events, 5);
    assert_eq!(report.by_domain.len(), 1);
    assert_eq!(report.by_domain[0].last_seq, 10);
    assert!(report.silent_domains.is_empty());
    assert_eq!(report.coverage.scanned_from_seq, Some(6));
    assert_eq!(report.coverage.scanned_through_seq, Some(10));
    assert_eq!(report.coverage.truncation, CoverageTruncation::Before);
}

#[test]
fn empty_ledger_flow_is_empty_without_error() {
    let fx = fixture();

    let report = fx.port.flow(FlowWindow { max_events: 100 }).unwrap();

    assert_eq!(report.window_events, 0);
    assert_eq!(report.span_seconds, None);
    assert!(report.by_domain.is_empty());
    assert!(report.by_type.is_empty());
    assert!(report.silent_domains.is_empty());
}

#[test]
fn flow_rejects_unbounded_or_empty_windows_before_reading() {
    let fx = fixture();

    for (max_events, expected) in [
        (
            0,
            "event flow window must be in 1..=100000, got 0".to_string(),
        ),
        (
            100_001,
            "event flow window must be in 1..=100000, got 100001".to_string(),
        ),
        (
            usize::MAX,
            format!(
                "event flow window must be in 1..=100000, got {}",
                usize::MAX
            ),
        ),
    ] {
        let error = fx.port.flow(FlowWindow { max_events }).unwrap_err();
        match error {
            StorageError::InvalidPath(message) => assert_eq!(message, expected),
            other => panic!("expected invalid request compatibility error, got {other}"),
        }
    }

    let report = fx
        .port
        .flow(FlowWindow {
            max_events: 100_000,
        })
        .unwrap();
    assert_eq!(report.window_events, 0);
}
