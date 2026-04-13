use meld::telemetry::{DomainObjectRef, EventRelation};
use meld::world_state::contracts::{
    ClaimKind, ClaimRecord, EvidenceRecord, SettlementStatus,
};
use meld::world_state::query::WorldStateQuery;
use meld::world_state::store::{StoredWorldStateFact, WorldStateStore};

#[test]
fn world_state_records_round_trip() {
    let claim = ClaimRecord {
        claim_id: "claim_a".to_string(),
        claim_kind: ClaimKind::GenerationSucceeded,
        subject: DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap(),
        status: SettlementStatus::Active,
        supporting_fact_ids: vec!["fact_a".to_string()],
        superseded_by: None,
        created_by_fact_id: "fact_a".to_string(),
        created_at_seq: 1,
        last_updated_seq: 1,
    };

    let serialized = serde_json::to_string(&claim).unwrap();
    let parsed: ClaimRecord = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed.claim_id, "claim_a");
    assert_eq!(parsed.claim_kind, ClaimKind::GenerationSucceeded);
}

#[test]
fn fact_store_indexes_claims_by_object_ref() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let store = WorldStateStore::new(db).unwrap();
    let subject = DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap();
    let claim = ClaimRecord {
        claim_id: "claim_a".to_string(),
        claim_kind: ClaimKind::GenerationSucceeded,
        subject: subject.clone(),
        status: SettlementStatus::Active,
        supporting_fact_ids: vec!["fact_a".to_string()],
        superseded_by: None,
        created_by_fact_id: "fact_a".to_string(),
        created_at_seq: 1,
        last_updated_seq: 1,
    };
    let evidence = EvidenceRecord {
        evidence_id: "evidence_a".to_string(),
        claim_id: "claim_a".to_string(),
        source_fact_id: "spine_fact_a".to_string(),
        source_event_type: "execution.control.node_completed".to_string(),
        objects: vec![
            subject.clone(),
            DomainObjectRef::new("context", "frame", "frame_a").unwrap(),
        ],
        relations: vec![EventRelation::new(
            "produced",
            subject.clone(),
            DomainObjectRef::new("context", "frame", "frame_a").unwrap(),
        )
        .unwrap()],
    };

    store
        .put_fact(&StoredWorldStateFact {
            fact_id: "fact_a".to_string(),
            event_type: "world_state.claim_added".to_string(),
            claim_id: Some("claim_a".to_string()),
            evidence_id: None,
            source_spine_fact_id: Some("spine_fact_a".to_string()),
            seq: 10,
        })
        .unwrap();
    store.put_claim(&claim).unwrap();
    store.set_claim_active(&subject, &claim.claim_id).unwrap();
    store.put_evidence(&evidence).unwrap();

    let query = WorldStateQuery::new(&store);
    let current = query.current_claims_for_object(&subject).unwrap();
    let provenance = query.provenance_for_claim("claim_a").unwrap();

    assert_eq!(current.len(), 1);
    assert_eq!(current[0].claim_id, "claim_a");
    assert_eq!(provenance.evidence_ids, vec!["evidence_a".to_string()]);
    assert_eq!(provenance.source_fact_ids, vec!["spine_fact_a".to_string()]);
}

#[test]
fn supersession_chain_remains_queryable() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let store = WorldStateStore::new(db).unwrap();
    let subject = DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap();

    let claim_a = ClaimRecord {
        claim_id: "claim_a".to_string(),
        claim_kind: ClaimKind::GenerationFailed,
        subject: subject.clone(),
        status: SettlementStatus::Superseded,
        supporting_fact_ids: vec!["fact_a".to_string()],
        superseded_by: Some("claim_b".to_string()),
        created_by_fact_id: "fact_a".to_string(),
        created_at_seq: 1,
        last_updated_seq: 2,
    };
    let claim_b = ClaimRecord {
        claim_id: "claim_b".to_string(),
        claim_kind: ClaimKind::GenerationSucceeded,
        subject: subject.clone(),
        status: SettlementStatus::Active,
        supporting_fact_ids: vec!["fact_b".to_string()],
        superseded_by: None,
        created_by_fact_id: "fact_b".to_string(),
        created_at_seq: 2,
        last_updated_seq: 2,
    };

    store.put_claim(&claim_a).unwrap();
    store.put_claim(&claim_b).unwrap();
    store.put_supersession("claim_a", "claim_b").unwrap();

    let query = WorldStateQuery::new(&store);
    let chain = query.supersession_chain_for_claim("claim_a").unwrap();

    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].claim_id, "claim_b");
}
