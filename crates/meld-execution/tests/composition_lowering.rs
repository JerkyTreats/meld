#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::DomainObjectRef;
use meld_execution::planning::{CompositionLoweringRequest, ExecutionCompositionLowerer};
use meld_execution::task::TaskCompiler;
use meld_execution::task_network::mutation::Mutation;
use meld_execution::task_network::state::{DependencyKind, TaskInitSource};
use meld_lang::{Bindings, Condition, Edge, EdgeKind, Term};
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
        .task_run_context
        .task_run_id
        .starts_with("task-network-run-"));
    assert!(inject.task_node.init_sources.is_empty());
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
    let plan = task_network_support::lower_phase8();

    assert_eq!(plan.mutations.mutations.len(), 3);
    let write = plan
        .mutations
        .mutations
        .iter()
        .find_map(|mutation| {
            let Mutation::Inject(inject) = mutation;
            (inject.task_node.lineage.step_id == "write_summary").then_some(inject)
        })
        .unwrap();
    assert_eq!(write.incoming_edges.len(), 2);
    assert!(write.incoming_edges.iter().all(|edge| {
        edge.to == write.task_node.task_instance_id
            && matches!(edge.kind, DependencyKind::DataFlow { .. })
    }));
    assert_eq!(write.task_node.init_sources.len(), 2);
    assert!(write.task_node.init_sources.iter().any(|source| {
        matches!(
            source,
            TaskInitSource::UpstreamArtifact(source)
                if source.init_slot_id == "metadata"
                    && source.upstream_artifact_type_id == "metadata_doc"
        )
    }));
    assert!(write.task_node.init_sources.iter().any(|source| {
        matches!(
            source,
            TaskInitSource::UpstreamArtifact(source)
                if source.init_slot_id == "context"
                    && source.upstream_artifact_type_id == "context_bundle"
        )
    }));
}

#[test]
fn composition_lowering_emits_all_operator_steps_in_order() {
    let plan = task_network_support::lower_phase8();

    let step_ids = plan
        .mutations
        .mutations
        .iter()
        .map(|mutation| {
            let Mutation::Inject(inject) = mutation;
            inject.task_node.lineage.step_id.as_str()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        step_ids,
        vec!["prepare_metadata", "collect_context", "write_summary"]
    );
}

#[test]
fn unresolved_operator_among_many_rejects_whole_lowering() {
    let mut composition = task_network_support::phase8_composition();
    composition.operator_resolutions[1].status =
        meld_execution::planning::OperatorResolutionStatus::Unresolved;
    let lowerer = ExecutionCompositionLowerer::new(
        TaskCompiler::new(),
        task_network_support::phase8_catalog(),
    );

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition,
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    assert!(plan.mutations.mutations.is_empty());
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code
            == meld_execution::planning::CompositionLoweringDiagnosticCode::OperatorUnresolved
            && diagnostic.step_id.as_deref() == Some("collect_context")
    }));
}

#[test]
fn missing_capability_contract_rejects_whole_lowering() {
    let composition = task_network_support::phase8_composition();
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

    assert!(plan.mutations.mutations.is_empty());
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code
            == meld_execution::planning::CompositionLoweringDiagnosticCode::CapabilityContractMissing
    }));
}

#[test]
fn executable_edge_with_one_missing_endpoint_rejects_whole_lowering() {
    let mut composition = task_network_support::phase8_composition();
    composition.composition.edges.push(Edge {
        from: "prepare_metadata".to_string(),
        to: "missing_step".to_string(),
        kind: EdgeKind::Ordering,
    });
    let lowerer = ExecutionCompositionLowerer::new(
        TaskCompiler::new(),
        task_network_support::phase8_catalog(),
    );

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition,
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    assert!(plan.mutations.mutations.is_empty());
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code
            == meld_execution::planning::CompositionLoweringDiagnosticCode::MissingExecutableEdgeEndpoint
    }));
}

#[test]
fn required_init_slots_without_data_flow_lower_to_static_seed_sources() {
    let mut composition = task_network_support::phase8_composition();
    composition.composition.edges.clear();
    let lowerer = ExecutionCompositionLowerer::new(
        TaskCompiler::new(),
        task_network_support::phase8_catalog(),
    );

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition,
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    let write = plan
        .mutations
        .mutations
        .iter()
        .find_map(|mutation| {
            let Mutation::Inject(inject) = mutation;
            (inject.task_node.lineage.step_id == "write_summary").then_some(inject)
        })
        .unwrap();

    assert_eq!(write.task_node.init_sources.len(), 2);
    assert!(write
        .task_node
        .init_sources
        .iter()
        .all(|source| matches!(source, TaskInitSource::StaticSeed(_))));
}

#[test]
fn single_input_data_flow_lowers_to_upstream_source() {
    let lowerer = ExecutionCompositionLowerer::new(
        TaskCompiler::new(),
        task_network_support::single_input_dataflow_catalog(),
    );

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition: task_network_support::single_input_dataflow_composition(),
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    let write = plan
        .mutations
        .mutations
        .iter()
        .find_map(|mutation| {
            let Mutation::Inject(inject) = mutation;
            (inject.task_node.lineage.step_id == "write_metadata").then_some(inject)
        })
        .unwrap();

    assert_eq!(write.task_node.init_sources.len(), 1);
    assert!(matches!(
        &write.task_node.init_sources[0],
        TaskInitSource::UpstreamArtifact(source)
            if source.init_slot_id == "metadata"
                && source.upstream_artifact_type_id == "metadata_doc"
    ));
}

#[test]
fn optional_input_data_flow_lowers_to_upstream_source_with_output_schema() {
    let lowerer = ExecutionCompositionLowerer::new(
        TaskCompiler::new(),
        task_network_support::optional_input_dataflow_catalog(),
    );

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition: task_network_support::optional_input_dataflow_composition(),
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    assert!(plan.diagnostics.is_empty());
    assert_eq!(plan.mutations.mutations.len(), 2);
    let consume = plan
        .mutations
        .mutations
        .iter()
        .find_map(|mutation| {
            let Mutation::Inject(inject) = mutation;
            (inject.task_node.lineage.step_id == "consume_optional_note").then_some(inject)
        })
        .unwrap();

    assert_eq!(consume.task_node.compiled_task.init_slots.len(), 1);
    assert_eq!(
        consume.task_node.compiled_task.init_slots[0].init_slot_id,
        "note"
    );
    assert_eq!(
        consume.task_node.compiled_task.init_slots[0].artifact_type_id,
        "optional_note"
    );
    assert_eq!(
        consume.task_node.compiled_task.init_slots[0].schema_version,
        2
    );
    assert!(matches!(
        &consume.task_node.init_sources[0],
        TaskInitSource::UpstreamArtifact(source)
            if source.init_slot_id == "note"
                && source.upstream_artifact_type_id == "optional_note"
                && source.schema_version == 2
    ));
}

#[test]
fn repeat_lowering_produces_same_mutation_set_identity() {
    let first = task_network_support::lower_phase8();
    let second = task_network_support::lower_phase8();

    assert_eq!(first.mutations.set_id, second.mutations.set_id);
    assert_eq!(first.mutations, second.mutations);
}

#[test]
fn conditional_edges_are_deferred_without_dependency_emission() {
    let mut composition = task_network_support::phase8_composition();
    composition.composition.edges.push(Edge {
        from: "prepare_metadata".to_string(),
        to: "collect_context".to_string(),
        kind: EdgeKind::Conditional {
            field_path: "ready".to_string(),
            guard: Condition::Present,
        },
    });
    let lowerer = ExecutionCompositionLowerer::new(
        TaskCompiler::new(),
        task_network_support::phase8_catalog(),
    );

    let plan = lowerer
        .lower(CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition,
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap();

    assert_eq!(plan.mutations.mutations.len(), 3);
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code
            == meld_execution::planning::CompositionLoweringDiagnosticCode::ConditionalEdgeDeferred
    }));
    let collect = plan
        .mutations
        .mutations
        .iter()
        .find_map(|mutation| {
            let Mutation::Inject(inject) = mutation;
            (inject.task_node.lineage.step_id == "collect_context").then_some(inject)
        })
        .unwrap();
    assert!(collect.incoming_edges.is_empty());
}
