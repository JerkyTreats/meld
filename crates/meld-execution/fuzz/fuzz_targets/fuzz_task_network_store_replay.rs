#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::capability::CapabilityCatalog;
use meld_execution::task::{CompiledTaskRecord, TaskRunContext};
use meld_execution::task_admission::{
    ExecutionTask, TaskAdmissionApi, TaskAdmissionLineage, TaskAdmissionRequest,
};
use meld_execution::task_network::{
    command::{Command, Request},
    mutation::{Inject, Mutation, ReadPrecondition, Set},
    state::{TaskLineage, TaskNode},
    store::InMemoryTaskNetworkStore,
};
use meld_lang::{Bindings, Composition};

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
        lineage: TaskLineage::unattributed(
            format!("step-{id}"),
            format!("operator-{id}"),
            "docs.write".to_string(),
            1,
        ),
    }
}

fn admission(index: usize, byte: u8) -> TaskAdmissionRequest {
    let task_id = format!("admission-task-{index}-{byte}");
    TaskAdmissionRequest {
        lineage: TaskAdmissionLineage {
            agent_id: "agent-fuzz".to_string(),
            goal_id: "goal-fuzz".to_string(),
            plan_revision_id: "plan-fuzz".to_string(),
            product_id: task_id.clone(),
            authorization_id: format!("authorization-{index}-{byte}"),
            context_id: "context-fuzz".to_string(),
            authority_scope_id: "scope-fuzz".to_string(),
            authority_policy_content_hash: String::new(),
            authority_decision: None,
            activation_generation: format!("generation-{byte}"),

            admission_epoch: None,
        },
        task: ExecutionTask {
            initial_inputs: Vec::new(),
            task_id: task_id.clone(),
            composition: Composition {
                steps: Vec::new(),
                edges: Vec::new(),
            },
            bindings: Bindings::empty(),
            capability_contract_ids: Vec::new(),
            expected_outcome_contract_id: "outcome-fuzz".to_string(),
            authority_requirements: Vec::new(),
            idempotency_key: task_id.clone(),
        },
        idempotency_key: if byte % 3 == 1 {
            format!("rejected-{task_id}")
        } else {
            task_id
        },
    }
}

#[derive(Clone)]
enum Action {
    Admit {
        request: TaskAdmissionRequest,
        live_generation: String,
    },
    Command(Request),
}

fn apply(store: &mut InMemoryTaskNetworkStore, action: &Action) {
    match action {
        Action::Admit {
            request,
            live_generation,
        } => {
            let _ = TaskAdmissionApi::new(store, &CapabilityCatalog::new(), live_generation, "")
                .admit(request.clone());
        }
        Action::Command(request) => {
            let _ = store.submit(request.clone());
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let mut first = InMemoryTaskNetworkStore::new("network-fuzz");
    let mut actions = Vec::new();
    for (index, byte) in data.iter().take(8).enumerate() {
        let action = if byte % 2 == 0 {
            let request = admission(index, *byte);
            let live_generation = if byte % 3 == 2 {
                format!("generation-{}", byte.wrapping_add(1))
            } else {
                format!("generation-{byte}")
            };
            Action::Admit {
                request,
                live_generation,
            }
        } else {
            let id = format!("task-{index}-{byte}");
            let node = task_node(&id);
            Action::Command(Request {
                command_id: format!("command-{index}"),
                network_id: "network-fuzz".to_string(),
                base_revision: first.state().revision,
                base_state_hash: first.state().state_hash.clone(),
                read_preconditions: vec![ReadPrecondition::RevisionIs(first.state().revision)],
                command: Command::ApplyMutationSet(Set::new(
                    "network-fuzz",
                    "composition-fuzz",
                    format!("once-{index}-{byte}"),
                    vec![Mutation::Inject(Inject::new(node, vec![]))],
                )),
            })
        };
        apply(&mut first, &action);
        actions.push(action);
    }

    let mut replayed = InMemoryTaskNetworkStore::new("network-fuzz");
    for action in &actions {
        apply(&mut replayed, action);
    }
    assert_eq!(first.state(), replayed.state());
    assert_eq!(first.journal(), replayed.journal());
});
