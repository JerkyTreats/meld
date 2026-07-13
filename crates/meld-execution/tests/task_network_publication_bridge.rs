#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::{
    AppendDisposition, AppendReceipt, EventAppendCapability, EventAuthority,
    EventAuthorityOpenOptions, EventEnvelope, EventReplayCapability, LedgerCursor, LedgerIdentity,
    ReplayRequest,
};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::OutcomeStatus;
use meld_execution::task_network::journal::JournalRecord;
use meld_execution::task_network::mutation::Rejection;
use meld_execution::task_network::outcome::PublicationState;
use meld_execution::task_network::publication::{
    build_publication_envelope, publish_pending_publications as authority_publish_pending,
    publish_publication as authority_publish_one, EventAppendFailure, EventAppendSink,
    PublicationBridgeError, PublicationBridgeReport, PublicationPublishResult,
    PublishPendingPublicationsRequest,
};
use meld_execution::task_network::store::{TaskNetworkStoreError, TaskNetworkStoreFactory};
use meld_execution::task_network::{
    PublicationRuntime, SledTaskNetworkStore, TaskNetworkAuthority, TaskNetworkCommandPort,
    TaskNetworkQueryPort,
};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};

struct FailingSink;

impl EventAppendSink for FailingSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        test_ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        _envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        Err(EventAppendFailure::retryable("append unavailable"))
    }
}

struct FatalSink;

impl EventAppendSink for FatalSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        test_ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        _envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        Err(EventAppendFailure::fatal("append rejected"))
    }
}

#[derive(Clone)]
struct BarrierSink {
    append: EventAppendCapability,
    barrier: Arc<Barrier>,
}

impl EventAppendSink for BarrierSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.append.ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        let receipt = EventAppendSink::append_envelope_idempotent(&self.append, envelope)?;
        self.barrier.wait();
        Ok(receipt)
    }
}

struct DishonestSink {
    advertised: LedgerIdentity,
    returned: LedgerIdentity,
}

struct ShutdownAfterAppendSink<'a> {
    events: &'a TestEvents,
    authority: RefCell<TaskNetworkAuthority>,
}

impl EventAppendSink for ShutdownAfterAppendSink<'_> {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.events.ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        let receipt = self.events.append_envelope_idempotent(envelope)?;
        self.authority
            .borrow_mut()
            .shutdown()
            .map_err(|error| EventAppendFailure::retryable(error.to_string()))?;
        Ok(receipt)
    }
}

impl EventAppendSink for DishonestSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.advertised
    }

    fn append_envelope_idempotent(
        &self,
        _envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        Ok(AppendReceipt {
            ledger_id: self.returned,
            seq: 99,
            disposition: AppendDisposition::Inserted,
        })
    }
}

#[derive(Default)]
struct RecordingSink {
    envelopes: RefCell<Vec<EventEnvelope>>,
}

impl EventAppendSink for RecordingSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        test_ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        let mut envelopes = self.envelopes.borrow_mut();
        envelopes.push(envelope);
        Ok(AppendReceipt {
            ledger_id: self.ledger_identity(),
            seq: envelopes.len() as u64,
            disposition: AppendDisposition::Inserted,
        })
    }
}

fn test_ledger_identity() -> LedgerIdentity {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}

struct TestEvents {
    append: EventAppendCapability,
    replay: EventReplayCapability,
}

impl EventAppendSink for TestEvents {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.append.ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        EventAppendSink::append_envelope_idempotent(&self.append, envelope)
    }
}

struct TaskDb {
    _temp: tempfile::TempDir,
    factory: TaskNetworkStoreFactory,
}

struct TestNetwork {
    factory: TaskNetworkStoreFactory,
    store: Option<SledTaskNetworkStore>,
}

impl TestNetwork {
    fn raw_mut(&mut self) -> &mut SledTaskNetworkStore {
        self.store.as_mut().unwrap()
    }

    fn state(&self) -> &meld_execution::task_network::NetworkState {
        self.store.as_ref().unwrap().state()
    }

    fn flush(&self) -> Result<(), TaskNetworkStoreError> {
        self.store.as_ref().unwrap().flush()
    }

    fn with_authority<T>(
        &mut self,
        operation: impl FnOnce(&TaskNetworkQueryPort, &TaskNetworkCommandPort) -> T,
    ) -> T {
        let store = self.store.take().unwrap();
        store.flush().unwrap();
        drop(store);
        let mut authority = TaskNetworkAuthority::open(&self.factory, "network-docs", 32).unwrap();
        let query = authority.query_port();
        let commands = authority.command_port();
        let result = operation(&query, &commands);
        authority.shutdown().unwrap();
        self.store = Some(self.factory.open_network("network-docs").unwrap());
        result
    }
}

fn open_store(db: &TaskDb) -> TestNetwork {
    TestNetwork {
        factory: db.factory.clone(),
        store: Some(db.factory.open_network("network-docs").unwrap()),
    }
}

fn open_task_db() -> TaskDb {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    TaskDb {
        _temp: temp,
        factory,
    }
}

fn open_events(tempdir: &tempfile::TempDir) -> TestEvents {
    let db = sled::open(tempdir.path()).unwrap();
    let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
    TestEvents {
        append: authority.append_capability(),
        replay: authority.replay_capability(),
    }
}

fn read_session(events: &TestEvents, session_id: &str) -> Vec<meld_events::EventRecord> {
    // Publication fixtures are intentionally small enough for one bounded
    // authority replay. Filtering here asserts the producer's session routing
    // without reaching through the capability to raw storage.
    events
        .replay
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: events.ledger_identity(),
                after_seq: 0,
            },
            limit: 1_024,
        })
        .unwrap()
        .records
        .into_iter()
        .filter(|record| record.envelope.session == session_id)
        .collect()
}

fn request(limit: Option<usize>) -> PublishPendingPublicationsRequest {
    PublishPendingPublicationsRequest {
        session_id: "session-publication".to_string(),
        worker_id: "worker-publication".to_string(),
        limit,
    }
}

fn publish_pending_publications<E: EventAppendSink>(
    network: &mut TestNetwork,
    events: &E,
    request: PublishPendingPublicationsRequest,
) -> Result<PublicationBridgeReport, PublicationBridgeError> {
    network.with_authority(|query, commands| {
        authority_publish_pending(query, commands, events, request)
    })
}

fn publish_publication<E: EventAppendSink>(
    network: &mut TestNetwork,
    events: &E,
    request: &PublishPendingPublicationsRequest,
    publication_id: &str,
) -> Result<PublicationPublishResult, PublicationBridgeError> {
    network.with_authority(|query, commands| {
        authority_publish_one(query, commands, events, request, publication_id)
    })
}

fn record_success_publication(
    store: &mut TestNetwork,
    task_instance_id: &str,
    outcome_id: &str,
    claim_id: &str,
) -> String {
    let store = store.raw_mut();
    task_network_support::commit_single_task_sled(store, task_instance_id);
    let claimed_task = task_network_support::claim_ready_sled(
        store,
        &format!("command-claim-{claim_id}"),
        claim_id,
    );
    assert_eq!(claimed_task, task_instance_id);
    let claim = store.state().claims.get(claim_id).unwrap().clone();
    let outcome = task_network_support::outcome_for_claim(outcome_id, task_instance_id, &claim);
    let command = task_network_support::apply_sled_command(
        store,
        &format!("command-outcome-{outcome_id}"),
        Command::RecordTaskOutcome(outcome),
    );
    assert!(matches!(
        store.submit(command).unwrap(),
        Response::Accepted { .. }
    ));
    publication_id_for_outcome(store, outcome_id)
}

fn record_failed_publication(
    store: &mut TestNetwork,
    task_instance_id: &str,
    outcome_id: &str,
    claim_id: &str,
) -> String {
    let store = store.raw_mut();
    task_network_support::commit_single_task_sled(store, task_instance_id);
    let claimed_task = task_network_support::claim_ready_sled(
        store,
        &format!("command-claim-{claim_id}"),
        claim_id,
    );
    assert_eq!(claimed_task, task_instance_id);
    let claim = store.state().claims.get(claim_id).unwrap().clone();
    let mut outcome = task_network_support::outcome_for_claim(outcome_id, task_instance_id, &claim);
    outcome.status = OutcomeStatus::Failed;
    outcome.error = Some("runtime failed".to_string());
    outcome.artifact_records.clear();
    let command = task_network_support::apply_sled_command(
        store,
        &format!("command-outcome-{outcome_id}"),
        Command::RecordTaskOutcome(outcome),
    );
    assert!(matches!(
        store.submit(command).unwrap(),
        Response::Accepted { .. }
    ));
    publication_id_for_outcome(store, outcome_id)
}

fn publication_id_for_outcome(store: &SledTaskNetworkStore, outcome_id: &str) -> String {
    store
        .state()
        .publications
        .values()
        .find(|publication| publication.outcome.outcome_id == outcome_id)
        .unwrap()
        .publication_id
        .clone()
}

fn reopen_with_frozen_legacy_publication(
    mut store: TestNetwork,
    db: &TaskDb,
    publication_id: &str,
    legacy_event_seq: u64,
) -> TestNetwork {
    let revision = store.state().revision + 1;
    let mut publication = store.state().publications[publication_id].clone();
    publication.state = PublicationState::Published {
        marked_revision: revision,
        receipt: None,
        legacy_event_seq: Some(legacy_event_seq),
    };
    let mut legacy_state = store.state().clone();
    legacy_state
        .publications
        .insert(publication_id.to_string(), publication.clone());
    legacy_state.set_revision_and_hash(revision);
    let frozen_journal = serde_json::json!({
        "record": JournalRecord::Publication(publication),
    });
    assert!(
        frozen_journal["record"]["Publication"]["state"]["Published"]
            .get("receipt")
            .is_none()
    );
    assert_eq!(
        frozen_journal["record"]["Publication"]["state"]["Published"]["event_seq"],
        serde_json::json!(legacy_event_seq)
    );
    let raw_store = store.store.take().unwrap();
    raw_store.flush().unwrap();
    drop(raw_store);

    let storage_path = db.factory.root().join("network-docs.sled");
    let sled_db = sled::open(storage_path).unwrap();

    sled_db
        .open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(
            revision.to_be_bytes(),
            serde_json::to_vec(&frozen_journal).unwrap(),
        )
        .unwrap();
    sled_db
        .open_tree("task_network_latest_state")
        .unwrap()
        .insert(
            "latest",
            serde_json::to_vec(&serde_json::json!({ "state": legacy_state })).unwrap(),
        )
        .unwrap();
    sled_db.flush().unwrap();
    drop(sled_db);
    open_store(db)
}

#[test]
fn publication_bridge_appends_pending_task_outcome_once() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    assert!(matches!(publication.state, PublicationState::Pending));
    let events = open_events(&event_tempdir);

    let report = publish_pending_publications(&mut store, &events, request(None)).unwrap();

    assert_eq!(report.actor_id, "execution.task_network.publication");
    assert_eq!(report.scope.network_id, "network-docs");
    assert_eq!(report.scope.session_id, "session-publication");
    assert_eq!(report.scope.worker_id, "worker-publication");
    assert!(report.input_revision < report.output_revision);
    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(!report.budget_exhausted);
    assert_eq!(report.results.len(), 1);
    let PublicationPublishResult::Published {
        publication_id: reported_id,
        event_record_id,
        receipt,
        marked_revision,
    } = &report.results[0]
    else {
        panic!("publication bridge did not publish");
    };
    assert_eq!(reported_id, &publication_id);
    assert_eq!(
        event_record_id,
        &format!("execution::task_network_publication::{publication_id}")
    );
    let event_records = read_session(&events, "session-publication");
    assert_eq!(event_records.len(), 1);
    let event = &event_records[0];
    assert_eq!(receipt.seq, event.seq);
    assert_eq!(event.envelope.event_type, "execution.task.succeeded");
    assert_eq!(event.envelope.data, publication.event_payload());
    assert_eq!(
        event.envelope.record_id.as_deref(),
        Some(event_record_id.as_str())
    );
    assert_eq!(event.envelope.domain_id, "execution");
    assert_eq!(
        event.envelope.stream_id,
        format!(
            "task_network::{}::task::{}",
            publication.network_id, publication.outcome.task_instance_id
        )
    );

    let object_keys = event
        .envelope
        .objects
        .iter()
        .map(|object| object.index_key())
        .collect::<BTreeSet<_>>();
    assert!(object_keys.contains(&format!(
        "execution::task_network::{}",
        publication.network_id
    )));
    assert!(object_keys.contains(&format!(
        "execution::task_run::{}",
        publication.outcome.task_instance_id
    )));
    assert!(object_keys.contains(&format!(
        "execution::task_outcome::{}",
        publication.outcome.outcome_id
    )));
    assert!(object_keys.contains(&format!("execution::task_publication::{publication_id}")));
    let artifact = &publication.outcome.artifact_records[0];
    assert!(object_keys.contains(&format!("execution::artifact::{}", artifact.artifact_id)));
    assert!(object_keys.contains(&format!(
        "execution::artifact_slot::{}::{}",
        publication.outcome.task_instance_id, artifact.artifact_type_id
    )));

    let relation_types = event
        .envelope
        .relations
        .iter()
        .map(|relation| relation.relation_type.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        relation_types,
        BTreeSet::from([
            "attached_to",
            "member_of",
            "produced_by",
            "published_from",
            "selected"
        ])
    );

    let stored = store.state().publications.get(&publication_id).unwrap();
    let ledger_id = events.ledger_identity();
    assert!(matches!(
        stored.state,
        PublicationState::Published {
            marked_revision: stored_revision,
            receipt: Some(stored_receipt),
            legacy_event_seq: None,
        } if stored_revision == *marked_revision
            && stored_receipt == *receipt
            && stored_receipt.ledger_id == ledger_id
    ));
    assert_eq!(*marked_revision, store.state().revision);
    store.flush().unwrap();
    drop(store);

    let reopened = open_store(&task_db);
    assert!(matches!(
        reopened.state().publications.get(&publication_id).unwrap().state,
        PublicationState::Published {
            marked_revision: reopened_revision,
            receipt: Some(reopened_receipt),
            legacy_event_seq: None,
        } if reopened_revision == *marked_revision
            && reopened_receipt == *receipt
            && reopened_receipt.ledger_id == ledger_id
    ));
}

#[test]
fn persisted_legacy_receipt_reopens_and_upgrades() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let events = open_events(&event_tempdir);
    let publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    let envelope = build_publication_envelope("session-publication", &publication).unwrap();
    let original_receipt = events.append_envelope_idempotent(envelope).unwrap();
    let mut store = reopen_with_frozen_legacy_publication(
        store,
        &task_db,
        &publication_id,
        original_receipt.seq,
    );
    assert!(matches!(
        store.state().publications[&publication_id].state,
        PublicationState::Published {
            receipt: None,
            legacy_event_seq: Some(seq),
            ..
        } if seq == original_receipt.seq
    ));

    let report = publish_pending_publications(&mut store, &events, request(None)).unwrap();

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    assert_eq!(read_session(&events, "session-publication").len(), 1);
    assert!(matches!(
        store.state().publications[&publication_id].state,
        PublicationState::Published {
            receipt: Some(AppendReceipt {
                ledger_id,
                seq,
                disposition: AppendDisposition::Duplicate,
            }),
            legacy_event_seq: None,
            ..
        } if seq == original_receipt.seq && ledger_id == events.ledger_identity()
    ));
    store.flush().unwrap();
    drop(store);

    let reopened = open_store(&task_db);
    assert!(matches!(
        reopened.state().publications[&publication_id].state,
        PublicationState::Published {
            receipt: Some(AppendReceipt {
                ledger_id,
                seq,
                disposition: AppendDisposition::Duplicate,
            }),
            legacy_event_seq: None,
            ..
        } if seq == original_receipt.seq && ledger_id == events.ledger_identity()
    ));
}

#[test]
fn legacy_upgrade_failure_stays_retryable_without_state_regression() {
    let task_db = open_task_db();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let mut store = reopen_with_frozen_legacy_publication(store, &task_db, &publication_id, 7);
    let revision = store.state().revision;

    let report = publish_pending_publications(&mut store, &FailingSink, request(None)).unwrap();

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 0);
    assert_eq!(report.retryable_errors.len(), 1);
    assert_eq!(store.state().revision, revision);
    assert!(matches!(
        store.state().publications[&publication_id].state,
        PublicationState::Published {
            receipt: None,
            legacy_event_seq: Some(7),
            ..
        }
    ));
}

#[test]
fn dishonest_sink_receipt_is_rejected_before_publication_mutation() {
    let task_db = open_task_db();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let revision = store.state().revision;
    let sink = DishonestSink {
        advertised: LedgerIdentity::new(),
        returned: LedgerIdentity::new(),
    };

    let result = publish_publication(&mut store, &sink, &request(None), &publication_id).unwrap();

    let PublicationPublishResult::MarkRejected { reason, .. } = result else {
        panic!("dishonest sink receipt was accepted");
    };
    assert!(reason.contains(&sink.advertised.to_string()));
    assert!(reason.contains(&sink.returned.to_string()));
    assert_eq!(store.state().revision, revision);
    assert!(matches!(
        store.state().publications[&publication_id].state,
        PublicationState::Pending
    ));
}

#[test]
fn published_receipt_from_another_ledger_is_rejected_without_fallback() {
    let task_db = open_task_db();
    let first_tempdir = tempfile::tempdir().unwrap();
    let second_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let first = open_events(&first_tempdir);
    let second = open_events(&second_tempdir);
    publish_pending_publications(&mut store, &first, request(None)).unwrap();

    let result = publish_publication(&mut store, &second, &request(None), &publication_id).unwrap();

    let PublicationPublishResult::MarkRejected { reason, .. } = result else {
        panic!("foreign authority receipt was not rejected");
    };
    assert!(reason.contains(&first.ledger_identity().to_string()));
    assert!(reason.contains(&second.ledger_identity().to_string()));
    assert!(read_session(&second, "session-publication").is_empty());
}

#[test]
fn bulk_preflight_rejects_foreign_receipt_before_pending_append() {
    let task_db = open_task_db();
    let first_tempdir = tempfile::tempdir().unwrap();
    let second_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let published_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let first = open_events(&first_tempdir);
    publish_pending_publications(&mut store, &first, request(None)).unwrap();
    let pending_id =
        record_success_publication(&mut store, "task-beta", "outcome-beta", "claim-beta");
    let second = open_events(&second_tempdir);
    let revision = store.state().revision;

    let error = publish_pending_publications(&mut store, &second, request(None)).unwrap_err();

    assert!(matches!(
        error,
        PublicationBridgeError::LedgerIdentityMismatch {
            publication_id,
            expected,
            actual,
        } if publication_id == published_id
            && expected == second.ledger_identity()
            && actual == first.ledger_identity()
    ));
    assert_eq!(store.state().revision, revision);
    assert!(matches!(
        store.state().publications[&pending_id].state,
        PublicationState::Pending
    ));
    assert!(read_session(&second, "session-publication").is_empty());
}

#[test]
fn reducer_persists_foreign_ledger_rejection_without_publication_mutation() {
    let task_db = open_task_db();
    let first_tempdir = tempfile::tempdir().unwrap();
    let second_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let first = open_events(&first_tempdir);
    publish_pending_publications(&mut store, &first, request(None)).unwrap();
    let pending_id =
        record_success_publication(&mut store, "task-beta", "outcome-beta", "claim-beta");
    let second = open_events(&second_tempdir);

    let mut publication = store.state().publications[&pending_id].clone();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(AppendReceipt {
            ledger_id: second.ledger_identity(),
            seq: 1,
            disposition: AppendDisposition::Inserted,
        }),
        legacy_event_seq: None,
    };
    let command = task_network_support::apply_sled_command(
        store.raw_mut(),
        "command-mark-publication-on-foreign-ledger",
        Command::MarkPublication(publication),
    );

    let response = store.raw_mut().submit(command.clone()).unwrap();
    assert!(matches!(
        response,
        Response::Rejected(Rejection::PublicationLedgerMismatch { expected, actual })
            if expected == first.ledger_identity() && actual == second.ledger_identity()
    ));
    assert!(matches!(
        store.state().publications[&pending_id].state,
        PublicationState::Pending
    ));

    store.flush().unwrap();
    drop(store.store.take().unwrap());
    store.store = Some(store.factory.open_network("network-docs").unwrap());
    assert_eq!(store.raw_mut().submit(command).unwrap(), response);
    assert!(matches!(
        store.state().publications[&pending_id].state,
        PublicationState::Pending
    ));
}

#[test]
fn publication_runtime_actor_publishes_through_event_append_sink() {
    let task_db = open_task_db();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let events = RecordingSink::default();
    let runtime = PublicationRuntime::new();

    let report = store.with_authority(|query, commands| {
        runtime
            .publish_pending(query, commands, &events, request(None))
            .unwrap()
    });

    assert_eq!(
        report.actor_id,
        "execution.task_network.publication.runtime"
    );
    assert_eq!(report.scope.network_id, "network-docs");
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(!report.budget_exhausted);
    let envelopes = events.envelopes.borrow();
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0].event_type, "execution.task.succeeded");
    let expected_record_id = format!("execution::task_network_publication::{publication_id}");
    assert_eq!(
        envelopes[0].record_id.as_deref(),
        Some(expected_record_id.as_str())
    );
    assert!(matches!(
        store
            .state()
            .publications
            .get(&publication_id)
            .unwrap()
            .state,
        PublicationState::Published {
            receipt: Some(AppendReceipt { seq: 1, .. }),
            ..
        }
    ));
}

#[test]
fn publication_bridge_retry_does_not_append_duplicate_event() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let events = open_events(&event_tempdir);

    let first = publish_pending_publications(&mut store, &events, request(None)).unwrap();
    assert_eq!(first.results.len(), 1);
    let second = publish_pending_publications(&mut store, &events, request(None)).unwrap();
    assert!(second.results.is_empty());
    assert_eq!(second.items_attempted, 0);
    assert_eq!(second.items_committed, 0);
    assert_eq!(second.input_revision, second.output_revision);
    assert!(!second.budget_exhausted);
    assert_eq!(read_session(&events, "session-publication").len(), 1);

    let direct = publish_publication(&mut store, &events, &request(None), &publication_id).unwrap();
    assert_eq!(
        direct,
        PublicationPublishResult::AlreadyPublished { publication_id }
    );
    assert_eq!(read_session(&events, "session-publication").len(), 1);
}

#[test]
fn overlapping_publishers_converge_inserted_and_duplicate_receipts() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut network = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut network, "task-alpha", "outcome-alpha", "claim-alpha");
    let raw_store = network.store.take().unwrap();
    raw_store.flush().unwrap();
    drop(raw_store);

    let mut authority = TaskNetworkAuthority::open(&network.factory, "network-docs", 32).unwrap();
    let events = open_events(&event_tempdir);
    let sink = BarrierSink {
        append: events.append.clone(),
        barrier: Arc::new(Barrier::new(2)),
    };
    let first_query = authority.query_port();
    let first_commands = authority.command_port();
    let first_sink = sink.clone();
    let first_publication_id = publication_id.clone();
    let first = std::thread::spawn(move || {
        authority_publish_one(
            &first_query,
            &first_commands,
            &first_sink,
            &request(None),
            &first_publication_id,
        )
        .unwrap()
    });
    let second_query = authority.query_port();
    let second_commands = authority.command_port();
    let second_publication_id = publication_id.clone();
    let second = std::thread::spawn(move || {
        authority_publish_one(
            &second_query,
            &second_commands,
            &sink,
            &request(None),
            &second_publication_id,
        )
        .unwrap()
    });

    let results = [first.join().unwrap(), second.join().unwrap()];

    assert!(results
        .iter()
        .any(|result| matches!(result, PublicationPublishResult::Published { .. })));
    assert!(results
        .iter()
        .any(|result| matches!(result, PublicationPublishResult::AlreadyPublished { .. })));
    assert_eq!(read_session(&events, "session-publication").len(), 1);
    assert!(matches!(
        authority
            .query_port()
            .publication(&publication_id)
            .unwrap()
            .unwrap()
            .state,
        PublicationState::Published {
            receipt: Some(_),
            ..
        }
    ));
    authority.shutdown().unwrap();
}

#[test]
fn append_survives_authority_loss_before_mark_and_reopens_without_duplicate_event() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut network = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut network, "task-alpha", "outcome-alpha", "claim-alpha");
    let raw_store = network.store.take().unwrap();
    raw_store.flush().unwrap();
    drop(raw_store);

    let authority = TaskNetworkAuthority::open(&network.factory, "network-docs", 32).unwrap();
    let query = authority.query_port();
    let commands = authority.command_port();
    let events = open_events(&event_tempdir);
    let sink = ShutdownAfterAppendSink {
        events: &events,
        authority: RefCell::new(authority),
    };

    let error = authority_publish_one(&query, &commands, &sink, &request(None), &publication_id)
        .unwrap_err();

    assert!(error.retryable());
    assert_eq!(read_session(&events, "session-publication").len(), 1);
    drop(sink);
    network.store = Some(network.factory.open_network("network-docs").unwrap());
    assert!(matches!(
        network.state().publications[&publication_id].state,
        PublicationState::Pending
    ));

    let report = publish_pending_publications(&mut network, &events, request(None)).unwrap();

    assert_eq!(report.items_committed, 1);
    assert_eq!(read_session(&events, "session-publication").len(), 1);
    assert!(matches!(
        network.state().publications[&publication_id].state,
        PublicationState::Published {
            receipt: Some(AppendReceipt {
                disposition: AppendDisposition::Duplicate,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn publication_bridge_records_failure_without_published_mark() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let events = open_events(&event_tempdir);

    let result =
        publish_publication(&mut store, &FailingSink, &request(None), &publication_id).unwrap();

    assert_eq!(
        result,
        PublicationPublishResult::AppendFailed {
            publication_id: publication_id.clone(),
            error: "append unavailable".to_string()
        }
    );
    assert_eq!(read_session(&events, "session-publication").len(), 0);
    let publication = store.state().publications.get(&publication_id).unwrap();
    assert!(matches!(
        &publication.state,
        PublicationState::Failed { error } if error == "append unavailable"
    ));
}

#[test]
fn fatal_append_failure_does_not_persist_retry_state() {
    let task_db = open_task_db();
    let mut network = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut network, "task-alpha", "outcome-alpha", "claim-alpha");
    let revision = network.state().revision;

    let error =
        publish_publication(&mut network, &FatalSink, &request(None), &publication_id).unwrap_err();

    assert!(matches!(
        error,
        PublicationBridgeError::EventAppend {
            retryable: false,
            ..
        }
    ));
    assert!(!error.retryable());
    assert_eq!(network.state().revision, revision);
    assert!(matches!(
        network.state().publications[&publication_id].state,
        PublicationState::Pending
    ));
}

#[test]
fn publication_bridge_repeated_append_failure_stays_retryable() {
    let task_db = open_task_db();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");

    let first =
        publish_publication(&mut store, &FailingSink, &request(None), &publication_id).unwrap();
    let second = publish_pending_publications(&mut store, &FailingSink, request(None)).unwrap();

    assert!(matches!(
        first,
        PublicationPublishResult::AppendFailed { .. }
    ));
    assert_eq!(second.results.len(), 1);
    assert_eq!(second.items_attempted, 1);
    assert_eq!(second.items_committed, 0);
    assert_eq!(second.retryable_errors.len(), 1);
    assert!(second.fatal_errors.is_empty());
    assert!(matches!(
        second.results[0],
        PublicationPublishResult::AppendFailed { .. }
    ));
    assert!(matches!(
        store
            .state()
            .publications
            .get(&publication_id)
            .unwrap()
            .state,
        PublicationState::Failed { .. }
    ));
}

#[test]
fn publication_bridge_retries_failed_publication() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let events = open_events(&event_tempdir);

    let failed =
        publish_publication(&mut store, &FailingSink, &request(None), &publication_id).unwrap();
    assert!(matches!(
        failed,
        PublicationPublishResult::AppendFailed { .. }
    ));
    let retried = publish_pending_publications(&mut store, &events, request(None)).unwrap();

    assert_eq!(retried.results.len(), 1);
    assert_eq!(retried.items_attempted, 1);
    assert_eq!(retried.items_committed, 1);
    assert!(retried.retryable_errors.is_empty());
    assert!(retried.fatal_errors.is_empty());
    assert!(matches!(
        retried.results[0],
        PublicationPublishResult::Published { .. }
    ));
    assert_eq!(read_session(&events, "session-publication").len(), 1);
    assert!(matches!(
        store
            .state()
            .publications
            .get(&publication_id)
            .unwrap()
            .state,
        PublicationState::Published {
            receipt: Some(_),
            ..
        }
    ));
}

#[test]
fn publication_bridge_preserves_failure_outcome_event_type() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    let publication_id =
        record_failed_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    assert_eq!(publication.event_type(), "execution.task.failed");
    let events = open_events(&event_tempdir);

    let report = publish_pending_publications(&mut store, &events, request(None)).unwrap();

    assert_eq!(report.results.len(), 1);
    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    let event_records = read_session(&events, "session-publication");
    assert_eq!(event_records.len(), 1);
    assert_eq!(
        event_records[0].envelope.event_type,
        "execution.task.failed"
    );
    assert_eq!(event_records[0].envelope.data, publication.event_payload());
    assert_eq!(
        event_records[0].envelope.data["error"],
        serde_json::json!("runtime failed")
    );
}

#[test]
fn publication_bridge_honors_limit() {
    let task_db = open_task_db();
    let event_tempdir = tempfile::tempdir().unwrap();
    let mut store = open_store(&task_db);
    record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    record_success_publication(&mut store, "task-beta", "outcome-beta", "claim-beta");
    let events = open_events(&event_tempdir);

    let report = publish_pending_publications(&mut store, &events, request(Some(1))).unwrap();

    assert_eq!(report.results.len(), 1);
    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    assert!(report.budget_exhausted);
    assert_eq!(read_session(&events, "session-publication").len(), 1);
    let published_count = store
        .state()
        .publications
        .values()
        .filter(|publication| matches!(publication.state, PublicationState::Published { .. }))
        .count();
    let pending_count = store
        .state()
        .publications
        .values()
        .filter(|publication| matches!(publication.state, PublicationState::Pending))
        .count();
    assert_eq!(published_count, 1);
    assert_eq!(pending_count, 1);
}

#[test]
fn publication_bridge_report_classifies_retryable_append_failure() {
    let task_db = open_task_db();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");

    let report = publish_pending_publications(&mut store, &FailingSink, request(None)).unwrap();

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 0);
    assert_eq!(report.retryable_errors.len(), 1);
    assert!(report.fatal_errors.is_empty());
    assert_eq!(
        report.retryable_errors[0].publication_id.as_deref(),
        Some(publication_id.as_str())
    );
    assert_eq!(report.retryable_errors[0].code, "publication_append_failed");
}
