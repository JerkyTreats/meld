#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::task::{CompiledTaskRecord, TaskRunContext};
use meld_execution::task_network::{
    command::{Command, Request},
    mutation::{Inject, Mutation, ReadPrecondition, Set},
    state::{TaskLineage, TaskNode},
    store::InMemoryTaskNetworkStore,
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
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
        },
    }
}

fuzz_target!(|data: &[u8]| {
    let mut first = InMemoryTaskNetworkStore::new("network-fuzz");
    let mut commands = Vec::new();
    for (index, byte) in data.iter().take(8).enumerate() {
        let id = format!("task-{index}-{byte}");
        let node = task_node(&id);
        let lineage = node.lineage.clone();
        let set = Set::new(
            "network-fuzz",
            "composition-fuzz",
            format!("once-{index}-{byte}"),
            vec![Mutation::Inject(Inject::new(node, vec![], lineage))],
            vec![],
        );
        let request = Request {
            command_id: format!("command-{index}"),
            network_id: "network-fuzz".to_string(),
            base_revision: first.state().revision,
            base_state_hash: first.state().state_hash.clone(),
            read_preconditions: vec![ReadPrecondition::RevisionIs(first.state().revision)],
            command: Command::ApplyMutationSet(set),
        };
        let _ = first.submit(request.clone());
        commands.push(request);
    }

    let mut replayed = InMemoryTaskNetworkStore::new("network-fuzz");
    for command in commands {
        let _ = replayed.submit(command);
    }
    assert_eq!(first.state(), replayed.state());
    assert_eq!(first.journal(), replayed.journal());
});
