use meld::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, BoundBindingValue,
    BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityCatalog,
    CapabilityTypeContract, EffectKind, EffectSpec, ExecutionClass, ExecutionContract,
    InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld::task::{compile_task_definition, TaskDefinition, TaskDependencyKind, TaskInitSlotSpec};
use serde_json::json;

fn register_contracts(catalog: &mut CapabilityCatalog) {
    let resolve_contract = CapabilityTypeContract {
        capability_type_id: "workspace_resolve_node_id".to_string(),
        capability_version: 1,
        owning_domain: "workspace".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "workspace".to_string(),
            scope_ref_kind: "workspace_root".to_string(),
            allow_fan_out: false,
        },
        binding_contract: vec![],
        input_contract: vec![InputSlotSpec {
            slot_id: "target_selector".to_string(),
            accepted_artifact_type_ids: vec!["target_selector".to_string()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
            required: true,
            cardinality: InputCardinality::One,
        }],
        output_contract: vec![OutputSlotSpec {
            slot_id: "resolved_node_ref".to_string(),
            artifact_type_id: "resolved_node_ref".to_string(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "artifact".to_string(),
            retry_class: "none".to_string(),
            cancellation_supported: false,
        },
    };
    let traversal_contract = CapabilityTypeContract {
        capability_type_id: "merkle_traversal".to_string(),
        capability_version: 1,
        owning_domain: "merkle_traversal".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "node".to_string(),
            scope_ref_kind: "node_id".to_string(),
            allow_fan_out: true,
        },
        binding_contract: vec![BindingSpec {
            binding_id: "strategy".to_string(),
            value_kind: BindingValueKind::Literal,
            required: true,
            affects_deterministic_identity: true,
        }],
        input_contract: vec![InputSlotSpec {
            slot_id: "resolved_node_ref".to_string(),
            accepted_artifact_type_ids: vec!["resolved_node_ref".to_string()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
            required: true,
            cardinality: InputCardinality::One,
        }],
        output_contract: vec![OutputSlotSpec {
            slot_id: "ordered_merkle_node_batches".to_string(),
            artifact_type_id: "ordered_merkle_node_batches".to_string(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![EffectSpec {
            effect_id: "read_tree".to_string(),
            kind: EffectKind::Read,
            target: "workspace_tree".to_string(),
            exclusive: false,
        }],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "artifact".to_string(),
            retry_class: "none".to_string(),
            cancellation_supported: false,
        },
    };

    catalog.register(resolve_contract).unwrap();
    catalog.register(traversal_contract).unwrap();
}

#[test]
fn task_compiler_builds_dependency_edges_from_upstream_outputs() {
    let mut catalog = CapabilityCatalog::new();
    register_contracts(&mut catalog);
    let compiled = compile_task_definition(
        &TaskDefinition {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![TaskInitSlotSpec {
                init_slot_id: "target_selector".to_string(),
                artifact_type_id: "target_selector".to_string(),
                schema_version: 1,
                required: true,
            }],
            capability_instances: vec![
                BoundCapabilityInstance {
                    capability_instance_id: "capinst_resolve".to_string(),
                    capability_type_id: "workspace_resolve_node_id".to_string(),
                    capability_version: 1,
                    scope_ref: "workspace".to_string(),
                    scope_kind: "workspace".to_string(),
                    binding_values: vec![],
                    input_wiring: vec![BoundInputWiring {
                        slot_id: "target_selector".to_string(),
                        sources: vec![BoundInputWiringSource::TaskInitSlot {
                            init_slot_id: "target_selector".to_string(),
                            artifact_type_id: "target_selector".to_string(),
                            schema_version: 1,
                        }],
                    }],
                },
                BoundCapabilityInstance {
                    capability_instance_id: "capinst_traversal".to_string(),
                    capability_type_id: "merkle_traversal".to_string(),
                    capability_version: 1,
                    scope_ref: "node_root".to_string(),
                    scope_kind: "node".to_string(),
                    binding_values: vec![BoundBindingValue {
                        binding_id: "strategy".to_string(),
                        value: json!("bottom_up"),
                    }],
                    input_wiring: vec![BoundInputWiring {
                        slot_id: "resolved_node_ref".to_string(),
                        sources: vec![BoundInputWiringSource::UpstreamOutput {
                            capability_instance_id: "capinst_resolve".to_string(),
                            output_slot_id: "resolved_node_ref".to_string(),
                            artifact_type_id: "resolved_node_ref".to_string(),
                            schema_version: 1,
                        }],
                    }],
                },
            ],
        },
        &catalog,
    )
    .unwrap();

    assert_eq!(compiled.dependency_edges.len(), 1);
    assert_eq!(
        compiled.dependency_edges[0].kind,
        TaskDependencyKind::Artifact
    );
}

#[test]
fn task_compiler_rejects_missing_declared_init_slot() {
    let mut catalog = CapabilityCatalog::new();
    register_contracts(&mut catalog);

    let error = compile_task_definition(
        &TaskDefinition {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![BoundCapabilityInstance {
                capability_instance_id: "capinst_resolve".to_string(),
                capability_type_id: "workspace_resolve_node_id".to_string(),
                capability_version: 1,
                scope_ref: "workspace".to_string(),
                scope_kind: "workspace".to_string(),
                binding_values: vec![],
                input_wiring: vec![BoundInputWiring {
                    slot_id: "target_selector".to_string(),
                    sources: vec![BoundInputWiringSource::TaskInitSlot {
                        init_slot_id: "target_selector".to_string(),
                        artifact_type_id: "target_selector".to_string(),
                        schema_version: 1,
                    }],
                }],
            }],
        },
        &catalog,
    )
    .unwrap_err();

    assert!(error.to_string().contains("missing init slot"));
}
