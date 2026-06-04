#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::mutation::{Mutation, ReadPrecondition, Rejection, Set};
use meld_execution::task_network::state::{DependencyEdge, DependencyKind, TaskStatus};
use meld_execution::task_network::store::InMemoryTaskNetworkStore;

#[test]
fn accepted_command_increments_revision_once() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );

    let response = store.submit(request);

    assert!(matches!(response, Response::Accepted { revision: 1, .. }));
    assert_eq!(store.state().revision, 1);
    assert_eq!(store.journal().len(), 1);
}

#[test]
fn exact_duplicate_accepted_command_returns_duplicate() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );

    assert!(matches!(
        store.submit(request.clone()),
        Response::Accepted { .. }
    ));
    let duplicate = store.submit(request);

    assert!(matches!(duplicate, Response::Duplicate { revision: 1, .. }));
    assert_eq!(store.journal().len(), 1);
}

#[test]
fn duplicate_rejected_command_returns_same_rejection() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::command_for_state(
        "wrong-network",
        store.state().revision,
        &store.state().state_hash,
        "command-wrong-network",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );

    let first = store.submit(request.clone());
    let second = store.submit(request);

    assert!(matches!(
        first,
        Response::Rejected(Rejection::InvalidGraph(_))
    ));
    assert_eq!(first, second);
    assert_eq!(store.state().revision, 0);
}

#[test]
fn duplicate_rejected_command_replays_original_rejection_after_state_changes() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let failed_precondition = ReadPrecondition::NodeExists("task-alpha".to_string());
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-rejected-replay",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    request.read_preconditions.push(failed_precondition.clone());

    assert_eq!(
        store.submit(request.clone()),
        Response::Rejected(Rejection::FailedPrecondition(failed_precondition.clone()))
    );

    task_network_support::commit_single_task(&mut store, "task-alpha");

    assert_eq!(
        store.submit(request),
        Response::Rejected(Rejection::FailedPrecondition(failed_precondition))
    );
}

#[test]
fn same_command_id_with_different_request_hash_returns_duplicate_command() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );
    assert!(matches!(store.submit(request), Response::Accepted { .. }));

    let duplicate = task_network_support::apply_memory_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-beta")),
    );

    assert!(matches!(
        store.submit(duplicate),
        Response::Rejected(Rejection::DuplicateCommand(command_id))
            if command_id == "command-commit"
    ));
}

#[test]
fn wrong_network_id_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::command_for_state(
        "wrong-network",
        0,
        &store.state().state_hash,
        "command-wrong-network",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidGraph(_))
    ));
}

#[test]
fn stale_base_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::command_for_state(
        "network-docs",
        7,
        &store.state().state_hash,
        "command-stale",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::StaleBase {
            expected: 7,
            actual: 0
        })
    ));
}

#[test]
fn state_hash_mismatch_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::command_for_state(
        "network-docs",
        0,
        "wrong-hash",
        "command-hash",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::StateHashMismatch { .. })
    ));
}

#[test]
fn state_hash_read_precondition_accepts_current_hash() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-state-hash-precondition",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    request
        .read_preconditions
        .push(ReadPrecondition::StateHashIs(
            store.state().state_hash.clone(),
        ));

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
}

#[test]
fn failed_read_precondition_rejects() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-precondition",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );
    request
        .read_preconditions
        .push(ReadPrecondition::NodeExists("task-alpha".to_string()));

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::FailedPrecondition(
            ReadPrecondition::NodeExists(task_instance_id)
        )) if task_instance_id == "task-alpha"
    ));
    assert!(store.state().tasks.is_empty());
}

#[test]
fn node_absent_precondition_accepts_missing_node() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-node-absent-precondition",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    request
        .read_preconditions
        .push(ReadPrecondition::NodeAbsent("task-alpha".to_string()));

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
}

#[test]
fn node_status_precondition_accepts_exact_status() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-status-precondition",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    request
        .read_preconditions
        .push(ReadPrecondition::NodeStatusIs {
            task_instance_id: "task-alpha".to_string(),
            status: TaskStatus::Pending,
        });

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
}

#[test]
fn claim_current_precondition_accepts_current_claim_revision() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    let task_instance_id =
        task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-claim-current-precondition",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    request
        .read_preconditions
        .push(ReadPrecondition::ClaimCurrent {
            task_instance_id,
            claim_id: claim.claim_id,
            claim_revision: claim.claim_revision,
        });

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
}

#[test]
fn claim_current_precondition_rejects_wrong_task_identity() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let failed_precondition = ReadPrecondition::ClaimCurrent {
        task_instance_id: "task-beta".to_string(),
        claim_id: claim.claim_id,
        claim_revision: claim.claim_revision,
    };
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-claim-current-wrong-task",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    request.read_preconditions.push(failed_precondition.clone());

    assert_eq!(
        store.submit(request),
        Response::Rejected(Rejection::FailedPrecondition(failed_precondition))
    );
}

#[test]
fn invalid_graph_rejects_before_state_mutation() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let node = task_network_support::single_task_node("task-alpha");
    let set = Set::new(
        "network-docs",
        "composition-fixture",
        "bad-edge",
        vec![Mutation::Inject(task_network_support::inject_for_node(
            node,
            vec![DependencyEdge {
                from: "missing".to_string(),
                to: "task-alpha".to_string(),
                kind: DependencyKind::Ordering,
            }],
        ))],
        vec![],
    );
    let request = task_network_support::apply_memory_command(
        &store,
        "command-invalid",
        Command::ApplyMutationSet(set),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidGraph(_))
    ));
    assert!(store.state().tasks.is_empty());
}

#[test]
fn duplicate_edges_are_deduped_on_commit() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let duplicate_edge = DependencyEdge {
        from: "task-alpha".to_string(),
        to: "task-beta".to_string(),
        kind: DependencyKind::Ordering,
    };
    let set = Set::new(
        "network-docs",
        "composition-fixture",
        "duplicate-edges",
        vec![
            Mutation::Inject(task_network_support::inject_for_node(
                task_network_support::single_task_node("task-alpha"),
                vec![],
            )),
            Mutation::Inject(task_network_support::inject_for_node(
                task_network_support::single_task_node("task-beta"),
                vec![duplicate_edge.clone(), duplicate_edge.clone()],
            )),
        ],
        vec![],
    );
    let request = task_network_support::apply_memory_command(
        &store,
        "command-duplicate-edges",
        Command::ApplyMutationSet(set),
    );

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
    assert_eq!(store.state().edges, vec![duplicate_edge]);
}

#[test]
fn empty_mutation_set_commits_when_graph_is_valid() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &store,
        "command-empty",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );

    assert!(matches!(
        store.submit(request),
        Response::Accepted { revision: 1, .. }
    ));
    assert!(store.state().tasks.is_empty());
}

#[test]
fn artifact_available_precondition_requires_matching_task_and_artifact_type() {
    let (mut store, task_instance_id, _) =
        task_network_support::memory_store_with_pending_publication();
    let wrong_artifact_type = "other_patch".to_string();
    let mut request = task_network_support::apply_memory_command(
        &store,
        "command-artifact-precondition",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    request
        .read_preconditions
        .push(ReadPrecondition::ArtifactAvailable {
            task_instance_id,
            artifact_type_id: wrong_artifact_type.clone(),
        });

    let response = store.submit(request);

    assert!(matches!(
        response,
        Response::Rejected(Rejection::FailedPrecondition(
            ReadPrecondition::ArtifactAvailable {
                artifact_type_id,
                ..
            }
        )) if artifact_type_id == wrong_artifact_type
    ));
}

#[test]
fn edge_exists_precondition_requires_exact_edge() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let commit = task_network_support::apply_memory_command(
        &store,
        "command-commit-two",
        Command::ApplyMutationSet(task_network_support::two_task_ordering_set(
            "task-alpha",
            "task-beta",
        )),
    );
    assert!(matches!(store.submit(commit), Response::Accepted { .. }));

    let mut accepted = task_network_support::apply_memory_command(
        &store,
        "command-edge-positive",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    accepted
        .read_preconditions
        .push(ReadPrecondition::EdgeExists {
            from: "task-alpha".to_string(),
            to: "task-beta".to_string(),
        });
    assert!(matches!(store.submit(accepted), Response::Accepted { .. }));

    let failed_precondition = ReadPrecondition::EdgeExists {
        from: "task-alpha".to_string(),
        to: "missing".to_string(),
    };
    let mut rejected = task_network_support::apply_memory_command(
        &store,
        "command-edge-negative",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    rejected
        .read_preconditions
        .push(failed_precondition.clone());

    assert_eq!(
        store.submit(rejected),
        Response::Rejected(Rejection::FailedPrecondition(failed_precondition))
    );
}

#[test]
fn no_path_precondition_distinguishes_reachable_and_unreachable_nodes() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let set = Set::new(
        "network-docs",
        "composition-fixture",
        "inject-chain",
        vec![
            Mutation::Inject(task_network_support::inject_for_node(
                task_network_support::single_task_node("task-alpha"),
                vec![],
            )),
            Mutation::Inject(task_network_support::inject_for_node(
                task_network_support::single_task_node("task-beta"),
                vec![DependencyEdge {
                    from: "task-alpha".to_string(),
                    to: "task-beta".to_string(),
                    kind: DependencyKind::Ordering,
                }],
            )),
            Mutation::Inject(task_network_support::inject_for_node(
                task_network_support::single_task_node("task-gamma"),
                vec![DependencyEdge {
                    from: "task-beta".to_string(),
                    to: "task-gamma".to_string(),
                    kind: DependencyKind::Ordering,
                }],
            )),
        ],
        vec![],
    );
    let commit = task_network_support::apply_memory_command(
        &store,
        "command-commit-chain",
        Command::ApplyMutationSet(set),
    );
    assert!(matches!(store.submit(commit), Response::Accepted { .. }));

    let reachable_precondition = ReadPrecondition::NoPath {
        from: "task-alpha".to_string(),
        to: "task-gamma".to_string(),
    };
    let mut reachable = task_network_support::apply_memory_command(
        &store,
        "command-no-path-reachable",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    reachable
        .read_preconditions
        .push(reachable_precondition.clone());
    assert_eq!(
        store.submit(reachable),
        Response::Rejected(Rejection::FailedPrecondition(reachable_precondition))
    );

    let mut unreachable = task_network_support::apply_memory_command(
        &store,
        "command-no-path-unreachable",
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    unreachable
        .read_preconditions
        .push(ReadPrecondition::NoPath {
            from: "task-gamma".to_string(),
            to: "task-alpha".to_string(),
        });
    assert!(matches!(
        store.submit(unreachable),
        Response::Accepted { .. }
    ));
}
