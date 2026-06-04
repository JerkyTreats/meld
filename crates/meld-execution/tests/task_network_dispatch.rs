#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::dispatch::{
    build_executor_for_claim, Claim, Outcome, OutcomeStatus, Request as DispatchRequest,
};
use meld_execution::task_network::mutation::Rejection;
use meld_execution::task_network::state::{ArtifactAvailability, TaskStatus};
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
    let mut claim = Claim::accepted("network-docs", &request, node.lifecycle_epoch, 1);

    assert!(build_executor_for_claim(&node, &claim, "repo-alpha").is_ok());

    claim.task_instance_id = "task-beta".to_string();
    let error = build_executor_for_claim(&node, &claim, "repo-alpha").unwrap_err();

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
        artifacts: vec![],
        artifact_records: vec![],
        task_events: vec![],
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
