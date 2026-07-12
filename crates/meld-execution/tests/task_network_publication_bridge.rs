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
use meld_execution::task_network::outcome::PublicationState;
use meld_execution::task_network::publication::{
    build_publication_envelope, publish_pending_publications, publish_publication, EventAppendSink,
    PublicationPublishResult, PublishPendingPublicationsRequest,
};
use meld_execution::task_network::{PublicationRuntime, SledTaskNetworkStore};
use std::cell::RefCell;
use std::collections::BTreeSet;

struct FailingSink;

impl EventAppendSink for FailingSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        test_ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        _envelope: EventEnvelope,
    ) -> Result<AppendReceipt, String> {
        Err("append unavailable".to_string())
    }
}

struct DishonestSink {
    advertised: LedgerIdentity,
    returned: LedgerIdentity,
}

impl EventAppendSink for DishonestSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.advertised
    }

    fn append_envelope_idempotent(
        &self,
        _envelope: EventEnvelope,
    ) -> Result<AppendReceipt, String> {
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

    fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<AppendReceipt, String> {
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

    fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<AppendReceipt, String> {
        EventAppendSink::append_envelope_idempotent(&self.append, envelope)
    }
}

fn open_store(db: &sled::Db) -> SledTaskNetworkStore {
    SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap()
}

fn open_task_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
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

fn record_success_publication(
    store: &mut SledTaskNetworkStore,
    task_instance_id: &str,
    outcome_id: &str,
    claim_id: &str,
) -> String {
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
    store: &mut SledTaskNetworkStore,
    task_instance_id: &str,
    outcome_id: &str,
    claim_id: &str,
) -> String {
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
    store: SledTaskNetworkStore,
    db: &sled::Db,
    publication_id: &str,
    legacy_event_seq: u64,
) -> SledTaskNetworkStore {
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
    drop(store);

    db.open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(
            revision.to_be_bytes(),
            serde_json::to_vec(&frozen_journal).unwrap(),
        )
        .unwrap();
    db.open_tree("task_network_latest_state")
        .unwrap()
        .insert(
            "latest",
            serde_json::to_vec(&serde_json::json!({ "state": legacy_state })).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();
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
        meld_execution::task_network::publication::PublicationBridgeError::LedgerIdentityMismatch {
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
fn publication_runtime_actor_publishes_through_event_append_sink() {
    let task_db = open_task_db();
    let mut store = open_store(&task_db);
    let publication_id =
        record_success_publication(&mut store, "task-alpha", "outcome-alpha", "claim-alpha");
    let events = RecordingSink::default();
    let runtime = PublicationRuntime::new();

    let report = runtime
        .publish_pending(&mut store, &events, request(None))
        .unwrap();

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
