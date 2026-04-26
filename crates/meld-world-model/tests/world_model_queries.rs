use meld_world_model::events::{DomainObjectRef, EventRelation};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::store::WorldStateStore;
use meld_world_model::{
    AnchorSelectionRecord, ClaimKind, ClaimRecord, EvidenceRecord, GraphWalkSpec, PerspectiveKey,
    SettlementStatus, TraversalDirection, TraversalFactRecord, TraversalQuery, WorldStateQuery,
};

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

#[test]
fn world_state_query_reads_claims_and_provenance() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = WorldStateStore::new(sled::open(temp_dir.path().join("world")).unwrap()).unwrap();
    let subject = object("execution", "task_run", "run-a");
    let source = object("execution", "workflow", "workflow-a");

    let claim = ClaimRecord {
        claim_id: "claim-a".to_string(),
        claim_kind: ClaimKind::GenerationSucceeded,
        subject: subject.clone(),
        status: SettlementStatus::Active,
        supporting_fact_ids: vec!["fact-a".to_string()],
        superseded_by: None,
        created_by_fact_id: "fact-a".to_string(),
        created_at_seq: 1,
        last_updated_seq: 1,
    };
    let evidence = EvidenceRecord {
        evidence_id: "evidence-a".to_string(),
        claim_id: claim.claim_id.clone(),
        source_fact_id: "spine-a".to_string(),
        source_event_type: "execution.task.completed".to_string(),
        objects: vec![subject.clone(), source],
        relations: Vec::new(),
    };

    store.put_claim(&claim).unwrap();
    store.set_claim_active(&subject, &claim.claim_id).unwrap();
    store.put_evidence(&evidence).unwrap();

    let query = WorldStateQuery::new(&store);
    let current_claims = query.current_claims_for_object(&subject).unwrap();
    let provenance = query.provenance_for_claim(&claim.claim_id).unwrap();

    assert_eq!(current_claims, vec![claim]);
    assert_eq!(provenance.evidence_ids, vec!["evidence-a"]);
    assert_eq!(provenance.source_fact_ids, vec!["spine-a"]);
    assert_eq!(provenance.objects.len(), 2);
}

#[test]
fn traversal_query_reads_current_anchor_and_neighbors() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = TraversalStore::new(sled::open(temp_dir.path().join("graph")).unwrap()).unwrap();
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let anchor_ref = object("context", "head", "node-a::analysis");
    let relation = EventRelation::new("selected", node.clone(), frame.clone()).unwrap();

    let fact = TraversalFactRecord {
        fact_id: "fact-a".to_string(),
        source_spine_fact_id: "spine-a".to_string(),
        seq: 1,
        event_type: "context.head.selected".to_string(),
        objects: vec![node.clone(), frame.clone()],
        relations: vec![relation],
    };
    let anchor = AnchorSelectionRecord {
        anchor_id: "anchor-a".to_string(),
        anchor_ref: anchor_ref.clone(),
        subject: node.clone(),
        perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
        target: frame.clone(),
        source_fact_ids: vec!["spine-a".to_string()],
        created_by_fact_id: "fact-a".to_string(),
        selected_at_seq: 1,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };

    store.put_fact(&fact).unwrap();
    store.put_anchor(&anchor).unwrap();
    store.set_current_anchor(&anchor).unwrap();

    let query = TraversalQuery::new(&store);
    let current = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let neighbors = query
        .neighbors(&node, TraversalDirection::Outgoing, None, false)
        .unwrap();
    let walk = query
        .walk(
            &node,
            &GraphWalkSpec {
                direction: TraversalDirection::Outgoing,
                relation_types: None,
                max_depth: 1,
                current_only: false,
                include_facts: true,
            },
        )
        .unwrap();

    assert_eq!(current, anchor);
    assert_eq!(neighbors, vec![frame]);
    assert_eq!(walk.visited_facts, vec![fact]);
}
