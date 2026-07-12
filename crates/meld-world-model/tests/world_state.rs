use meld_world_model::events::store::EventStore;
use std::str::FromStr;

use meld_world_model::events::{
    DomainObjectRef, EventEnvelope, EventRecord, EventRecordRef, EventRelation, LedgerIdentity,
};
use meld_world_model::world_state::graph::compat::LegacyClaimAdapter;
use meld_world_model::world_state::graph::events::{
    anchor_selected_envelope_from_record, anchor_superseded_envelope_from_record,
    AnchorSelectedEventData, AnchorSupersededEventData,
};
use meld_world_model::world_state::graph::runtime::GraphRuntime;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::reducer::WorldStateReducer;
use meld_world_model::world_state::store::{StoredWorldStateFact, WorldStateStore};
use meld_world_model::{
    AnchorSelectionRecord, ClaimKind, ClaimRecord, EvidenceRecord, GraphWalkSpec, PerspectiveKey,
    SettlementStatus, TraversalDirection, TraversalFactRecord, TraversalQuery, WorldModelQueries,
    WorldStateQuery,
};
use serde_json::json;

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
    let runtime = std::sync::Arc::new(
        GraphRuntime::new(sled::open(temp_dir.path().join("runtime")).unwrap()).unwrap(),
    );
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    runtime
        .append_envelope(event(
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
    let runtime = GraphRuntime::new(db.clone()).unwrap();
    let ledger_id = runtime.ledger_identity();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    let source_seq = runtime
        .append_envelope(event(
            "context",
            "context.head_selected",
            vec![head, node, frame],
            Vec::new(),
        ))
        .unwrap();

    runtime.catch_up().unwrap();

    let ledger = EventStore::new(db).unwrap();
    let derived = ledger
        .read_all_events_after(source_seq)
        .unwrap()
        .into_iter()
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

    let reducer =
        meld_world_model::world_state::graph::reducer::TraversalReducer::replay_from_ledger(
            &ledger, &store, 0,
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

    let reducer =
        meld_world_model::world_state::graph::reducer::TraversalReducer::replay_from_ledger(
            &ledger, &store, 0,
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

    meld_world_model::world_state::graph::reducer::TraversalReducer::replay_from_ledger(
        &ledger, &store, 0,
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

    meld_world_model::world_state::graph::reducer::TraversalReducer::replay_from_ledger(
        &ledger, &store, 0,
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
    let runtime = std::sync::Arc::new(
        GraphRuntime::new(sled::open(temp_dir.path().join("runtime")).unwrap()).unwrap(),
    );
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let head = object("context", "head", "node-a::analysis");
    runtime
        .append_envelope(event(
            "context",
            "context.head_selected",
            vec![head.clone(), node.clone(), frame],
            Vec::new(),
        ))
        .unwrap();
    runtime
        .append_envelope(event(
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

    let reducer = WorldStateReducer::replay_from_ledger(&ledger, &store, 0).unwrap();
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

    WorldStateReducer::replay_from_ledger(&ledger, &store, 0).unwrap();
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

    WorldStateReducer::replay_from_ledger(&ledger, &store, 0).unwrap();
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

    let reducer = WorldStateReducer::replay_from_ledger(&ledger, &store, 0).unwrap();
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
