use std::collections::BTreeMap;
use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope, EventPage,
    EventReplayCapability, LedgerIdentity, ReplayRequest,
};
use meld_world_model::belief::{
    AssessmentLease, BeliefAssessmentActor, BeliefAssessmentRequest, BeliefConfigLoader,
    BeliefDirtyKeyTickRequest, BeliefEvidenceNormalizer, BeliefKey, BeliefRuntime, BeliefStore,
    BranchScope, EvidenceEventReplaySource, EvidenceIngestionActor, EvidenceIngestionActorRequest,
    EvidenceIngestionReceiptDisposition, EvidenceIngestionReceiptWriteDisposition, EvidenceValue,
    LeaseStatus, PromotedEvidenceRecord,
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
        max_items: 1,
        lease_owner_id: "supervisor-lease-assessment".to_string(),
    });

    assert_eq!(report.input_sequence, 0);
    assert_eq!(report.output_sequence, 1);
    assert_eq!(report.source_high_water, 0);
    assert_eq!(report.lease_clock, 1);
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
fn assessment_actor_cursor_survives_reopen_and_passes_a_blocked_head() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("world-model");
    let config = BeliefConfigLoader::load_json(CONFIG_JSON).unwrap();
    let perspective = PerspectiveKey::new("agent", "default").unwrap();
    let branch_scope = BranchScope::main();
    let key_a;
    let key_b;
    {
        let db = sled::open(&path).unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let normalizer = BeliefEvidenceNormalizer::new(
            config.config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        key_a = seed_assignment(&store, &normalizer, "event-a", "node-a", 10);
        key_b = seed_assignment(&store, &normalizer, "event-b", "node-b", 20);
        let key_c = seed_assignment(&store, &normalizer, "event-c", "node-c", 30);
        store
            .acquire_lease(active_lease(&key_a, &config, "blocked-head", 10, 1000))
            .unwrap();
        store.flush().unwrap();
        let actor = BeliefAssessmentActor::new(
            Arc::clone(&store),
            traversal,
            config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );

        let first = actor.tick_dirty(BeliefDirtyKeyTickRequest {
            max_items: 1,
            lease_owner_id: "supervisor-lease-a".to_string(),
        });

        assert_eq!(first.input_sequence, 0);
        assert_eq!(first.output_sequence, 0);
        assert_eq!(first.source_high_water, 0);
        assert_eq!(first.lease_clock, 1);
        assert_eq!(first.selected_count, 1);
        assert_eq!(first.committed_count, 0);
        assert_eq!(first.no_work_count, 0);
        assert_eq!(first.retryable_errors.len(), 1);
        assert!(first.budget_exhausted);
        assert!(store.current_revision(&key_a).unwrap().is_none());
        assert!(store.current_revision(&key_b).unwrap().is_none());
        assert!(store.current_revision(&key_c).unwrap().is_none());
    }
    {
        let db = reopen_sled(&path);
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let actor = BeliefAssessmentActor::new(
            Arc::clone(&store),
            traversal,
            config,
            perspective,
            branch_scope,
        );

        let resumed = actor.tick_dirty(BeliefDirtyKeyTickRequest {
            max_items: 1,
            lease_owner_id: "supervisor-lease-b".to_string(),
        });

        assert_eq!(resumed.input_sequence, 0);
        assert_eq!(resumed.output_sequence, 1);
        assert_eq!(resumed.source_high_water, 0);
        assert_eq!(resumed.lease_clock, 2);
        assert_eq!(resumed.selected_count, 1);
        assert_eq!(resumed.committed_count, 1);
        assert!(resumed.retryable_errors.is_empty());
        assert!(resumed.fatal_errors.is_empty());
        assert!(store.current_revision(&key_b).unwrap().is_some());
        assert!(store.current_revision(&key_a).unwrap().is_none());
    }
}

#[test]
fn assessment_actor_recovers_expired_lease_from_idle_clock_after_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("belief-idle-recovery");
    let config = BeliefConfigLoader::load_json(CONFIG_JSON).unwrap();
    let perspective = PerspectiveKey::new("agent", "default").unwrap();
    let branch_scope = BranchScope::main();
    let key;
    {
        let db = sled::open(&path).unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let normalizer = BeliefEvidenceNormalizer::new(
            config.config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        key = seed_assignment(&store, &normalizer, "event-a", "node-a", 10);
        store
            .acquire_lease(active_lease(&key, &config, "expired", 10, 2))
            .unwrap();
        store.flush().unwrap();
        let actor = BeliefAssessmentActor::new(
            Arc::clone(&store),
            traversal,
            config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        let first = actor.tick_dirty(BeliefDirtyKeyTickRequest {
            max_items: 1,
            lease_owner_id: "supervisor-lease-recovery".to_string(),
        });
        assert_eq!(first.lease_clock, 1);
        assert_eq!(first.recovered_lease_count, 0);
        assert_eq!(first.committed_count, 0);
        assert_eq!(first.retryable_errors.len(), 1);
    }
    {
        let db = reopen_sled(&path);
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let actor = BeliefAssessmentActor::new(
            Arc::clone(&store),
            traversal,
            config,
            perspective,
            branch_scope,
        );
        let resumed = actor.tick_dirty(BeliefDirtyKeyTickRequest {
            max_items: 1,
            lease_owner_id: "supervisor-lease-recovery".to_string(),
        });
        assert_eq!(resumed.input_sequence, 0);
        assert_eq!(resumed.output_sequence, 1);
        assert_eq!(resumed.source_high_water, 0);
        assert_eq!(resumed.lease_clock, 2);
        assert_eq!(resumed.recovered_lease_count, 1);
        assert_eq!(resumed.committed_count, 1);
        assert!(resumed.retryable_errors.is_empty());
        assert!(resumed.fatal_errors.is_empty());
        assert_eq!(
            store.get_lease("expired").unwrap().unwrap().status,
            LeaseStatus::Abandoned
        );
        assert!(store.current_revision(&key).unwrap().is_some());
    }
}

#[test]
fn dirty_assessment_settles_bounded_evidence_windows_without_losing_tail_work() {
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
    let key = seed_assignment(&store, &normalizer, "event-1", "node-a", 1);
    seed_assignment(&store, &normalizer, "event-2", "node-a", 2);
    seed_assignment(&store, &normalizer, "event-3", "node-a", 3);
    let runtime = BeliefRuntime::new(
        Arc::clone(&store),
        traversal,
        config,
        perspective,
        branch_scope,
    );

    for expected_sequence in 1..=3 {
        let result = runtime
            .assess_dirty_key_bounded(&key, "supervisor-lease-window", 1)
            .unwrap()
            .unwrap();
        assert_eq!(result.evidence_count, 1);
        assert_eq!(result.source_cursor_end, expected_sequence);
        if expected_sequence < 3 {
            let dirty = store.dirty_state(&key).unwrap().unwrap();
            assert_eq!(dirty.dirty_since_seq, 1);
            assert_eq!(dirty.latest_seq, 3);
        }
    }

    assert!(store.dirty_state(&key).unwrap().is_none());
    assert_eq!(store.revision_history(&key).unwrap().len(), 3);
}

#[test]
fn single_blocked_dirty_key_remains_retryable_after_cursor_wrap() {
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
    let key = seed_assignment(&store, &normalizer, "event-blocked", "node-a", 10);
    store
        .acquire_lease(active_lease(&key, &config, "blocked-only", 10, 1000))
        .unwrap();
    store.flush().unwrap();
    let actor = BeliefAssessmentActor::new(
        Arc::clone(&store),
        traversal,
        config,
        perspective,
        branch_scope,
    );
    let request = BeliefDirtyKeyTickRequest {
        max_items: 1,
        lease_owner_id: "supervisor-lease-wrap".to_string(),
    };

    let first = actor.tick_dirty(request.clone());
    let second = actor.tick_dirty(request);

    for report in [&first, &second] {
        assert_eq!(report.selected_count, 1);
        assert_eq!(report.committed_count, 0);
        assert_eq!(report.no_work_count, 0);
        assert_eq!(report.retryable_errors.len(), 1);
        assert!(report.fatal_errors.is_empty());
    }
    assert_eq!(first.lease_clock, 1);
    assert_eq!(second.lease_clock, 2);
    assert!(store.current_revision(&key).unwrap().is_none());
}

#[test]
fn recurring_settlement_is_independent_of_assignment_page_size() {
    let narrow = settlement_signature(1);
    let wide = settlement_signature(1024);

    assert_eq!(narrow.len(), 3);
    assert_eq!(narrow, wide);
}

#[test]
fn late_source_sequences_use_assignment_cursor_progress_and_reopen_safely() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("late-source-belief");
    let config = BeliefConfigLoader::load_json(CONFIG_JSON).unwrap();
    let perspective = PerspectiveKey::new("agent", "default").unwrap();
    let branch_scope = BranchScope::main();
    let key;
    let first_revision_id;
    {
        let db = sled::open(&path).unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let normalizer = BeliefEvidenceNormalizer::new(
            config.config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        key = seed_assignment(&store, &normalizer, "event-100", "node-a", 100);
        seed_assignment(&store, &normalizer, "event-50", "node-a", 50);
        let dirty = store.dirty_state(&key).unwrap().unwrap();
        assert_eq!(dirty.dirty_since_seq, 50);
        assert_eq!(dirty.latest_seq, 100);
        let runtime = BeliefRuntime::new(
            Arc::clone(&store),
            traversal,
            config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        let first = runtime
            .assess_dirty_key_bounded(&key, "late-source-first", 1)
            .unwrap()
            .unwrap();
        assert_eq!(first.source_cursor_end, 100);
        first_revision_id = first.revision_id;
        let remaining = store.dirty_state(&key).unwrap().unwrap();
        assert_eq!(remaining.dirty_since_seq, 50);
        assert_eq!(remaining.latest_seq, 100);
        store.flush().unwrap();
    }
    {
        let db = reopen_sled(&path);
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let runtime = BeliefRuntime::new(
            Arc::clone(&store),
            traversal,
            config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        let late = runtime
            .assess_dirty_key_bounded(&key, "late-source-second", 1)
            .unwrap()
            .unwrap();
        assert_eq!(late.source_cursor_end, 100);
        assert_eq!(
            store
                .current_revision(&key)
                .unwrap()
                .unwrap()
                .source_cursor_end,
            100
        );
        let history = store.revision_history(&key).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].revision_id, first_revision_id);
        assert_eq!(
            history[1].prior_revision_id.as_deref(),
            Some(history[0].revision_id.as_str())
        );
        assert!(store.dirty_state(&key).unwrap().is_none());
    }

    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
    let traversal = Arc::new(TraversalStore::new(db).unwrap());
    let normalizer = BeliefEvidenceNormalizer::new(
        config.config.clone(),
        perspective.clone(),
        branch_scope.clone(),
    );
    let ordered_key = seed_assignment(&store, &normalizer, "event-50", "node-a", 50);
    seed_assignment(&store, &normalizer, "event-100", "node-a", 100);
    let runtime = BeliefRuntime::new(store, traversal, config, perspective, branch_scope);
    assert_eq!(
        runtime
            .assess_dirty_key_bounded(&ordered_key, "ordered-source-first", 1)
            .unwrap()
            .unwrap()
            .source_cursor_end,
        50
    );
    assert_eq!(
        runtime
            .assess_dirty_key_bounded(&ordered_key, "ordered-source-second", 1)
            .unwrap()
            .unwrap()
            .source_cursor_end,
        100
    );
}

#[test]
fn bounded_assignment_cursor_crosses_long_committed_history_before_new_evidence() {
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
    let runtime = BeliefRuntime::new(
        Arc::clone(&store),
        traversal,
        config,
        perspective,
        branch_scope,
    );
    let mut key = None;
    for sequence in 1..=32 {
        let selected = seed_assignment(
            &store,
            &normalizer,
            &format!("event-{sequence}"),
            "node-a",
            sequence,
        );
        key = Some(selected.clone());
        assert!(runtime
            .assess_dirty_key_bounded(&selected, "history-builder", 1024)
            .unwrap()
            .is_some());
    }
    let key = key.unwrap();
    seed_assignment(&store, &normalizer, "event-33", "node-a", 33);

    for _ in 0..8 {
        assert!(runtime
            .assess_dirty_key_bounded(&key, "bounded-history-reader", 4)
            .unwrap()
            .is_none());
        assert!(store.dirty_state(&key).unwrap().is_some());
    }
    let settled = runtime
        .assess_dirty_key_bounded(&key, "bounded-history-reader", 4)
        .unwrap()
        .unwrap();

    assert_eq!(settled.evidence_count, 1);
    assert_eq!(settled.source_cursor_end, 33);
    assert!(store.dirty_state(&key).unwrap().is_none());
    assert_eq!(store.revision_history(&key).unwrap().len(), 33);
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
fn evidence_actor_replay_waits_for_exact_settlement_after_active_lease_conflict() {
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
    let request = evidence_request(1);
    let normalizer = BeliefEvidenceNormalizer::new(
        request.config.config.clone(),
        request.perspective.clone(),
        request.branch_scope.clone(),
    );
    let key = normalizer
        .normalize_promoted(&promoted_record("event-spine::1", "node-a", 1))
        .unwrap()
        .remove(0)
        .candidate_key;
    store.mark_dirty(&key, 1).unwrap();
    store
        .acquire_lease(active_lease(
            &key,
            &request.config,
            "prior-active-assessment",
            1,
            2,
        ))
        .unwrap();
    let actor = EvidenceIngestionActor::new(replay, Arc::clone(&store), traversal);

    let blocked = actor.tick(request.clone());

    assert_eq!(blocked.output_event_sequence, 0);
    assert_eq!(blocked.retryable_errors.len(), 1);
    assert!(blocked.receipts.is_empty());
    assert!(actor.durable_cursor(&request).unwrap().is_none());
    assert!(store.current_revision(&key).unwrap().is_none());
    assert!(store
        .get_runtime_meta("assessment_source_high_water")
        .unwrap()
        .is_none());

    let settled = actor.tick(request.clone());

    assert_eq!(settled.input_event_sequence, 0);
    assert_eq!(settled.output_event_sequence, 1);
    assert_eq!(settled.committed_revision_count, 1);
    assert_eq!(settled.receipts.len(), 1);
    assert!(settled.retryable_errors.is_empty());
    assert_eq!(
        store
            .get_lease("prior-active-assessment")
            .unwrap()
            .unwrap()
            .status,
        LeaseStatus::Abandoned
    );
    let revision = store.current_revision(&key).unwrap().unwrap();
    assert_eq!(revision.evidence_ids.len(), 1);
    assert!(store
        .get_runtime_meta("assessment_source_high_water")
        .unwrap()
        .is_some());
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

#[test]
fn belief_config_rejects_unbounded_mapping_fanout() {
    let mut config: serde_json::Value = serde_json::from_str(CONFIG_JSON).unwrap();
    let mapping = config["source_mappings"][0].clone();
    config["source_mappings"] = serde_json::Value::Array(vec![mapping; 1025]);

    let error =
        BeliefConfigLoader::load_json(&serde_json::to_string(&config).unwrap()).unwrap_err();

    assert_eq!(
        error.to_string(),
        "Invalid path: belief source mappings exceed the 1024-item limit"
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

fn seed_assignment(
    store: &BeliefStore,
    normalizer: &BeliefEvidenceNormalizer,
    source_id: &str,
    object_id: &str,
    sequence: u64,
) -> BeliefKey {
    let item = normalizer
        .normalize_promoted(&promoted_record(source_id, object_id, sequence))
        .unwrap()
        .remove(0);
    let key = item.candidate_key.clone();
    store.put_evidence_once(&item).unwrap();
    store
        .put_assignment_once(&normalizer.assign(&item).unwrap())
        .unwrap();
    key
}

fn settlement_signature(limit: usize) -> Vec<(String, Vec<String>, u64, u64)> {
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
    let key = seed_assignment(&store, &normalizer, "event-1", "node-a", 1);
    seed_assignment(&store, &normalizer, "event-2", "node-a", 2);
    seed_assignment(&store, &normalizer, "event-3", "node-a", 3);
    let runtime = BeliefRuntime::new(store.clone(), traversal, config, perspective, branch_scope);
    for _ in 0..8 {
        if store.dirty_state(&key).unwrap().is_none() {
            break;
        }
        runtime
            .assess_dirty_key_bounded(&key, "partition-equivalence", limit)
            .unwrap();
    }
    assert!(store.dirty_state(&key).unwrap().is_none());
    store
        .revision_history(&key)
        .unwrap()
        .into_iter()
        .map(|revision| {
            (
                revision.revision_id,
                revision.evidence_ids,
                revision.source_cursor_end,
                revision.posterior.probability.to_bits(),
            )
        })
        .collect()
}

fn active_lease(
    key: &BeliefKey,
    config: &meld_world_model::belief::ConfigSnapshot,
    lease_id: &str,
    source_sequence: u64,
    expires_at_sequence: u64,
) -> AssessmentLease {
    AssessmentLease {
        lease_id: lease_id.to_string(),
        belief_key: key.clone(),
        epoch: 1,
        owner_id: "prior-supervisor-lease".to_string(),
        input_cursor_start: source_sequence,
        input_cursor_end: source_sequence,
        assignment_cursor_start: None,
        assignment_cursor_end: None,
        assignment_window_complete: true,
        started_at_seq: expires_at_sequence.saturating_sub(1),
        expires_at_seq: expires_at_sequence,
        comparator_engine_id: config.config.comparator.engine_id.clone(),
        config_snapshot_hash: config.hash.clone(),
        status: LeaseStatus::Queued,
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
