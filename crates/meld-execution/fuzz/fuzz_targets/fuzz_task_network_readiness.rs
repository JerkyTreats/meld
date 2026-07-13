#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::task::{CompiledTaskRecord, TaskRunContext};
use meld_execution::task_network::{
    readiness::compute_ready_set,
    state::{DependencyEdge, DependencyKind, NetworkState, TaskLineage, TaskNode, TaskStatus},
};

fn task_node(id: &str) -> TaskNode {
    let task_id = format!("compiled-{id}");
    TaskNode {
        task_instance_id: id.to_string(),
        lifecycle_epoch: 1,
        compiled_task: CompiledTaskRecord {
            task_id: task_id.clone(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![],
            dependency_edges: vec![],
        },
        init_sources: vec![],
        task_run_context: TaskRunContext {
            task_run_id: format!("run-{id}"),
            session_id: None,
            trigger: "fuzz".to_string(),
        },
        lineage: TaskLineage {
            composition_id: "composition-fuzz".to_string(),
            goal_id: "goal-fuzz".to_string(),
            method_id: "method-fuzz".to_string(),
            step_id: format!("step-{id}"),
            operator_id: format!("operator-{id}"),
            world_state_frame_id: "frame-fuzz".to_string(),
            subject: None,
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
        },
    }
}

fuzz_target!(|data: &[u8]| {
    let count = data.len().min(8);
    let mut state = NetworkState::empty("network-fuzz");
    for index in 0..count {
        let id = format!("task-{index}");
        state.tasks.insert(id.clone(), task_node(&id));
        let status = match data[index] % 4 {
            0 => TaskStatus::Pending,
            1 => TaskStatus::Running {
                claim_id: format!("claim-{index}"),
            },
            2 => TaskStatus::Succeeded {
                outcome_id: format!("outcome-{index}"),
            },
            _ => TaskStatus::Failed {
                outcome_id: format!("outcome-{index}"),
                error: "failed".to_string(),
            },
        };
        state.statuses.insert(id, status);
    }
    for index in 1..count {
        if data[index] % 2 == 0 {
            state.edges.push(DependencyEdge {
                from: format!("task-{}", index - 1),
                to: format!("task-{index}"),
                kind: DependencyKind::Ordering,
            });
        }
    }
    state.set_revision_and_hash(1);
    let ready = compute_ready_set(&state);
    assert_eq!(ready, compute_ready_set(&state));
    let mut sorted = ready.task_instance_ids.clone();
    sorted.sort();
    assert_eq!(ready.task_instance_ids, sorted);
});
