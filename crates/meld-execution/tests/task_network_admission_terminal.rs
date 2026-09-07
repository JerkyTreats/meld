//! Terminal outcome queries for one admitted operational region.

#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task_network::dispatch::{Outcome, OutcomeStatus};
use meld_execution::task_network::state::{
    admission_region_terminal_outcome_id, DependencyEdge, DependencyEdgeOrigin, DependencyKind,
    NetworkState, TaskAdmissionAttribution, TaskStatus,
};

fn attribution(admission_id: &str) -> TaskAdmissionAttribution {
    TaskAdmissionAttribution {
        request_ref: None,
        admission_epoch: None,
        agent_id: "agent-docs".to_string(),
        goal_id: "goal-docs".to_string(),
        plan_revision_id: "plan-docs-v1".to_string(),
        task_id: "task-docs".to_string(),
        authorization_id: "authorization-docs-v1".to_string(),
        admission_id: admission_id.to_string(),
        authority_scope_id: "policy-docs".to_string(),
        authority_policy_content_hash: "policy-content-docs-v1".to_string(),
        activation_generation: "generation-v1".to_string(),
    }
}

fn outcome(outcome_id: &str, task_instance_id: &str, status: OutcomeStatus) -> Outcome {
    Outcome {
        outcome_id: outcome_id.to_string(),
        task_instance_id: task_instance_id.to_string(),
        lifecycle_epoch: 1,
        claim_id: format!("claim-{task_instance_id}"),
        claim_revision: 1,
        status,
        error: None,
        artifact_records: Vec::new(),
        task_events: Vec::new(),
        admission: Some(attribution("admission-docs")),
    }
}

fn insert_node(state: &mut NetworkState, task_instance_id: &str) {
    let mut node = task_network_support::single_task_node(task_instance_id);
    node.lineage.admission = Some(attribution("admission-docs"));
    state.tasks.insert(task_instance_id.to_string(), node);
    state
        .statuses
        .insert(task_instance_id.to_string(), TaskStatus::Pending);
}

#[test]
fn failed_branch_waits_for_independent_branch_before_terminal_return() {
    let mut state = NetworkState::empty("network-docs");
    for task_id in ["failed", "blocked-child", "independent"] {
        insert_node(&mut state, task_id);
    }
    state.edges.push(DependencyEdge {
        from: "failed".to_string(),
        to: "blocked-child".to_string(),
        kind: DependencyKind::Ordering,
        origin: DependencyEdgeOrigin::Semantic,
    });
    state.statuses.insert(
        "failed".to_string(),
        TaskStatus::Failed {
            outcome_id: "outcome-failed".to_string(),
            error: "provider failed".to_string(),
        },
    );
    state.outcomes.insert(
        "outcome-failed".to_string(),
        outcome("outcome-failed", "failed", OutcomeStatus::Failed),
    );

    assert_eq!(
        admission_region_terminal_outcome_id(&state, "admission-docs"),
        None
    );

    state.statuses.insert(
        "independent".to_string(),
        TaskStatus::Succeeded {
            outcome_id: "outcome-independent".to_string(),
        },
    );
    state.outcomes.insert(
        "outcome-independent".to_string(),
        outcome(
            "outcome-independent",
            "independent",
            OutcomeStatus::Succeeded,
        ),
    );

    assert_eq!(
        admission_region_terminal_outcome_id(&state, "admission-docs"),
        Some("outcome-failed")
    );
}

#[test]
fn successful_region_returns_only_its_exact_single_sink_outcome() {
    let mut state = NetworkState::empty("network-docs");
    for task_id in ["inspect", "assess"] {
        insert_node(&mut state, task_id);
    }
    state.edges.push(DependencyEdge {
        from: "inspect".to_string(),
        to: "assess".to_string(),
        kind: DependencyKind::Ordering,
        origin: DependencyEdgeOrigin::Semantic,
    });
    for task_id in ["inspect", "assess"] {
        let outcome_id = format!("outcome-{task_id}");
        state.statuses.insert(
            task_id.to_string(),
            TaskStatus::Succeeded {
                outcome_id: outcome_id.clone(),
            },
        );
        state.outcomes.insert(
            outcome_id.clone(),
            outcome(&outcome_id, task_id, OutcomeStatus::Succeeded),
        );
    }

    assert_eq!(
        admission_region_terminal_outcome_id(&state, "admission-docs"),
        Some("outcome-assess")
    );
}
