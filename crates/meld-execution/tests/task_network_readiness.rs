#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::state::{
    ArtifactAvailability, DependencyEdge, DependencyEdgeOrigin, DependencyKind, NetworkState,
    ReadinessDiagnosticCode, TaskStatus,
};
use meld_execution::task_network::store::InMemoryTaskNetworkStore;
use proptest::prelude::*;

fn state_with_task(task_instance_id: &str, status: TaskStatus) -> NetworkState {
    let mut state = NetworkState::empty("network-docs");
    state.tasks.insert(
        task_instance_id.to_string(),
        task_network_support::single_task_node(task_instance_id),
    );
    state.statuses.insert(task_instance_id.to_string(), status);
    state.set_revision_and_hash(1);
    state
}

#[test]
fn source_pending_task_is_ready() {
    let state = state_with_task("task-alpha", TaskStatus::Pending);

    let ready = compute_ready_set(&state);

    assert_eq!(ready.task_instance_ids, vec!["task-alpha"]);
}

#[test]
fn running_task_is_not_ready() {
    let state = state_with_task(
        "task-alpha",
        TaskStatus::Running {
            claim_id: "claim-alpha".to_string(),
        },
    );

    assert!(compute_ready_set(&state).task_instance_ids.is_empty());
}

#[test]
fn succeeded_task_is_not_ready() {
    let state = state_with_task(
        "task-alpha",
        TaskStatus::Succeeded {
            outcome_id: "outcome-alpha".to_string(),
        },
    );

    assert!(compute_ready_set(&state).task_instance_ids.is_empty());
}

#[test]
fn failed_task_is_not_ready() {
    let state = state_with_task(
        "task-alpha",
        TaskStatus::Failed {
            outcome_id: "outcome-alpha".to_string(),
            error: "failed".to_string(),
        },
    );

    assert!(compute_ready_set(&state).task_instance_ids.is_empty());
}

#[test]
fn cancelled_task_is_not_ready() {
    let state = state_with_task(
        "task-alpha",
        TaskStatus::Cancelled {
            reason: "cancelled".to_string(),
        },
    );

    assert!(compute_ready_set(&state).task_instance_ids.is_empty());
}

#[test]
fn ordering_edge_waits_for_upstream_success() {
    let mut state = NetworkState::empty("network-docs");
    state.tasks.insert(
        "task-upstream".to_string(),
        task_network_support::single_task_node("task-upstream"),
    );
    state.tasks.insert(
        "task-downstream".to_string(),
        task_network_support::task_node_with_upstream_source(
            "task-downstream",
            "task-upstream",
            "metadata",
            "docs_patch",
            1,
        ),
    );
    state
        .statuses
        .insert("task-upstream".to_string(), TaskStatus::Pending);
    state
        .statuses
        .insert("task-downstream".to_string(), TaskStatus::Pending);
    state.edges.push(DependencyEdge {
        from: "task-upstream".to_string(),
        to: "task-downstream".to_string(),
        kind: DependencyKind::Ordering,

        origin: DependencyEdgeOrigin::Unrecorded,
    });
    state.set_revision_and_hash(1);
    assert_eq!(
        compute_ready_set(&state).task_instance_ids,
        vec!["task-upstream"]
    );

    state.statuses.insert(
        "task-upstream".to_string(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-upstream".to_string(),
        },
    );
    state.set_revision_and_hash(2);

    assert_eq!(
        compute_ready_set(&state).task_instance_ids,
        vec!["task-downstream"]
    );
}

#[test]
fn data_flow_edge_waits_for_matching_artifact_availability() {
    let mut state = NetworkState::empty("network-docs");
    state.tasks.insert(
        "task-upstream".to_string(),
        task_network_support::single_task_node("task-upstream"),
    );
    state.tasks.insert(
        "task-downstream".to_string(),
        task_network_support::task_node_with_upstream_source(
            "task-downstream",
            "task-upstream",
            "metadata",
            "docs_patch",
            1,
        ),
    );
    state.statuses.insert(
        "task-upstream".to_string(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-upstream".to_string(),
        },
    );
    state
        .statuses
        .insert("task-downstream".to_string(), TaskStatus::Pending);
    state.edges.push(DependencyEdge {
        from: "task-upstream".to_string(),
        to: "task-downstream".to_string(),
        kind: DependencyKind::DataFlow {
            artifact_type_id: "docs_patch".to_string(),
        },

        origin: DependencyEdgeOrigin::Unrecorded,
    });
    state.set_revision_and_hash(1);
    assert!(compute_ready_set(&state).task_instance_ids.is_empty());

    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: "other-task".to_string(),
        artifact_type_id: "docs_patch".to_string(),
        artifact_id: "artifact-wrong-task".to_string(),
        schema_version: 1,
    });
    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: "task-upstream".to_string(),
        artifact_type_id: "other_patch".to_string(),
        artifact_id: "artifact-wrong-type".to_string(),
        schema_version: 1,
    });
    state.set_revision_and_hash(2);
    assert!(compute_ready_set(&state).task_instance_ids.is_empty());

    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: "task-upstream".to_string(),
        artifact_type_id: "docs_patch".to_string(),
        artifact_id: "artifact-alpha".to_string(),
        schema_version: 1,
    });
    state.set_revision_and_hash(3);

    assert_eq!(
        compute_ready_set(&state).task_instance_ids,
        vec!["task-downstream"]
    );
}

#[test]
fn missing_dependency_endpoint_reports_diagnostic() {
    let mut state = state_with_task("task-alpha", TaskStatus::Pending);
    state.edges.push(DependencyEdge {
        from: "missing".to_string(),
        to: "task-alpha".to_string(),
        kind: DependencyKind::Ordering,

        origin: DependencyEdgeOrigin::Unrecorded,
    });
    state.set_revision_and_hash(2);

    let ready = compute_ready_set(&state);

    assert!(ready
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == ReadinessDiagnosticCode::MissingEndpoint));
}

#[test]
fn cycle_reports_diagnostic() {
    let mut state = NetworkState::empty("network-docs");
    state.tasks.insert(
        "task-a".to_string(),
        task_network_support::single_task_node("task-a"),
    );
    state.tasks.insert(
        "task-b".to_string(),
        task_network_support::single_task_node("task-b"),
    );
    state
        .statuses
        .insert("task-a".to_string(), TaskStatus::Pending);
    state
        .statuses
        .insert("task-b".to_string(), TaskStatus::Pending);
    state.edges.push(DependencyEdge {
        from: "task-a".to_string(),
        to: "task-b".to_string(),
        kind: DependencyKind::Ordering,

        origin: DependencyEdgeOrigin::Unrecorded,
    });
    state.edges.push(DependencyEdge {
        from: "task-b".to_string(),
        to: "task-a".to_string(),
        kind: DependencyKind::Ordering,

        origin: DependencyEdgeOrigin::Unrecorded,
    });
    state.set_revision_and_hash(1);

    let ready = compute_ready_set(&state);

    assert!(ready
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == ReadinessDiagnosticCode::CycleDetected));
}

#[test]
fn repeated_input_state_returns_identical_ready_set_ordering() {
    let mut state = NetworkState::empty("network-docs");
    for id in ["task-c", "task-a", "task-b"] {
        state
            .tasks
            .insert(id.to_string(), task_network_support::single_task_node(id));
        state.statuses.insert(id.to_string(), TaskStatus::Pending);
    }
    state.set_revision_and_hash(1);

    assert_eq!(compute_ready_set(&state), compute_ready_set(&state));
    assert_eq!(
        compute_ready_set(&state).task_instance_ids,
        vec!["task-a", "task-b", "task-c"]
    );
}

#[test]
fn independent_source_tasks_are_ready_together() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::admit_and_realize_phase8_in_memory(&mut store);

    let ready_steps = compute_ready_set(store.state())
        .task_instance_ids
        .iter()
        .map(|task_instance_id| {
            store
                .state()
                .tasks
                .get(task_instance_id)
                .unwrap()
                .lineage
                .step_id
                .clone()
        })
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(
        ready_steps,
        std::collections::BTreeSet::from([
            "collect_context".to_string(),
            "prepare_metadata".to_string()
        ])
    );
}

#[test]
fn downstream_join_waits_for_all_incoming_artifacts() {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::admit_and_realize_phase8_in_memory(&mut store);
    let mut state = store.state().clone();
    let ids = state
        .tasks
        .iter()
        .map(|(task_instance_id, node)| (node.lineage.step_id.clone(), task_instance_id.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();

    state.statuses.insert(
        ids["prepare_metadata"].clone(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-prepare".to_string(),
        },
    );
    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: ids["prepare_metadata"].clone(),
        artifact_type_id: "metadata_doc".to_string(),
        artifact_id: "artifact-metadata".to_string(),
        schema_version: 1,
    });
    state.set_revision_and_hash(2);

    let ready_steps = compute_ready_set(&state)
        .task_instance_ids
        .iter()
        .map(|task_instance_id| state.tasks[task_instance_id].lineage.step_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ready_steps, vec!["collect_context"]);

    state.statuses.insert(
        ids["collect_context"].clone(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-context".to_string(),
        },
    );
    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: ids["collect_context"].clone(),
        artifact_type_id: "context_bundle".to_string(),
        artifact_id: "artifact-context".to_string(),
        schema_version: 1,
    });
    state.set_revision_and_hash(3);

    let ready_steps = compute_ready_set(&state)
        .task_instance_ids
        .iter()
        .map(|task_instance_id| state.tasks[task_instance_id].lineage.step_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ready_steps, vec!["write_summary"]);
}

#[test]
fn data_flow_readiness_uses_source_schema_version_when_present() {
    let mut state = NetworkState::empty("network-docs");
    state.tasks.insert(
        "task-upstream".to_string(),
        task_network_support::single_task_node("task-upstream"),
    );
    state.tasks.insert(
        "task-downstream".to_string(),
        task_network_support::task_node_with_upstream_source(
            "task-downstream",
            "task-upstream",
            "metadata",
            "metadata_doc",
            2,
        ),
    );
    state.statuses.insert(
        "task-upstream".to_string(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-upstream".to_string(),
        },
    );
    state
        .statuses
        .insert("task-downstream".to_string(), TaskStatus::Pending);
    state.edges.push(DependencyEdge {
        from: "task-upstream".to_string(),
        to: "task-downstream".to_string(),
        kind: DependencyKind::DataFlow {
            artifact_type_id: "metadata_doc".to_string(),
        },

        origin: DependencyEdgeOrigin::Unrecorded,
    });
    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: "task-upstream".to_string(),
        artifact_type_id: "metadata_doc".to_string(),
        artifact_id: "artifact-wrong-schema".to_string(),
        schema_version: 1,
    });
    state.set_revision_and_hash(1);

    assert!(compute_ready_set(&state).task_instance_ids.is_empty());

    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: "task-upstream".to_string(),
        artifact_type_id: "metadata_doc".to_string(),
        artifact_id: "artifact-right-schema".to_string(),
        schema_version: 2,
    });
    state.set_revision_and_hash(2);

    assert_eq!(
        compute_ready_set(&state).task_instance_ids,
        vec!["task-downstream"]
    );
}

#[test]
fn data_flow_readiness_blocks_when_source_record_does_not_match_edge() {
    let mut state = NetworkState::empty("network-docs");
    state.tasks.insert(
        "task-upstream".to_string(),
        task_network_support::single_task_node("task-upstream"),
    );
    state.tasks.insert(
        "task-downstream".to_string(),
        task_network_support::task_node_with_upstream_source(
            "task-downstream",
            "other-upstream",
            "metadata",
            "metadata_doc",
            2,
        ),
    );
    state.statuses.insert(
        "task-upstream".to_string(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-upstream".to_string(),
        },
    );
    state
        .statuses
        .insert("task-downstream".to_string(), TaskStatus::Pending);
    state.edges.push(DependencyEdge {
        from: "task-upstream".to_string(),
        to: "task-downstream".to_string(),
        kind: DependencyKind::DataFlow {
            artifact_type_id: "metadata_doc".to_string(),
        },

        origin: DependencyEdgeOrigin::Unrecorded,
    });
    state.artifact_availability.push(ArtifactAvailability {
        task_instance_id: "task-upstream".to_string(),
        artifact_type_id: "metadata_doc".to_string(),
        artifact_id: "artifact-schema-one".to_string(),
        schema_version: 1,
    });
    state.set_revision_and_hash(1);

    assert!(compute_ready_set(&state).task_instance_ids.is_empty());
}

proptest! {
    #[test]
    fn ready_set_is_sorted_deterministic_and_excludes_non_pending(count in 1usize..=8) {
        let mut state = NetworkState::empty("network-docs");
        for index in 0..count {
            let id = format!("task-{index}");
            state.tasks.insert(id.clone(), task_network_support::single_task_node(&id));
            let status = match index % 4 {
                0 => TaskStatus::Pending,
                1 => TaskStatus::Running { claim_id: format!("claim-{index}") },
                2 => TaskStatus::Succeeded { outcome_id: format!("outcome-{index}") },
                _ => TaskStatus::Failed {
                    outcome_id: format!("outcome-{index}"),
                    error: "failed".to_string(),
                },
            };
            state.statuses.insert(id, status);
        }
        state.set_revision_and_hash(1);

        let ready = compute_ready_set(&state);
        let mut sorted = ready.task_instance_ids.clone();
        sorted.sort();
        prop_assert_eq!(&ready.task_instance_ids, &sorted);
        prop_assert_eq!(ready.clone(), compute_ready_set(&state));
        for id in &ready.task_instance_ids {
            prop_assert_eq!(state.statuses.get(id), Some(&TaskStatus::Pending));
        }
    }
}
