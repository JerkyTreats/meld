#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::events::store::EventStore;
use meld_events::EventEnvelope;
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::OutcomeStatus;
use meld_execution::task_network::outcome::PublicationState;
use meld_execution::task_network::publication::{
    publish_pending_publications, publish_publication, EventAppendSink, PublicationPublishResult,
    PublishPendingPublicationsRequest,
};
use meld_execution::task_network::store::SledTaskNetworkStore;
use std::collections::BTreeSet;

struct FailingSink;

impl EventAppendSink for FailingSink {
    fn append_envelope_idempotent(&self, _envelope: EventEnvelope) -> Result<u64, String> {
        Err("append unavailable".to_string())
    }
}

fn open_store(db: &sled::Db) -> SledTaskNetworkStore {
    SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap()
}

fn open_task_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn open_events(tempdir: &tempfile::TempDir) -> EventStore {
    let db = sled::open(tempdir.path()).unwrap();
    EventStore::new(db).unwrap()
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
        event_seq,
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
    let event_records = events.read_events("session-publication").unwrap();
    assert_eq!(event_records.len(), 1);
    let event = &event_records[0];
    assert_eq!(*event_seq, event.seq);
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
    assert!(matches!(
        stored.state,
        PublicationState::Published {
            marked_revision: stored_revision,
            event_seq: Some(stored_event_seq)
        } if stored_revision == *marked_revision && stored_event_seq == *event_seq
    ));
    assert_eq!(*marked_revision, store.state().revision);
    store.flush().unwrap();
    drop(store);

    let reopened = open_store(&task_db);
    assert!(matches!(
        reopened.state().publications.get(&publication_id).unwrap().state,
        PublicationState::Published {
            marked_revision: reopened_revision,
            event_seq: Some(reopened_event_seq)
        } if reopened_revision == *marked_revision && reopened_event_seq == *event_seq
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
    assert_eq!(events.read_events("session-publication").unwrap().len(), 1);

    let direct = publish_publication(&mut store, &events, &request(None), &publication_id).unwrap();
    assert_eq!(
        direct,
        PublicationPublishResult::AlreadyPublished { publication_id }
    );
    assert_eq!(events.read_events("session-publication").unwrap().len(), 1);
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
    assert_eq!(events.read_events("session-publication").unwrap().len(), 0);
    let publication = store.state().publications.get(&publication_id).unwrap();
    assert!(matches!(
        &publication.state,
        PublicationState::Failed { error } if error == "append unavailable"
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
    assert_eq!(events.read_events("session-publication").unwrap().len(), 1);
    assert!(matches!(
        store
            .state()
            .publications
            .get(&publication_id)
            .unwrap()
            .state,
        PublicationState::Published {
            event_seq: Some(_),
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
    let event_records = events.read_events("session-publication").unwrap();
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
    assert_eq!(events.read_events("session-publication").unwrap().len(), 1);
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
