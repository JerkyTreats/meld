use std::collections::BTreeMap;
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

struct MockEvents {
    ledger_id: LedgerIdentity,
    watermark: u64,
    envelopes: Mutex<BTreeMap<String, EventEnvelope>>,
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
        source_event: meld_events::EventRecordRef {
            ledger_id: next_cut.event_position.ledger_id,
            seq: next_cut.event_position.after_seq,
        },
        projection_position: next_cut.graph_position,
    }];
    next_cut.cut_id = traversal_cut_identity(&next_cut).unwrap();
    *fixture.traversal.cut.lock().unwrap() = next_cut.clone();
    *fixture.traversal.result.lock().unwrap() = TraversalResult {
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
fn owner_neutral_rule_authors_dissimilar_installed_vocabulary() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(CurationStore::new(db).unwrap());
    let subject = DomainObjectRef::new("inventory", "asset", "asset-a").unwrap();
    let neutral_rule = StandingCurationRule {
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
    };
    let revision = store.install_rule(neutral_rule, 1).unwrap();
    let mut source_cut = cut(10);
    source_cut.owners[1].owner_id = "inventory".to_string();
    source_cut.receipts[0].owner_id = "inventory".to_string();
    source_cut.cut_id = traversal_cut_identity(&source_cut).unwrap();
    let ledger_id = source_cut.event_position.ledger_id;
    let traversal = Arc::new(MockTraversal {
        result: Mutex::new(TraversalResult {
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

struct Fixture {
    actor: StandingCurationActor,
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
            rule,
            store,
            Arc::clone(&traversal) as Arc<dyn CurationTraversalPort>,
            Arc::clone(&events) as Arc<dyn CurationEventPort>,
        )
        .unwrap();
        Self {
            actor,
            traversal,
            events,
        }
    }
}

fn rule() -> StandingCurationRule {
    StandingCurationRule {
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
    }
}

fn authority() -> CurationAuthority {
    CurationAuthority {
        agent_id: "agent-a".to_string(),
        perspective: PerspectiveKey::new("frame", "analysis").unwrap(),
        branch_scope: BranchScope::main(),
        activation_generation: "activation-a".to_string(),
        subject: DomainObjectRef::new("workspace_fs", "node", "subject-a").unwrap(),
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
                owner_id: CURATION_OWNER_ID.to_string(),
                scope: scope(),
                required: false,
            },
            TraversalOwnerRequirement {
                owner_id: "workspace_fs".to_string(),
                scope: scope(),
                required: true,
            },
        ],
        receipts: vec![OwnerGraphRevisionReceipt {
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
            source_event: meld_events::EventRecordRef {
                ledger_id,
                seq: after_seq,
            },
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
