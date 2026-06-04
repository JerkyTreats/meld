//! Ready set computation over reduced task network state.
//!
//! Owner: task network.
//! Inputs: reduced network state.
//! Outputs: deterministic ready task query records and graph diagnostics.
//! Does not own: this module does not persist state or claim tasks.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::readiness::compute_ready_set;
//! use meld_execution::task_network::state::NetworkState;
//!
//! let state = NetworkState::empty("network-a");
//! let ready = compute_ready_set(&state);
//! assert!(ready.task_instance_ids.is_empty());
//! ```

use crate::task_network::state::{
    DependencyKind, NetworkState, ReadinessDiagnostic, ReadinessDiagnosticCode, ReadySet,
    TaskStatus,
};
use petgraph::algo::is_cyclic_directed;
use petgraph::graphmap::DiGraphMap;

/// Computes the ready set for pending task nodes at the supplied revision.
pub fn compute_ready_set(state: &NetworkState) -> ReadySet {
    let mut diagnostics = validate_active_graph(state);
    let mut task_instance_ids = Vec::new();

    for task_instance_id in state.tasks.keys() {
        if state.statuses.get(task_instance_id) != Some(&TaskStatus::Pending) {
            continue;
        }

        let mut blocked = false;
        for edge in state
            .edges
            .iter()
            .filter(|edge| edge.to == *task_instance_id)
        {
            match &edge.kind {
                DependencyKind::Ordering => {
                    if !matches!(
                        state.statuses.get(&edge.from),
                        Some(TaskStatus::Succeeded { .. })
                    ) {
                        blocked = true;
                    }
                }
                DependencyKind::DataFlow { artifact_type_id } => {
                    let artifact_available = state.artifact_availability.iter().any(|artifact| {
                        artifact.task_instance_id == edge.from
                            && artifact.artifact_type_id == *artifact_type_id
                    });
                    if !artifact_available {
                        diagnostics.push(
                            ReadinessDiagnostic::new(
                                ReadinessDiagnosticCode::ArtifactUnavailable,
                                format!(
                                    "artifact '{}' is not available from '{}'",
                                    artifact_type_id, edge.from
                                ),
                            )
                            .with_task(task_instance_id.clone()),
                        );
                        blocked = true;
                    }
                }
                DependencyKind::Conditional { .. } => {
                    diagnostics.push(
                        ReadinessDiagnostic::new(
                            ReadinessDiagnosticCode::ConditionalDeferred,
                            "conditional task network edges are deferred",
                        )
                        .with_task(task_instance_id.clone()),
                    );
                    blocked = true;
                }
            }
        }

        if !blocked {
            task_instance_ids.push(task_instance_id.clone());
        }
    }

    ReadySet {
        network_id: state.network_id.clone(),
        revision: state.revision,
        state_hash: state.state_hash.clone(),
        task_instance_ids,
        diagnostics,
    }
}

/// Validates endpoint presence and acyclicity for the active graph.
pub fn validate_active_graph(state: &NetworkState) -> Vec<ReadinessDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut graph = DiGraphMap::<&str, ()>::new();

    for task_instance_id in state.tasks.keys() {
        graph.add_node(task_instance_id.as_str());
    }

    for edge in &state.edges {
        let from_exists = state.tasks.contains_key(&edge.from);
        let to_exists = state.tasks.contains_key(&edge.to);
        if !from_exists || !to_exists {
            diagnostics.push(ReadinessDiagnostic::new(
                ReadinessDiagnosticCode::MissingEndpoint,
                format!(
                    "edge from '{}' to '{}' references a missing endpoint",
                    edge.from, edge.to
                ),
            ));
            continue;
        }
        graph.add_edge(edge.from.as_str(), edge.to.as_str(), ());
    }

    if is_cyclic_directed(&graph) {
        diagnostics.push(ReadinessDiagnostic::new(
            ReadinessDiagnosticCode::CycleDetected,
            "task network contains a cycle",
        ));
    }

    diagnostics
}
