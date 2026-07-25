//! Aggregate package publication over durable package-run state.
//!
//! Proves the selected-tree eligibility rule end to end: no aggregate event
//! reaches the ledger before every required per-folder work unit completed
//! durably, completed runs append exactly one aggregate record idempotently,
//! failed terminal runs publish the distinct failed event type, and the
//! aggregate payload preserves durable outcome and artifact identities intact
//! beside the per-folder publication facts.

#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::{
    AppendReceipt, DomainObjectRef, EventAppendCapability, EventAuthority,
    EventAuthorityOpenOptions, EventEnvelope, EventReplayCapability, LedgerCursor, LedgerIdentity,
    ReplayRequest,
};
use meld_execution::capability::BoundCapabilityInstance;
use meld_execution::task::{
    ArtifactProducerRef, ArtifactRecord, CompiledTaskRecord, TaskExecutorSnapshot,
    TaskInitializationPayload, TaskProgressStore, TaskRunContext,
};
use meld_execution::task_network::aggregate::{
    AggregatePackageOutcome, AggregatePackageStatus, AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE,
    AGGREGATE_PACKAGE_FAILED_EVENT_TYPE,
};
use meld_execution::task_network::aggregate_publication::{
    aggregate_event_record_id, check_aggregate_completion, publish_aggregate,
    publish_aggregate_for_run, AggregatePublicationError, AggregatePublicationState,
    AggregatePublicationStore, AggregatePublishResult, AggregateRunBinding, AggregateSkipReason,
    PublishAggregateRequest,
};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::{Outcome, OutcomeStatus};
use meld_execution::task_network::publication::{
    publish_pending_publications, EventAppendSink, PublishPendingPublicationsRequest,
};
use meld_execution::task_network::SledTaskNetworkStore;
use serde_json::json;

const PACKAGE_RUN_ID: &str = "run-package-1";
const NETWORK_ID: &str = "network-docs";
const SESSION_ID: &str = "session-aggregate";

struct FailingSink;

impl EventAppendSink for FailingSink {
    fn ledger_identity(&self) -> LedgerIdentity {
        "00000000-0000-4000-8000-000000000001".parse().unwrap()
    }

    fn append_envelope_idempotent(
        &self,
        _envelope: EventEnvelope,
    ) -> Result<AppendReceipt, String> {
        Err("append unavailable".to_string())
    }
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

fn open_events(tempdir: &tempfile::TempDir) -> TestEvents {
    let db = sled::open(tempdir.path()).unwrap();
    let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
    TestEvents {
        append: authority.append_capability(),
        replay: authority.replay_capability(),
    }
}

fn read_all(events: &TestEvents) -> Vec<meld_events::EventRecord> {
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
}

fn aggregate_records(events: &TestEvents) -> Vec<meld_events::EventRecord> {
    read_all(events)
        .into_iter()
        .filter(|record| {
            record.envelope.event_type == AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE
                || record.envelope.event_type == AGGREGATE_PACKAGE_FAILED_EVENT_TYPE
        })
        .collect()
}

fn typed_instance(
    capability_instance_id: &str,
    capability_type_id: &str,
    scope_ref: &str,
) -> BoundCapabilityInstance {
    BoundCapabilityInstance {
        capability_instance_id: capability_instance_id.to_string(),
        capability_type_id: capability_type_id.to_string(),
        capability_version: 1,
        scope_ref: scope_ref.to_string(),
        scope_kind: "filesystem".to_string(),
        binding_values: vec![],
        input_wiring: vec![],
    }
}

fn instance(capability_instance_id: &str, scope_ref: &str) -> BoundCapabilityInstance {
    typed_instance(capability_instance_id, "docs.write", scope_ref)
}

/// Branching fan-out over a root folder and two child folders.
fn branching_snapshot(completed_instance_ids: &[&str]) -> TaskExecutorSnapshot {
    TaskExecutorSnapshot {
        compiled_task: CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![
                instance("write::root", "root"),
                instance("write::a", "root/a"),
                instance("write::b", "root/b"),
            ],
            dependency_edges: vec![],
        },
        init_payload: TaskInitializationPayload {
            task_id: "task_docs_writer".to_string(),
            compiled_task_ref: "compiled_task_docs_writer".to_string(),
            init_artifacts: vec![],
            task_run_context: TaskRunContext {
                task_run_id: PACKAGE_RUN_ID.to_string(),
                session_id: None,
                trigger: "test".to_string(),
            },
        },
        invocation_records: vec![],
        expansion_records: vec![],
        completed_instance_ids: completed_instance_ids
            .iter()
            .map(ToString::to_string)
            .collect(),
        started: true,
    }
}

fn artifact(artifact_id: &str, capability_instance_id: &str) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: artifact_id.to_string(),
        artifact_type_id: "docs_patch".to_string(),
        schema_version: 1,
        content: json!({ "artifact_id": artifact_id }),
        producer: ArtifactProducerRef {
            task_id: "task_docs_writer".to_string(),
            capability_instance_id: capability_instance_id.to_string(),
            invocation_id: Some(format!("invoke-{artifact_id}")),
            output_slot_id: Some("patch".to_string()),
        },
    }
}

fn terminal_outcome(status: OutcomeStatus, artifact_records: Vec<ArtifactRecord>) -> Outcome {
    Outcome {
        outcome_id: "outcome-package-1".to_string(),
        task_instance_id: "task-package".to_string(),
        lifecycle_epoch: 1,
        claim_id: "claim-package".to_string(),
        claim_revision: 1,
        status: status.clone(),
        error: match status {
            OutcomeStatus::Succeeded => None,
            OutcomeStatus::Failed => Some("provider failed".to_string()),
        },
        artifact_records,
        task_events: vec![],
    }
}

fn branching_artifacts() -> Vec<ArtifactRecord> {
    vec![
        artifact("artifact-root", "write::root"),
        artifact("artifact-a", "write::a"),
        artifact("artifact-b1", "write::b"),
        artifact("artifact-b2", "write::b"),
    ]
}

fn binding() -> AggregateRunBinding {
    AggregateRunBinding {
        package_run_id: PACKAGE_RUN_ID.to_string(),
        network_id: NETWORK_ID.to_string(),
        selected_scope: DomainObjectRef::new("workspace_fs", "node", "root").unwrap(),
        folder_unit_capability_types: vec!["docs.write".to_string()],
    }
}

fn request() -> PublishAggregateRequest {
    PublishAggregateRequest {
        session_id: SESSION_ID.to_string(),
        worker_id: "worker-aggregate".to_string(),
    }
}

fn open_progress(snapshot: Option<&TaskExecutorSnapshot>) -> TaskProgressStore {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let progress = TaskProgressStore::open(db).unwrap();
    if let Some(snapshot) = snapshot {
        progress
            .save(
                &snapshot.init_payload.task_run_context.task_run_id,
                snapshot,
            )
            .unwrap();
    }
    progress
}

fn open_outbox() -> AggregatePublicationStore {
    let db = sled::Config::new().temporary(true).open().unwrap();
    AggregatePublicationStore::open(db).unwrap()
}

fn expected_aggregate_id() -> String {
    AggregatePackageOutcome::derive_aggregate_id(PACKAGE_RUN_ID, NETWORK_ID)
}

#[test]
fn incomplete_branching_run_produces_no_aggregate_and_appends_nothing() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::a"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();

    let result =
        publish_aggregate_for_run(&progress, None, &binding(), &outbox, &events, &request())
            .unwrap();

    let AggregatePublishResult::Skipped(AggregateSkipReason::RunNotTerminal {
        pending_instance_ids,
    }) = result
    else {
        panic!("incomplete run was not skipped: {result:?}");
    };
    assert_eq!(pending_instance_ids, vec!["write::root", "write::b"]);
    assert!(outbox.load(&expected_aggregate_id()).unwrap().is_none());
    assert!(read_all(&events).is_empty());
}

#[test]
fn run_without_durable_progress_is_skipped_without_writes() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let progress = open_progress(None);
    let outbox = open_outbox();

    let result = publish_aggregate_for_run(
        &progress,
        Some(&terminal_outcome(OutcomeStatus::Succeeded, vec![])),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();

    assert_eq!(
        result,
        AggregatePublishResult::Skipped(AggregateSkipReason::RunProgressNotFound)
    );
    assert!(outbox.load(&expected_aggregate_id()).unwrap().is_none());
    assert!(read_all(&events).is_empty());
}

#[test]
fn succeeded_terminal_outcome_over_incomplete_progress_is_rejected() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::a", "write::b"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(OutcomeStatus::Succeeded, branching_artifacts());

    let error = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap_err();

    let AggregatePublicationError::PrematureCompletion {
        package_run_id,
        pending_instance_ids,
    } = error
    else {
        panic!("premature completion was not rejected: {error}");
    };
    assert_eq!(package_run_id, PACKAGE_RUN_ID);
    assert_eq!(pending_instance_ids, vec!["write::root"]);
    assert!(outbox.load(&expected_aggregate_id()).unwrap().is_none());
    assert!(read_all(&events).is_empty());
}

#[test]
fn empty_fan_out_never_reads_as_complete() {
    let mut snapshot = branching_snapshot(&[]);
    snapshot.compiled_task.capability_instances.clear();

    let check = check_aggregate_completion(&snapshot);

    assert_eq!(check.known_units, 0);
    assert!(!check.is_complete());
}

#[test]
fn completed_run_publishes_one_aggregate_with_the_full_intact_folder_set() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::root", "write::a", "write::b"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(OutcomeStatus::Succeeded, branching_artifacts());

    let result = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();

    let AggregatePublishResult::Published {
        aggregate_id,
        event_record_id,
        receipt,
    } = &result
    else {
        panic!("completed run did not publish: {result:?}");
    };
    assert_eq!(aggregate_id, &expected_aggregate_id());
    assert_eq!(event_record_id, &aggregate_event_record_id(aggregate_id));

    let records = read_all(&events);
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.seq, receipt.seq);
    assert_eq!(
        record.envelope.event_type,
        AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE
    );
    assert_eq!(record.envelope.session, SESSION_ID);
    assert_eq!(
        record.envelope.record_id.as_deref(),
        Some(event_record_id.as_str())
    );
    assert_eq!(
        record.envelope.stream_id,
        format!("task_network::{NETWORK_ID}::package::{PACKAGE_RUN_ID}")
    );

    let payload: AggregatePackageOutcome =
        serde_json::from_value(record.envelope.data.clone()).unwrap();
    assert_eq!(payload.aggregate_id, expected_aggregate_id());
    assert_eq!(payload.package_run_id, PACKAGE_RUN_ID);
    assert_eq!(payload.network_id, NETWORK_ID);
    assert_eq!(payload.selected_scope, binding().selected_scope);
    assert_eq!(payload.status, AggregatePackageStatus::Completed);
    let folders: Vec<_> = payload
        .folder_results
        .iter()
        .map(|result| result.folder.clone())
        .collect();
    assert_eq!(
        folders,
        vec![
            DomainObjectRef::new("workspace_fs", "node", "root").unwrap(),
            DomainObjectRef::new("workspace_fs", "node", "root/a").unwrap(),
            DomainObjectRef::new("workspace_fs", "node", "root/b").unwrap(),
        ]
    );
    let artifact_ids: Vec<Vec<String>> = payload
        .folder_results
        .iter()
        .map(|result| result.artifact_ids.clone())
        .collect();
    assert_eq!(
        artifact_ids,
        vec![
            vec!["artifact-root".to_string()],
            vec!["artifact-a".to_string()],
            vec!["artifact-b1".to_string(), "artifact-b2".to_string()],
        ]
    );
    for folder_result in &payload.folder_results {
        assert_eq!(folder_result.task_instance_id, outcome.task_instance_id);
        assert_eq!(folder_result.outcome_id, outcome.outcome_id);
    }

    let stored = outbox.load(&expected_aggregate_id()).unwrap().unwrap();
    assert_eq!(
        stored.state,
        AggregatePublicationState::Published { receipt: *receipt }
    );
    assert_eq!(stored.aggregate, payload);
    assert_eq!(stored.worker_id, "worker-aggregate");
}

#[test]
fn duplicate_aggregate_publication_is_idempotent_at_outbox_and_ledger() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::root", "write::a", "write::b"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(OutcomeStatus::Succeeded, branching_artifacts());

    let first = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();
    let AggregatePublishResult::Published { receipt, .. } = first else {
        panic!("first publish did not append: {first:?}");
    };

    // Same durable outbox: the persisted receipt short-circuits the append.
    let second = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();
    assert_eq!(
        second,
        AggregatePublishResult::AlreadyPublished {
            aggregate_id: expected_aggregate_id(),
            receipt,
        }
    );
    assert_eq!(read_all(&events).len(), 1);

    // Fresh outbox without the receipt: the deterministic record id keeps the
    // ledger free of duplicates and returns the original sequence.
    let fresh_outbox = open_outbox();
    let third = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &fresh_outbox,
        &events,
        &request(),
    )
    .unwrap();
    let AggregatePublishResult::Published {
        receipt: replay_receipt,
        ..
    } = third
    else {
        panic!("replayed publish did not complete: {third:?}");
    };
    assert_eq!(replay_receipt.seq, receipt.seq);
    assert_eq!(read_all(&events).len(), 1);
}

#[test]
fn failed_terminal_run_publishes_the_failed_event_type() {
    assert_ne!(
        AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE,
        AGGREGATE_PACKAGE_FAILED_EVENT_TYPE
    );

    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::a"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(
        OutcomeStatus::Failed,
        vec![artifact("artifact-a", "write::a")],
    );

    let result = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();

    assert!(matches!(result, AggregatePublishResult::Published { .. }));
    let records = read_all(&events);
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].envelope.event_type,
        AGGREGATE_PACKAGE_FAILED_EVENT_TYPE
    );
    let payload: AggregatePackageOutcome =
        serde_json::from_value(records[0].envelope.data.clone()).unwrap();
    assert_eq!(payload.status, AggregatePackageStatus::Failed);
    // Only the folder whose work fully completed contributes a result; the
    // pending folders never read as finished work inside the failed record.
    assert_eq!(payload.folder_results.len(), 1);
    assert_eq!(
        payload.folder_results[0].folder,
        DomainObjectRef::new("workspace_fs", "node", "root/a").unwrap()
    );
    assert_eq!(
        payload.folder_results[0].artifact_ids,
        vec!["artifact-a".to_string()]
    );
}

#[test]
fn append_failure_marks_the_record_failed_and_a_retry_publishes() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::root", "write::a", "write::b"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(OutcomeStatus::Succeeded, branching_artifacts());

    let failed = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &FailingSink,
        &request(),
    )
    .unwrap();

    assert_eq!(
        failed,
        AggregatePublishResult::AppendFailed {
            aggregate_id: expected_aggregate_id(),
            error: "append unavailable".to_string(),
        }
    );
    let stored = outbox.load(&expected_aggregate_id()).unwrap().unwrap();
    assert_eq!(
        stored.state,
        AggregatePublicationState::Failed {
            error: "append unavailable".to_string(),
        }
    );

    let retried = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();
    assert!(matches!(retried, AggregatePublishResult::Published { .. }));
    assert_eq!(read_all(&events).len(), 1);
}

#[test]
fn aggregate_identity_drift_is_rejected() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::root", "write::a", "write::b"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(OutcomeStatus::Succeeded, branching_artifacts());

    publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();

    let mut drifted = match meld_execution::task_network::produce_aggregate_outcome(
        &snapshot,
        Some(&outcome),
        &binding(),
    )
    .unwrap()
    {
        meld_execution::task_network::AggregateProduction::Produced(aggregate) => aggregate,
        other => panic!("no aggregate produced: {other:?}"),
    };
    drifted.folder_results.pop();

    let error = publish_aggregate(&outbox, &events, &request(), &drifted).unwrap_err();
    assert!(matches!(
        error,
        AggregatePublicationError::IdentityDrift { .. }
    ));
    assert_eq!(read_all(&events).len(), 1);
}

/// Full route: per-folder facts flow through the existing per-task
/// publication bridge, the aggregate stays off the ledger while durable
/// progress is incomplete, and after completion both fact layers coexist
/// with the aggregate preserving the durable identities byte for byte.
#[test]
fn per_folder_publications_coexist_and_the_aggregate_arrives_only_after_completion() {
    let task_db = sled::Config::new().temporary(true).open().unwrap();
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let mut store = SledTaskNetworkStore::open(task_db, NETWORK_ID).unwrap();

    // Record and publish the per-folder task fact through the existing route.
    task_network_support::commit_single_task_sled(&mut store, "task-alpha");
    let task_instance_id =
        task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let durable_outcome =
        task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
    let record = task_network_support::apply_sled_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(durable_outcome),
    );
    assert!(matches!(
        store.submit(record).unwrap(),
        Response::Accepted { .. }
    ));
    let durable_outcome = store.state().outcomes.get("outcome-alpha").unwrap().clone();
    let bridge_report = publish_pending_publications(
        &mut store,
        &events,
        PublishPendingPublicationsRequest {
            session_id: SESSION_ID.to_string(),
            worker_id: "worker-publication".to_string(),
            limit: None,
        },
    )
    .unwrap();
    assert_eq!(bridge_report.items_committed, 1);

    // One work unit targeting one folder; the outcome's artifact producer is
    // the fixture's "write" capability instance.
    let mut snapshot = branching_snapshot(&[]);
    snapshot.compiled_task.capability_instances = vec![instance("write", "readme")];
    let run_binding = AggregateRunBinding {
        package_run_id: PACKAGE_RUN_ID.to_string(),
        network_id: NETWORK_ID.to_string(),
        selected_scope: DomainObjectRef::new("workspace_fs", "node", "readme").unwrap(),
        folder_unit_capability_types: vec!["docs.write".to_string()],
    };
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();

    // Incomplete durable progress: the per-folder fact is on the ledger, the
    // aggregate is not, so ledger absence is selected-tree ineligibility.
    let skipped =
        publish_aggregate_for_run(&progress, None, &run_binding, &outbox, &events, &request())
            .unwrap();
    assert!(matches!(skipped, AggregatePublishResult::Skipped(_)));
    assert_eq!(read_all(&events).len(), 1);
    assert!(aggregate_records(&events).is_empty());

    // Complete the folder work durably, then publish the aggregate from the
    // durable task-network outcome.
    snapshot.completed_instance_ids = vec!["write".to_string()];
    progress.save(PACKAGE_RUN_ID, &snapshot).unwrap();
    let published = publish_aggregate_for_run(
        &progress,
        Some(&durable_outcome),
        &run_binding,
        &outbox,
        &events,
        &request(),
    )
    .unwrap();
    assert!(matches!(
        published,
        AggregatePublishResult::Published { .. }
    ));

    let records = read_all(&events);
    assert_eq!(records.len(), 2);
    let event_types: Vec<_> = records
        .iter()
        .map(|record| record.envelope.event_type.as_str())
        .collect();
    assert!(event_types.contains(&"execution.task.succeeded"));
    assert!(event_types.contains(&AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE));

    // The aggregate payload preserves the durable outcome and artifact
    // identities byte for byte.
    let aggregate_record = &aggregate_records(&events)[0];
    let payload: AggregatePackageOutcome =
        serde_json::from_value(aggregate_record.envelope.data.clone()).unwrap();
    assert_eq!(payload.folder_results.len(), 1);
    let folder_result = &payload.folder_results[0];
    assert_eq!(
        folder_result.outcome_id.as_bytes(),
        durable_outcome.outcome_id.as_bytes()
    );
    assert_eq!(
        folder_result.task_instance_id.as_bytes(),
        durable_outcome.task_instance_id.as_bytes()
    );
    let durable_artifact_ids: Vec<&[u8]> = durable_outcome
        .artifact_records
        .iter()
        .map(|artifact| artifact.artifact_id.as_bytes())
        .collect();
    let payload_artifact_ids: Vec<&[u8]> = folder_result
        .artifact_ids
        .iter()
        .map(|artifact_id| artifact_id.as_bytes())
        .collect();
    assert_eq!(payload_artifact_ids, durable_artifact_ids);
}

/// Regression for the phantom-folder finding: the real package's initial
/// compiled task carries a traversal instance bound to the literal
/// placeholder scope reference "target" (a folder subject only
/// expansion-added node instances carry). The binding's data-driven folder
/// classification must keep that unit out of the folder rows and off the
/// envelope graph while it still counts toward completion.
#[test]
fn traversal_placeholder_scope_never_becomes_a_folder_row() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);

    // Mirror the real package shape: the seeded traversal instance with the
    // placeholder scope plus expansion-added per-node writer instances.
    let mut snapshot = branching_snapshot(&[]);
    snapshot.compiled_task.capability_instances = vec![
        typed_instance("traversal", "merkle_traversal", "target"),
        instance("write::a", "root/a"),
        instance("write::b", "root/b"),
    ];
    snapshot.completed_instance_ids = vec![
        "traversal".to_string(),
        "write::a".to_string(),
        "write::b".to_string(),
    ];
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();

    // The traversal batch artifact rides the same terminal outcome as the
    // per-node artifacts.
    let outcome = terminal_outcome(
        OutcomeStatus::Succeeded,
        vec![
            artifact("artifact-traversal-batch", "traversal"),
            artifact("artifact-a", "write::a"),
            artifact("artifact-b", "write::b"),
        ],
    );

    let result = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();
    assert!(matches!(result, AggregatePublishResult::Published { .. }));

    let records = aggregate_records(&events);
    assert_eq!(records.len(), 1);
    let payload: AggregatePackageOutcome =
        serde_json::from_value(records[0].envelope.data.clone()).unwrap();

    let phantom = DomainObjectRef::new("workspace_fs", "node", "target").unwrap();
    let folders: Vec<_> = payload
        .folder_results
        .iter()
        .map(|result| result.folder.clone())
        .collect();
    assert_eq!(
        folders,
        vec![
            DomainObjectRef::new("workspace_fs", "node", "root/a").unwrap(),
            DomainObjectRef::new("workspace_fs", "node", "root/b").unwrap(),
        ]
    );
    assert!(!folders.contains(&phantom));
    for folder_result in &payload.folder_results {
        assert!(!folder_result
            .artifact_ids
            .contains(&"artifact-traversal-batch".to_string()));
    }
    assert!(!records[0].envelope.objects.contains(&phantom));
}

#[test]
fn empty_folder_classification_is_rejected() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::root", "write::a", "write::b"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(OutcomeStatus::Succeeded, branching_artifacts());
    let mut unclassified = binding();
    unclassified.folder_unit_capability_types.clear();

    let error = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &unclassified,
        &outbox,
        &events,
        &request(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        AggregatePublicationError::InvalidRequest(_)
    ));
    assert!(read_all(&events).is_empty());
}

#[test]
fn completed_run_matching_no_folder_units_is_rejected() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let snapshot = branching_snapshot(&["write::root", "write::a", "write::b"]);
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(OutcomeStatus::Succeeded, branching_artifacts());
    let mut mismatched = binding();
    mismatched.folder_unit_capability_types = vec!["docs.unknown".to_string()];

    let error = publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &mismatched,
        &outbox,
        &events,
        &request(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        AggregatePublicationError::NoFolderWorkUnits { .. }
    ));
    assert!(outbox.load(&expected_aggregate_id()).unwrap().is_none());
    assert!(read_all(&events).is_empty());
}

/// Cross-module parity: the aggregate's folder subject minting must match
/// the workspace node coordinates the task event mapping publishes, so the
/// world model sees one subject vocabulary across per-task facts and the
/// aggregate.
#[test]
fn folder_subject_minting_matches_the_task_event_workspace_node_mapping() {
    let event_tempdir = tempfile::tempdir().unwrap();
    let events = open_events(&event_tempdir);
    let mut snapshot = branching_snapshot(&[]);
    snapshot.compiled_task.capability_instances = vec![instance("write", "root/a")];
    snapshot.completed_instance_ids = vec!["write".to_string()];
    let progress = open_progress(Some(&snapshot));
    let outbox = open_outbox();
    let outcome = terminal_outcome(
        OutcomeStatus::Succeeded,
        vec![artifact("artifact-a", "write")],
    );

    publish_aggregate_for_run(
        &progress,
        Some(&outcome),
        &binding(),
        &outbox,
        &events,
        &request(),
    )
    .unwrap();
    let payload: AggregatePackageOutcome =
        serde_json::from_value(aggregate_records(&events)[0].envelope.data.clone()).unwrap();
    let aggregate_folder = payload.folder_results[0].folder.clone();

    let mut task_event =
        meld_execution::task::TaskEvent::new("task_started", "task_docs_writer", PACKAGE_RUN_ID);
    task_event.target_node_id = Some("root/a".to_string());
    let task_envelope =
        meld_execution::task::build_execution_task_envelope("session-parity", &task_event).unwrap();
    let task_event_folder = task_envelope
        .objects
        .iter()
        .find(|object| object.object_id == "root/a")
        .cloned()
        .unwrap();

    assert_eq!(aggregate_folder, task_event_folder);
}
