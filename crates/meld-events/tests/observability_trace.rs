//! Behavior tests for the trace surface: structural causal walks over
//! object references, relation endpoints, and stored `spine::{seq}`
//! provenance, through the public observability port.

use std::sync::Arc;

use meld_events::events::observability::EventObservabilityPort;
use meld_events::events::registry::EventCursorRegistry;
use meld_events::events::store::EventStore;
use meld_events::{
    DomainObjectRef, EventEnvelope, EventRelation, EventWriter, LedgerObservability, TraceLink,
    TraceSubject,
};
use serde_json::json;

/// Ledger fixture: the port plus direct store access for appends.
///
/// The writer only supplies the watermark and drop-counter handles the
/// backing requires; the fixture appends straight to the store because
/// trace reads persisted records, not the commit pipeline.
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

fn node_ref(id: &str) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", id).unwrap()
}

/// Appends the shared causal chain and returns the sequences:
/// an observation carrying node-a, a relation event linking node-a to
/// node-b, and a derived-style event whose payload embeds `spine::{obs}`
/// provenance pointing at the observation.
fn append_causal_chain(store: &EventStore) -> (u64, u64, u64) {
    let observed = store
        .append_envelope(
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
        )
        .unwrap();

    let related = store
        .append_envelope(
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
                vec![
                    EventRelation::new("contains", node_ref("node-a"), node_ref("node-b")).unwrap(),
                ],
            ),
        )
        .unwrap();

    let derived = store
        .append_envelope(EventEnvelope::new_domain(
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
        ))
        .unwrap();

    (observed, related, derived)
}

#[test]
fn object_trace_walks_references_relations_and_provenance_in_order() {
    let fx = fixture();
    let (observed, related, derived) = append_causal_chain(&fx.store);

    let report = fx
        .port
        .trace(TraceSubject::Object(node_ref("node-a")))
        .unwrap();

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
                TraceLink::SourceFact {
                    fact_id: format!("spine::{observed}"),
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
    let (_, _, derived) = append_causal_chain(&fx.store);

    // References the derived record, which is provenance-linked but not a
    // seed, so a single-generation trace must not include it.
    let second_generation = fx
        .store
        .append_envelope(EventEnvelope::new_domain(
            "2026-07-08T00:00:03Z".to_string(),
            "session-a",
            "world_state",
            "graph",
            "world_state.belief_formed",
            None,
            json!({ "source_fact_ids": [format!("spine::{derived}")] }),
        ))
        .unwrap();

    let report = fx
        .port
        .trace(TraceSubject::Object(node_ref("node-a")))
        .unwrap();

    assert!(report.hops.iter().any(|hop| hop.seq == derived));
    assert!(!report.hops.iter().any(|hop| hop.seq == second_generation));
}

#[test]
fn stream_trace_matches_domain_and_stream_exactly() {
    let fx = fixture();
    let (observed, related, derived) = append_causal_chain(&fx.store);

    let report = fx
        .port
        .trace(TraceSubject::Stream {
            domain_id: "workspace_fs".to_string(),
            stream_id: "scan-a".to_string(),
        })
        .unwrap();

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
    let (observed, related, derived) = append_causal_chain(&fx.store);

    let report = fx
        .port
        .trace(TraceSubject::Record { seq: observed })
        .unwrap();

    let links: Vec<(u64, TraceLink)> = report
        .hops
        .iter()
        .map(|hop| (hop.seq, hop.link.clone()))
        .collect();
    assert_eq!(
        links,
        vec![
            (observed, TraceLink::Subject),
            (related, TraceLink::ObjectRef),
            (
                derived,
                TraceLink::SourceFact {
                    fact_id: format!("spine::{observed}"),
                }
            ),
        ]
    );
}

#[test]
fn dedup_keeps_the_strongest_link_per_record() {
    let fx = fixture();
    let (observed, related, _) = append_causal_chain(&fx.store);

    // The relation event also carries node-a in its object references;
    // relation beats object_ref, so one hop with the stronger link remains.
    let object_report = fx
        .port
        .trace(TraceSubject::Object(node_ref("node-a")))
        .unwrap();
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
    let record_report = fx
        .port
        .trace(TraceSubject::Record { seq: observed })
        .unwrap();
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
    append_causal_chain(&fx.store);

    let object_report = fx
        .port
        .trace(TraceSubject::Object(node_ref("node-missing")))
        .unwrap();
    assert!(object_report.hops.is_empty());

    let stream_report = fx
        .port
        .trace(TraceSubject::Stream {
            domain_id: "workspace_fs".to_string(),
            stream_id: "scan-missing".to_string(),
        })
        .unwrap();
    assert!(stream_report.hops.is_empty());

    let record_report = fx.port.trace(TraceSubject::Record { seq: 999 }).unwrap();
    assert!(record_report.hops.is_empty());
}

#[test]
fn trace_scans_from_the_retained_boundary_without_a_gap_error() {
    let fx = fixture();
    let (observed, related, derived) = append_causal_chain(&fx.store);
    let late = fx
        .store
        .append_envelope(
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
        )
        .unwrap();

    fx.store.set_retained_lower_boundary(derived).unwrap();

    let report = fx
        .port
        .trace(TraceSubject::Object(node_ref("node-a")))
        .unwrap();

    assert!(!report.hops.iter().any(|hop| hop.seq == observed));
    assert!(!report.hops.iter().any(|hop| hop.seq == related));
    assert!(report.hops.iter().any(|hop| hop.seq == late));
    assert!(report.hops.iter().all(|hop| hop.seq >= derived));
}
