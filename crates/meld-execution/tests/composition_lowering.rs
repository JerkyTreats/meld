#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::DomainObjectRef;
use meld_execution::planning::{CompositionLoweringRequest, ExecutionCompositionLowerer};
use meld_execution::task::TaskCompiler;
use meld_execution::task_network::mutation::Mutation;
use meld_execution::task_network::state::DependencyKind;
use meld_lang::{Bindings, Edge, EdgeKind, Term};
use serde_json::json;

#[test]
fn composition_lowering_emits_one_inject_mutation() {
    let plan = task_network_support::lower();

    assert_eq!(plan.composition_id, "composition-docs");
    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.mutations.mutations.len(), 1);
    let Mutation::Inject(inject) = &plan.mutations.mutations[0];
    assert!(inject
        .task_node
        .task_instance_id
        .starts_with("task-network-task-"));
    assert_ne!(inject.task_node.task_instance_id, "xyzzy");
    assert!(inject
        .task_node
        .init_payload
        .task_run_context
        .task_run_id
        .starts_with("task-network-run-"));
    assert_eq!(inject.task_node.lineage.goal_id, "goal-docs");
    assert_eq!(inject.task_node.lineage.method_id, "refresh_docs_v1");
    assert_eq!(inject.task_node.compiled_task.capability_instances.len(), 1);
}

#[test]
fn unresolved_operator_returns_typed_lowering_diagnostic() {
    let plan = task_network_support::lower_unresolved();

    assert!(plan.mutations.mutations.is_empty());
    task_network_support::assert_unresolved_operator_diagnostic(&plan);
}

#[test]
fn composition_lowering_preserves_required_binding_values() {
    let expected_term = Term::Object(DomainObjectRef::new("workspace", "node", "readme").unwrap());
    let mut composition = task_network_support::composition();
    composition.bindings = Bindings::empty()
        .bind("node".to_string(), expected_term.clone())
        .unwrap();
    let lowerer = ExecutionCompositionLowerer::new(
        TaskCompiler::new(),
        task_network_support::catalog_with_required_node_binding(),
    );

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition,
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    assert_eq!(plan.mutations.mutations.len(), 1);
    let Mutation::Inject(inject) = &plan.mutations.mutations[0];
    let bindings = &inject.task_node.compiled_task.capability_instances[0].binding_values;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].binding_id, "node");
    assert_eq!(
        inject.task_node.compiled_task.capability_instances[0].scope_ref,
        "readme"
    );
    assert_ne!(
        inject.task_node.compiled_task.capability_instances[0].scope_ref,
        "xyzzy"
    );
    assert_eq!(
        bindings[0].value,
        serde_json::to_value(&expected_term).unwrap()
    );
    assert_ne!(
        bindings[0].value,
        json!({
            "source": "composition_lowering_default",
            "binding_id": "node",
        })
    );
}

#[test]
fn composition_lowering_preserves_incoming_edges() {
    let mut composition = task_network_support::composition();
    composition.composition.edges.push(Edge {
        from: "prepare".to_string(),
        to: "write".to_string(),
        kind: EdgeKind::DataFlow {
            artifact_type: "docs_patch".to_string(),
        },
    });
    let lowerer =
        ExecutionCompositionLowerer::new(TaskCompiler::new(), task_network_support::catalog());

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition,
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    assert_eq!(plan.mutations.mutations.len(), 1);
    let Mutation::Inject(inject) = &plan.mutations.mutations[0];
    assert_eq!(inject.incoming_edges.len(), 1);
    let edge = &inject.incoming_edges[0];
    assert!(edge.from.starts_with("task-network-task-"));
    assert_eq!(edge.to, inject.task_node.task_instance_id);
    assert_ne!(edge.from, edge.to);
    assert_eq!(
        edge.kind,
        DependencyKind::DataFlow {
            artifact_type_id: "docs_patch".to_string()
        }
    );
}
