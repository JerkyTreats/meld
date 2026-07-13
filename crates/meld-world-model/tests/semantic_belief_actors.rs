use std::collections::BTreeMap;
use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope, EventPage,
    EventReplayCapability, LedgerIdentity, ReplayRequest,
};
use meld_world_model::belief::{
    BeliefAssessmentActor, BeliefAssessmentRequest, BeliefConfigLoader, BeliefDirtyKeyTickRequest,
    BeliefEvidenceNormalizer, BeliefStore, BranchScope, EvidenceEventReplaySource,
    EvidenceIngestionActor, EvidenceIngestionActorRequest, EvidenceIngestionReceiptDisposition,
    EvidenceIngestionReceiptWriteDisposition, EvidenceValue, LeaseStatus, PromotedEvidenceRecord,
};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::PerspectiveKey;
use serde_json::json;

const CONFIG_JSON: &str = r#"{
  "family_id": "docs_freshness",
  "dimension_id": "docs_freshness",
  "predicate_id": "confidence",
  "evidence_policy_id": "default_policy",
  "evidence_schemas": [
    {
      "schema_id": "content_written_signal",
      "required": false,
      "role": "Support",
      "reliability": 1.0,
      "precision": 1.0
    }
  ],
  "source_mappings": [
    {
      "mapping_id": "content_written_to_signal",
      "source_kind": "content_written",
      "evidence_schema_id": "content_written_signal",
      "subject_from": "record.subject",
      "value_field": "stale_probability",
      "factor_id": "content_written_signal"
    }
  ],
  "comparator": {
    "engine_id": "weighted_bayesian",
    "engine_version": "1",
    "factors": [
      {
        "factor_id": "content_written_signal",
        "evidence_schema_id": "content_written_signal",
        "weight": 1.0,
        "polarity": "Supports"
      }
    ],
    "missing_evidence_uncertainty": 0.9
  },
  "default_prior": 0.8,
  "planner_projection": {
    "confidence_field": "confidence",
    "threshold": 0.7,
    "posterior_meaning": "stale_probability"
  },
  "config_version": "1"
}"#;

#[derive(Clone)]
struct ReplayPort {
    replay: EventReplayCapability,
}

impl EvidenceEventReplaySource for ReplayPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

#[test]
fn assessment_actor_selects_dirty_keys_deterministically_and_uses_supervisor_lease() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
    let traversal = Arc::new(TraversalStore::new(db).unwrap());
    let config = BeliefConfigLoader::load_json(CONFIG_JSON).unwrap();
    let perspective = PerspectiveKey::new("agent", "default").unwrap();
    let branch_scope = BranchScope::main();
    let normalizer = BeliefEvidenceNormalizer::new(
        config.config.clone(),
        perspective.clone(),
        branch_scope.clone(),
    );
    let first = promoted_record("event-10", "node-a", 10);
    let second = promoted_record("event-20", "node-b", 20);
    for record in [&second, &first] {
        for item in normalizer.normalize_promoted(record).unwrap() {
            store.put_evidence_once(&item).unwrap();
            store
                .put_assignment_once(&normalizer.assign(&item).unwrap())
                .unwrap();
        }
    }
    let actor = BeliefAssessmentActor::new(
        Arc::clone(&store),
        traversal,
        config,
        perspective,
        branch_scope,
    );

    let report = actor.tick_dirty(BeliefDirtyKeyTickRequest {
        current_sequence: 0,
        max_items: 1,
        lease_owner_id: "supervisor-lease-assessment".to_string(),
    });

    assert_eq!(report.selected_count, 1);
    assert_eq!(report.committed_count, 1);
    assert!(report.budget_exhausted);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    let remaining = store.dirty_key_states().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].belief_key.subject.object_id, "node-b");
    let key = normalizer
        .normalize_promoted(&first)
        .unwrap()
        .remove(0)
        .candidate_key;
    let revision = store.current_revision(&key).unwrap().unwrap();
    let lease_id = format!("lease-1-10-{}", key.index_key());
    let lease = store.get_lease(&lease_id).unwrap().unwrap();
    assert_eq!(lease.owner_id, "supervisor-lease-assessment");
    assert_eq!(lease.status, LeaseStatus::Completed);
    assert_eq!(revision.source_cursor_end, 10);
}

#[test]
fn assessment_actor_reports_missing_anchor_as_retryable_no_work() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
    let traversal = Arc::new(TraversalStore::new(db).unwrap());
    let actor = BeliefAssessmentActor::new(
        store,
        traversal,
        BeliefConfigLoader::load_json(CONFIG_JSON).unwrap(),
        PerspectiveKey::new("agent", "default").unwrap(),
        BranchScope::main(),
    );

    let report = actor.assess_subject(BeliefAssessmentRequest {
        subject: DomainObjectRef::new("workspace_fs", "node", "missing").unwrap(),
        anchor_perspective_kind: "frame_type".to_string(),
        anchor_perspective_id: "analysis".to_string(),
        lease_owner_id: "supervisor-lease-assessment".to_string(),
    });

    assert_eq!(report.no_work_count, 1);
    assert_eq!(report.committed_count, 0);
    assert_eq!(report.retryable_errors.len(), 1);
    assert_eq!(report.retryable_errors[0].code, "missing_graph_anchor");
    assert!(report.fatal_errors.is_empty());
}

#[test]
fn evidence_actor_receipts_every_disposition_and_resumes_after_reopen() {
    let event_db = sled::Config::new().temporary(true).open().unwrap();
    let authority = EventAuthority::open(event_db, EventAuthorityOpenOptions::default()).unwrap();
    let append = authority.append_capability();
    append
        .append_durable(
            EventEnvelope::with_now_domain(
                "session-a",
                "runtime",
                "runtime-a",
                "runtime.heartbeat",
                None,
                json!({}),
            ),
            AppendMode::Plain,
        )
        .unwrap();
    append
        .append_durable(
            EventEnvelope::with_now_domain(
                "session-a",
                "execution",
                "task-a",
                "execution.task.failed",
                None,
                json!({"artifact_records": []}),
            ),
            AppendMode::Plain,
        )
        .unwrap();
    append
        .append_durable(
            EventEnvelope::with_now_domain(
                "session-a",
                "execution",
                "task-a",
                "execution.task.succeeded",
                None,
                json!({
                    "artifact_records": [{"artifact_type_id": "docs_patch"}]
                }),
            ),
            AppendMode::Plain,
        )
        .unwrap();
    let replay: Arc<dyn EvidenceEventReplaySource> = Arc::new(ReplayPort {
        replay: authority.replay_capability(),
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("world-model");
    let request = evidence_request(2);
    {
        let db = sled::open(&path).unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let actor = EvidenceIngestionActor::new(Arc::clone(&replay), store, traversal);

        let first = actor.tick(request.clone());

        assert_eq!(first.input_event_sequence, 0);
        assert_eq!(first.output_event_sequence, 2);
        assert!(first.budget_exhausted);
        assert_eq!(first.irrelevant_record_count, 2);
        assert_eq!(first.receipts.len(), 2);
        assert!(first.receipts.iter().all(|receipt| {
            receipt.disposition == EvidenceIngestionReceiptDisposition::Irrelevant
                && receipt.write_disposition == EvidenceIngestionReceiptWriteDisposition::Inserted
        }));
        assert_eq!(
            actor
                .durable_cursor(&request)
                .unwrap()
                .unwrap()
                .ledger_cursor
                .after_seq,
            2
        );
    }
    {
        let db = reopen_sled(&path);
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let actor = EvidenceIngestionActor::new(Arc::clone(&replay), Arc::clone(&store), traversal);
        let mut resumed_request = request.clone();
        resumed_request.max_items = 2;

        let resumed = actor.tick(resumed_request.clone());

        assert_eq!(resumed.input_event_sequence, 2);
        assert_eq!(resumed.output_event_sequence, 3);
        assert_eq!(resumed.promoted_record_count, 1);
        assert_eq!(resumed.normalized_evidence_count, 1);
        assert_eq!(resumed.new_assignment_count, 1);
        assert_eq!(resumed.committed_revision_count, 1);
        assert_eq!(resumed.receipts.len(), 1);
        assert_eq!(
            resumed.receipts[0].disposition,
            EvidenceIngestionReceiptDisposition::Promoted
        );
        assert!(resumed.retryable_errors.is_empty());
        assert!(resumed.fatal_errors.is_empty());
        let cursor = actor.durable_cursor(&resumed_request).unwrap().unwrap();
        assert_eq!(cursor.ledger_cursor.after_seq, 3);

        let key = BeliefEvidenceNormalizer::new(
            resumed_request.config.config.clone(),
            resumed_request.perspective.clone(),
            resumed_request.branch_scope.clone(),
        )
        .normalize_promoted(&promoted_record("event-spine::3", "node-a", 3))
        .unwrap()
        .remove(0)
        .candidate_key;
        let revision = store.current_revision(&key).unwrap().unwrap();
        let lease_id = format!("lease-1-3-{}", key.index_key());
        assert_eq!(
            store.get_lease(&lease_id).unwrap().unwrap().owner_id,
            "supervisor-lease-evidence"
        );
        assert_eq!(revision.source_cursor_end, 3);

        let no_work = actor.tick(resumed_request);
        assert_eq!(no_work.input_event_sequence, 3);
        assert_eq!(no_work.output_event_sequence, 3);
        assert_eq!(no_work.events_attempted, 0);
    }
}

#[test]
fn evidence_actor_persists_rejected_disposition_before_advancing() {
    let event_db = sled::Config::new().temporary(true).open().unwrap();
    let authority = EventAuthority::open(event_db, EventAuthorityOpenOptions::default()).unwrap();
    authority
        .append_capability()
        .append_durable(
            EventEnvelope::with_now_domain(
                "session-a",
                "execution",
                "task-a",
                "execution.task.succeeded",
                None,
                json!({
                    "artifact_records": [{"artifact_type_id": "docs_patch"}]
                }),
            ),
            AppendMode::Plain,
        )
        .unwrap();
    let replay: Arc<dyn EvidenceEventReplaySource> = Arc::new(ReplayPort {
        replay: authority.replay_capability(),
    });
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
    let traversal = Arc::new(TraversalStore::new(db).unwrap());
    let actor = EvidenceIngestionActor::new(replay, store, traversal);
    let mut request = evidence_request(1);
    request.config = BeliefConfigLoader::load_json(&CONFIG_JSON.replace(
        "\"value_field\": \"stale_probability\"",
        "\"value_field\": \"missing_probability\"",
    ))
    .unwrap();

    let report = actor.tick(request.clone());

    assert_eq!(report.output_event_sequence, 1);
    assert_eq!(report.rejected_record_count, 1);
    assert_eq!(report.normalized_evidence_count, 0);
    assert_eq!(report.receipts.len(), 1);
    assert_eq!(
        report.receipts[0].disposition,
        EvidenceIngestionReceiptDisposition::Rejected
    );
    assert_eq!(
        actor
            .durable_cursor(&request)
            .unwrap()
            .unwrap()
            .ledger_cursor
            .after_seq,
        1
    );
}

#[test]
fn config_snapshots_accept_exact_replay_and_reject_hash_conflicts() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = BeliefStore::new(db).unwrap();

    assert!(store.put_config_snapshot_once("hash-a", "one").unwrap());
    assert!(!store.put_config_snapshot_once("hash-a", "one").unwrap());
    assert!(store.put_config_snapshot_once("hash-a", "two").is_err());
    assert_eq!(
        store.get_config_snapshot("hash-a").unwrap().as_deref(),
        Some("one")
    );
}

fn evidence_request(max_items: usize) -> EvidenceIngestionActorRequest {
    EvidenceIngestionActorRequest {
        subject: DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
        config: BeliefConfigLoader::load_json(CONFIG_JSON).unwrap(),
        perspective: PerspectiveKey::new("agent", "default").unwrap(),
        branch_scope: BranchScope::main(),
        lease_owner_id: "supervisor-lease-evidence".to_string(),
        max_items,
        required_artifact_type_id: Some("docs_patch".to_string()),
    }
}

fn promoted_record(source_id: &str, object_id: &str, seq: u64) -> PromotedEvidenceRecord {
    let mut fields = BTreeMap::new();
    fields.insert("stale_probability".to_string(), EvidenceValue::Scalar(0.0));
    PromotedEvidenceRecord {
        source_kind: "content_written".to_string(),
        source_id: source_id.to_string(),
        subject: DomainObjectRef::new("workspace_fs", "node", object_id).unwrap(),
        source_fact_ids: vec![source_id.to_string()],
        graph_anchor_ids: Vec::new(),
        objects: Vec::new(),
        relations: Vec::new(),
        source_cursor_start: seq,
        source_cursor_end: seq,
        reference_time: None,
        transaction_seq: seq,
        content_hash: None,
        fields,
    }
}

fn reopen_sled(path: &std::path::Path) -> sled::Db {
    for _ in 0..100 {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error) if error.to_string().contains("could not acquire lock") => {
                std::thread::yield_now();
            }
            Err(error) => panic!("failed to reopen sled database: {error}"),
        }
    }
    panic!("failed to reopen sled database after close")
}
