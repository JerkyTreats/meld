//! Behavior tests for the trace surface: structural causal walks over
//! object references, relation endpoints, and identity-bearing source-record
//! provenance, through the public authority observability capability.

#![cfg(feature = "test-support")]

use std::sync::Arc;

use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_events::{
    AppendMode, CoverageTruncation, DomainObjectRef, EventAppendCapability, EventAuthority,
    EventAuthorityOpenOptions, EventEnvelope, EventObservabilityCapability, EventRecordRef,
    EventRelation, LedgerIdentity, TraceLink, TraceSubject,
};
use serde_json::json;

/// Authority fixture plus a raw test-only handle for retention mutation.
struct Fixture {
    _dir: tempfile::TempDir,
    store: Arc<EventStore>,
    ledger_id: LedgerIdentity,
    append: EventAppendCapability,
    observability: EventObservabilityCapability,
}

fn fixture() -> Fixture {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let store = EventStore::shared(db.clone()).unwrap();
    let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
    Fixture {
        _dir: dir,
        store,
        ledger_id: authority.ledger_identity(),
        append: authority.append_capability(),
        observability: authority.observability_capability(),
    }
}

impl Fixture {
    fn append(&self, envelope: EventEnvelope) -> u64 {
        self.append
            .append_durable(envelope, AppendMode::Plain)
            .unwrap()
            .seq
    }

    fn trace(&self, subject: TraceSubject) -> meld_events::EventTraceReport {
        self.observability.trace(self.ledger_id, subject).unwrap()
    }
}

fn node_ref(id: &str) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", id).unwrap()
}

/// Appends the shared causal chain and returns the sequences:
/// an observation carrying node-a, a relation event linking node-a to
/// node-b, and a derived event whose structural provenance names the
/// observation. The payload retains frozen `spine::{seq}` domain IDs without
/// making them trace edges.
fn append_causal_chain(fx: &Fixture) -> (u64, u64, u64) {
    let observed = fx.append(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:00Z".to_string(),
            "session-a",
            "workspace_fs",
            "scan-a",
            "workspace_fs.node_observed",
            None,
            json!({ "path": "src/lib.rs" }),
        )
        .with_graph(vec![node_ref("node-a")], Vec::new()),
    );

    let related = fx.append(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:01Z".to_string(),
            "session-a",
            "workspace_fs",
            "scan-a",
            "workspace_fs.edge_observed",
            None,
            json!({ "edge": "contains" }),
        )
        .with_graph(
            vec![node_ref("node-a"), node_ref("node-b")],
            vec![EventRelation::new("contains", node_ref("node-a"), node_ref("node-b")).unwrap()],
        ),
    );

    let derived = fx.append(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:02Z".to_string(),
            "session-a",
            "world_state",
            "graph",
            "world_state.anchor_selected",
            None,
            json!({
                "anchor": {
                    "anchor_id": "anchor::workspace_fs::node::node-a::1",
                    "source_fact_ids": [format!("spine::{observed}")],
                    "source_spine_fact_id": format!("spine::{observed}")
                }
            }),
        )
        .with_source_records(vec![EventRecordRef {
            ledger_id: fx.ledger_id,
            seq: observed,
        }]),
    );

    (observed, related, derived)
}

#[test]
fn object_trace_walks_references_relations_and_provenance_in_order() {
    let fx = fixture();
    let (observed, related, derived) = append_causal_chain(&fx);

    let report = fx.trace(TraceSubject::Object(node_ref("node-a")));
    assert_eq!(report.ledger_id, fx.ledger_id);

    assert_eq!(report.coverage.retained_from, 1);
    assert_eq!(report.coverage.tip_seq, derived);
    assert_eq!(report.coverage.scanned_from_seq, Some(observed));
    assert_eq!(report.coverage.scanned_through_seq, Some(derived));
    assert_eq!(report.coverage.truncation, CoverageTruncation::None);

    let links: Vec<(u64, TraceLink)> = report
        .hops
        .iter()
        .map(|hop| (hop.seq, hop.link.clone()))
        .collect();
    assert_eq!(
        links,
        vec![
            (observed, TraceLink::ObjectRef),
            (
                related,
                TraceLink::Relation {
                    relation_type: "contains".to_string(),
                }
            ),
            (
                derived,
                TraceLink::SourceRecord {
                    record: EventRecordRef {
                        ledger_id: fx.ledger_id,
                        seq: observed,
                    },
                }
            ),
        ]
    );
    for hop in &report.hops {
        assert_eq!(hop.session_id, "session-a");
    }
}

#[test]
fn object_trace_provenance_is_one_generation_only() {
    let fx = fixture();
    let (_, _, derived) = append_causal_chain(&fx);

    // References the derived record, which is provenance-linked but not a
    // seed, so a single-generation trace must not include it.
    let second_generation = fx.append(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:03Z".to_string(),
            "session-a",
            "world_state",
            "graph",
            "world_state.belief_formed",
            None,
            json!({}),
        )
        .with_source_records(vec![EventRecordRef {
            ledger_id: fx.ledger_id,
            seq: derived,
        }]),
    );

    let report = fx.trace(TraceSubject::Object(node_ref("node-a")));

    assert!(report.hops.iter().any(|hop| hop.seq == derived));
    assert!(!report.hops.iter().any(|hop| hop.seq == second_generation));
}

#[test]
fn payload_strings_that_look_like_spine_ids_do_not_create_trace_edges() {
    let fx = fixture();
    let (observed, _, _) = append_causal_chain(&fx);
    let payload_only = fx.append(EventEnvelope::new_domain(
        "2026-07-08T00:00:04Z".to_string(),
        "session-a",
        "world_state",
        "graph",
        "world_state.payload_only",
        None,
        json!({
            "nested": {
                "source_fact_ids": [format!("spine::{observed}")],
                "arbitrary": format!("spine::{observed}")
            }
        }),
    ));

    let report = fx.trace(TraceSubject::Record { seq: observed });

    assert!(!report.hops.iter().any(|hop| hop.seq == payload_only));
}

#[test]
fn authority_rejects_foreign_ledger_provenance_before_append() {
    let fx = fixture();
    let foreign_ledger = LedgerIdentity::new();
    let error = fx
        .append
        .append_durable(
            EventEnvelope::new_domain(
                "2026-07-08T00:00:00Z".to_string(),
                "session-a",
                "world_state",
                "graph",
                "world_state.foreign_provenance",
                None,
                json!({}),
            )
            .with_source_records(vec![EventRecordRef {
                ledger_id: foreign_ledger,
                seq: 1,
            }]),
            AppendMode::Plain,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        meld_events::error::EventAuthorityError::IdentityMismatch { expected, actual }
            if expected == fx.ledger_id && actual == foreign_ledger
    ));
    assert_eq!(
        fx.observability
            .trace(fx.ledger_id, TraceSubject::Record { seq: 1 })
            .unwrap()
            .coverage
            .tip_seq,
        0
    );
}

#[test]
fn foreign_provenance_rejects_batches_and_best_effort_without_advancing_the_ledger() {
    let fx = fixture();
    let foreign_ledger = LedgerIdentity::new();
    let valid = EventEnvelope::new_domain(
        "2026-07-08T00:00:00Z".to_string(),
        "session-a",
        "world_state",
        "graph",
        "world_state.valid",
        None,
        json!({}),
    );
    let foreign = EventEnvelope::new_domain(
        "2026-07-08T00:00:01Z".to_string(),
        "session-a",
        "world_state",
        "graph",
        "world_state.foreign",
        None,
        json!({}),
    )
    .with_source_records(vec![EventRecordRef {
        ledger_id: foreign_ledger,
        seq: 1,
    }]);

    let batch_error = fx
        .append
        .append_durable_batch(vec![valid, foreign.clone()], AppendMode::Plain)
        .unwrap_err();
    assert!(matches!(
        batch_error,
        meld_events::error::EventAuthorityError::IdentityMismatch { expected, actual }
            if expected == fx.ledger_id && actual == foreign_ledger
    ));

    let best_effort_error = fx
        .append
        .append_best_effort(foreign, AppendMode::Plain)
        .unwrap_err();
    assert!(matches!(
        best_effort_error,
        meld_events::error::EventAuthorityError::IdentityMismatch { expected, actual }
            if expected == fx.ledger_id && actual == foreign_ledger
    ));

    fx.append.barrier().unwrap();
    let report = fx.trace(TraceSubject::Record { seq: 1 });
    assert_eq!(report.coverage.tip_seq, 0);
    assert!(report.hops.is_empty());
}

#[test]
fn source_record_link_wire_shape_carries_ledger_and_sequence() {
    let ledger_id = LedgerIdentity::new();
    let value = serde_json::to_value(TraceLink::SourceRecord {
        record: EventRecordRef { ledger_id, seq: 7 },
    })
    .unwrap();

    assert_eq!(
        value,
        json!({
            "source_record": {
                "record": {
                    "ledger_id": ledger_id,
                    "seq": 7
                }
            }
        })
    );
}

#[test]
fn stream_trace_matches_domain_and_stream_exactly() {
    let fx = fixture();
    let (observed, related, derived) = append_causal_chain(&fx);

    let report = fx.trace(TraceSubject::Stream {
        domain_id: "workspace_fs".to_string(),
        stream_id: "scan-a".to_string(),
    });

    let links: Vec<(u64, TraceLink)> = report
        .hops
        .iter()
        .map(|hop| (hop.seq, hop.link.clone()))
        .collect();
    assert_eq!(
        links,
        vec![(observed, TraceLink::Stream), (related, TraceLink::Stream)]
    );
    assert!(!report.hops.iter().any(|hop| hop.seq == derived));
}

#[test]
fn record_trace_links_subject_shared_objects_and_provenance() {
    let fx = fixture();
    let (observed, related, derived) = append_causal_chain(&fx);

    let report = fx.trace(TraceSubject::Record { seq: observed });

    let links: Vec<(u64, TraceLink)> = report
        .hops
        .iter()
        .map(|hop| (hop.seq, hop.link.clone()))
        .collect();
    assert_eq!(
        links,
        vec![
            (observed, TraceLink::Subject),
            (
                related,
                TraceLink::Relation {
                    relation_type: "contains".to_string(),
                }
            ),
            (
                derived,
                TraceLink::SourceRecord {
                    record: EventRecordRef {
                        ledger_id: fx.ledger_id,
                        seq: observed,
                    },
                }
            ),
        ]
    );
}

#[test]
fn dedup_keeps_the_strongest_link_per_record() {
    let fx = fixture();
    let (observed, related, _) = append_causal_chain(&fx);

    // The relation event also carries node-a in its object references;
    // relation beats object_ref, so one hop with the stronger link remains.
    let object_report = fx.trace(TraceSubject::Object(node_ref("node-a")));
    let related_hops: Vec<&TraceLink> = object_report
        .hops
        .iter()
        .filter(|hop| hop.seq == related)
        .map(|hop| &hop.link)
        .collect();
    assert_eq!(
        related_hops,
        vec![&TraceLink::Relation {
            relation_type: "contains".to_string(),
        }]
    );

    // The subject record shares objects with itself; subject beats
    // object_ref.
    let record_report = fx.trace(TraceSubject::Record { seq: observed });
    let subject_hops: Vec<&TraceLink> = record_report
        .hops
        .iter()
        .filter(|hop| hop.seq == observed)
        .map(|hop| &hop.link)
        .collect();
    assert_eq!(subject_hops, vec![&TraceLink::Subject]);
}

#[test]
fn unknown_subjects_trace_to_empty_reports() {
    let fx = fixture();
    append_causal_chain(&fx);

    let object_report = fx.trace(TraceSubject::Object(node_ref("node-missing")));
    assert!(object_report.hops.is_empty());

    let stream_report = fx.trace(TraceSubject::Stream {
        domain_id: "workspace_fs".to_string(),
        stream_id: "scan-missing".to_string(),
    });
    assert!(stream_report.hops.is_empty());

    let record_report = fx.trace(TraceSubject::Record { seq: 999 });
    assert!(record_report.hops.is_empty());
}

#[test]
fn declared_relation_endpoints_support_object_and_record_neighborhoods() {
    let fx = fixture();
    let first = fx.append(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:00Z".to_string(),
            "session-a",
            "world_state",
            "graph",
            "world_state.edge_observed",
            None,
            json!({}),
        )
        .with_graph(
            vec![node_ref("node-a"), node_ref("node-b")],
            vec![EventRelation::new("contains", node_ref("node-a"), node_ref("node-b")).unwrap()],
        ),
    );
    let second = fx.append(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:01Z".to_string(),
            "session-a",
            "world_state",
            "graph",
            "world_state.edge_observed",
            None,
            json!({}),
        )
        .with_graph(
            vec![node_ref("node-b"), node_ref("node-c")],
            vec![EventRelation::new("depends_on", node_ref("node-b"), node_ref("node-c")).unwrap()],
        ),
    );

    let object_report = fx.trace(TraceSubject::Object(node_ref("node-a")));
    assert_eq!(object_report.hops.len(), 1);
    assert_eq!(object_report.hops[0].seq, first);
    assert_eq!(
        object_report.hops[0].link,
        TraceLink::Relation {
            relation_type: "contains".to_string()
        }
    );

    let record_report = fx.trace(TraceSubject::Record { seq: first });
    assert_eq!(
        record_report
            .hops
            .iter()
            .map(|hop| (hop.seq, hop.link.clone()))
            .collect::<Vec<_>>(),
        vec![
            (first, TraceLink::Subject),
            (
                second,
                TraceLink::Relation {
                    relation_type: "depends_on".to_string()
                }
            )
        ]
    );
}

#[test]
fn empty_trace_reports_empty_complete_coverage() {
    let fx = fixture();

    let report = fx.trace(TraceSubject::Record { seq: 1 });

    assert!(report.hops.is_empty());
    assert_eq!(report.coverage.retained_from, 1);
    assert_eq!(report.coverage.tip_seq, 0);
    assert_eq!(report.coverage.scanned_from_seq, None);
    assert_eq!(report.coverage.scanned_through_seq, None);
    assert_eq!(report.coverage.truncation, CoverageTruncation::None);
}

#[test]
fn trace_scans_from_the_retained_boundary_without_a_gap_error() {
    let fx = fixture();
    let (observed, related, derived) = append_causal_chain(&fx);
    let late = fx.append(
        EventEnvelope::new_domain(
            "2026-07-08T00:00:04Z".to_string(),
            "session-a",
            "workspace_fs",
            "scan-b",
            "workspace_fs.node_observed",
            None,
            json!({ "path": "src/lib.rs" }),
        )
        .with_graph(vec![node_ref("node-a")], Vec::new()),
    );

    fx.store.set_retained_lower_boundary(derived).unwrap();

    let report = fx.trace(TraceSubject::Object(node_ref("node-a")));

    assert!(!report.hops.iter().any(|hop| hop.seq == observed));
    assert!(!report.hops.iter().any(|hop| hop.seq == related));
    assert!(report.hops.iter().any(|hop| hop.seq == late));
    assert!(report.hops.iter().all(|hop| hop.seq >= derived));
    assert_eq!(report.coverage.retained_from, derived);
    assert_eq!(report.coverage.tip_seq, late);
    assert_eq!(report.coverage.scanned_from_seq, Some(derived));
    assert_eq!(report.coverage.scanned_through_seq, Some(late));
    assert_eq!(report.coverage.truncation, CoverageTruncation::Before);
}
