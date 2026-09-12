use std::collections::BTreeMap;
use std::sync::Arc;

use meld_events::events::remote::LocalEventAuthorityClient;
use meld_events::{
    AppendMode, DomainObjectRef, EventAuthority, EventAuthorityOpenOptions, EventEnvelope,
};
use meld_world_model::world_state::graph::{contracts::*, store::TraversalStore};

use super::super::{graph::OwnerGraphCallbacks, *};

#[test]
fn graph_callback_preserves_current_incompleteness_and_cannot_select_unbound_inputs() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default()).unwrap();
    let ledger_id = authority.ledger_identity();
    let proof = authority
        .append_capability()
        .append_durable_proven(
            EventEnvelope::with_now_domain(
                "session",
                "specimen",
                "subject",
                "changed",
                None,
                serde_json::json!({}),
            )
            .with_record_id("source-change"),
            AppendMode::Idempotent,
        )
        .unwrap();
    let input = TraversalOwnerRequirement {
        owner_id: "producer".into(),
        scope: OwnerPublicationScope {
            scope_id: "source".into(),
            branch_id: None,
            perspective_id: None,
            valid_at: None,
        },
        required: true,
        event_source: None,
    };
    let callbacks = OwnerGraphCallbacks {
        next: Arc::new(NoOwnerCallbacks),
        graph: Arc::new(TraversalStore::new(db).unwrap()),
        events: Arc::new(LocalEventAuthorityClient::new(&authority)),
        ledger_id,
        inputs: BTreeMap::from([("selected".into(), input.clone())]),
    };
    let traversal = BoundedTraversalRequest {
        roots: vec![DomainObjectRef::new("producer", "scope", "source").unwrap()],
        direction: TraversalDirection::Outgoing,
        relation_types: None,
        bounds: TraversalBounds {
            max_depth: 1,
            max_objects: 2,
            max_occurrences: 2,
            max_paths: 2,
        },
    };
    let read: OwnerGraphReadV1 = serde_json::from_value(
        callbacks
            .call(OwnerCallbackV1::GraphRead {
                input_id: "selected".into(),
                traversal: traversal.clone(),
            })
            .unwrap(),
    )
    .unwrap();
    assert_eq!(read.cut.owners, vec![input]);
    assert_eq!(read.cut.event_position.after_seq, proof.seq());
    assert_eq!(read.cut.status, TraversalCutStatus::Incomplete);
    assert!(read.cut.receipts.is_empty());
    assert!(read
        .cut
        .issues
        .iter()
        .any(|issue| matches!(issue, TraversalCutIssue::ProjectionLag { .. })));
    assert_eq!(
        callbacks
            .call(OwnerCallbackV1::GraphRead {
                input_id: "foreign".into(),
                traversal: traversal.clone()
            })
            .unwrap_err()
            .code,
        "owner_graph_read_invalid"
    );
    let mut unbounded = traversal;
    unbounded.bounds.max_objects = 0;
    assert_eq!(
        callbacks
            .call(OwnerCallbackV1::GraphRead {
                input_id: "selected".into(),
                traversal: unbounded
            })
            .unwrap_err()
            .code,
        "owner_graph_read_invalid"
    );
}
