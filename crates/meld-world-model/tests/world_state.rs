//! Canonical Graph replay, identity and historical-source coverage.
use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_world_model::events::error::EventAuthorityError;
use meld_world_model::events::{
    AppendMode, DomainObjectRef, EventAuthority, EventAuthorityOpenOptions,
    EventConsumerRegistryCapability, EventEnvelope, EventPage, EventRelation,
    EventReplayCapability, LedgerCursor, LedgerIdentity, ReplayRequest,
};
use meld_world_model::world_state::graph::runtime::GraphRuntime;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::{GraphConsumerCursorReporter, GraphEventReplaySource};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn event(
    domain_id: &str,
    event_type: &str,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
) -> EventEnvelope {
    EventEnvelope::with_now_domain(
        "session-a",
        domain_id,
        "stream-a",
        event_type,
        None,
        json!({ "ok": true }),
    )
    .with_graph(objects, relations)
}

#[derive(Clone)]
struct AuthorityGraphPorts {
    replay: EventReplayCapability,
    registry: EventConsumerRegistryCapability,
}

impl AuthorityGraphPorts {
    fn new(authority: &EventAuthority) -> Self {
        Self {
            replay: authority.replay_capability(),
            registry: authority.consumer_registry_capability(),
        }
    }
}

impl GraphEventReplaySource for AuthorityGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

impl GraphConsumerCursorReporter for AuthorityGraphPorts {
    fn report_owner_source_cursor(
        &self,
        source: &meld_world_model::world_state::graph::admission::OwnerEventSourceRef,
        cursor: LedgerCursor,
    ) -> Result<(), EventAuthorityError> {
        self.registry
            .report(&source.consumer_id(), cursor)
            .map(|_| ())
    }

    fn ledger_identity(&self) -> LedgerIdentity {
        self.registry.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        self.registry.report("world_state.graph.reducer", cursor)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ForeignPageField {
    PageIdentity,
    NextCursorIdentity,
}

struct ForeignPageReplay {
    inner: EventReplayCapability,
    foreign_ledger_id: LedgerIdentity,
    field: ForeignPageField,
}

impl GraphEventReplaySource for ForeignPageReplay {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        let mut page = self.inner.replay(request)?;
        match self.field {
            ForeignPageField::PageIdentity => page.ledger_id = self.foreign_ledger_id,
            ForeignPageField::NextCursorIdentity => {
                page.next_cursor.ledger_id = self.foreign_ledger_id;
            }
        }
        Ok(page)
    }
}

struct FailOnceCursorReporter {
    inner: EventConsumerRegistryCapability,
    fail_next: AtomicBool,
}

impl GraphConsumerCursorReporter for FailOnceCursorReporter {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(EventAuthorityError::Unavailable {
                message: "injected registry failure".to_string(),
            });
        }
        self.inner.report("world_state.graph.reducer", cursor)?;
        Ok(())
    }
}

#[test]
fn graph_runtime_resets_legacy_cursor_and_preserves_migration_evidence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp_dir.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));
    let traversal =
        TraversalStore::shared(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap();
    traversal
        .db()
        .open_tree("traversal_runtime_meta")
        .unwrap()
        .insert("last_reduced_seq", b"42")
        .unwrap();
    traversal.flush().unwrap();
    let source = authority
        .append_capability()
        .append_durable(
            event(
                "context",
                "context.head_tombstoned",
                vec![object("context", "head", "node-a::analysis")],
                Vec::new(),
            ),
            AppendMode::Plain,
        )
        .unwrap();

    let runtime = GraphRuntime::from_ports(ports.clone(), ports, Arc::clone(&traversal)).unwrap();
    let report = runtime
        .catch_up_bounded(
            meld_world_model::world_state::graph::runtime::GraphCatchUpBudget { max_items: 10 },
        )
        .unwrap();

    assert_eq!(report.input_event_seq, 0);
    assert_eq!(report.output_event_seq, source.seq);
    let meta = traversal.db().open_tree("traversal_runtime_meta").unwrap();
    assert_eq!(
        meta.get("legacy_last_reduced_seq_evidence")
            .unwrap()
            .unwrap(),
        b"42".as_slice()
    );
    let cursor: serde_json::Value =
        serde_json::from_slice(&meta.get("event_authority_cursor").unwrap().unwrap()).unwrap();
    assert_eq!(cursor["ledger_id"], authority.ledger_identity().to_string());
    assert_eq!(cursor["after_seq"], source.seq);
}

#[test]
fn graph_runtime_rejects_mismatched_port_identities_before_replay() {
    let temp_dir = tempfile::tempdir().unwrap();
    let first = EventAuthority::open(
        sled::open(temp_dir.path().join("first-events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let second = EventAuthority::open(
        sled::open(temp_dir.path().join("second-events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let first_ports = Arc::new(AuthorityGraphPorts::new(&first));
    let second_ports = Arc::new(AuthorityGraphPorts::new(&second));
    let traversal =
        TraversalStore::shared(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap();

    let result = GraphRuntime::from_ports(first_ports, second_ports, traversal);

    assert!(matches!(
        result,
        Err(meld_world_model::error::StorageError::IdentityMismatch {
            expected,
            actual,
        }) if expected == first.ledger_identity() && actual == second.ledger_identity()
    ));
}

#[test]
fn graph_runtime_rejects_foreign_page_and_next_cursor_identities() {
    let temp_dir = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp_dir.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let foreign = EventAuthority::open(
        sled::open(temp_dir.path().join("foreign-events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));

    for (field, suffix) in [
        (ForeignPageField::PageIdentity, "page"),
        (ForeignPageField::NextCursorIdentity, "next"),
    ] {
        let replay = Arc::new(ForeignPageReplay {
            inner: authority.replay_capability(),
            foreign_ledger_id: foreign.ledger_identity(),
            field,
        });
        let traversal = TraversalStore::shared(
            sled::open(temp_dir.path().join(format!("traversal-{suffix}"))).unwrap(),
        )
        .unwrap();
        let runtime = GraphRuntime::from_ports(replay, ports.clone(), traversal).unwrap();

        assert!(matches!(
            runtime.catch_up(),
            Err(meld_world_model::error::StorageError::IdentityMismatch {
                expected,
                actual,
            }) if expected == authority.ledger_identity() && actual == foreign.ledger_identity()
        ));
        assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, 0);
    }
}

#[test]
fn graph_runtime_from_ports_reports_retention_gap_without_cursor_movement() {
    let temp_dir = tempfile::tempdir().unwrap();
    let event_db = sled::open(temp_dir.path().join("events")).unwrap();
    let authority =
        EventAuthority::open(event_db.clone(), EventAuthorityOpenOptions::default()).unwrap();
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));
    authority
        .append_capability()
        .append_durable(
            event("context", "context.noop", Vec::new(), Vec::new()),
            AppendMode::Plain,
        )
        .unwrap();
    EventStore::new(event_db)
        .unwrap()
        .set_retained_lower_boundary(3)
        .unwrap();
    let traversal =
        TraversalStore::shared(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap();
    let route = meld_world_model::world_state::graph::admission::GraphOwnerEventRoute {
        complete_event_source: true,
        route_id: "retained-owner-route".into(),
        owner_id: "sample".into(),
        event_type: "sample.publication".into(),
        enumeration_rule_revision: "sample-v1".into(),
    };
    traversal.install_owner_event_route(&route).unwrap();
    let runtime = GraphRuntime::from_ports(ports.clone(), ports, Arc::clone(&traversal)).unwrap();

    let report = runtime
        .catch_up_bounded(
            meld_world_model::world_state::graph::runtime::GraphCatchUpBudget { max_items: 10 },
        )
        .unwrap();
    assert_eq!(report.fatal_errors[0].code, "retention_gap");
    assert!(!traversal
        .covers_event_source(
            authority.ledger_identity(),
            "sample",
            &route.source_ref().unwrap()
        )
        .unwrap());
    assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, 0);
    assert!(authority
        .consumer_registry_capability()
        .get("world_state.graph.reducer")
        .unwrap()
        .is_none());
    assert!(traversal
        .db()
        .open_tree("traversal_runtime_meta")
        .unwrap()
        .get("pending_derived_events")
        .unwrap()
        .is_none());
    assert!(matches!(
        runtime.catch_up(),
        Err(meld_world_model::error::StorageError::RetentionGap {
            after_seq: 0,
            retained_from: 3,
        })
    ));
    assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, 0);
    assert!(authority
        .consumer_registry_capability()
        .get("world_state.graph.reducer")
        .unwrap()
        .is_none());
    assert!(traversal
        .db()
        .open_tree("traversal_runtime_meta")
        .unwrap()
        .get("pending_derived_events")
        .unwrap()
        .is_none());
}

#[test]
fn graph_runtime_retries_registry_report_after_local_cursor_is_durable() {
    let temp_dir = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp_dir.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));
    let source = authority
        .append_capability()
        .append_durable(
            event(
                "context",
                "context.head_selected",
                vec![
                    object("context", "head", "node-a::analysis"),
                    object("workspace_fs", "node", "node-a"),
                    object("context", "frame", "frame-a"),
                ],
                Vec::new(),
            ),
            AppendMode::Plain,
        )
        .unwrap();
    let reporter = Arc::new(FailOnceCursorReporter {
        inner: authority.consumer_registry_capability(),
        fail_next: AtomicBool::new(true),
    });
    let runtime = GraphRuntime::from_ports(
        ports.clone(),
        reporter,
        TraversalStore::shared(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        runtime.catch_up(),
        Err(meld_world_model::error::StorageError::Unavailable(_))
    ));
    // The local cursor is the recovery source of truth. Registry publication
    // follows it and is retried on the next tick when a report fails.
    assert_eq!(
        runtime.durable_event_cursor().unwrap().after_seq,
        source.seq
    );
    assert!(authority
        .consumer_registry_capability()
        .get("world_state.graph.reducer")
        .unwrap()
        .is_none());

    runtime.catch_up().unwrap();
    assert!(
        authority
            .consumer_registry_capability()
            .get("world_state.graph.reducer")
            .unwrap()
            .unwrap()
            .reported_seq
            >= source.seq
    );
    let derived = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: authority.ledger_identity(),
                after_seq: 0,
            },
            limit: 10,
        })
        .unwrap()
        .records
        .into_iter()
        .filter(|record| record.event_type == "world_state.anchor_selected")
        .count();
    assert_eq!(derived, 0);
}

#[test]
fn late_owner_source_replay_refuses_pruned_history_without_claiming_coverage() {
    use meld_world_model::world_state::graph::admission::GraphOwnerEventRoute;
    use meld_world_model::world_state::graph::runtime::GraphCatchUpBudget;
    let temp = tempfile::tempdir().unwrap();
    let event_db = sled::open(temp.path().join("events")).unwrap();
    let authority =
        EventAuthority::open(event_db.clone(), EventAuthorityOpenOptions::default()).unwrap();
    for _ in 0..3 {
        authority
            .append_capability()
            .append_durable(
                event("context", "context.noop", Vec::new(), Vec::new()),
                AppendMode::Plain,
            )
            .unwrap();
    }
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));
    let store = TraversalStore::shared(sled::open(temp.path().join("world")).unwrap()).unwrap();
    let graph = GraphRuntime::from_ports(ports.clone(), ports, store.clone()).unwrap();
    graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 8 })
        .unwrap();
    let before = graph.durable_event_cursor().unwrap();
    EventStore::new(event_db)
        .unwrap()
        .set_retained_lower_boundary(2)
        .unwrap();
    let route = GraphOwnerEventRoute {
        complete_event_source: true,
        route_id: "late-source".into(),
        owner_id: "sample".into(),
        event_type: "sample.event".into(),
        enumeration_rule_revision: "sample-v1".into(),
    };
    store.install_owner_event_route(&route).unwrap();
    let report = graph
        .catch_up_bounded(GraphCatchUpBudget { max_items: 1 })
        .unwrap();
    assert_eq!(report.fatal_errors[0].code, "owner_source_retention_gap");
    assert_eq!(report.events_attempted, 0);
    assert_eq!(report.source_replay.unwrap().after.after_seq, 0);
    assert_eq!(graph.durable_event_cursor().unwrap(), before);
    assert!(!store
        .covers_event_source(before.ledger_id, "sample", &route.source_ref().unwrap())
        .unwrap());
    assert!(!store.owner_event_replay_states(before.ledger_id).unwrap()[0].covered);
}
