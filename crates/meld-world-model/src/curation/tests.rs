use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use meld_events::{
    AppendDisposition, AppendReceipt, DomainObjectRef, EventEnvelope, EventWatermark, LedgerCursor,
    LedgerIdentity,
};

use super::*;
use crate::belief::{BranchScope, TheoryRevisionRef};
use crate::error::StorageError;
use crate::world_state::graph::contracts::{
    traversal_cut_identity, BoundedTraversalRequest, OwnerCompletenessReceipt,
    OwnerCompletenessStatus, OwnerCurrentnessPolicy, OwnerGraphRevisionReceipt,
    OwnerObjectPublication, OwnerPublicationExclusion, OwnerPublicationScope,
    OwnerRelationOccurrence, PerspectiveKey, TraversalBounds, TraversalCut, TraversalCutIssue,
    TraversalCutRequest, TraversalCutStatus, TraversalDirection, TraversalFrontierEntry,
    TraversalFrontierReason, TraversalOwnerRequirement, TraversalResult, TraversalTruncation,
};

struct MockTraversal {
    cut: Mutex<TraversalCut>,
    result: Mutex<TraversalResult>,
    failures: AtomicUsize,
    traversals: AtomicUsize,
}

impl CurationTraversalPort for MockTraversal {
    fn cut(&self, _request: &TraversalCutRequest) -> Result<TraversalCut, StorageError> {
        Ok(self.cut.lock().unwrap().clone())
    }

    fn traverse(
        &self,
        _cut: &TraversalCut,
        _request: &BoundedTraversalRequest,
    ) -> Result<TraversalResult, StorageError> {
        self.traversals.fetch_add(1, Ordering::SeqCst);
        if self.failures.load(Ordering::SeqCst) > 0 {
            self.failures.fetch_sub(1, Ordering::SeqCst);
            return Err(StorageError::InvalidPath(
                "injected Traversal failure".to_string(),
            ));
        }
        Ok(self.result.lock().unwrap().clone())
    }
}

struct InterruptBeforeTraversal {
    inner: Arc<MockTraversal>,
}

impl CurationTraversalPort for InterruptBeforeTraversal {
    fn cut(&self, request: &TraversalCutRequest) -> Result<TraversalCut, StorageError> {
        self.inner.cut(request)
    }

    fn traverse(
        &self,
        _cut: &TraversalCut,
        _request: &BoundedTraversalRequest,
    ) -> Result<TraversalResult, StorageError> {
        panic!("injected interruption after Curation acceptance")
    }
}

struct MockEvents {
    ledger_id: LedgerIdentity,
    watermark: u64,
    envelopes: Mutex<BTreeMap<String, EventEnvelope>>,
}

struct InterruptingEvents {
    inner: Arc<MockEvents>,
    successful_appends: AtomicUsize,
    successes_before_failure: usize,
}

impl CurationEventPort for InterruptingEvents {
    fn watermark(&self) -> Result<EventWatermark, String> {
        self.inner.watermark()
    }

    fn append_idempotent(&self, envelope: EventEnvelope) -> Result<AppendReceipt, String> {
        if self.successful_appends.load(Ordering::SeqCst) >= self.successes_before_failure {
            return Err("injected publication interruption".to_string());
        }
        let receipt = self.inner.append_idempotent(envelope)?;
        self.successful_appends.fetch_add(1, Ordering::SeqCst);
        Ok(receipt)
    }
}

impl CurationEventPort for MockEvents {
    fn watermark(&self) -> Result<EventWatermark, String> {
        Ok(EventWatermark {
            ledger_id: self.ledger_id,
            committed_seq: self.watermark,
            tip_seq: self.watermark,
        })
    }

    fn append_idempotent(&self, envelope: EventEnvelope) -> Result<AppendReceipt, String> {
        let record_id = envelope
            .record_id
            .clone()
            .ok_or_else(|| "Curation Event requires a record id".to_string())?;
        let mut envelopes = self.envelopes.lock().unwrap();
        let disposition = if let Some(existing) = envelopes.get(&record_id) {
            if existing != &envelope {
                return Err("record id collision".to_string());
            }
            AppendDisposition::Duplicate
        } else {
            envelopes.insert(record_id, envelope);
            AppendDisposition::Inserted
        };
        Ok(AppendReceipt {
            ledger_id: self.ledger_id,
            seq: self.watermark + envelopes.len() as u64,
            disposition,
        })
    }
}

struct MutableCurationAuthority(Mutex<Option<CurationAuthority>>);
impl CurationAuthorityPort for MutableCurationAuthority {
    fn observe(&self) -> Result<Option<CurationAuthority>, StorageError> {
        Ok(self.0.lock().unwrap().clone())
    }
}

#[test]
fn closed_epoch_finishes_accepted_work_and_reopened_epoch_rejects_stale_intake() {
    let mut fixture = Fixture::new(0);
    let mut prior = authority();
    prior.admission_epoch = Some("epoch-1".into());
    let operation = CurationOperation::reconstruct(
        prior.clone(),
        fixture.rule.revision_ref(),
        fixture.traversal.cut.lock().unwrap().clone(),
        fixture.rule.rule.traversal_request(),
    )
    .unwrap();
    fixture.store.put_operation(&operation).unwrap();
    let accepted =
        CurationAcceptanceRecord::for_current_authority(&operation, &fixture.rule, &prior).unwrap();
    fixture.store.put_acceptance(&accepted).unwrap();
    let observer = Arc::new(MutableCurationAuthority(Mutex::new(None)));
    fixture.actor = fixture.actor.with_authority_port(observer.clone());
    let recovered = fixture.actor.bounded_step(1);
    assert!(recovered.fatal_errors.is_empty(), "{recovered:?}");
    assert_eq!(recovered.results_persisted, 1);
    assert_eq!(recovered.publications_appended, 2);
    let closed = fixture.actor.bounded_step(1);
    assert_eq!(closed.operations_attempted, 0);
    assert_eq!(closed.publications_appended, 0);
    let mut old = authority();
    old.admission_epoch = Some("epoch-0".into());
    let stale = CurationOperation::reconstruct(
        old,
        fixture.rule.revision_ref(),
        fixture.traversal.cut.lock().unwrap().clone(),
        fixture.rule.rule.traversal_request(),
    )
    .unwrap();
    let mut grant = planned_authorization(&stale);
    grant.admission_epoch = stale.authority.admission_epoch.clone();
    fixture
        .store
        .submit_planned(&stale.clone().with_planned_authorization(grant).unwrap())
        .unwrap();
    let mut current = prior;
    current.admission_epoch = Some("epoch-2".into());
    *observer.0.lock().unwrap() = Some(current);
    let refused = fixture.actor.bounded_step(1);
    assert_eq!(refused.results_persisted, 0);
    assert_eq!(
        fixture
            .store
            .acceptance_for_planned_operation(&stale.operation_id)
            .unwrap()
            .unwrap()
            .decision,
        CurationAdmissionDecision::Rejected
    );
    assert!(fixture
        .store
        .next_planned_operation(&authority().agent_id)
        .unwrap()
        .is_none());
    let next = fixture.actor.bounded_step(1);
    assert_eq!(next.results_persisted, 1, "{next:?}");
    assert_eq!(fixture.traversal.traversals.load(Ordering::SeqCst), 2);
}

#[test]
fn native_lifecycle_accounts_for_pending_work_and_completed_publication() {
    let fixture = Fixture::new(0);
    let operation = CurationOperation::reconstruct(
        authority(),
        fixture.rule.revision_ref(),
        fixture.traversal.cut.lock().unwrap().clone(),
        fixture.rule.rule.traversal_request(),
    )
    .unwrap();
    fixture.store.put_operation(&operation).unwrap();
    let wake = |resource: &str, operation_id: &str| {
        crate::waiting::StructuralWakeAddress::DurableOperation(format!(
            "world-model::{resource}::curation-operation::{operation_id}::completion"
        ))
    };
    assert!(fixture
        .actor
        .resolves_wake(&wake(fixture.store.resource_id(), &operation.operation_id))
        .unwrap());
    assert!(!fixture
        .actor
        .resolves_wake(&wake("foreign-resource", &operation.operation_id))
        .unwrap());
    assert!(!fixture
        .actor
        .resolves_wake(&wake(fixture.store.resource_id(), "foreign-operation"))
        .unwrap());
    let identity =
        crate::lifecycle::NativeLifecycleIdentity::new("generation-a".into(), "curation-a".into())
            .unwrap();
    let (ready, transition) = fixture.actor.lifecycle_start(identity.clone()).unwrap();
    assert_eq!(transition.checkpoint_ref, ready.proof_position_ref);
    assert_eq!(
        fixture.store.unresolved_operations(&authority()).unwrap(),
        vec![(operation.operation_id, "result_pending".into())]
    );
    let result = fixture.actor.bounded_step(1);
    assert_eq!(result.publications_appended, 2, "{result:?}");
    let (safe, transition) = fixture.actor.lifecycle_safe_point(identity).unwrap();
    assert_eq!(transition.checkpoint_ref, safe.proof_position_ref);
    assert_ne!(
        safe.unresolved_operation_summary_ref,
        ready.unresolved_operation_summary_ref
    );
    assert!(fixture
        .store
        .unresolved_operations(&authority())
        .unwrap()
        .is_empty());
}

#[test]
fn applied_result_replays_without_reexecuting_or_republishing() {
    let fixture = Fixture::new(0);

    let first = fixture.actor.bounded_step(1);
    let second = fixture.actor.bounded_step(1);

    assert_eq!(first.operations_attempted, 1);
    assert_eq!(first.results_persisted, 1);
    assert_eq!(first.publications_appended, 2);
    assert_eq!(first.output_after_seq, 12);
    assert_eq!(second.reused_results, 1);
    assert_eq!(second.publications_appended, 0);
    assert_eq!(fixture.traversal.traversals.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.events.envelopes.lock().unwrap().len(), 2);
}

#[test]
fn planned_curation_reopens_after_acceptance_without_duplicate_execution() {
    assert_planned_curation_recovery(PlannedInterruption::AfterAcceptance);
}

#[test]
fn planned_curation_reopens_after_result_before_event_without_duplicate_publication() {
    assert_planned_curation_recovery(PlannedInterruption::AfterResultBeforeEvent);
}

#[test]
fn named_confirmation_is_not_a_retroactive_standing_acceptance() {
    let fixture = Fixture::new(0);
    fixture.actor.bounded_step(1);
    let standing = CurationOperation::reconstruct(
        authority(),
        fixture.rule.revision_ref(),
        fixture.traversal.cut.lock().unwrap().clone(),
        fixture.rule.rule.traversal_request(),
    )
    .unwrap();
    let requested = standing
        .clone()
        .for_request("agent-goal-confirmation".into())
        .unwrap();
    assert_ne!(requested.selection_id, standing.selection_id);
    assert_ne!(requested.operation_id, standing.operation_id);
    assert_eq!(
        fixture.store.resolve_operation(requested.clone()).unwrap(),
        requested
    );
    let planned = requested
        .clone()
        .with_planned_authorization(planned_authorization(&requested))
        .unwrap();
    fixture.store.submit_planned(&planned).unwrap();
    assert!(fixture
        .store
        .acceptance_for_planned_operation(&requested.operation_id)
        .unwrap()
        .is_none());
    let confirmation = fixture.actor.bounded_step(1);
    assert!(confirmation.fatal_errors.is_empty(), "{confirmation:?}");
    assert_eq!(confirmation.results_persisted, 1);
    assert!(fixture
        .store
        .acceptance_for_planned_operation(&requested.operation_id)
        .unwrap()
        .is_some());
    assert_eq!(
        fixture.store.resolve_operation(requested.clone()).unwrap(),
        requested
    );
    let mut tampered = requested;
    tampered.request_id = Some("another-goal".into());
    assert!(tampered.validate().is_err());
}

#[test]
fn standing_then_planned_same_selection_converges_without_identity_drift() {
    let fixture = Fixture::new(0);

    let standing = fixture.actor.bounded_step(1);
    assert_eq!(standing.results_persisted, 1);
    assert_eq!(standing.publications_appended, 2);

    let semantic_operation = fixture
        .store
        .operation_for_selection(
            &CurationOperation::reconstruct(
                authority(),
                fixture.rule.revision_ref(),
                fixture.traversal.cut.lock().unwrap().clone(),
                fixture.rule.rule.traversal_request(),
            )
            .unwrap()
            .selection_id,
        )
        .unwrap()
        .unwrap();
    let operation_id = semantic_operation.operation_id.clone();
    let standing_acceptance =
        CurationAcceptanceRecord::for_operation(&semantic_operation, &fixture.rule).unwrap();
    assert_eq!(
        fixture
            .store
            .acceptance(&standing_acceptance.acceptance_id)
            .unwrap(),
        Some(standing_acceptance.clone())
    );
    assert!(fixture
        .store
        .acceptance_for_planned_operation(&operation_id)
        .unwrap()
        .is_none());

    let authorization = planned_authorization(&semantic_operation);
    let authorization_id = authorization.authorization_id.clone();
    let planned = semantic_operation
        .clone()
        .with_planned_authorization(authorization)
        .unwrap();
    fixture.store.submit_planned(&planned).unwrap();

    assert_eq!(
        fixture
            .store
            .operation_for_selection(&semantic_operation.selection_id)
            .unwrap(),
        Some(semantic_operation.clone())
    );
    assert_eq!(
        fixture
            .store
            .planned_authorization_for_operation(&operation_id)
            .unwrap()
            .unwrap()
            .authorization_id,
        authorization_id
    );
    assert_eq!(
        fixture
            .store
            .acceptance_for_planned_operation(&operation_id)
            .unwrap(),
        Some(standing_acceptance)
    );
    assert!(fixture
        .store
        .next_planned_operation("agent-a")
        .unwrap()
        .is_none());

    let replay = fixture.actor.bounded_step(1);
    assert_eq!(replay.reused_results, 1);
    assert_eq!(replay.results_persisted, 0);
    assert_eq!(replay.publications_appended, 0);
    assert!(replay.fatal_errors.is_empty());
    assert_eq!(fixture.traversal.traversals.load(Ordering::SeqCst), 1);
}

#[test]
fn bounded_incomplete_result_retains_frontier_exclusions_failures_and_cut() {
    let fixture = Fixture::new(0);
    let source_cut = fixture.traversal.cut.lock().unwrap().clone();
    let exclusion = OwnerPublicationExclusion {
        source_ref: "ignored-generated-file".to_string(),
        reason: "outside installed observation policy".to_string(),
    };
    {
        let mut traversal = fixture.traversal.result.lock().unwrap();
        traversal.receipts = source_cut.receipts.clone();
        traversal.receipts[0].completeness.exclusions = vec![exclusion.clone()];
        traversal.frontier = vec![TraversalFrontierEntry {
            object_ref: authority().subject,
            depth: 4,
            reason: TraversalFrontierReason::DepthBound,
        }];
        traversal.truncation.depth = true;
    }

    let report = fixture.actor.bounded_step(1);
    let result = result_events(&fixture.events).pop().unwrap();

    assert_eq!(report.results_persisted, 1);
    assert_eq!(result.disposition, CurationTerminalDisposition::Incomplete);
    assert_eq!(result.source_cut_id, source_cut.cut_id);
    assert_eq!(result.frontier.len(), 1);
    assert_eq!(result.exclusions, vec![exclusion]);
    assert_eq!(result.failures, vec!["depth bound reached"]);
    assert!(result.semantic_publication.is_none());
}

#[test]
fn failed_operation_is_terminal_for_the_same_exact_input() {
    let fixture = Fixture::new(1);

    let first = fixture.actor.bounded_step(1);
    let second = fixture.actor.bounded_step(1);

    assert_eq!(first.results_persisted, 1);
    assert_eq!(first.publications_appended, 1);
    assert_eq!(second.reused_results, 1);
    assert_eq!(fixture.traversal.traversals.load(Ordering::SeqCst), 1);
    let result_event = fixture
        .events
        .envelopes
        .lock()
        .unwrap()
        .values()
        .find(|event| event.event_type == CURATION_RESULT_EVENT_TYPE)
        .cloned()
        .unwrap();
    let result: CurationResult = serde_json::from_value(result_event.data).unwrap();
    assert_eq!(result.disposition, CurationTerminalDisposition::Failed);
    assert!(result.semantic_publication.is_none());
}

#[test]
fn incomplete_cut_waits_without_admission_or_publication() {
    let fixture = Fixture::new(0);
    let mut source_cut = fixture.traversal.cut.lock().unwrap().clone();
    source_cut.status = TraversalCutStatus::Incomplete;
    source_cut.issues = vec![TraversalCutIssue::MissingRequiredOwner {
        owner_id: "workspace_fs".to_string(),
        scope_id: scope().scope_id,
    }];
    source_cut.cut_id = traversal_cut_identity(&source_cut).unwrap();
    *fixture.traversal.cut.lock().unwrap() = source_cut;

    let report = fixture.actor.bounded_step(1);

    assert_eq!(report.operations_attempted, 0);
    assert_eq!(report.acceptances_persisted, 0);
    assert_eq!(report.results_persisted, 0);
    assert_eq!(report.publications_appended, 0);
    assert_eq!(report.waiting_on.len(), 1);
    assert_eq!(fixture.traversal.traversals.load(Ordering::SeqCst), 0);
    assert!(fixture.events.envelopes.lock().unwrap().is_empty());
}

#[test]
fn preadmission_rejection_persists_without_semantic_execution() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(CurationStore::new(db).unwrap());
    let revision = store.install_rule(rule(), 1).unwrap();
    let source_cut = cut(10);
    let ledger_id = source_cut.event_position.ledger_id;
    let traversal = Arc::new(MockTraversal {
        result: Mutex::new(TraversalResult {
            absent_roots: Vec::new(),
            result_id: "rejected-traversal".to_string(),
            cut_id: source_cut.cut_id.clone(),
            objects: Vec::new(),
            occurrences: Vec::new(),
            paths: Vec::new(),
            receipts: Vec::new(),
            frontier: Vec::new(),
            truncation: TraversalTruncation::default(),
        }),
        cut: Mutex::new(source_cut.clone()),
        failures: AtomicUsize::new(0),
        traversals: AtomicUsize::new(0),
    });
    let events = Arc::new(MockEvents {
        ledger_id,
        watermark: 10,
        envelopes: Mutex::new(BTreeMap::new()),
    });
    let mut foreign_authority = authority();
    foreign_authority.subject =
        DomainObjectRef::new("workspace_fs", "node", "outside-rule").unwrap();
    let expected_operation = CurationOperation::reconstruct(
        foreign_authority.clone(),
        revision.revision_ref(),
        source_cut,
        revision.rule.traversal_request(),
    )
    .unwrap();
    let expected_acceptance =
        CurationAcceptanceRecord::for_operation(&expected_operation, &revision).unwrap();
    let actor = StandingCurationActor::new(
        "world_model.standing_curation",
        "session-a",
        foreign_authority,
        revision,
        Arc::clone(&store),
        Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
        Arc::clone(&events) as Arc<dyn CurationEventPort>,
    )
    .unwrap();

    let report = actor.bounded_step(1);

    assert_eq!(report.acceptances_persisted, 1);
    assert_eq!(report.results_persisted, 0);
    assert_eq!(traversal.traversals.load(Ordering::SeqCst), 0);
    assert!(events.envelopes.lock().unwrap().is_empty());
    assert_eq!(
        store
            .acceptance(&expected_acceptance.acceptance_id)
            .unwrap()
            .unwrap()
            .decision,
        CurationAdmissionDecision::Rejected
    );
}

#[test]
fn successor_cut_observes_existing_state_as_unchanged() {
    let fixture = Fixture::new(0);
    fixture.actor.bounded_step(1);
    let first_result = result_events(&fixture.events).into_iter().next().unwrap();
    let publication = first_result.semantic_publication.unwrap();
    let mut next_cut = fixture.traversal.cut.lock().unwrap().clone();
    next_cut.event_position.after_seq += 1;
    next_cut.graph_position.after_seq += 1;
    next_cut.receipts = vec![OwnerGraphRevisionReceipt {
        event_coverage: None,
        owner_id: "workspace_fs".to_string(),
        revision_id: "workspace-v2".to_string(),
        scope: scope(),
        completeness: OwnerCompletenessReceipt {
            receipt_id: "workspace-v2-complete".to_string(),
            scope: scope(),
            included_ids: vec!["workspace-v2-root".to_string()],
            exclusions: Vec::new(),
            failures: Vec::new(),
            status: OwnerCompletenessStatus::Complete,
        },
        source_event: Some(meld_events::EventRecordRef {
            ledger_id: next_cut.event_position.ledger_id,
            seq: next_cut.event_position.after_seq,
        }),
        projection_position: next_cut.graph_position,
    }];
    next_cut.cut_id = traversal_cut_identity(&next_cut).unwrap();
    *fixture.traversal.cut.lock().unwrap() = next_cut.clone();
    *fixture.traversal.result.lock().unwrap() = TraversalResult {
        absent_roots: Vec::new(),
        result_id: "successor-traversal".to_string(),
        cut_id: next_cut.cut_id,
        objects: publication.batch.objects,
        occurrences: publication.batch.relations,
        paths: Vec::new(),
        receipts: Vec::new(),
        frontier: Vec::new(),
        truncation: TraversalTruncation::default(),
    };

    let report = fixture.actor.bounded_step(1);
    let results = result_events(&fixture.events);

    assert_eq!(report.results_persisted, 1);
    assert_eq!(report.publications_appended, 1);
    assert_eq!(results.len(), 2);
    assert_eq!(
        results.last().unwrap().disposition,
        CurationTerminalDisposition::Unchanged
    );
    assert!(results.last().unwrap().semantic_publication.is_none());
}

#[test]
fn rule_and_terminal_identity_survive_store_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("world-model.sled");
    let rule = rule();
    let operation;
    let result;
    {
        let db = sled::open(&path).unwrap();
        let store = CurationStore::new(db).unwrap();
        let revision = store.install_rule(rule.clone(), 7).unwrap();
        operation = CurationOperation::reconstruct(
            authority(),
            revision.revision_ref(),
            cut(11),
            rule.traversal_request(),
        )
        .unwrap();
        store.put_operation(&operation).unwrap();
        store
            .put_acceptance(
                &CurationAcceptanceRecord::for_operation(&operation, &revision).unwrap(),
            )
            .unwrap();
        result = CurationResult::new(
            &operation,
            CurationTerminalDisposition::Abstained,
            "test abstention",
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        store.put_result(&result).unwrap();
        store.flush().unwrap();
    }

    let reopened = CurationStore::new(sled::open(&path).unwrap()).unwrap();

    assert_eq!(reopened.active_rule("agent-a").unwrap().unwrap().rule, rule);
    assert_eq!(
        reopened
            .result_for_operation(&operation.operation_id)
            .unwrap(),
        Some(result)
    );
}

#[test]
fn reopened_actor_reuses_the_same_terminal_identity_for_the_same_cut() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("world-model.sled");
    let source_cut = cut(12);
    let ledger_id = source_cut.event_position.ledger_id;
    let traversal = Arc::new(MockTraversal {
        result: Mutex::new(TraversalResult {
            absent_roots: Vec::new(),
            result_id: "reopen-traversal".to_string(),
            cut_id: source_cut.cut_id.clone(),
            objects: Vec::new(),
            occurrences: Vec::new(),
            paths: Vec::new(),
            receipts: Vec::new(),
            frontier: Vec::new(),
            truncation: TraversalTruncation::default(),
        }),
        cut: Mutex::new(source_cut),
        failures: AtomicUsize::new(0),
        traversals: AtomicUsize::new(0),
    });
    let events = Arc::new(MockEvents {
        ledger_id,
        watermark: 12,
        envelopes: Mutex::new(BTreeMap::new()),
    });
    let first_result;
    {
        let store = Arc::new(CurationStore::new(sled::open(&path).unwrap()).unwrap());
        let revision = store.install_rule(rule(), 1).unwrap();
        let actor = StandingCurationActor::new(
            "world_model.standing_curation",
            "session-a",
            authority(),
            revision,
            store,
            Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
            Arc::clone(&events) as Arc<dyn CurationEventPort>,
        )
        .unwrap();
        actor.bounded_step(1);
        first_result = result_events(&events).pop().unwrap();
    }

    let reopened_store = Arc::new(CurationStore::new(sled::open(&path).unwrap()).unwrap());
    let reopened_actor = StandingCurationActor::new(
        "world_model.standing_curation",
        "session-a",
        authority(),
        reopened_store
            .clone()
            .active_rule("agent-a")
            .unwrap()
            .unwrap(),
        reopened_store,
        Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
        Arc::clone(&events) as Arc<dyn CurationEventPort>,
    )
    .unwrap();

    let report = reopened_actor.bounded_step(1);
    let replayed_result = result_events(&events).pop().unwrap();

    assert_eq!(report.reused_results, 1);
    assert_eq!(report.publications_appended, 0);
    assert_eq!(traversal.traversals.load(Ordering::SeqCst), 1);
    assert_eq!(replayed_result.result_id, first_result.result_id);
    assert_eq!(replayed_result.operation_id, first_result.operation_id);
}

#[test]
fn reopen_recovers_terminal_result_with_one_or_both_publications_pending() {
    for successes_before_failure in [0, 1] {
        assert_publication_recovery_after_reopen(successes_before_failure);
    }
}

#[test]
fn owner_neutral_rule_authors_dissimilar_installed_vocabulary() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(CurationStore::new(db).unwrap());
    let subject = DomainObjectRef::new("inventory", "asset", "asset-a").unwrap();
    let neutral_rule = StandingCurationRule {
        coverage: None,
        source_event_route: None,
        judgment_scope: None,
        rule_id: "inventory-rule".to_string(),
        agent_id: "agent-inventory".to_string(),
        source_owner_id: "inventory".to_string(),
        scope: scope(),
        roots: vec![subject.clone()],
        traversal_direction: TraversalDirection::Incoming,
        bounds: rule().bounds,
        expected_object_kind: "finding".to_string(),
        expected_object_id: "asset-a::finding".to_string(),
        relation_type: "catalogues".to_string(),
        output_policy_revision: "inventory-policy-v1".to_string(),
        realization: None,
    };
    let revision = store.install_rule(neutral_rule, 1).unwrap();
    let mut source_cut = cut(10);
    source_cut.owners[1].owner_id = "inventory".to_string();
    source_cut.receipts[0].owner_id = "inventory".to_string();
    source_cut.cut_id = traversal_cut_identity(&source_cut).unwrap();
    let ledger_id = source_cut.event_position.ledger_id;
    let traversal = Arc::new(MockTraversal {
        result: Mutex::new(TraversalResult {
            absent_roots: Vec::new(),
            result_id: "inventory-traversal".to_string(),
            cut_id: source_cut.cut_id.clone(),
            objects: Vec::new(),
            occurrences: Vec::new(),
            paths: Vec::new(),
            receipts: Vec::new(),
            frontier: Vec::new(),
            truncation: TraversalTruncation::default(),
        }),
        cut: Mutex::new(source_cut),
        failures: AtomicUsize::new(0),
        traversals: AtomicUsize::new(0),
    });
    let events = Arc::new(MockEvents {
        ledger_id,
        watermark: 10,
        envelopes: Mutex::new(BTreeMap::new()),
    });
    let actor = StandingCurationActor::new(
        "world_model.standing_curation",
        "session-inventory",
        CurationAuthority {
            agent_id: "agent-inventory".to_string(),
            perspective: PerspectiveKey::new("frame", "analysis").unwrap(),
            branch_scope: BranchScope::main(),
            activation_generation: "inventory-generation".to_string(),
            admission_epoch: None,
            subject,
        },
        revision,
        store,
        Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
        Arc::clone(&events) as Arc<dyn CurationEventPort>,
    )
    .unwrap();

    actor.bounded_step(1);
    let result = result_events(&events).pop().unwrap();
    let publication = result.semantic_publication.unwrap();

    assert_eq!(
        publication.batch.objects[0].object_ref.object_kind,
        "finding"
    );
    assert_eq!(publication.batch.relations[0].relation_type, "catalogues");
    assert!(!serde_json::to_string(&publication)
        .unwrap()
        .contains("docs"));
}

proptest::proptest! {
    #[test]
    fn operation_identity_is_invariant_under_normalized_traversal_order(reverse in proptest::bool::ANY) {
        let root_a = DomainObjectRef::new("workspace_fs", "node", "a").unwrap();
        let root_b = DomainObjectRef::new("workspace_fs", "node", "b").unwrap();
        let mut roots = vec![root_a, root_b];
        let mut relation_types = vec!["depends_on".to_string(), "contains".to_string()];
        if reverse {
            roots.reverse();
            relation_types.reverse();
        }
        let request = BoundedTraversalRequest {
            roots,
            direction: TraversalDirection::Incoming,
            relation_types: Some(relation_types),
            bounds: rule().bounds,
        };
        let canonical = BoundedTraversalRequest {
            roots: vec![
                DomainObjectRef::new("workspace_fs", "node", "a").unwrap(),
                DomainObjectRef::new("workspace_fs", "node", "b").unwrap(),
            ],
            direction: TraversalDirection::Incoming,
            relation_types: Some(vec!["contains".to_string(), "depends_on".to_string()]),
            bounds: rule().bounds,
        };
        let source_cut = cut(20);
        let left = CurationOperation::reconstruct(
            authority(),
            TheoryRevisionRef {
                registry: CURATION_RULE_REGISTRY_ID.to_string(),
                id: "rule-a".to_string(),
                content_hash: "hash-a".to_string(),
            },
            source_cut.clone(),
            request,
        ).unwrap();
        let right = CurationOperation::reconstruct(
            authority(),
            TheoryRevisionRef {
                registry: CURATION_RULE_REGISTRY_ID.to_string(),
                id: "rule-a".to_string(),
                content_hash: "hash-a".to_string(),
            },
            source_cut,
            canonical,
        ).unwrap();

        proptest::prop_assert_eq!(left.operation_id, right.operation_id);
        proptest::prop_assert_eq!(left.selection_id, right.selection_id);
    }
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(32))]

    #[test]
    fn durable_replay_state_machine_preserves_exact_records_across_reopen(
        actions in proptest::collection::vec(proptest::num::u8::ANY, 1..32)
    ) {
        let temp = tempfile::tempdir().unwrap();
        let store_path = temp.path().join("world-model.sled");
        let revision = {
            let store = CurationStore::new(sled::open(&store_path).unwrap()).unwrap();
            let revision = store.install_rule(rule(), 1).unwrap();
            store.flush().unwrap();
            revision
        };
        let operation = CurationOperation::reconstruct(
            authority(),
            revision.revision_ref(),
            cut(30),
            revision.rule.traversal_request(),
        )
        .unwrap();
        let acceptance = CurationAcceptanceRecord::for_operation(&operation, &revision).unwrap();
        let result = CurationResult::new(
            &operation,
            CurationTerminalDisposition::Abstained,
            "state-machine abstention",
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let terminal_receipt = CurationPublicationReceipt {
            result_id: result.result_id.clone(),
            kind: CurationPublicationKind::Terminal,
            event_record_id: result.event_record_id(),
            event_position: LedgerCursor {
                ledger_id: operation.source_cut.event_position.ledger_id,
                after_seq: operation.source_cut.event_position.after_seq + 1,
            },
        };
        let mut operation_persisted = false;
        let mut acceptance_persisted = false;
        let mut result_persisted = false;
        let mut receipt_persisted = false;

        for action in actions {
            let store = CurationStore::new(sled::open(&store_path).unwrap()).unwrap();
            match action % 6 {
                0 => {
                    let decoded: CurationOperation = serde_json::from_slice(
                        &serde_json::to_vec(&operation).unwrap()
                    ).unwrap();
                    store.put_operation(&decoded).unwrap();
                    operation_persisted = true;
                }
                1 if operation_persisted => {
                    let decoded: CurationAcceptanceRecord = serde_json::from_slice(
                        &serde_json::to_vec(&acceptance).unwrap()
                    ).unwrap();
                    store.put_acceptance(&decoded).unwrap();
                    acceptance_persisted = true;
                }
                2 if acceptance_persisted => {
                    let decoded: CurationResult = serde_json::from_slice(
                        &serde_json::to_vec(&result).unwrap()
                    ).unwrap();
                    store.put_result(&decoded).unwrap();
                    result_persisted = true;
                }
                3 if result_persisted => {
                    let decoded: CurationPublicationReceipt = serde_json::from_slice(
                        &serde_json::to_vec(&terminal_receipt).unwrap()
                    ).unwrap();
                    store.put_publication_receipt(&decoded).unwrap();
                    receipt_persisted = true;
                }
                4 => {
                    let decoded: StandingCurationRuleRevision = serde_json::from_slice(
                        &serde_json::to_vec(&revision).unwrap()
                    ).unwrap();
                    decoded.validate().unwrap();
                    operation.validate().unwrap();
                    result.validate(&operation).unwrap();
                }
                _ => {
                    proptest::prop_assert_eq!(
                        store.operation_for_selection(&operation.selection_id).unwrap(),
                        operation_persisted.then(|| operation.clone())
                    );
                    proptest::prop_assert_eq!(
                        store.acceptance(&acceptance.acceptance_id).unwrap(),
                        acceptance_persisted.then(|| acceptance.clone())
                    );
                    proptest::prop_assert_eq!(
                        store.result_for_operation(&operation.operation_id).unwrap(),
                        result_persisted.then(|| result.clone())
                    );
                    proptest::prop_assert_eq!(
                        store.publication_receipt(
                            &result.result_id,
                            CurationPublicationKind::Terminal,
                        ).unwrap(),
                        receipt_persisted.then(|| terminal_receipt.clone())
                    );
                }
            }
            store.flush().unwrap();
        }

        {
            let store = CurationStore::new(sled::open(&store_path).unwrap()).unwrap();
            store.put_operation(&operation).unwrap();
            store.put_acceptance(&acceptance).unwrap();
            store.put_result(&result).unwrap();
            store.put_publication_receipt(&terminal_receipt).unwrap();
            store.flush().unwrap();
        }
        let reopened = CurationStore::new(sled::open(&store_path).unwrap()).unwrap();
        proptest::prop_assert_eq!(
            reopened.operation_for_selection(&operation.selection_id).unwrap(),
            Some(operation.clone())
        );
        proptest::prop_assert_eq!(
            reopened.acceptance(&acceptance.acceptance_id).unwrap(),
            Some(acceptance)
        );
        proptest::prop_assert_eq!(
            reopened.result_for_operation(&operation.operation_id).unwrap(),
            Some(result.clone())
        );
        proptest::prop_assert_eq!(
            reopened.publication_receipt(
                &result.result_id,
                CurationPublicationKind::Terminal,
            ).unwrap(),
            Some(terminal_receipt)
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlannedInterruption {
    AfterAcceptance,
    AfterResultBeforeEvent,
}

fn assert_planned_curation_recovery(interruption: PlannedInterruption) {
    let temp = tempfile::tempdir().unwrap();
    let store_path = temp.path().join("planned-curation.sled");
    let source_cut = cut(13);
    let ledger_id = source_cut.event_position.ledger_id;
    let traversal = Arc::new(MockTraversal {
        result: Mutex::new(TraversalResult {
            absent_roots: Vec::new(),
            result_id: "planned-recovery-traversal".to_string(),
            cut_id: source_cut.cut_id.clone(),
            objects: Vec::new(),
            occurrences: Vec::new(),
            paths: Vec::new(),
            receipts: source_cut.receipts.clone(),
            frontier: Vec::new(),
            truncation: TraversalTruncation::default(),
        }),
        cut: Mutex::new(source_cut.clone()),
        failures: AtomicUsize::new(0),
        traversals: AtomicUsize::new(0),
    });
    let events = Arc::new(MockEvents {
        ledger_id,
        watermark: 13,
        envelopes: Mutex::new(BTreeMap::new()),
    });
    let operation;
    let authorization;
    let acceptance;
    let result_before_reopen;

    {
        let store = Arc::new(CurationStore::new(sled::open(&store_path).unwrap()).unwrap());
        let revision = store.install_rule(rule(), 1).unwrap();
        let semantic_operation = CurationOperation::reconstruct(
            authority(),
            revision.revision_ref(),
            source_cut,
            revision.rule.traversal_request(),
        )
        .unwrap();
        authorization = planned_authorization(&semantic_operation);
        operation = semantic_operation
            .with_planned_authorization(authorization.clone())
            .unwrap();
        acceptance = CurationAcceptanceRecord::for_operation(&operation, &revision).unwrap();
        store.submit_planned(&operation).unwrap();

        match interruption {
            PlannedInterruption::AfterAcceptance => {
                let interrupting_traversal = Arc::new(InterruptBeforeTraversal {
                    inner: Arc::clone(&traversal),
                });
                let actor = StandingCurationActor::new(
                    "world_model.standing_curation",
                    "session-planned-recovery",
                    authority(),
                    revision,
                    Arc::clone(&store),
                    interrupting_traversal as Arc<dyn CurationTraversalPort>,
                    Arc::clone(&events) as Arc<dyn CurationEventPort>,
                )
                .unwrap();
                let interrupted = catch_unwind(AssertUnwindSafe(|| actor.bounded_step(1)));
                assert!(interrupted.is_err());
            }
            PlannedInterruption::AfterResultBeforeEvent => {
                let interrupting_events = Arc::new(InterruptingEvents {
                    inner: Arc::clone(&events),
                    successful_appends: AtomicUsize::new(0),
                    successes_before_failure: 0,
                });
                let actor = StandingCurationActor::new(
                    "world_model.standing_curation",
                    "session-planned-recovery",
                    authority(),
                    revision,
                    Arc::clone(&store),
                    Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
                    interrupting_events as Arc<dyn CurationEventPort>,
                )
                .unwrap();
                let interrupted = actor.bounded_step(1);
                assert_eq!(interrupted.acceptances_persisted, 1);
                assert_eq!(interrupted.results_persisted, 1);
                assert_eq!(interrupted.publications_appended, 0);
                assert_eq!(interrupted.retryable_errors.len(), 1);
                assert!(interrupted.fatal_errors.is_empty());
            }
        }

        assert_eq!(
            store
                .planned_authorization_for_operation(&operation.operation_id)
                .unwrap(),
            Some(authorization.clone())
        );
        assert_eq!(
            store
                .acceptance_for_planned_operation(&operation.operation_id)
                .unwrap(),
            Some(acceptance.clone())
        );
        result_before_reopen = store.result_for_operation(&operation.operation_id).unwrap();
        assert_eq!(
            result_before_reopen.is_some(),
            interruption == PlannedInterruption::AfterResultBeforeEvent
        );
        assert!(events.envelopes.lock().unwrap().is_empty());
        assert!(result_before_reopen.as_ref().is_none_or(|result| {
            store
                .publication_receipt(&result.result_id, CurationPublicationKind::Semantic)
                .unwrap()
                .is_none()
                && store
                    .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)
                    .unwrap()
                    .is_none()
        }));
        assert_eq!(
            traversal.traversals.load(Ordering::SeqCst),
            usize::from(interruption == PlannedInterruption::AfterResultBeforeEvent)
        );
        store.flush().unwrap();
    }

    let result;
    let semantic_receipt;
    let terminal_receipt;
    {
        let store = Arc::new(CurationStore::new(sled::open(&store_path).unwrap()).unwrap());
        let actor = StandingCurationActor::new(
            "world_model.standing_curation",
            "session-planned-recovery",
            authority(),
            store.active_rule("agent-a").unwrap().unwrap(),
            Arc::clone(&store),
            Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
            Arc::clone(&events) as Arc<dyn CurationEventPort>,
        )
        .unwrap();
        let recovered = actor.bounded_step(1);

        assert_eq!(recovered.acceptances_persisted, 0);
        assert_eq!(
            recovered.results_persisted,
            usize::from(interruption == PlannedInterruption::AfterAcceptance)
        );
        assert_eq!(
            recovered.reused_results,
            usize::from(interruption == PlannedInterruption::AfterResultBeforeEvent)
        );
        assert_eq!(recovered.publications_appended, 2);
        assert!(recovered.retryable_errors.is_empty());
        assert!(recovered.fatal_errors.is_empty());
        assert_eq!(traversal.traversals.load(Ordering::SeqCst), 1);
        assert_eq!(events.envelopes.lock().unwrap().len(), 2);
        assert_eq!(
            store
                .planned_authorization_for_operation(&operation.operation_id)
                .unwrap(),
            Some(authorization.clone())
        );
        assert_eq!(
            store
                .acceptance_for_planned_operation(&operation.operation_id)
                .unwrap(),
            Some(acceptance.clone())
        );
        result = store
            .result_for_operation(&operation.operation_id)
            .unwrap()
            .unwrap();
        if let Some(result_before_reopen) = &result_before_reopen {
            assert_eq!(&result, result_before_reopen);
        }
        assert_eq!(result.operation_id, operation.operation_id);
        semantic_receipt = store
            .publication_receipt(&result.result_id, CurationPublicationKind::Semantic)
            .unwrap()
            .unwrap();
        terminal_receipt = store
            .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)
            .unwrap()
            .unwrap();
        assert_eq!(
            semantic_receipt.event_record_id,
            result
                .semantic_publication
                .as_ref()
                .unwrap()
                .event_record_id()
        );
        assert_eq!(terminal_receipt.event_record_id, result.event_record_id());
        assert_ne!(
            semantic_receipt.event_record_id,
            terminal_receipt.event_record_id
        );
        assert!(events
            .envelopes
            .lock()
            .unwrap()
            .contains_key(&semantic_receipt.event_record_id));
        assert!(events
            .envelopes
            .lock()
            .unwrap()
            .contains_key(&terminal_receipt.event_record_id));
        assert!(store.next_planned_operation("agent-a").unwrap().is_none());
        store.flush().unwrap();
    }

    let store = Arc::new(CurationStore::new(sled::open(&store_path).unwrap()).unwrap());
    let actor = StandingCurationActor::new(
        "world_model.standing_curation",
        "session-planned-recovery",
        authority(),
        store.active_rule("agent-a").unwrap().unwrap(),
        Arc::clone(&store),
        Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
        Arc::clone(&events) as Arc<dyn CurationEventPort>,
    )
    .unwrap();
    let replayed = actor.bounded_step(1);

    assert_eq!(replayed.acceptances_persisted, 0);
    assert_eq!(replayed.results_persisted, 0);
    assert_eq!(replayed.reused_results, 1);
    assert_eq!(replayed.publications_appended, 0);
    assert!(replayed.retryable_errors.is_empty());
    assert!(replayed.fatal_errors.is_empty());
    assert_eq!(traversal.traversals.load(Ordering::SeqCst), 1);
    assert_eq!(events.envelopes.lock().unwrap().len(), 2);
    assert_eq!(
        store
            .acceptance_for_planned_operation(&operation.operation_id)
            .unwrap(),
        Some(acceptance)
    );
    assert_eq!(
        store.result_for_operation(&operation.operation_id).unwrap(),
        Some(result.clone())
    );
    assert_eq!(
        store
            .publication_receipt(&result.result_id, CurationPublicationKind::Semantic)
            .unwrap(),
        Some(semantic_receipt)
    );
    assert_eq!(
        store
            .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)
            .unwrap(),
        Some(terminal_receipt)
    );
    assert!(store.next_planned_operation("agent-a").unwrap().is_none());
}

fn result_events(events: &MockEvents) -> Vec<CurationResult> {
    let mut results = events
        .envelopes
        .lock()
        .unwrap()
        .values()
        .filter(|event| event.event_type == CURATION_RESULT_EVENT_TYPE)
        .map(|event| serde_json::from_value::<CurationResult>(event.data.clone()).unwrap())
        .collect::<Vec<_>>();
    results.sort_by_key(|result| result.source_event_position.after_seq);
    results
}

fn assert_publication_recovery_after_reopen(successes_before_failure: usize) {
    let temp = tempfile::tempdir().unwrap();
    let store_path = temp.path().join("world-model.sled");
    let source_cut = cut(14);
    let ledger_id = source_cut.event_position.ledger_id;
    let traversal = Arc::new(MockTraversal {
        result: Mutex::new(TraversalResult {
            absent_roots: Vec::new(),
            result_id: "pending-publication-traversal".to_string(),
            cut_id: source_cut.cut_id.clone(),
            objects: Vec::new(),
            occurrences: Vec::new(),
            paths: Vec::new(),
            receipts: source_cut.receipts.clone(),
            frontier: Vec::new(),
            truncation: TraversalTruncation::default(),
        }),
        cut: Mutex::new(source_cut.clone()),
        failures: AtomicUsize::new(0),
        traversals: AtomicUsize::new(0),
    });
    let durable_events = Arc::new(MockEvents {
        ledger_id,
        watermark: 14,
        envelopes: Mutex::new(BTreeMap::new()),
    });
    let operation;
    let result;

    {
        let store = Arc::new(CurationStore::new(sled::open(&store_path).unwrap()).unwrap());
        let revision = store.install_rule(rule(), 1).unwrap();
        operation = CurationOperation::reconstruct(
            authority(),
            revision.revision_ref(),
            source_cut,
            revision.rule.traversal_request(),
        )
        .unwrap();
        let interrupted_events = Arc::new(InterruptingEvents {
            inner: Arc::clone(&durable_events),
            successful_appends: AtomicUsize::new(0),
            successes_before_failure,
        });
        let actor = StandingCurationActor::new(
            "world_model.standing_curation",
            "session-pending-publication",
            authority(),
            revision,
            Arc::clone(&store),
            Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
            interrupted_events as Arc<dyn CurationEventPort>,
        )
        .unwrap();

        let interrupted = actor.bounded_step(1);

        assert_eq!(interrupted.results_persisted, 1);
        assert_eq!(interrupted.publications_appended, successes_before_failure);
        assert_eq!(interrupted.retryable_errors.len(), 1);
        result = store
            .result_for_operation(&operation.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(result.disposition, CurationTerminalDisposition::Applied);
        assert_eq!(
            store
                .publication_receipt(&result.result_id, CurationPublicationKind::Semantic)
                .unwrap()
                .is_some(),
            successes_before_failure == 1
        );
        assert!(store
            .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)
            .unwrap()
            .is_none());
        store.flush().unwrap();
    }

    let reopened_store = Arc::new(CurationStore::new(sled::open(&store_path).unwrap()).unwrap());
    let reopened_actor = StandingCurationActor::new(
        "world_model.standing_curation",
        "session-pending-publication",
        authority(),
        reopened_store.active_rule("agent-a").unwrap().unwrap(),
        Arc::clone(&reopened_store),
        Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
        Arc::clone(&durable_events) as Arc<dyn CurationEventPort>,
    )
    .unwrap();

    let recovered = reopened_actor.bounded_step(1);

    assert_eq!(recovered.reused_results, 1);
    assert_eq!(recovered.results_persisted, 0);
    assert_eq!(
        recovered.publications_appended,
        2 - successes_before_failure
    );
    assert_eq!(traversal.traversals.load(Ordering::SeqCst), 1);
    assert_eq!(durable_events.envelopes.lock().unwrap().len(), 2);
    assert_eq!(
        reopened_store
            .result_for_operation(&operation.operation_id)
            .unwrap(),
        Some(result.clone())
    );
    assert!(reopened_store
        .publication_receipt(&result.result_id, CurationPublicationKind::Semantic)
        .unwrap()
        .is_some());
    assert!(reopened_store
        .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)
        .unwrap()
        .is_some());
}

struct Fixture {
    actor: StandingCurationActor,
    store: Arc<CurationStore>,
    rule: StandingCurationRuleRevision,
    traversal: Arc<MockTraversal>,
    events: Arc<MockEvents>,
}

impl Fixture {
    fn new(failures: usize) -> Self {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = Arc::new(CurationStore::new(db).unwrap());
        let rule = store.install_rule(rule(), 1).unwrap();
        let cut = cut(10);
        let ledger_id = cut.event_position.ledger_id;
        let traversal = Arc::new(MockTraversal {
            result: Mutex::new(TraversalResult {
                absent_roots: Vec::new(),
                result_id: "initial-traversal".to_string(),
                cut_id: cut.cut_id.clone(),
                objects: Vec::<OwnerObjectPublication>::new(),
                occurrences: Vec::<OwnerRelationOccurrence>::new(),
                paths: Vec::new(),
                receipts: Vec::new(),
                frontier: Vec::new(),
                truncation: TraversalTruncation::default(),
            }),
            cut: Mutex::new(cut),
            failures: AtomicUsize::new(failures),
            traversals: AtomicUsize::new(0),
        });
        let events = Arc::new(MockEvents {
            ledger_id,
            watermark: 10,
            envelopes: Mutex::new(BTreeMap::new()),
        });
        let actor = StandingCurationActor::new(
            "world_model.standing_curation",
            "session-a",
            authority(),
            rule.clone(),
            Arc::clone(&store),
            Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
            Arc::clone(&events) as Arc<dyn CurationEventPort>,
        )
        .unwrap();
        Self {
            actor,
            store,
            rule,
            traversal,
            events,
        }
    }
}

fn rule() -> StandingCurationRule {
    StandingCurationRule {
        coverage: None,
        source_event_route: None,
        judgment_scope: None,
        rule_id: "rule-a".to_string(),
        agent_id: "agent-a".to_string(),
        source_owner_id: "workspace_fs".to_string(),
        scope: scope(),
        roots: vec![DomainObjectRef::new("workspace_fs", "node", "subject-a").unwrap()],
        traversal_direction: TraversalDirection::Incoming,
        bounds: TraversalBounds {
            max_depth: 4,
            max_objects: 32,
            max_occurrences: 32,
            max_paths: 32,
        },
        expected_object_kind: "assessment".to_string(),
        expected_object_id: "subject-a::expected".to_string(),
        relation_type: "curation_assesses".to_string(),
        output_policy_revision: "output-policy-v1".to_string(),
        realization: None,
    }
}

fn authority() -> CurationAuthority {
    CurationAuthority {
        agent_id: "agent-a".to_string(),
        perspective: PerspectiveKey::new("frame", "analysis").unwrap(),
        branch_scope: BranchScope::main(),
        activation_generation: "activation-a".to_string(),
        admission_epoch: None,
        subject: DomainObjectRef::new("workspace_fs", "node", "subject-a").unwrap(),
    }
}

fn planned_authorization(operation: &CurationOperation) -> CurationPlannedAuthorization {
    let fields = (
        "agent-a",
        "goal-a",
        "plan-a",
        "epistemic-a",
        &operation.operation_id,
        "context-a",
        "authority-a",
        "activation-a",
        "idempotency-a",
    );
    CurationPlannedAuthorization {
        authorization_id: stable_identity("agent-product-authorization-v1", &fields).unwrap(),
        agent_id: fields.0.to_string(),
        goal_id: fields.1.to_string(),
        plan_revision_id: fields.2.to_string(),
        product_id: fields.3.to_string(),
        operation_id: fields.4.clone(),
        context_id: fields.5.to_string(),
        authority_scope_id: fields.6.to_string(),
        activation_generation: fields.7.to_string(),
        admission_epoch: None,
        idempotency_key: fields.8.to_string(),
    }
}

fn scope() -> OwnerPublicationScope {
    OwnerPublicationScope {
        scope_id: "workspace-a".to_string(),
        branch_id: Some("main".to_string()),
        perspective_id: Some("analysis".to_string()),
        valid_at: None,
    }
}

fn cut(after_seq: u64) -> TraversalCut {
    let ledger_id = LedgerIdentity::new();
    let mut cut = TraversalCut {
        cut_id: String::new(),
        owners: vec![
            TraversalOwnerRequirement {
                event_source: None,
                owner_id: CURATION_OWNER_ID.to_string(),
                scope: scope(),
                required: false,
            },
            TraversalOwnerRequirement {
                event_source: None,
                owner_id: "workspace_fs".to_string(),
                scope: scope(),
                required: true,
            },
        ],
        receipts: vec![OwnerGraphRevisionReceipt {
            event_coverage: None,
            owner_id: "workspace_fs".to_string(),
            revision_id: "workspace-v1".to_string(),
            scope: scope(),
            completeness: OwnerCompletenessReceipt {
                receipt_id: "workspace-v1-complete".to_string(),
                scope: scope(),
                included_ids: vec!["workspace-v1-root".to_string()],
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
            source_event: Some(meld_events::EventRecordRef {
                ledger_id,
                seq: after_seq,
            }),
            projection_position: LedgerCursor {
                ledger_id,
                after_seq,
            },
        }],
        scope: scope(),
        currentness: OwnerCurrentnessPolicy::LatestComplete,
        event_position: LedgerCursor {
            ledger_id,
            after_seq,
        },
        graph_position: LedgerCursor {
            ledger_id,
            after_seq,
        },
        status: TraversalCutStatus::Complete,
        issues: Vec::new(),
    };
    cut.cut_id = traversal_cut_identity(&cut).unwrap();
    cut
}

#[test]
fn realization_requires_exact_owner_object_qualifications_and_complete_coverage() {
    use crate::world_state::graph::contracts::{HydrationReference, OwnerPublicationState};
    for scenario in [
        "absent",
        "foreign_epoch",
        "realized",
        "withdrawn",
        "incomplete",
    ] {
        let mut fixture = Fixture::new(0);
        let observed_object = DomainObjectRef::new("workspace_fs", "nonce", "nonce-a").unwrap();
        let mut installed = rule();
        installed.roots.push(observed_object.clone());
        installed.realization = Some(CurationRealizationRule {
            observed_object: observed_object.clone(),
            required_qualifications: BTreeMap::from([("epoch".into(), "epoch-a".into())]),
            realized_relation_type: "realizes".into(),
            not_realized_relation_type: "not_realized_within_cut".into(),
        });
        let revision = fixture.store.install_rule(installed, 2).unwrap();
        fixture.actor = StandingCurationActor::new(
            "curation",
            "session-realization",
            authority(),
            revision,
            fixture.store.clone(),
            fixture.traversal.clone(),
            fixture.events.clone(),
        )
        .unwrap();
        {
            let mut result = fixture.traversal.result.lock().unwrap();
            if scenario != "incomplete" {
                result.receipts = fixture.traversal.cut.lock().unwrap().receipts.clone();
            }
            if scenario != "absent" {
                result.objects.push(OwnerObjectPublication {
                    publication_id: "nonce-publication-a".into(),
                    object_ref: observed_object.clone(),
                    state: if scenario == "withdrawn" {
                        OwnerPublicationState::Withdrawn
                    } else {
                        OwnerPublicationState::Observed
                    },
                    source_product_ref: "nonce-event-a".into(),
                    hydration: HydrationReference {
                        owner_id: "workspace_fs".into(),
                        product_kind: "nonce".into(),
                        product_id: "nonce-a".into(),
                        revision_id: "nonce-a-v1".into(),
                        role: "observed".into(),
                    },
                    provenance_refs: vec!["nonce-event-a".into()],
                    qualifications: BTreeMap::from([(
                        "epoch".into(),
                        if scenario == "foreign_epoch" {
                            "epoch-b"
                        } else {
                            "epoch-a"
                        }
                        .into(),
                    )]),
                });
            }
        }
        let report = fixture.actor.bounded_step(1);
        assert!(report.fatal_errors.is_empty(), "{scenario}: {report:?}");
        let result = result_events(&fixture.events).pop().unwrap();
        if scenario == "incomplete" {
            assert_eq!(result.disposition, CurationTerminalDisposition::Incomplete);
            assert!(result.semantic_publication.is_none());
            continue;
        }
        assert_eq!(result.disposition, CurationTerminalDisposition::Applied);
        let publication = result.semantic_publication.unwrap();
        let relation = publication
            .batch
            .relations
            .iter()
            .find(|relation| relation.dst == observed_object)
            .unwrap();
        assert_eq!(relation.dst, observed_object);
        assert_eq!(
            relation.relation_type,
            if scenario == "realized" {
                "realizes"
            } else {
                "not_realized_within_cut"
            }
        );
        if scenario == "realized" {
            assert!(relation
                .provenance_refs
                .contains(&"nonce-publication-a".into()));
        }
        // Seeing our own publication must settle without an endless chain of new semantic revisions.
        {
            let mut traversal = fixture.traversal.result.lock().unwrap();
            traversal.objects.extend(publication.batch.objects);
            traversal.occurrences.extend(publication.batch.relations);
        }
        {
            let mut cut = fixture.traversal.cut.lock().unwrap();
            cut.event_position.after_seq += 1;
            cut.graph_position.after_seq += 1;
            // A new owner receipt changes selection while retaining the same source predicate.
            let mut own = cut.receipts[0].clone();
            own.owner_id = CURATION_OWNER_ID.into();
            own.revision_id = publication.batch.revision_id;
            cut.receipts.push(own);
            cut.cut_id = traversal_cut_identity(&cut).unwrap();
        }
        let replay = fixture.actor.bounded_step(1);
        assert_eq!(replay.publications_appended, 1, "{scenario}: {replay:?}");
        assert_eq!(
            result_events(&fixture.events).last().unwrap().disposition,
            CurationTerminalDisposition::Unchanged
        );
    }
}

#[test]
fn accepted_operation_finishes_its_original_rule_after_the_actor_is_rebound() {
    let fixture = Fixture::new(0);
    let original = CurationOperation::reconstruct(
        authority(),
        fixture.rule.revision_ref(),
        fixture.traversal.cut.lock().unwrap().clone(),
        fixture.rule.rule.traversal_request(),
    )
    .unwrap();
    fixture.store.put_operation(&original).unwrap();
    fixture
        .store
        .put_acceptance(&CurationAcceptanceRecord::for_operation(&original, &fixture.rule).unwrap())
        .unwrap();
    let mut replacement = rule();
    replacement.expected_object_id = "successor-expectation".into();
    replacement.roots[0].object_id = "successor-source".into();
    replacement.judgment_scope = Some(CurationJudgmentScope {
        subject: authority().subject,
        perspective: authority().perspective,
        branch_scope: authority().branch_scope,
    });
    let new_rule = fixture.store.install_rule(replacement, 20).unwrap();
    let actor = StandingCurationActor::new(
        "curation",
        "session-rebound",
        authority(),
        new_rule.clone(),
        fixture.store.clone(),
        fixture.traversal.clone(),
        fixture.events.clone(),
    )
    .unwrap()
    .with_authority_port(Arc::new(MutableCurationAuthority(Mutex::new(None))));
    let report = actor.bounded_step(1);
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert_eq!(report.results_persisted, 1, "{report:?}");
    let result = fixture
        .store
        .result_for_operation(&original.operation_id)
        .unwrap()
        .unwrap();
    let publication = result.semantic_publication.unwrap();
    assert_eq!(
        publication.batch.objects[0].object_ref,
        fixture.rule.rule.expected_object().unwrap()
    );
    assert_ne!(
        publication.batch.objects[0].object_ref,
        new_rule.rule.expected_object().unwrap()
    );
    assert_eq!(
        publication.batch.relations[0].dst,
        fixture.rule.rule.roots[0]
    );
    assert_eq!(actor.bounded_step(1).operations_attempted, 0);
}

#[test]
fn coverage_requires_ready_owner_evidence_and_exact_relational_items() {
    use crate::world_state::graph::contracts::{HydrationReference, OwnerPublicationState};
    for scenario in [
        "missing",
        "satisfied",
        "unready",
        "incomplete",
        "foreign_source",
        "ambiguous_target",
    ] {
        let mut fixture = Fixture::new(0);
        let mut configured = rule();
        configured.coverage = Some(CurationCoverageRule {
            source_required_qualifications: BTreeMap::from([("ready".into(), "true".into())]),
            target_kind: "target".into(),
            target_required_qualifications: BTreeMap::from([("available".into(), "true".into())]),
            item_kind: "comparison".into(),
            item_target_relation: "compares_target".into(),
            item_source_relation: "compares_source".into(),
            source_kind: "fact".into(),
            item_required_qualifications: BTreeMap::from([("represented".into(), "true".into())]),
            expectation_kind: "expected_target".into(),
            requirement_kind: "required_fact".into(),
            expected_target_relation: "expects".into(),
            expectation_requirement_relation: "requires".into(),
            requirement_source_relation: "requires_fact".into(),
        });
        fixture.rule = fixture.store.install_rule(configured, 2).unwrap();
        fixture.actor = StandingCurationActor::new(
            "world_model.standing_curation",
            "session-a",
            authority(),
            fixture.rule.clone(),
            fixture.store.clone(),
            fixture.traversal.clone(),
            fixture.events.clone(),
        )
        .unwrap();
        let hydration = HydrationReference {
            owner_id: "workspace_fs".into(),
            product_kind: "snapshot".into(),
            product_id: "workspace-v1".into(),
            revision_id: "workspace-v1".into(),
            role: "observed".into(),
        };
        let object = |kind: &str, id: &str, qualifications: BTreeMap<String, String>| {
            OwnerObjectPublication {
                publication_id: format!("workspace-v1::{kind}::{id}"),
                object_ref: DomainObjectRef::new("workspace_fs", kind, id).unwrap(),
                state: OwnerPublicationState::Observed,
                source_product_ref: "workspace-v1".into(),
                hydration: hydration.clone(),
                provenance_refs: vec![],
                qualifications,
            }
        };
        let root = object(
            "node",
            "subject-a",
            BTreeMap::from([("ready".into(), (scenario != "unready").to_string())]),
        );
        let target = object(
            "target",
            "target-a",
            BTreeMap::from([("available".into(), "true".into())]),
        );
        let a = object("fact", "a", BTreeMap::new());
        let b = object("fact", "b", BTreeMap::new());
        let first = object(
            "comparison",
            "a",
            BTreeMap::from([("represented".into(), "true".into())]),
        );
        let second = object(
            "comparison",
            "b",
            BTreeMap::from([("represented".into(), (scenario == "satisfied").to_string())]),
        );
        let relation = |id: &str,
                        relation_type: &str,
                        src: &DomainObjectRef,
                        dst: &DomainObjectRef| OwnerRelationOccurrence {
            occurrence_id: id.into(),
            relation_type: relation_type.into(),
            src: src.clone(),
            dst: dst.clone(),
            source_product_ref: "workspace-v1".into(),
            hydration: hydration.clone(),
            qualifications: BTreeMap::new(),
            provenance_refs: vec![],
        };
        {
            let mut traversal = fixture.traversal.result.lock().unwrap();
            traversal.receipts = fixture.traversal.cut.lock().unwrap().receipts.clone();
            if scenario == "incomplete" {
                traversal.receipts[0].completeness.status = OwnerCompletenessStatus::Incomplete;
            }
            traversal.occurrences = vec![
                relation(
                    "target-a",
                    "compares_target",
                    &first.object_ref,
                    &target.object_ref,
                ),
                relation(
                    "target-b",
                    "compares_target",
                    &second.object_ref,
                    &target.object_ref,
                ),
                relation(
                    "source-a",
                    "compares_source",
                    &first.object_ref,
                    &a.object_ref,
                ),
                relation(
                    "source-b",
                    "compares_source",
                    &second.object_ref,
                    &b.object_ref,
                ),
            ];
            if scenario == "foreign_source" {
                traversal.occurrences[3].dst =
                    DomainObjectRef::new("foreign", "fact", "b").unwrap();
            }
            if scenario == "ambiguous_target" {
                let mut duplicate = traversal.occurrences[0].clone();
                duplicate.occurrence_id = "duplicate".into();
                traversal.occurrences.push(duplicate);
            }
            traversal.objects = vec![root, target, a, b, first, second];
        }
        let step = fixture.actor.bounded_step(1);
        if ["foreign_source", "ambiguous_target"].contains(&scenario) {
            let failed = result_events(&fixture.events).pop().unwrap();
            assert_eq!(failed.disposition, CurationTerminalDisposition::Failed);
            assert!(failed.semantic_publication.is_none());
            continue;
        }
        assert!(
            step.fatal_errors.is_empty() && step.retryable_errors.is_empty(),
            "{scenario}: {step:?}"
        );
        let result = result_events(&fixture.events).pop().unwrap();
        if ["unready", "incomplete"].contains(&scenario) {
            assert_eq!(result.disposition, CurationTerminalDisposition::Incomplete);
            assert!(result.semantic_publication.is_none());
            continue;
        }
        let publication = result.semantic_publication.as_ref().unwrap();
        assert_eq!(
            publication
                .batch
                .objects
                .iter()
                .filter(|object| object.object_ref.object_kind == "required_fact")
                .count(),
            2
        );
        assert_eq!(
            publication.batch.objects[0]
                .qualifications
                .get("coverage")
                .map(String::as_str),
            Some(if scenario == "satisfied" {
                "satisfied"
            } else {
                "unsatisfied"
            })
        );
        let replayed = fixture.actor.bounded_step(1);
        assert_eq!(replayed.publications_appended, 0);
        assert_eq!(fixture.traversal.traversals.load(Ordering::SeqCst), 1);
        assert_eq!(result_events(&fixture.events).pop().unwrap(), result);
    }
}
