#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::{
    task::{CompiledTaskRecord, TaskRunContext},
    task_network::{
        command::{Command, Request, Response},
        mutation::{Inject, Mutation, ReadPrecondition, Set},
        state::{DependencyEdge, DependencyKind, TaskLineage, TaskNode},
        store::TaskNetworkStoreFactory,
        TaskNetworkAuthority,
    },
};

fn task_node(id: &str) -> TaskNode {
    TaskNode {
        task_instance_id: id.to_string(),
        lifecycle_epoch: 1,
        compiled_task: CompiledTaskRecord {
            task_id: format!("compiled-{id}"),
            task_version: 1,
            init_slots: Vec::new(),
            capability_instances: Vec::new(),
            dependency_edges: Vec::new(),
        },
        init_sources: Vec::new(),
        task_run_context: TaskRunContext {
            task_run_id: format!("run-{id}"),
            session_id: None,
            trigger: "fuzz authority recovery".to_string(),
        },
        lineage: TaskLineage {
            composition_id: "composition-fuzz".to_string(),
            goal_id: "goal-fuzz".to_string(),
            method_id: "method-fuzz".to_string(),
            step_id: format!("step-{id}"),
            operator_id: format!("operator-{id}"),
            world_state_frame_id: "frame-fuzz".to_string(),
            subject: None,
            capability_type_id: "fuzz.run".to_string(),
            capability_version: 1,
        },
    }
}

fuzz_target!(|data: &[u8]| {
    let count = data.len().min(6);
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let expected;
    {
        let mut authority = TaskNetworkAuthority::open(&factory, "network-fuzz", 16).unwrap();
        let query = authority.query_port();
        let command = authority.command_port();
        let mut previous_task = None;
        for (index, byte) in data.iter().copied().take(count).enumerate() {
            let head = query.head().unwrap();
            let id = format!("task-{index}-{byte}");
            let incoming_edges = previous_task
                .as_ref()
                .map(|previous: &String| {
                    vec![DependencyEdge {
                        from: previous.clone(),
                        to: id.clone(),
                        kind: DependencyKind::Ordering,
                    }]
                })
                .unwrap_or_default();
            let request = Request {
                command_id: format!("command-{index}"),
                network_id: "network-fuzz".to_string(),
                base_revision: head.revision,
                base_state_hash: head.state_hash,
                read_preconditions: vec![ReadPrecondition::RevisionIs(head.revision)],
                command: Command::ApplyMutationSet(Set::new(
                    "network-fuzz",
                    "composition-fuzz",
                    format!("once-{index}"),
                    vec![Mutation::Inject(Inject::new(
                        task_node(&id),
                        incoming_edges,
                    ))],
                    Vec::new(),
                )),
            };
            assert!(matches!(
                command.try_submit(request).unwrap(),
                Response::Accepted { .. }
            ));
            previous_task = Some(id);
        }
        expected = query.state().unwrap();
        let mut requested = expected
            .tasks
            .keys()
            .enumerate()
            .filter(|(index, _)| data.get(*index).copied().unwrap_or(0) % 2 == 0)
            .map(|(_, id)| id.clone())
            .collect::<Vec<_>>();
        requested.push("task-missing".to_string());
        let materialization = query.materialization(requested.clone()).unwrap();
        assert_eq!(materialization.head, query.head().unwrap());
        for id in requested {
            assert_eq!(materialization.tasks.get(&id), expected.tasks.get(&id));
            let expected_edges = expected
                .edges
                .iter()
                .filter(|edge| edge.to == id)
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(materialization.incoming_edges[&id], expected_edges);
        }
        authority.shutdown().unwrap();
    }

    let mut reopened = TaskNetworkAuthority::open(&factory, "network-fuzz", 16).unwrap();
    assert_eq!(reopened.query_port().state().unwrap(), expected);
    for index in 0..count {
        let outcome = reopened
            .query_port()
            .command_outcome(format!("command-{index}"))
            .unwrap()
            .unwrap();
        assert!(matches!(outcome.response(), Response::Accepted { .. }));
    }
    reopened.shutdown().unwrap();
});
