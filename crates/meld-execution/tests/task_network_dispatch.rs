#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task::TaskInitializationPayload;
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::{
    build_executor_for_claim, Claim, Outcome, OutcomeStatus, Request as DispatchRequest,
};
use meld_execution::task_network::mutation::Rejection;
use meld_execution::task_network::mutation::{Mutation, Set};
use meld_execution::task_network::state::{
    ArtifactAvailability, DependencyEdge, DependencyEdgeOrigin, DependencyKind, TaskStatus,
};
use meld_execution::task_network::store::InMemoryTaskNetworkStore;

#[test]
fn ready_task_claim_moves_status_to_running() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");

    let task_instance_id =
        task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");

    assert!(matches!(
        store.state().statuses.get(&task_instance_id),
        Some(TaskStatus::Running { claim_id }) if claim_id == "claim-alpha"
    ));
}

#[test]
fn claim_for_missing_task_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &store,
        "command-claim",
        Command::ClaimReadyTask(DispatchRequest {
            claim_id: "claim-missing".to_string(),
            task_instance_id: "missing".to_string(),
            worker_id: "worker-a".to_string(),
            idempotency_key: "claim-missing-once".to_string(),
        }),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::FailedPrecondition(_))
    ));
}

#[test]
fn claim_for_non_ready_task_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(task_network_support::two_task_ordering_set(
            "task-upstream",
            "task-downstream",
        )),
    );
    store.submit(request);
    let claim = DispatchRequest {
        claim_id: "claim-downstream".to_string(),
        task_instance_id: "task-downstream".to_string(),
        worker_id: "worker-a".to_string(),
        idempotency_key: "claim-downstream-once".to_string(),
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-claim",
        Command::ClaimReadyTask(claim),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(_))
    ));
}

#[test]
fn claim_blocks_when_upstream_outcome_has_no_available_artifact() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let set = Set::new(
        "network-docs",
        "composition-fixture",
        "upstream-source-without-artifact",
        vec![
            Mutation::Inject(task_network_support::inject_for_node(
                task_network_support::single_task_node("task-upstream"),
                vec![],
            )),
            Mutation::Inject(task_network_support::inject_for_node(
                task_network_support::task_node_with_upstream_source(
                    "task-downstream",
                    "task-upstream",
                    "metadata",
                    "metadata_doc",
                    1,
                ),
                vec![DependencyEdge {
                    from: "task-upstream".to_string(),
                    to: "task-downstream".to_string(),
                    kind: DependencyKind::DataFlow {
                        artifact_type_id: "metadata_doc".to_string(),
                    },

                    origin: DependencyEdgeOrigin::Unrecorded,
                }],
            )),
        ],
    );
    let commit = task_network_support::apply_memory_command(
        &store,
        "command-commit-materialization",
        Command::ApplyMutationSet(set),
    );
    assert!(matches!(store.submit(commit), Response::Accepted { .. }));

    let upstream_task_instance_id = task_network_support::claim_ready_memory(
        &mut store,
        "command-claim-upstream",
        "claim-upstream",
    );
    let upstream_claim = store.state().claims.get("claim-upstream").unwrap().clone();
    let upstream_outcome = Outcome {
        outcome_id: "outcome-upstream".to_string(),
        task_instance_id: upstream_task_instance_id,
        lifecycle_epoch: upstream_claim.lifecycle_epoch,
        claim_id: upstream_claim.claim_id,
        claim_revision: upstream_claim.claim_revision,
        status: OutcomeStatus::Succeeded,
        error: None,
        artifact_records: vec![],
        task_events: vec![],
        admission: upstream_claim.admission,
    };
    let outcome = task_network_support::apply_memory_command(
        &store,
        "command-outcome-upstream",
        Command::RecordTaskOutcome(upstream_outcome),
    );
    assert!(matches!(store.submit(outcome), Response::Accepted { .. }));

    let claim = DispatchRequest {
        claim_id: "claim-downstream".to_string(),
        task_instance_id: "task-downstream".to_string(),
        worker_id: "worker-a".to_string(),
        idempotency_key: "claim-downstream-once".to_string(),
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-claim-downstream",
        Command::ClaimReadyTask(claim),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(message))
            if message.contains("not ready")
    ));
}

#[test]
fn second_claim_for_running_task_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let task_instance_id =
        task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = DispatchRequest {
        claim_id: "claim-second".to_string(),
        task_instance_id,
        worker_id: "worker-b".to_string(),
        idempotency_key: "claim-second-once".to_string(),
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-second-claim",
        Command::ClaimReadyTask(claim),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(_))
    ));
}

#[test]
fn build_executor_for_claim_requires_matching_task_identity() {
    let node = task_network_support::single_task_node("task-alpha");
    let request = DispatchRequest {
        claim_id: "claim-alpha".to_string(),
        task_instance_id: "task-alpha".to_string(),
        worker_id: "worker-a".to_string(),
        idempotency_key: "claim-alpha-once".to_string(),
    };
    let mut claim = Claim::accepted(
        "network-docs",
        &request,
        node.lifecycle_epoch,
        1,
        node.lineage.admission.clone(),
    );
    let payload = TaskInitializationPayload {
        task_id: node.compiled_task.task_id.clone(),
        compiled_task_ref: format!(
            "{}@{}",
            node.compiled_task.task_id, node.compiled_task.task_version
        ),
        init_artifacts: vec![],
        task_run_context: node.task_run_context.clone(),
    };

    assert!(build_executor_for_claim(&node, &claim, payload.clone(), "repo-alpha").is_ok());

    claim.task_instance_id = "task-beta".to_string();
    let error = build_executor_for_claim(&node, &claim, payload, "repo-alpha").unwrap_err();

    assert!(error.to_string().contains("does not match task instance"));
}

#[test]
fn outcome_with_stale_claim_revision_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let task_instance_id =
        task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let mut outcome =
        task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
    outcome.claim_revision += 1;
    let request = task_network_support::apply_memory_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::StaleClaim { .. })
    ));
}

#[test]
fn outcome_with_wrong_lifecycle_epoch_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let task_instance_id =
        task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let mut outcome =
        task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
    outcome.lifecycle_epoch += 1;
    let request = task_network_support::apply_memory_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::StaleClaim { .. })
    ));
}

#[test]
fn failed_outcome_moves_status_to_failed_and_preserves_error_text() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let task_instance_id =
        task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let outcome = Outcome {
        outcome_id: "outcome-failed".to_string(),
        task_instance_id: task_instance_id.clone(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id,
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Failed,
        error: Some("provider failed".to_string()),
        artifact_records: vec![],
        task_events: vec![],
        admission: claim.admission,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
    assert!(matches!(
        store.state().statuses.get(&task_instance_id),
        Some(TaskStatus::Failed { error, .. }) if error == "provider failed"
    ));
}

#[test]
fn succeeded_outcome_records_artifact_availability() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let task_instance_id =
        task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let outcome =
        task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
    let request = task_network_support::apply_memory_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
    assert_eq!(
        store.state().artifact_availability,
        vec![ArtifactAvailability {
            task_instance_id,
            artifact_type_id: "docs_patch".to_string(),
            artifact_id: "artifact-outcome-alpha".to_string(),
            schema_version: 1,
        }]
    );
}
