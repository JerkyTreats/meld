use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_world_model::events::error::EventAuthorityError;
use meld_world_model::events::{
    AppendMode, AppendReceipt, DomainObjectRef, EventAppendCapability, EventAuthority,
    EventAuthorityOpenOptions, EventConsumerRegistryCapability, EventEnvelope, EventPage,
    EventRecord, EventRecordRef, EventRelation, EventReplayCapability, LedgerCursor,
    LedgerIdentity, ReplayRequest,
};
use meld_world_model::world_state::graph::compat::LegacyClaimAdapter;
use meld_world_model::world_state::graph::events::{
    anchor_selected_envelope_from_record, anchor_superseded_envelope_from_record,
    AnchorSelectedEventData, AnchorSupersededEventData,
};
use meld_world_model::world_state::graph::reducer::TraversalReducer;
use meld_world_model::world_state::graph::runtime::GraphRuntime;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::{
    GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource,
};
use meld_world_model::world_state::reducer::WorldStateReducer;
use meld_world_model::world_state::store::{StoredWorldStateFact, WorldStateStore};
use meld_world_model::{
    AnchorSelectionRecord, ClaimKind, ClaimRecord, EvidenceRecord, GraphWalkSpec, PerspectiveKey,
    SettlementStatus, TraversalDirection, TraversalFactRecord, TraversalQuery, WorldModelQueries,
    WorldStateQuery,
};
use serde_json::json;

mod support;
use support::GraphRuntimeTestFixture;

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

fn event_record(
    seq: u64,
    domain_id: &str,
    event_type: &str,
    objects: Vec<DomainObjectRef>,
) -> EventRecord {
    EventRecord::from_envelope(event(domain_id, event_type, objects, Vec::new()), seq)
}

#[derive(Clone)]
struct AuthorityGraphPorts {
    replay: EventReplayCapability,
    append: EventAppendCapability,
    registry: EventConsumerRegistryCapability,
}

impl AuthorityGraphPorts {
    fn new(authority: &EventAuthority) -> Self {
        Self {
            replay: authority.replay_capability(),
            append: authority.append_capability(),
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

impl GraphDerivedEventSink for AuthorityGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.append.ledger_identity()
    }

    fn append_derived(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        self.append.append_durable(envelope, AppendMode::Idempotent)
    }
}

impl GraphConsumerCursorReporter for AuthorityGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.registry.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        self.registry.report("world_state.graph.reducer", cursor)?;
        Ok(())
    }
}

struct FailingDerivedSink {
    ledger_id: LedgerIdentity,
}

struct ForeignReceiptSink {
    inner: EventAppendCapability,
    foreign_ledger_id: LedgerIdentity,
}

impl GraphDerivedEventSink for ForeignReceiptSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_identity()
    }

    fn append_derived(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        let mut receipt = self
            .inner
            .append_durable(envelope, AppendMode::Idempotent)?;
        receipt.ledger_id = self.foreign_ledger_id;
        Ok(receipt)
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

impl GraphDerivedEventSink for FailingDerivedSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.ledger_id
    }

    fn append_derived(
        &self,
        _envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        Err(EventAuthorityError::Unavailable {
            message: "injected derived append failure".to_string(),
        })
    }
}

fn traversal_store() -> (tempfile::TempDir, TraversalStore) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = TraversalStore::new(sled::open(temp_dir.path().join("graph")).unwrap()).unwrap();
    (temp_dir, store)
}

fn world_state_store() -> (tempfile::TempDir, WorldStateStore) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = WorldStateStore::new(sled::open(temp_dir.path().join("world")).unwrap()).unwrap();
    (temp_dir, store)
}

fn frame_anchor(
    anchor_id: &str,
    node: &DomainObjectRef,
    frame: &DomainObjectRef,
    seq: u64,
) -> AnchorSelectionRecord {
    AnchorSelectionRecord {
        anchor_id: anchor_id.to_string(),
        anchor_ref: object("context", "head", "node-a::analysis"),
        subject: node.clone(),
        perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
        target: frame.clone(),
        source_fact_ids: vec![format!("spine::{seq}")],
        created_by_fact_id: format!("fact-{seq}"),
        selected_at_seq: seq,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    }
}

fn generation_claim(
    claim_id: &str,
    kind: ClaimKind,
    subject: &DomainObjectRef,
    seq: u64,
) -> ClaimRecord {
    ClaimRecord {
        claim_id: claim_id.to_string(),
        claim_kind: kind,
        subject: subject.clone(),
        status: SettlementStatus::Active,
        supporting_fact_ids: vec![format!("spine::{seq}")],
        superseded_by: None,
        created_by_fact_id: format!("fact-{seq}"),
        created_at_seq: seq,
        last_updated_seq: seq,
    }
}

#[test]
fn graph_derived_envelopes_carry_structural_source_record_provenance() {
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let anchor = frame_anchor("anchor-a", &node, &frame, 41);
    let source_record = EventRecordRef {
        ledger_id: LedgerIdentity::from_str("018d2fd1-6030-7c6a-b03f-41d44f348d63").unwrap(),
        seq: 41,
    };

    let selected = anchor_selected_envelope_from_record(
        "session-a",
        source_record,
        AnchorSelectedEventData {
            anchor: anchor.clone(),
        },
    );
    let superseded = anchor_superseded_envelope_from_record(
        "session-a",
        source_record,
        AnchorSupersededEventData { anchor },
    );

    assert_eq!(selected.provenance.source_records, vec![source_record]);
    assert_eq!(superseded.provenance.source_records, vec![source_record]);
    assert_eq!(selected.data["anchor"]["source_fact_ids"][0], "spine::41");
    assert_eq!(superseded.data["anchor"]["source_fact_ids"][0], "spine::41");
}

#[test]
fn graph_store_indexes_facts_by_object_and_sequence() {
    let (_temp_dir, store) = traversal_store();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let relation = EventRelation::new("selected", node.clone(), frame.clone()).unwrap();
    let first = TraversalFactRecord {
        fact_id: "fact-1".to_string(),
        source_spine_fact_id: "spine::1".to_string(),
        seq: 1,
        event_type: "context.head_selected".to_string(),
        objects: vec![node.clone()],
        relations: Vec::new(),
    };
    let second = TraversalFactRecord {
        fact_id: "fact-2".to_string(),
        source_spine_fact_id: "spine::2".to_string(),
        seq: 2,
        event_type: "context.head_selected".to_string(),
        objects: vec![node.clone(), frame.clone()],
        relations: vec![relation],
    };

    store.put_fact(&second).unwrap();
    store.put_fact(&first).unwrap();

    let facts = store.facts_for_object(&node, 1).unwrap();
    assert_eq!(facts, vec![second]);
}

#[test]
fn world_state_store_persists_source_facts() {
    let (_temp_dir, store) = world_state_store();
    let fact = StoredWorldStateFact {
        fact_id: "world-fact-a".to_string(),
        event_type: "world_state.claim_added".to_string(),
        claim_id: Some("claim-a".to_string()),
        evidence_id: None,
        source_spine_fact_id: Some("spine::7".to_string()),
        seq: 7,
    };

    store.put_fact(&fact).unwrap();

    assert_eq!(store.get_fact(&fact.fact_id).unwrap(), Some(fact));
}

#[test]
fn graph_store_reads_anchor_history_and_provenance() {
    let (_temp_dir, store) = traversal_store();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let fact = TraversalFactRecord {
        fact_id: "fact-1".to_string(),
        source_spine_fact_id: "spine::1".to_string(),
        seq: 1,
        event_type: "context.head_selected".to_string(),
        objects: vec![node.clone(), frame.clone()],
        relations: vec![EventRelation::new("selected", node.clone(), frame.clone()).unwrap()],
    };
    let mut anchor = frame_anchor("anchor-a", &node, &frame, 1);
    anchor.ended_by_fact_id = Some("fact-ended".to_string());

    store.put_fact(&fact).unwrap();
    store.put_anchor(&anchor).unwrap();
    store.set_current_anchor(&anchor).unwrap();

    let query = TraversalQuery::new(&store);
    assert_eq!(
        query.current_anchor(&anchor.anchor_ref).unwrap().unwrap(),
        anchor.clone()
    );
    assert_eq!(
        query.anchor_history(&anchor.anchor_ref).unwrap(),
        vec![anchor.clone()]
    );
    assert_eq!(
        query.facts_for_object(&node, 0).unwrap(),
        vec![fact.clone()]
    );

    let history = store.anchor_history(&anchor.anchor_ref).unwrap();
    let provenance = store.anchor_provenance(&anchor.anchor_id).unwrap();

    assert_eq!(history, vec![anchor.clone()]);
    assert_eq!(provenance.source_fact_ids, vec!["spine::1"]);
    assert_eq!(
        provenance.derived_fact_ids,
        vec!["fact-1".to_string(), "fact-ended".to_string()]
    );
    assert_eq!(provenance.objects, vec![frame, node]);
    assert_eq!(provenance.relations.len(), 1);
}

#[test]
fn traversal_query_specialized_helpers_read_current_anchors() {
    let (_temp_dir, store) = traversal_store();
    let source = object("workspace_fs", "source", "workspace-a");
    let snapshot = object("workspace_fs", "snapshot", "snapshot-a");
    let snapshot_anchor = AnchorSelectionRecord {
        anchor_id: "anchor-snapshot".to_string(),
        anchor_ref: object("workspace_fs", "snapshot_head", "workspace-a"),
        subject: source.clone(),
        perspective: PerspectiveKey::new("snapshot", "current").unwrap(),
        target: snapshot.clone(),
        source_fact_ids: vec!["spine::1".to_string()],
        created_by_fact_id: "fact-1".to_string(),
        selected_at_seq: 1,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    let task_run = object("execution", "task_run", "run-a");
    let artifact = object("execution", "artifact", "artifact-a");
    let artifact_anchor = AnchorSelectionRecord {
        anchor_id: "anchor-artifact".to_string(),
        anchor_ref: object("execution", "artifact_slot", "run-a::draft"),
        subject: task_run.clone(),
        perspective: PerspectiveKey::new("artifact_type", "draft").unwrap(),
        target: artifact.clone(),
        source_fact_ids: vec!["spine::2".to_string()],
        created_by_fact_id: "fact-2".to_string(),
        selected_at_seq: 2,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    store.put_anchor(&snapshot_anchor).unwrap();
    store.set_current_anchor(&snapshot_anchor).unwrap();
    store.put_anchor(&artifact_anchor).unwrap();
    store.set_current_anchor(&artifact_anchor).unwrap();

    let query = TraversalQuery::new(&store);

    assert_eq!(
        query.current_snapshot_for_source(&source).unwrap().unwrap(),
        snapshot_anchor
    );
    assert_eq!(
        query
            .current_artifact_for_task_run(&task_run, "draft")
            .unwrap()
            .unwrap(),
        artifact_anchor
    );
}

#[test]
fn traversal_query_filters_frame_heads_and_counts_by_perspective() {
    let (_temp_dir, store) = traversal_store();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let snapshot = object("workspace_fs", "snapshot", "snapshot-a");
    let frame_anchor = frame_anchor("anchor-frame", &node, &frame, 1);
    let snapshot_anchor = AnchorSelectionRecord {
        anchor_id: "anchor-snapshot".to_string(),
        anchor_ref: object("workspace_fs", "snapshot_head", "node-a"),
        subject: node.clone(),
        perspective: PerspectiveKey::new("snapshot", "current").unwrap(),
        target: snapshot,
        source_fact_ids: vec!["spine::2".to_string()],
        created_by_fact_id: "fact-2".to_string(),
        selected_at_seq: 2,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    store.put_anchor(&snapshot_anchor).unwrap();
    store.set_current_anchor(&snapshot_anchor).unwrap();
    store.put_anchor(&frame_anchor).unwrap();
    store.set_current_anchor(&frame_anchor).unwrap();

    let query = TraversalQuery::new(&store);

    assert_eq!(
        query.current_frame_heads_for_node(&node).unwrap(),
        vec![frame_anchor.clone()]
    );
    assert_eq!(
        query.current_frame_head_count_by_type("analysis").unwrap(),
        1
    );
    assert_eq!(
        store
            .current_anchors_by_perspective("frame_type", "analysis")
            .unwrap(),
        vec![frame_anchor]
    );
    assert!(store
        .current_anchors_by_perspective("frame_type", "review")
        .unwrap()
        .is_empty());
}

#[test]
fn traversal_query_counts_multiple_frame_heads_by_type() {
    let (_temp_dir, store) = traversal_store();
    let first_node = object("workspace_fs", "node", "node-a");
    let second_node = object("workspace_fs", "node", "node-b");
    let first_frame = object("context", "frame", "frame-a");
    let second_frame = object("context", "frame", "frame-b");
    let first = frame_anchor("anchor-a", &first_node, &first_frame, 1);
    let mut second = frame_anchor("anchor-b", &second_node, &second_frame, 2);
    second.anchor_ref = object("context", "head", "node-b::analysis");
    store.put_anchor(&first).unwrap();
    store.set_current_anchor(&first).unwrap();
    store.put_anchor(&second).unwrap();
    store.set_current_anchor(&second).unwrap();

    assert_eq!(
        TraversalQuery::new(&store)
            .current_frame_head_count_by_type("analysis")
            .unwrap(),
        2
    );
}

#[test]
fn graph_walk_filters_relation_types_and_can_skip_facts() {
    let (_temp_dir, store) = traversal_store();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let task = object("execution", "task_run", "run-a");
    let fact = TraversalFactRecord {
        fact_id: "fact-a".to_string(),
        source_spine_fact_id: "spine::1".to_string(),
        seq: 1,
        event_type: "mixed".to_string(),
        objects: vec![node.clone(), frame.clone(), task.clone()],
        relations: vec![
            EventRelation::new("selected", node.clone(), frame.clone()).unwrap(),
            EventRelation::new("runs", node.clone(), task.clone()).unwrap(),
        ],
    };
    store.put_fact(&fact).unwrap();

    let walk = TraversalQuery::new(&store)
        .walk(
            &node,
            &GraphWalkSpec {
                direction: TraversalDirection::Outgoing,
                relation_types: Some(vec!["runs".to_string()]),
                max_depth: 1,
                current_only: false,
                include_facts: false,
            },
        )
        .unwrap();

    assert_eq!(walk.visited_objects, vec![task, node]);
    assert!(walk.visited_facts.is_empty());
    assert_eq!(walk.traversed_relations.len(), 1);
    assert_eq!(walk.traversed_relations[0].relation_type, "runs");
}

#[test]
fn graph_walk_respects_depth_and_both_direction_filters() {
    let (_temp_dir, store) = traversal_store();
    let root = object("workspace_fs", "node", "node-a");
    let middle = object("workspace_fs", "node", "node-b");
    let leaf = object("workspace_fs", "node", "node-c");
    let ignored = object("workspace_fs", "node", "node-d");
    store
        .put_fact(&TraversalFactRecord {
            fact_id: "fact-chain".to_string(),
            source_spine_fact_id: "spine::1".to_string(),
            seq: 1,
            event_type: "graph.chain".to_string(),
            objects: vec![root.clone(), middle.clone(), leaf.clone(), ignored.clone()],
            relations: vec![
                EventRelation::new("next", root.clone(), middle.clone()).unwrap(),
                EventRelation::new("next", middle.clone(), leaf.clone()).unwrap(),
                EventRelation::new("ignored", root.clone(), ignored).unwrap(),
            ],
        })
        .unwrap();

    let walk = TraversalQuery::new(&store)
        .walk(
            &root,
            &GraphWalkSpec {
                direction: TraversalDirection::Both,
                relation_types: Some(vec!["next".to_string()]),
                max_depth: 2,
                current_only: false,
                include_facts: false,
            },
        )
        .unwrap();

    assert_eq!(walk.visited_objects, vec![root, middle, leaf]);
    assert_eq!(walk.traversed_relations.len(), 3);
    assert!(walk
        .traversed_relations
        .iter()
        .all(|relation| relation.relation_type == "next"));
}

#[test]
fn graph_neighbors_current_only_keeps_only_current_selected_target() {
    let (_temp_dir, store) = traversal_store();
    let anchor_ref = object("context", "head", "node-a::analysis");
    let node = object("workspace_fs", "node", "node-a");
    let old_frame = object("context", "frame", "frame-old");
    let current_frame = object("context", "frame", "frame-current");
    let current_anchor = AnchorSelectionRecord {
        anchor_id: "anchor-current".to_string(),
        anchor_ref: anchor_ref.clone(),
        subject: node,
        perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
        target: current_frame.clone(),
        source_fact_ids: vec!["spine::2".to_string()],
        created_by_fact_id: "fact-2".to_string(),
        selected_at_seq: 2,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    store
        .put_fact(&TraversalFactRecord {
            fact_id: "fact-relations".to_string(),
            source_spine_fact_id: "spine::2".to_string(),
            seq: 2,
            event_type: "context.head_selected".to_string(),
            objects: vec![anchor_ref.clone(), old_frame.clone(), current_frame.clone()],
            relations: vec![
                EventRelation::new("selected", anchor_ref.clone(), old_frame).unwrap(),
                EventRelation::new("selected", anchor_ref.clone(), current_frame.clone()).unwrap(),
            ],
        })
        .unwrap();
    store.put_anchor(&current_anchor).unwrap();
    store.set_current_anchor(&current_anchor).unwrap();

    let neighbors = TraversalQuery::new(&store)
        .neighbors(&anchor_ref, TraversalDirection::Outgoing, None, true)
        .unwrap();

    assert_eq!(neighbors, vec![current_frame]);
}

#[test]
fn graph_runtime_catches_up_context_head_events() {
    let temp_dir = tempfile::tempdir().unwrap();
    let fixture =
        GraphRuntimeTestFixture::open(sled::open(temp_dir.path().join("runtime")).unwrap())
            .unwrap();
    let runtime = fixture.runtime();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    fixture
        .append(event(
            "context",
            "context.head_selected",
            vec![head, node.clone(), frame.clone()],
            vec![EventRelation::new("selected", node.clone(), frame.clone()).unwrap()],
        ))
        .unwrap();

    let queries = WorldModelQueries::new(runtime);
    let current = queries
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();

    assert_eq!(current.subject, node);
    assert_eq!(current.target, frame);
    assert_eq!(current.source_fact_ids, vec!["spine::1"]);
}

#[test]
fn graph_runtime_derived_events_carry_persisted_ledger_provenance() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("runtime")).unwrap();
    let fixture = GraphRuntimeTestFixture::open(db).unwrap();
    let runtime = fixture.runtime();
    let ledger_id = runtime.ledger_identity();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    let source_seq = fixture
        .append(event(
            "context",
            "context.head_selected",
            vec![head, node, frame],
            Vec::new(),
        ))
        .unwrap()
        .seq;

    runtime.catch_up().unwrap();

    let derived = fixture
        .records()
        .unwrap()
        .into_iter()
        .filter(|record| record.seq > source_seq)
        .find(|record| record.event_type == "world_state.anchor_selected")
        .expect("graph runtime appended the reducer output");
    assert_eq!(
        derived.provenance.source_records,
        vec![EventRecordRef {
            ledger_id,
            seq: source_seq,
        }]
    );
}

#[test]
fn graph_runtime_from_ports_replays_appends_and_reports_one_authority_cursor() {
    let temp_dir = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp_dir.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));
    let traversal =
        TraversalStore::shared(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap();
    let runtime =
        GraphRuntime::from_ports(ports.clone(), ports.clone(), ports.clone(), traversal).unwrap();
    let ledger_id = authority.ledger_identity();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    let source = authority
        .append_capability()
        .append_durable(
            event(
                "context",
                "context.head_selected",
                vec![head, node, frame],
                Vec::new(),
            ),
            AppendMode::Plain,
        )
        .unwrap();

    let report = runtime
        .catch_up_bounded(
            meld_world_model::world_state::graph::runtime::GraphCatchUpBudget { max_items: 10 },
        )
        .unwrap();

    assert_eq!(report.input_event_seq, 0);
    assert_eq!(report.output_event_seq, source.seq);
    assert_eq!(report.derived_events_appended, 1);
    assert_eq!(runtime.ledger_identity(), ledger_id);
    assert_eq!(
        authority
            .consumer_registry_capability()
            .get("world_state.graph.reducer")
            .unwrap()
            .unwrap()
            .reported_seq,
        source.seq
    );
    let page = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id,
                after_seq: source.seq,
            },
            limit: 10,
        })
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(
        page.records[0].provenance.source_records,
        vec![EventRecordRef {
            ledger_id,
            seq: source.seq,
        }]
    );
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
    traversal.set_last_reduced_seq(42).unwrap();
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

    let runtime =
        GraphRuntime::from_ports(ports.clone(), ports.clone(), ports, Arc::clone(&traversal))
            .unwrap();
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
fn graph_runtime_legacy_cursor_reset_rebuilds_conflicting_projection_sequence_space() {
    let temp_dir = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp_dir.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));
    let traversal =
        TraversalStore::shared(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap();
    let head = object("context", "head", "node-a::analysis");
    let node = object("workspace_fs", "node", "node-a");
    let legacy_frame = object("context", "frame", "legacy-frame");
    let authority_frame = object("context", "frame", "authority-frame");
    let conflicting_fact = TraversalFactRecord {
        fact_id: "traversal::fact::1".to_string(),
        source_spine_fact_id: "spine::1".to_string(),
        seq: 1,
        event_type: "legacy.conflicting_sequence".to_string(),
        objects: vec![legacy_frame.clone()],
        relations: Vec::new(),
    };
    traversal.put_fact(&conflicting_fact).unwrap();
    let conflicting_anchor = AnchorSelectionRecord {
        anchor_id: format!("anchor::{}::1", head.index_key()),
        anchor_ref: head.clone(),
        subject: node.clone(),
        perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
        target: legacy_frame,
        source_fact_ids: vec!["spine::1".to_string()],
        created_by_fact_id: "legacy::anchor_selected".to_string(),
        selected_at_seq: 1,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    traversal.put_anchor(&conflicting_anchor).unwrap();
    traversal.set_current_anchor(&conflicting_anchor).unwrap();
    traversal.set_last_reduced_seq(42).unwrap();
    let meta = traversal.db().open_tree("traversal_runtime_meta").unwrap();
    meta.insert(
        "pending_derived_events",
        serde_json::to_vec(&vec![EventEnvelope::with_now_domain(
            "legacy-session",
            "world_state",
            "legacy-stream",
            "world_state.legacy_derived",
            Some("legacy-derived-record".to_string()),
            json!({ "legacy": true }),
        )])
        .unwrap(),
    )
    .unwrap();
    traversal.flush().unwrap();

    let source = authority
        .append_capability()
        .append_durable(
            event(
                "context",
                "context.head_selected",
                vec![head.clone(), node, authority_frame.clone()],
                Vec::new(),
            ),
            AppendMode::Plain,
        )
        .unwrap();
    assert_eq!(source.seq, 1);

    let runtime =
        GraphRuntime::from_ports(ports.clone(), ports.clone(), ports, Arc::clone(&traversal))
            .unwrap();
    assert_eq!(traversal.last_reduced_seq().unwrap(), 0);
    assert!(traversal.get_fact("traversal::fact::1").unwrap().is_none());
    assert!(traversal.current_anchor(&head).unwrap().is_none());
    assert!(meta.get("pending_derived_events").unwrap().is_none());

    runtime.catch_up().unwrap();

    let rebuilt = traversal
        .get_fact("traversal::fact::1")
        .unwrap()
        .expect("authority event rebuilt the colliding fact id");
    assert_eq!(rebuilt.event_type, "context.head_selected");
    let current = traversal
        .current_anchor(&head)
        .unwrap()
        .expect("authority event rebuilt the current anchor");
    assert_eq!(current.target, authority_frame);
    assert_eq!(
        meta.get("legacy_last_reduced_seq_evidence")
            .unwrap()
            .unwrap(),
        b"42".as_slice()
    );
    let records = authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: authority.ledger_identity(),
                after_seq: 0,
            },
            limit: 10,
        })
        .unwrap()
        .records;
    assert!(records
        .iter()
        .all(|record| record.event_type != "world_state.legacy_derived"));
    assert_eq!(
        records
            .iter()
            .filter(|record| record.event_type == "world_state.anchor_selected")
            .count(),
        1
    );
}

#[test]
fn graph_runtime_does_not_advance_cursor_before_derived_append_succeeds() {
    let temp_dir = tempfile::tempdir().unwrap();
    let event_path = temp_dir.path().join("events");
    let traversal_path = temp_dir.path().join("traversal");
    let authority = EventAuthority::open(
        sled::open(&event_path).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ledger_id = authority.ledger_identity();
    let ports = Arc::new(AuthorityGraphPorts::new(&authority));
    let traversal = TraversalStore::shared(sled::open(&traversal_path).unwrap()).unwrap();
    authority
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
    let failing = GraphRuntime::from_ports(
        ports.clone(),
        Arc::new(FailingDerivedSink {
            ledger_id: authority.ledger_identity(),
        }),
        ports.clone(),
        Arc::clone(&traversal),
    )
    .unwrap();

    assert!(matches!(
        failing.catch_up(),
        Err(meld_world_model::error::StorageError::Unavailable(_))
    ));
    assert_eq!(failing.durable_event_cursor().unwrap().after_seq, 0);
    assert!(authority
        .consumer_registry_capability()
        .get("world_state.graph.reducer")
        .unwrap()
        .is_none());
    drop(failing);
    drop(ports);
    drop(traversal);
    drop(authority);

    let reopened_authority = EventAuthority::open(
        sled::open(&event_path).unwrap(),
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(ledger_id),
        },
    )
    .unwrap();
    let reopened_ports = Arc::new(AuthorityGraphPorts::new(&reopened_authority));
    let reopened_traversal = TraversalStore::shared(sled::open(&traversal_path).unwrap()).unwrap();
    let retry = GraphRuntime::from_ports(
        reopened_ports.clone(),
        reopened_ports.clone(),
        reopened_ports,
        reopened_traversal,
    )
    .unwrap();
    let report = retry
        .catch_up_bounded(
            meld_world_model::world_state::graph::runtime::GraphCatchUpBudget { max_items: 10 },
        )
        .unwrap();
    assert_eq!(report.input_event_seq, 0);
    assert_eq!(report.derived_events_appended, 1);
    let derived: Vec<_> = reopened_authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id,
                after_seq: 0,
            },
            limit: 10,
        })
        .unwrap()
        .records
        .into_iter()
        .filter(|record| record.event_type == "world_state.anchor_selected")
        .collect();
    assert_eq!(derived.len(), 1);
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

    let result =
        GraphRuntime::from_ports(first_ports.clone(), second_ports, first_ports, traversal);

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
        let runtime =
            GraphRuntime::from_ports(replay, ports.clone(), ports.clone(), traversal).unwrap();

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
fn graph_runtime_rejects_foreign_derived_receipt_without_advancing() {
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
    authority
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
    let runtime = GraphRuntime::from_ports(
        ports.clone(),
        Arc::new(ForeignReceiptSink {
            inner: authority.append_capability(),
            foreign_ledger_id: foreign.ledger_identity(),
        }),
        ports,
        TraversalStore::shared(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        runtime.catch_up(),
        Err(meld_world_model::error::StorageError::IdentityMismatch {
            expected,
            actual,
        }) if expected == authority.ledger_identity() && actual == foreign.ledger_identity()
    ));
    assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, 0);
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
    let existing_fact = TraversalFactRecord {
        fact_id: "pre-gap-fact".to_string(),
        source_spine_fact_id: "spine::77".to_string(),
        seq: 77,
        event_type: "existing.projection".to_string(),
        objects: vec![object("context", "frame", "existing")],
        relations: Vec::new(),
    };
    traversal.put_fact(&existing_fact).unwrap();
    traversal.flush().unwrap();
    let runtime =
        GraphRuntime::from_ports(ports.clone(), ports.clone(), ports, Arc::clone(&traversal))
            .unwrap();

    let report = runtime
        .catch_up_bounded(
            meld_world_model::world_state::graph::runtime::GraphCatchUpBudget { max_items: 10 },
        )
        .unwrap();
    assert_eq!(report.fatal_errors[0].code, "retention_gap");
    assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, 0);
    assert_eq!(
        traversal.get_fact("pre-gap-fact").unwrap(),
        Some(existing_fact.clone())
    );
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
    assert_eq!(
        traversal.get_fact("pre-gap-fact").unwrap(),
        Some(existing_fact)
    );
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
        ports,
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
    assert_eq!(derived, 1);
}

#[test]
fn traversal_reducer_reports_applied_events_and_ignores_other_domains() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("runtime")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = TraversalStore::new(db).unwrap();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    ledger
        .append_envelope(event(
            "context",
            "context.head_selected",
            vec![head.clone(), node.clone(), frame],
            Vec::new(),
        ))
        .unwrap();
    ledger
        .append_envelope(event(
            "context",
            "context.head_tombstoned",
            vec![head],
            Vec::new(),
        ))
        .unwrap();
    ledger
        .append_envelope(event(
            "unrelated",
            "unrelated.event",
            vec![node],
            Vec::new(),
        ))
        .unwrap();

    let reducer = TraversalReducer::replay_records(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        0,
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();

    assert_eq!(reducer.applied_events, 2);
    assert_eq!(reducer.last_seen_seq, 3);
    assert_eq!(store.last_reduced_seq().unwrap(), 0);
}

#[test]
fn traversal_reducer_anchor_events_carry_anchor_records() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("runtime")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = TraversalStore::new(db).unwrap();
    let node = object("workspace_fs", "node", "node-a");
    let head = object("context", "head", "node-a::analysis");
    let first_frame = object("context", "frame", "frame-a");
    let second_frame = object("context", "frame", "frame-b");
    ledger
        .append_envelope(event(
            "context",
            "context.head_selected",
            vec![head.clone(), node.clone(), first_frame],
            Vec::new(),
        ))
        .unwrap();
    ledger
        .append_envelope(event(
            "context",
            "context.head_selected",
            vec![head, node, second_frame],
            Vec::new(),
        ))
        .unwrap();

    let reducer = TraversalReducer::replay_records(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        0,
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();
    let selected = reducer
        .emitted_envelopes
        .iter()
        .find(|envelope| envelope.event_type == "world_state.anchor_selected")
        .expect("selected anchor event");
    let superseded = reducer
        .emitted_envelopes
        .iter()
        .find(|envelope| envelope.event_type == "world_state.anchor_superseded")
        .expect("superseded anchor event");

    assert!(selected.data.get("anchor").is_some());
    assert!(selected.data.get("anchor_id").is_none());
    assert!(selected.data.get("perspective_kind").is_none());
    assert!(superseded.data.get("anchor").is_some());
    assert!(superseded.data.get("superseded_by_anchor_id").is_none());
    assert!(superseded
        .data
        .get("anchor")
        .and_then(|anchor| anchor.get("ended_by_anchor_id"))
        .is_some());
    let ledger_id = selected.provenance.source_records[0].ledger_id;
    assert_eq!(
        selected.provenance.source_records,
        vec![EventRecordRef { ledger_id, seq: 1 }]
    );
    assert_eq!(
        superseded.provenance.source_records,
        vec![EventRecordRef { ledger_id, seq: 2 }]
    );
}

#[test]
fn traversal_reducer_replays_existing_anchor_over_equal_seq_current() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("runtime")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = TraversalStore::new(db).unwrap();
    let node = object("workspace_fs", "node", "node-a");
    let old_frame = object("context", "frame", "frame-old");
    let replayed_frame = object("context", "frame", "frame-replayed");
    let head = object("context", "head", "node-a::analysis");
    let mut current = frame_anchor("anchor-current", &node, &old_frame, 5);
    current.anchor_ref = head.clone();
    let mut existing = frame_anchor(
        &format!("anchor::{}::5", head.index_key()),
        &node,
        &replayed_frame,
        5,
    );
    existing.anchor_ref = head.clone();
    store.put_anchor(&current).unwrap();
    store.set_current_anchor(&current).unwrap();
    store.put_anchor(&existing).unwrap();
    ledger
        .append_event(&event_record(
            5,
            "context",
            "context.head_selected",
            vec![head.clone(), node.clone(), replayed_frame.clone()],
        ))
        .unwrap();

    TraversalReducer::replay_records(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        0,
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();

    assert_eq!(store.current_anchor(&head).unwrap().unwrap(), existing);
}

#[test]
fn traversal_reducer_ignores_older_existing_anchor_than_current() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("runtime")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = TraversalStore::new(db).unwrap();
    let node = object("workspace_fs", "node", "node-a");
    let current_frame = object("context", "frame", "frame-current");
    let older_frame = object("context", "frame", "frame-older");
    let head = object("context", "head", "node-a::analysis");
    let mut current = frame_anchor("anchor-current", &node, &current_frame, 10);
    current.anchor_ref = head.clone();
    let mut older = frame_anchor(
        &format!("anchor::{}::5", head.index_key()),
        &node,
        &older_frame,
        5,
    );
    older.anchor_ref = head.clone();
    store.put_anchor(&current).unwrap();
    store.set_current_anchor(&current).unwrap();
    store.put_anchor(&older).unwrap();
    ledger
        .append_event(&event_record(
            5,
            "context",
            "context.head_selected",
            vec![head.clone(), node.clone(), older_frame],
        ))
        .unwrap();

    TraversalReducer::replay_records(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        0,
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();

    assert_eq!(store.current_anchor(&head).unwrap().unwrap(), current);
}

#[test]
fn traversal_store_persists_reducer_cursor() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("graph");
    {
        let store = TraversalStore::new(sled::open(&path).unwrap()).unwrap();
        store.set_last_reduced_seq(42).unwrap();
        store.flush().unwrap();
    }
    let reopened = TraversalStore::new(sled::open(&path).unwrap()).unwrap();

    assert_eq!(reopened.last_reduced_seq().unwrap(), 42);
}

#[test]
fn graph_runtime_tombstone_clears_current_head() {
    let temp_dir = tempfile::tempdir().unwrap();
    let fixture =
        GraphRuntimeTestFixture::open(sled::open(temp_dir.path().join("runtime")).unwrap())
            .unwrap();
    let runtime = fixture.runtime();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    fixture
        .append(event(
            "context",
            "context.head_selected",
            vec![head.clone(), node.clone(), frame],
            Vec::new(),
        ))
        .unwrap();
    fixture
        .append(event(
            "context",
            "context.head_tombstoned",
            vec![head, node.clone()],
            Vec::new(),
        ))
        .unwrap();

    let queries = WorldModelQueries::new(runtime);

    assert!(queries
        .current_frame_head(&node, "analysis")
        .unwrap()
        .is_none());
}

#[test]
fn world_state_store_reads_current_history_evidence_and_supersession() {
    let (_temp_dir, store) = world_state_store();
    let subject = object("workspace_fs", "node", "node-a");
    let first = generation_claim("claim-a", ClaimKind::GenerationSucceeded, &subject, 1);
    let mut second = generation_claim("claim-b", ClaimKind::GenerationFailed, &subject, 2);
    second.superseded_by = None;
    let evidence = EvidenceRecord {
        evidence_id: "evidence-a".to_string(),
        claim_id: first.claim_id.clone(),
        source_fact_id: "spine::1".to_string(),
        source_event_type: "execution.control.node_completed".to_string(),
        objects: vec![subject.clone()],
        relations: Vec::new(),
    };

    store.put_claim(&second).unwrap();
    store.put_claim(&first).unwrap();
    store.set_claim_active(&subject, &second.claim_id).unwrap();
    store.put_evidence(&evidence).unwrap();
    store
        .put_supersession(&first.claim_id, &second.claim_id)
        .unwrap();

    assert_eq!(
        store.current_claims_for_object(&subject).unwrap(),
        vec![second.clone()]
    );
    assert_eq!(
        store.claim_history_for_object(&subject).unwrap(),
        vec![first.clone(), second.clone()]
    );
    assert_eq!(
        store.evidence_for_claim(&first.claim_id).unwrap(),
        vec![evidence]
    );
    assert_eq!(
        store
            .supersession_chain_for_claim(&first.claim_id)
            .unwrap()
            .len(),
        1
    );

    let query = WorldStateQuery::new(&store);
    assert_eq!(
        query.claim_history_for_object(&subject).unwrap(),
        vec![first.clone(), second.clone()]
    );
    assert_eq!(
        query.supersession_chain_for_claim(&first.claim_id).unwrap(),
        vec![second]
    );
}

#[test]
fn world_state_query_summarizes_provenance() {
    let (_temp_dir, store) = world_state_store();
    let subject = object("workspace_fs", "node", "node-a");
    let workflow = object("execution", "workflow", "workflow-a");
    let relation = EventRelation::new("produced", workflow.clone(), subject.clone()).unwrap();
    let claim = generation_claim("claim-a", ClaimKind::GenerationSucceeded, &subject, 1);
    let evidence = EvidenceRecord {
        evidence_id: "evidence-a".to_string(),
        claim_id: claim.claim_id.clone(),
        source_fact_id: "spine::1".to_string(),
        source_event_type: "execution.control.node_completed".to_string(),
        objects: vec![subject.clone(), workflow],
        relations: vec![relation.clone()],
    };

    store.put_claim(&claim).unwrap();
    store.put_evidence(&evidence).unwrap();

    let provenance = WorldStateQuery::new(&store)
        .provenance_for_claim(&claim.claim_id)
        .unwrap();

    assert_eq!(provenance.evidence_ids, vec!["evidence-a"]);
    assert_eq!(provenance.source_fact_ids, vec!["spine::1"]);
    assert_eq!(provenance.objects.len(), 2);
    assert_eq!(provenance.relations, vec![relation]);
}

#[test]
fn world_state_reducer_materializes_claim_and_evidence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("world")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = WorldStateStore::new(db).unwrap();
    let node = object("workspace_fs", "node", "node-a");
    ledger
        .append_envelope(event(
            "execution",
            "execution.control.node_completed",
            vec![node.clone()],
            Vec::new(),
        ))
        .unwrap();

    let reducer = WorldStateReducer::replay_events(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();
    let current = store.current_claims_for_object(&node).unwrap();
    let fact = store
        .get_fact(&format!(
            "world_state::claim_added::{}",
            current[0].claim_id
        ))
        .unwrap()
        .unwrap();

    assert_eq!(reducer.emitted_envelopes.len(), 2);
    let claim_event = &reducer.emitted_envelopes[0];
    let evidence_event = &reducer.emitted_envelopes[1];
    let source_record = EventRecordRef {
        ledger_id: ledger.compatibility_ledger_identity().unwrap(),
        seq: 1,
    };
    assert_eq!(claim_event.provenance.source_records, vec![source_record]);
    assert_eq!(
        evidence_event.provenance.source_records,
        vec![source_record]
    );

    assert!(claim_event.data.get("claim").is_some());
    assert!(claim_event.data.get("claim_id").is_none());
    assert!(claim_event.data.get("subject").is_none());
    assert_eq!(
        claim_event
            .data
            .get("claim")
            .and_then(|claim| claim.get("claim_id"))
            .and_then(serde_json::Value::as_str),
        Some(current[0].claim_id.as_str())
    );
    assert!(evidence_event.data.get("evidence").is_some());
    assert!(evidence_event.data.get("evidence_id").is_none());
    assert!(evidence_event.data.get("claim_id").is_none());
    assert_eq!(
        evidence_event
            .data
            .get("evidence")
            .and_then(|evidence| evidence.get("claim_id"))
            .and_then(serde_json::Value::as_str),
        Some(current[0].claim_id.as_str())
    );
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].claim_kind, ClaimKind::GenerationSucceeded);
    assert_eq!(current[0].subject, node);
    assert_eq!(current[0].supporting_fact_ids, vec!["spine::1"]);
    assert_eq!(fact.source_spine_fact_id.as_deref(), Some("spine::1"));
    assert_eq!(
        store
            .evidence_for_claim(&current[0].claim_id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn world_state_reducer_materializes_artifact_claims() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("world")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = WorldStateStore::new(db).unwrap();
    let task_run = object("execution", "task_run", "run-a");
    ledger
        .append_envelope(event(
            "execution",
            "execution.task.artifact_emitted",
            vec![task_run.clone()],
            Vec::new(),
        ))
        .unwrap();

    WorldStateReducer::replay_events(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();
    let current = store.current_claims_for_object(&task_run).unwrap();

    assert_eq!(current.len(), 1);
    assert_eq!(current[0].claim_kind, ClaimKind::ArtifactAvailable);
    assert_eq!(current[0].supporting_fact_ids, vec!["spine::1"]);
}

#[test]
fn world_state_reducer_finds_object_by_domain_and_kind() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("world")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = WorldStateStore::new(db).unwrap();
    let wrong_domain = object("execution", "node", "wrong-domain");
    let wrong_kind = object("workspace_fs", "artifact", "wrong-kind");
    let node = object("workspace_fs", "node", "node-a");
    ledger
        .append_envelope(event(
            "execution",
            "execution.control.node_completed",
            vec![wrong_domain, wrong_kind, node.clone()],
            Vec::new(),
        ))
        .unwrap();

    WorldStateReducer::replay_events(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();
    let current = store.current_claims_for_object(&node).unwrap();

    assert_eq!(current.len(), 1);
    assert_eq!(current[0].subject, node);
}

#[test]
fn world_state_reducer_supersedes_conflicting_generation_claims() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("world")).unwrap();
    let ledger = EventStore::new(db.clone()).unwrap();
    let store = WorldStateStore::new(db).unwrap();
    let node = object("workspace_fs", "node", "node-a");
    ledger
        .append_envelope(event(
            "execution",
            "execution.control.node_completed",
            vec![node.clone()],
            Vec::new(),
        ))
        .unwrap();
    ledger
        .append_envelope(event(
            "execution",
            "execution.control.node_failed",
            vec![node.clone()],
            Vec::new(),
        ))
        .unwrap();

    let reducer = WorldStateReducer::replay_events(
        &store,
        ledger.compatibility_ledger_identity().unwrap(),
        ledger.read_all_events_after(0).unwrap(),
    )
    .unwrap();
    let current = store.current_claims_for_object(&node).unwrap();
    let history = store.claim_history_for_object(&node).unwrap();
    let superseded_event = reducer
        .emitted_envelopes
        .iter()
        .find(|envelope| envelope.event_type == "world_state.claim_superseded")
        .expect("claim superseded event");

    assert_eq!(reducer.emitted_envelopes.len(), 5);
    assert!(superseded_event.data.get("claim").is_some());
    assert!(superseded_event.data.get("claim_id").is_none());
    assert!(superseded_event.data.get("superseded_by").is_none());
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].claim_kind, ClaimKind::GenerationFailed);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].status, SettlementStatus::Superseded);
    assert_eq!(
        superseded_event
            .data
            .get("claim")
            .and_then(|claim| claim.get("superseded_by"))
            .and_then(serde_json::Value::as_str),
        Some(current[0].claim_id.as_str())
    );
    assert_eq!(
        store
            .supersession_chain_for_claim(&history[0].claim_id)
            .unwrap(),
        vec![current[0].clone()]
    );
}

#[test]
fn legacy_claim_adapter_maps_frame_anchors_to_active_claims() {
    let (_temp_dir, store) = traversal_store();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let anchor = frame_anchor("anchor-a", &node, &frame, 1);
    store.put_anchor(&anchor).unwrap();
    store.set_current_anchor(&anchor).unwrap();

    let claims = LegacyClaimAdapter::new(&store)
        .current_claims_for_object(&node)
        .unwrap();

    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].claim_id, "anchor-a");
    assert_eq!(claims[0].claim_kind, ClaimKind::GenerationSucceeded);
    assert_eq!(claims[0].status, SettlementStatus::Active);
    assert_eq!(claims[0].supporting_fact_ids, vec!["spine::1"]);
}
