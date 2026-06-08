#[path = "support/task_network.rs"]
mod task_network_support;

use futures::executor::block_on;
use meld_events::EventEnvelope;
use meld_execution::capability::{
    BoundCapabilityInstance, CapabilityInvocationPayload, CapabilityInvocationResult,
};
use meld_execution::error::ExecutionInvariantError;
use meld_execution::execution::{EventPublicationPort, ExecutionEventContext};
use meld_execution::task::{
    execute_task_to_completion, ArtifactProducerRef, ArtifactRecord, CompiledTaskDelta,
    CompiledTaskRecord, TaskExpansionRequest,
};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::{
    build_executor_for_claim, failed_outcome_from_executor, succeeded_outcome_from_executor,
    Request as DispatchRequest,
};
use meld_execution::task_network::initialization::materialize_task_initialization;
use meld_execution::task_network::outcome::PublicationState;
use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::state::TaskStatus;
use meld_execution::task_network::store::SledTaskNetworkStore;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;

#[derive(Default)]
struct NoopApi;

impl EventPublicationPort for NoopApi {
    type Error = ExecutionInvariantError;
    type EventEnvelope = EventEnvelope;

    fn publish_execution_envelope(
        &self,
        _event_context: &ExecutionEventContext,
        _envelope: Self::EventEnvelope,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn invoke_success<'a>(
    _api: &'a NoopApi,
    _state: &'a (),
    instance: &'a BoundCapabilityInstance,
    payload: &'a CapabilityInvocationPayload,
    _event_context: Option<&'a ExecutionEventContext>,
) -> Pin<
    Box<
        dyn Future<Output = Result<CapabilityInvocationResult, ExecutionInvariantError>>
            + Send
            + 'a,
    >,
> {
    let artifact_type_id = match instance.capability_type_id.as_str() {
        "docs.prepare_metadata" => "metadata_doc",
        "docs.collect_context" => "context_bundle",
        "docs.write_summary" => "summary_doc",
        other => other,
    }
    .to_string();
    let invocation_id = payload.invocation_id.clone();
    let capability_instance_id = payload.capability_instance_id.clone();
    let supplied_input_count = payload.supplied_inputs.len();
    Box::pin(async move {
        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![ArtifactRecord {
                artifact_id: format!("{invocation_id}::{artifact_type_id}"),
                artifact_type_id: artifact_type_id.clone(),
                schema_version: 1,
                content: serde_json::json!({
                    "artifact_type_id": artifact_type_id,
                    "supplied_input_count": supplied_input_count,
                }),
                producer: ArtifactProducerRef {
                    task_id: capability_instance_id.clone(),
                    capability_instance_id,
                    invocation_id: Some(invocation_id),
                    output_slot_id: Some(artifact_type_id),
                },
            }],
        })
    })
}

fn invoke_failure<'a>(
    _api: &'a NoopApi,
    _state: &'a (),
    _instance: &'a BoundCapabilityInstance,
    _payload: &'a CapabilityInvocationPayload,
    _event_context: Option<&'a ExecutionEventContext>,
) -> Pin<
    Box<
        dyn Future<Output = Result<CapabilityInvocationResult, ExecutionInvariantError>>
            + Send
            + 'a,
    >,
> {
    Box::pin(async {
        Err(ExecutionInvariantError::GenerationFailed(
            "runtime failed".to_string(),
        ))
    })
}

fn compile_no_expansion(
    _api: &NoopApi,
    _compiled_task: &CompiledTaskRecord,
    _request: &TaskExpansionRequest,
    _catalog: &meld_execution::capability::CapabilityCatalog,
) -> Result<CompiledTaskDelta, ExecutionInvariantError> {
    Ok(CompiledTaskDelta::default())
}

fn step_ids(state: &meld_execution::task_network::NetworkState) -> BTreeMap<String, String> {
    state
        .tasks
        .iter()
        .map(|(task_instance_id, node)| (node.lineage.step_id.clone(), task_instance_id.clone()))
        .collect()
}

fn ready_steps(state: &meld_execution::task_network::NetworkState) -> BTreeSet<String> {
    compute_ready_set(state)
        .task_instance_ids
        .iter()
        .map(|task_instance_id| state.tasks[task_instance_id].lineage.step_id.clone())
        .collect()
}

fn claim_execute_and_record(
    store: &mut SledTaskNetworkStore,
    task_instance_id: &str,
    command_suffix: &str,
) {
    let claim_request = DispatchRequest {
        claim_id: format!("claim-{command_suffix}"),
        task_instance_id: task_instance_id.to_string(),
        worker_id: "worker-a".to_string(),
        idempotency_key: format!("claim-{command_suffix}-once"),
    };
    let claim_command = task_network_support::apply_sled_command(
        store,
        &format!("command-claim-{command_suffix}"),
        Command::ClaimReadyTask(claim_request),
    );
    assert!(matches!(
        store.submit(claim_command).unwrap(),
        Response::Accepted { .. }
    ));
    let claim = store
        .state()
        .claims
        .get(&format!("claim-{command_suffix}"))
        .unwrap()
        .clone();
    let node = store.state().tasks[task_instance_id].clone();
    let materialized = materialize_task_initialization(store.state(), task_instance_id).unwrap();
    let mut executor = build_executor_for_claim(
        &node,
        &claim,
        materialized.payload,
        format!("repo-{command_suffix}"),
    )
    .unwrap();
    block_on(execute_task_to_completion(
        &NoopApi,
        &mut executor,
        &task_network_support::phase8_catalog(),
        &(),
        invoke_success,
        compile_no_expansion,
        None,
        None,
    ))
    .unwrap();
    let outcome =
        succeeded_outcome_from_executor(format!("outcome-{command_suffix}"), &claim, &executor);
    let outcome_command = task_network_support::apply_sled_command(
        store,
        &format!("command-outcome-{command_suffix}"),
        Command::RecordTaskOutcome(outcome),
    );
    assert!(matches!(
        store.submit(outcome_command).unwrap(),
        Response::Accepted { .. }
    ));
}

#[test]
fn phase8_task_network_slice_runs_and_survives_reopen() {
    let plan = task_network_support::lower_phase8();
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let mut store = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    let commit = task_network_support::apply_sled_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(plan.mutations),
    );
    assert!(matches!(
        store.submit(commit).unwrap(),
        Response::Accepted { .. }
    ));
    let ids = step_ids(store.state());

    assert_eq!(
        ready_steps(store.state()),
        BTreeSet::from([
            "collect_context".to_string(),
            "prepare_metadata".to_string()
        ])
    );
    assert!(materialize_task_initialization(store.state(), &ids["write_summary"]).is_err());

    claim_execute_and_record(&mut store, &ids["prepare_metadata"], "prepare");
    assert_eq!(
        ready_steps(store.state()),
        BTreeSet::from(["collect_context".to_string()])
    );
    claim_execute_and_record(&mut store, &ids["collect_context"], "context");
    let write_payload =
        materialize_task_initialization(store.state(), &ids["write_summary"]).unwrap();
    let init_artifact_types = write_payload
        .payload
        .init_artifacts
        .iter()
        .map(|artifact| artifact.artifact_type_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        init_artifact_types,
        BTreeSet::from(["context_bundle", "metadata_doc"])
    );

    claim_execute_and_record(&mut store, &ids["write_summary"], "write");
    assert!(matches!(
        store.state().statuses.get(&ids["write_summary"]),
        Some(TaskStatus::Succeeded { outcome_id }) if outcome_id == "outcome-write"
    ));
    let mut publication = store
        .state()
        .publications
        .values()
        .find(|publication| publication.outcome.task_instance_id == ids["write_summary"])
        .unwrap()
        .clone();
    publication.state = PublicationState::Published { marked_revision: 0 };
    let publication_id = publication.publication_id.clone();
    let mark_command = task_network_support::apply_sled_command(
        &store,
        "command-mark-publication",
        Command::MarkPublication(publication),
    );
    assert!(matches!(
        store.submit(mark_command).unwrap(),
        Response::Accepted { .. }
    ));
    let expected_state = store.state().clone();
    let expected_journal_len = store.journal().len();
    drop(store);

    let db = sled::open(tempdir.path()).unwrap();
    let reopened = SledTaskNetworkStore::open(db, "network-docs").unwrap();

    assert_eq!(reopened.state().revision, expected_state.revision);
    assert_eq!(reopened.state().state_hash, expected_state.state_hash);
    assert_eq!(reopened.state().statuses, expected_state.statuses);
    assert_eq!(
        reopened.state().artifact_availability,
        expected_state.artifact_availability
    );
    assert!(matches!(
        reopened.state().publications.get(&publication_id).unwrap().state,
        PublicationState::Published { marked_revision }
            if marked_revision == expected_state.revision
    ));
    assert_eq!(reopened.journal().len(), expected_journal_len);
}

#[test]
fn task_runtime_failure_converts_to_failed_network_outcome() {
    let plan = task_network_support::lower_phase8();
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let mut store = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    let commit = task_network_support::apply_sled_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(plan.mutations),
    );
    store.submit(commit).unwrap();
    let ids = step_ids(store.state());
    let task_instance_id = &ids["prepare_metadata"];
    let claim_request = DispatchRequest {
        claim_id: "claim-failure".to_string(),
        task_instance_id: task_instance_id.clone(),
        worker_id: "worker-a".to_string(),
        idempotency_key: "claim-failure-once".to_string(),
    };
    let claim_command = task_network_support::apply_sled_command(
        &store,
        "command-claim-failure",
        Command::ClaimReadyTask(claim_request),
    );
    store.submit(claim_command).unwrap();
    let claim = store.state().claims["claim-failure"].clone();
    let node = store.state().tasks[task_instance_id].clone();
    let materialized = materialize_task_initialization(store.state(), task_instance_id).unwrap();
    let mut executor =
        build_executor_for_claim(&node, &claim, materialized.payload, "repo-failure").unwrap();
    let error = block_on(execute_task_to_completion(
        &NoopApi,
        &mut executor,
        &task_network_support::phase8_catalog(),
        &(),
        invoke_failure,
        compile_no_expansion,
        None,
        None,
    ))
    .unwrap_err();
    let outcome =
        failed_outcome_from_executor("outcome-failure", &claim, &executor, error.to_string());
    let outcome_command = task_network_support::apply_sled_command(
        &store,
        "command-outcome-failure",
        Command::RecordTaskOutcome(outcome),
    );

    assert!(matches!(
        store.submit(outcome_command).unwrap(),
        Response::Accepted { .. }
    ));
    assert!(matches!(
        store.state().statuses.get(task_instance_id),
        Some(TaskStatus::Failed { outcome_id, error })
            if outcome_id == "outcome-failure" && error.contains("runtime failed")
    ));
    assert!(store.state().outcomes["outcome-failure"]
        .task_events
        .iter()
        .any(|event| event.event_type == "task_failed"));
}
