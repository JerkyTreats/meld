//! Wire-shape contracts for observability reports.
//!
//! Adapters depend on these serialized field names; a failing test here means
//! a breaking change for every CLI, TUI, or dashboard consumer and must be
//! called out under the compatibility policy.

use meld_events::error::StorageError;
use meld_events::events::observability::{
    ConsumerLagReport, CoverageTruncation, DomainAppendRate, DomainFlow, EventFlowReport,
    EventHealthReport, EventPageRequest, EventReadCoverage, EventTraceReport, FlowWindow,
    LegacyEventPage, SessionStep, SessionTimelineReport, SilentDomain, TraceHop, TraceLink,
    TraceSubject, TypeFlow,
};
use meld_events::{DomainObjectRef, EventEnvelope, EventRecord};
use serde_json::json;

#[test]
fn health_report_shape_is_pinned() {
    let report = EventHealthReport {
        tip_seq: 12,
        committed_watermark: 10,
        retained_from: 1,
        dropped_events: 2,
        consumers: vec![ConsumerLagReport {
            name: "world_state.graph.reducer".to_string(),
            reported_seq: 9,
            lag: 1,
        }],
        append_rates: vec![DomainAppendRate {
            domain_id: "execution".to_string(),
            events_in_window: 5,
            window_seconds: Some(60),
        }],
    };

    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        json!({
            "tip_seq": 12,
            "committed_watermark": 10,
            "retained_from": 1,
            "dropped_events": 2,
            "consumers": [
                { "name": "world_state.graph.reducer", "reported_seq": 9, "lag": 1 }
            ],
            "append_rates": [
                { "domain_id": "execution", "events_in_window": 5, "window_seconds": 60 }
            ]
        })
    );
}

#[test]
fn flow_report_shape_is_pinned() {
    let report = EventFlowReport {
        window_events: 7,
        span_seconds: Some(30),
        by_domain: vec![DomainFlow {
            domain_id: "workspace_fs".to_string(),
            count: 4,
            last_seq: 40,
        }],
        by_type: vec![TypeFlow {
            event_type: "workspace_fs.node_observed".to_string(),
            count: 4,
        }],
        silent_domains: vec![SilentDomain {
            domain_id: "context".to_string(),
            last_seq: 12,
            last_recorded_at: "2026-07-08T00:00:00Z".to_string(),
        }],
    };

    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        json!({
            "window_events": 7,
            "span_seconds": 30,
            "by_domain": [
                { "domain_id": "workspace_fs", "count": 4, "last_seq": 40 }
            ],
            "by_type": [
                { "event_type": "workspace_fs.node_observed", "count": 4 }
            ],
            "silent_domains": [
                {
                    "domain_id": "context",
                    "last_seq": 12,
                    "last_recorded_at": "2026-07-08T00:00:00Z"
                }
            ]
        })
    );
}

#[test]
fn trace_report_shape_is_pinned() {
    let subject =
        TraceSubject::Object(DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap());
    let report = EventTraceReport {
        subject: subject.clone(),
        coverage: EventReadCoverage {
            retained_from: 1,
            tip_seq: 5,
            scanned_from_seq: Some(1),
            scanned_through_seq: Some(5),
            truncation: CoverageTruncation::None,
        },
        hops: vec![
            TraceHop {
                seq: 3,
                recorded_at: "2026-07-08T00:00:00Z".to_string(),
                domain_id: "workspace_fs".to_string(),
                stream_id: "source-a".to_string(),
                event_type: "workspace_fs.node_observed".to_string(),
                session_id: "session-a".to_string(),
                link: TraceLink::ObjectRef,
            },
            TraceHop {
                seq: 5,
                recorded_at: "2026-07-08T00:00:01Z".to_string(),
                domain_id: "world_state".to_string(),
                stream_id: "graph".to_string(),
                event_type: "world_state.anchor_selected".to_string(),
                session_id: "session-a".to_string(),
                link: TraceLink::SourceFact {
                    fact_id: "spine::3".to_string(),
                },
            },
        ],
    };

    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        json!({
            "subject": {
                "object": {
                    "domain_id": "workspace_fs",
                    "object_kind": "node",
                    "object_id": "node-a"
                }
            },
            "coverage": {
                "retained_from": 1,
                "tip_seq": 5,
                "scanned_from_seq": 1,
                "scanned_through_seq": 5,
                "truncation": "none"
            },
            "hops": [
                {
                    "seq": 3,
                    "recorded_at": "2026-07-08T00:00:00Z",
                    "domain_id": "workspace_fs",
                    "stream_id": "source-a",
                    "event_type": "workspace_fs.node_observed",
                    "session_id": "session-a",
                    "link": "object_ref"
                },
                {
                    "seq": 5,
                    "recorded_at": "2026-07-08T00:00:01Z",
                    "domain_id": "world_state",
                    "stream_id": "graph",
                    "event_type": "world_state.anchor_selected",
                    "session_id": "session-a",
                    "link": { "source_fact": { "fact_id": "spine::3" } }
                }
            ]
        })
    );
}

#[test]
fn coverage_truncation_variants_are_pinned() {
    assert_eq!(
        serde_json::to_value(CoverageTruncation::None).unwrap(),
        json!("none")
    );
    assert_eq!(
        serde_json::to_value(CoverageTruncation::Before).unwrap(),
        json!("before")
    );
    assert_eq!(
        serde_json::to_value(CoverageTruncation::After).unwrap(),
        json!("after")
    );
    assert_eq!(
        serde_json::to_value(CoverageTruncation::Both).unwrap(),
        json!("both")
    );
}

#[test]
fn trace_link_variants_are_pinned() {
    assert_eq!(
        serde_json::to_value(TraceLink::Subject).unwrap(),
        json!("subject")
    );
    assert_eq!(
        serde_json::to_value(TraceLink::Stream).unwrap(),
        json!("stream")
    );
    assert_eq!(
        serde_json::to_value(TraceLink::ObjectRef).unwrap(),
        json!("object_ref")
    );
    assert_eq!(
        serde_json::to_value(TraceLink::Relation {
            relation_type: "produced".to_string(),
        })
        .unwrap(),
        json!({ "relation": { "relation_type": "produced" } })
    );
    assert_eq!(
        serde_json::to_value(TraceLink::SourceFact {
            fact_id: "spine::3".to_string(),
        })
        .unwrap(),
        json!({ "source_fact": { "fact_id": "spine::3" } })
    );
}

#[test]
fn trace_subject_variants_are_pinned() {
    assert_eq!(
        serde_json::to_value(TraceSubject::Stream {
            domain_id: "execution".to_string(),
            stream_id: "run-a".to_string(),
        })
        .unwrap(),
        json!({ "stream": { "domain_id": "execution", "stream_id": "run-a" } })
    );
    assert_eq!(
        serde_json::to_value(TraceSubject::Record { seq: 7 }).unwrap(),
        json!({ "record": { "seq": 7 } })
    );
}

#[test]
fn session_timeline_shape_is_pinned() {
    let report = SessionTimelineReport {
        session_id: "session-a".to_string(),
        observed_started_at: Some("2026-07-08T00:00:00Z".to_string()),
        observed_ended_at: Some("2026-07-08T00:00:02Z".to_string()),
        events_returned: 2,
        coverage: EventReadCoverage {
            retained_from: 1,
            tip_seq: 2,
            scanned_from_seq: Some(1),
            scanned_through_seq: Some(2),
            truncation: CoverageTruncation::None,
        },
        steps: vec![
            SessionStep {
                seq: 1,
                recorded_at: "2026-07-08T00:00:00Z".to_string(),
                domain_id: "telemetry".to_string(),
                event_type: "session_started".to_string(),
                gap_ms: None,
            },
            SessionStep {
                seq: 2,
                recorded_at: "2026-07-08T00:00:02Z".to_string(),
                domain_id: "workspace_fs".to_string(),
                event_type: "workspace_fs.scan_completed".to_string(),
                gap_ms: Some(2000),
            },
        ],
    };

    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        json!({
            "session_id": "session-a",
            "observed_started_at": "2026-07-08T00:00:00Z",
            "observed_ended_at": "2026-07-08T00:00:02Z",
            "events_returned": 2,
            "coverage": {
                "retained_from": 1,
                "tip_seq": 2,
                "scanned_from_seq": 1,
                "scanned_through_seq": 2,
                "truncation": "none"
            },
            "steps": [
                {
                    "seq": 1,
                    "recorded_at": "2026-07-08T00:00:00Z",
                    "domain_id": "telemetry",
                    "event_type": "session_started",
                    "gap_ms": null
                },
                {
                    "seq": 2,
                    "recorded_at": "2026-07-08T00:00:02Z",
                    "domain_id": "workspace_fs",
                    "event_type": "workspace_fs.scan_completed",
                    "gap_ms": 2000
                }
            ]
        })
    );
}

#[test]
fn event_page_shape_is_pinned_and_carries_records_intact() {
    let record = EventRecord::from_envelope(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:00Z".to_string(),
            "session-a",
            "execution",
            "run-a",
            "execution.task.started",
            None,
            json!({ "task": "t" }),
        ),
        4,
    );
    let page = LegacyEventPage {
        records: vec![record.clone()],
        next_after_seq: 4,
    };

    let value = serde_json::to_value(&page).unwrap();
    assert_eq!(value["next_after_seq"], json!(4));
    // The canonical record travels intact inside the page.
    assert_eq!(value["records"][0], serde_json::to_value(&record).unwrap());

    let request = EventPageRequest {
        after_seq: 3,
        limit: 16,
        timeout_ms: 250,
    };
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        json!({ "after_seq": 3, "limit": 16, "timeout_ms": 250 })
    );
}

#[test]
fn flow_window_shape_is_pinned() {
    assert_eq!(
        serde_json::to_value(FlowWindow { max_events: 500 }).unwrap(),
        json!({ "max_events": 500 })
    );
}

#[test]
fn page_stream_blocks_and_pages_through_the_port() {
    use meld_events::events::observability::EventObservabilityPort;
    use meld_events::events::registry::EventCursorRegistry;
    use meld_events::{EventWriter, LedgerObservability};
    use std::sync::Arc;

    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let store = meld_events::events::store::EventStore::shared(db.clone()).unwrap();
    let registry = EventCursorRegistry::open(&db).unwrap();
    let writer = EventWriter::spawn(Arc::clone(&store));
    let port = LedgerObservability::new(
        Arc::clone(&store),
        writer.watermark(),
        registry,
        writer.dropped_handle(),
    );

    for i in 0..3 {
        writer
            .append_durable(
                EventEnvelope::with_now("session-a", "session.tick", json!({ "i": i })),
                false,
            )
            .unwrap();
    }

    let first = port
        .next_page(EventPageRequest {
            after_seq: 0,
            limit: 2,
            timeout_ms: 10,
        })
        .unwrap();
    assert_eq!(first.records.len(), 2);
    assert_eq!(first.next_after_seq, 2);

    let second = port
        .next_page(EventPageRequest {
            after_seq: first.next_after_seq,
            limit: 2,
            timeout_ms: 10,
        })
        .unwrap();
    assert_eq!(second.records.len(), 1);
    assert_eq!(second.next_after_seq, 3);

    let empty = port
        .next_page(EventPageRequest {
            after_seq: 3,
            limit: 2,
            timeout_ms: 10,
        })
        .unwrap();
    assert!(empty.records.is_empty());
    assert_eq!(empty.next_after_seq, 3);

    for (limit, expected) in [
        (0, "event page limit must be in 1..=1024, got 0".to_string()),
        (
            1_025,
            "event page limit must be in 1..=1024, got 1025".to_string(),
        ),
        (
            usize::MAX,
            format!("event page limit must be in 1..=1024, got {}", usize::MAX),
        ),
    ] {
        let error = port
            .next_page(EventPageRequest {
                after_seq: 3,
                limit,
                timeout_ms: 0,
            })
            .unwrap_err();
        match error {
            StorageError::InvalidPath(message) => assert_eq!(message, expected),
            other => panic!("expected invalid request compatibility error, got {other}"),
        }
    }

    for (timeout_ms, expected) in [
        (
            30_001,
            "event page timeout_ms must be in 0..=30000, got 30001".to_string(),
        ),
        (
            u64::MAX,
            format!(
                "event page timeout_ms must be in 0..=30000, got {}",
                u64::MAX
            ),
        ),
    ] {
        let error = port
            .next_page(EventPageRequest {
                after_seq: 3,
                limit: 1,
                timeout_ms,
            })
            .unwrap_err();
        match error {
            StorageError::InvalidPath(message) => assert_eq!(message, expected),
            other => panic!("expected invalid request compatibility error, got {other}"),
        }
    }

    // Both inclusive maxima are accepted. The existing records make this
    // immediate even though the timeout is the maximum permitted value.
    let maximum = port
        .next_page(EventPageRequest {
            after_seq: 0,
            limit: 1_024,
            timeout_ms: 30_000,
        })
        .unwrap();
    assert_eq!(maximum.records.len(), 3);

    // Zero is a valid non-blocking timeout.
    let non_blocking = port
        .next_page(EventPageRequest {
            after_seq: 3,
            limit: 1,
            timeout_ms: 0,
        })
        .unwrap();
    assert!(non_blocking.records.is_empty());
}

#[test]
fn oversized_json_numbers_do_not_reach_observability_allocations() {
    let flow_overflow = r#"{"max_events":18446744073709551616}"#;
    assert!(serde_json::from_str::<FlowWindow>(flow_overflow).is_err());

    let page_overflow = r#"{"after_seq":0,"limit":1,"timeout_ms":18446744073709551616}"#;
    assert!(serde_json::from_str::<EventPageRequest>(page_overflow).is_err());
}
