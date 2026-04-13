use meld::telemetry::{DomainObjectRef, EventRelation};
use meld::telemetry::events::ProgressEvent;
use meld::telemetry::sinks::store::ProgressStore;
use meld::control::events::{
    node_completed_envelope, node_failed_envelope, NodeCompletedEventData, NodeFailedEventData,
};
use meld::task::{build_execution_task_envelope, TaskEvent};
use meld::world_state::contracts::{
    ClaimKind, ClaimRecord, EvidenceRecord, SettlementStatus,
};
use meld::world_state::query::WorldStateQuery;
use meld::world_state::reducer::WorldStateReducer;
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

#[test]
fn replay_rebuilds_current_claim_projection() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let spine = ProgressStore::new(db.clone()).unwrap();
    let world_state = WorldStateStore::new(db).unwrap();

    spine
        .append_event(&ProgressEvent::from_envelope(
            node_completed_envelope(
                "session_a",
                NodeCompletedEventData {
                    plan_id: "plan_a".to_string(),
                    level_index: 0,
                    node_id: "node_a".to_string(),
                    path: "/tmp/a".to_string(),
                    frame_id: "frame_a".to_string(),
                    program_kind: "workflow".to_string(),
                    workflow_id: None,
                },
            ),
            1,
        ))
        .unwrap();

    let reducer = WorldStateReducer::replay_from_spine(&spine, &world_state, 0).unwrap();
    let query = WorldStateQuery::new(&world_state);
    let current = query
        .current_claims_for_object(&DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap())
        .unwrap();

    assert_eq!(reducer.current_claims.last_applied_seq, 1);
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].claim_kind, ClaimKind::GenerationSucceeded);
}

#[test]
fn later_generation_success_supersedes_prior_failure() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let spine = ProgressStore::new(db.clone()).unwrap();
    let world_state = WorldStateStore::new(db).unwrap();

    spine
        .append_event(&ProgressEvent::from_envelope(
            node_failed_envelope(
                "session_a",
                NodeFailedEventData {
                    plan_id: "plan_a".to_string(),
                    level_index: 0,
                    node_id: "node_a".to_string(),
                    path: "/tmp/a".to_string(),
                    error: "boom".to_string(),
                    program_kind: "workflow".to_string(),
                    workflow_id: None,
                },
            ),
            1,
        ))
        .unwrap();
    spine
        .append_event(&ProgressEvent::from_envelope(
            node_completed_envelope(
                "session_a",
                NodeCompletedEventData {
                    plan_id: "plan_a".to_string(),
                    level_index: 0,
                    node_id: "node_a".to_string(),
                    path: "/tmp/a".to_string(),
                    frame_id: "frame_a".to_string(),
                    program_kind: "workflow".to_string(),
                    workflow_id: None,
                },
            ),
            2,
        ))
        .unwrap();

    let reducer = WorldStateReducer::replay_from_spine(&spine, &world_state, 0).unwrap();
    let subject = DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap();
    let current = WorldStateQuery::new(&world_state)
        .current_claims_for_object(&subject)
        .unwrap();
    let history = WorldStateQuery::new(&world_state)
        .claim_history_for_object(&subject)
        .unwrap();

    assert_eq!(current.len(), 1);
    assert_eq!(current[0].claim_kind, ClaimKind::GenerationSucceeded);
    assert_eq!(history.len(), 2);
    assert!(history.iter().any(|claim| claim.status == SettlementStatus::Superseded));
    assert!(reducer
        .provenance
        .supersession_chain_by_claim
        .values()
        .any(|chain| !chain.is_empty()));
}

#[test]
fn provenance_query_returns_supporting_execution_fact() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let spine = ProgressStore::new(db.clone()).unwrap();
    let world_state = WorldStateStore::new(db).unwrap();
    let mut task_event = TaskEvent::new("task_artifact_emitted", "task_a", "run_a");
    task_event.artifact_id = Some("artifact_a".to_string());

    spine
        .append_event(&ProgressEvent::from_envelope(
            build_execution_task_envelope("session_a", &task_event).unwrap(),
            1,
        ))
        .unwrap();

    let _ = WorldStateReducer::replay_from_spine(&spine, &world_state, 0).unwrap();
    let provenance = WorldStateQuery::new(&world_state)
        .provenance_for_claim("claim::artifact_available::execution::task_run::run_a::1")
        .unwrap();

    assert_eq!(provenance.source_fact_ids, vec!["spine::1".to_string()]);
    assert_eq!(provenance.objects.len(), 2);
    assert_eq!(provenance.relations.len(), 1);
}
