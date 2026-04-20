use meld::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, BoundBindingValue,
    BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityCatalog,
    CapabilityExecutionContext, CapabilityTypeContract, EffectKind, EffectSpec, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld::task::{
    build_execution_task_envelope, compile_task_definition, ArtifactProducerRef, ArtifactRecord,
    InitArtifactValue, TaskDefinition, TaskExecutor, TaskInitSlotSpec, TaskRunContext,
};
use serde_json::json;

fn catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog
        .register(CapabilityTypeContract {
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
        })
        .unwrap();
    catalog
        .register(CapabilityTypeContract {
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
        })
        .unwrap();
    catalog
}

fn compiled_task() -> meld::task::CompiledTaskRecord {
    compile_task_definition(
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
        &catalog(),
    )
    .unwrap()
}

fn init_payload() -> meld::task::TaskInitializationPayload {
    meld::task::TaskInitializationPayload {
        task_id: "task_docs_writer".to_string(),
        compiled_task_ref: "compiled_task_docs_writer".to_string(),
        init_artifacts: vec![InitArtifactValue {
            init_slot_id: "target_selector".to_string(),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
            content: json!({ "path": "docs" }),
        }],
        task_run_context: TaskRunContext {
            task_run_id: "taskrun_1".to_string(),
            session_id: Some("session_1".to_string()),
            trigger: "workflow.execute".to_string(),
        },
    }
}

fn init_payload_with_node() -> meld::task::TaskInitializationPayload {
    meld::task::TaskInitializationPayload {
        task_id: "task_docs_writer".to_string(),
        compiled_task_ref: "compiled_task_docs_writer".to_string(),
        init_artifacts: vec![InitArtifactValue {
            init_slot_id: "target_selector".to_string(),
            artifact_type_id: "target_selector".to_string(),
            schema_version: 1,
            content: json!({ "path": "docs", "node_id": "node_root" }),
        }],
        task_run_context: TaskRunContext {
            task_run_id: "taskrun_1".to_string(),
            session_id: Some("session_1".to_string()),
            trigger: "workflow.execute".to_string(),
        },
    }
}

#[test]
fn task_executor_assembles_payload_and_records_events() {
    let mut executor =
        TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();

    let payloads = executor
        .release_ready_invocations(CapabilityExecutionContext::default())
        .unwrap();

    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].capability_instance_id, "capinst_resolve");
    assert_eq!(payloads[0].supplied_inputs.len(), 1);
    assert!(executor
        .events()
        .iter()
        .any(|event| event.event_type == "task_started"));
    assert!(executor
        .events()
        .iter()
        .any(|event| event.event_type == "task_progressed"));
}

#[test]
fn task_executor_unblocks_downstream_after_artifact_persist() {
    let mut executor =
        TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
    let payloads = executor
        .release_ready_invocations(CapabilityExecutionContext::default())
        .unwrap();

    executor
        .record_success(
            &payloads[0].invocation_id,
            vec![ArtifactRecord {
                artifact_id: "artifact_resolved".to_string(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                content: json!({ "node_id": "node_root" }),
                producer: ArtifactProducerRef {
                    task_id: "task_docs_writer".to_string(),
                    capability_instance_id: "capinst_resolve".to_string(),
                    invocation_id: Some(payloads[0].invocation_id.clone()),
                    output_slot_id: Some("resolved_node_ref".to_string()),
                },
            }],
        )
        .unwrap();

    let next_payloads = executor
        .release_ready_invocations(CapabilityExecutionContext::default())
        .unwrap();

    assert_eq!(next_payloads.len(), 1);
    assert_eq!(next_payloads[0].capability_instance_id, "capinst_traversal");
    assert!(executor
        .events()
        .iter()
        .any(|event| event.event_type == "task_artifact_emitted"));
    assert!(executor
        .events()
        .iter()
        .any(|event| event.event_type == "task_succeeded"));
}

#[test]
fn task_executor_publishes_canonical_events() {
    let mut executor =
        TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();

    let _ = executor
        .release_ready_invocations(CapabilityExecutionContext::default())
        .unwrap();

    let envelopes = executor
        .events()
        .iter()
        .filter_map(|event| build_execution_task_envelope("session_1", event))
        .collect::<Vec<_>>();

    assert!(envelopes
        .iter()
        .any(|event| event.event_type == "execution.task.requested"));
    assert!(envelopes
        .iter()
        .any(|event| event.event_type == "execution.task.started"));
    assert!(envelopes
        .iter()
        .any(|event| event.event_type == "execution.task.progressed"));
    assert!(envelopes.iter().all(|event| event.domain_id == "execution"));
    assert!(envelopes.iter().all(|event| event.stream_id == "taskrun_1"));
    assert!(envelopes
        .iter()
        .any(|event| event.event_type == "execution.task.started"
            && event
                .objects
                .iter()
                .any(|object| object.object_kind == "task_run")));
}

#[test]
fn task_artifact_event_emits_task_and_artifact_refs() {
    let mut executor = TaskExecutor::new(
        compiled_task(),
        init_payload_with_node(),
        "repo_docs_writer",
    )
    .unwrap();
    let payloads = executor
        .release_ready_invocations(CapabilityExecutionContext::default())
        .unwrap();

    executor
        .record_success(
            &payloads[0].invocation_id,
            vec![ArtifactRecord {
                artifact_id: "artifact_resolved".to_string(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
                content: json!({ "node_id": "node_root" }),
                producer: ArtifactProducerRef {
                    task_id: "task_docs_writer".to_string(),
                    capability_instance_id: "capinst_resolve".to_string(),
                    invocation_id: Some(payloads[0].invocation_id.clone()),
                    output_slot_id: Some("resolved_node_ref".to_string()),
                },
            }],
        )
        .unwrap();

    let artifact_event = executor
        .events()
        .iter()
        .find(|event| event.event_type == "task_artifact_emitted")
        .unwrap();
    let envelope = build_execution_task_envelope("session_1", artifact_event).unwrap();

    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "task_run"));
    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "node"));
    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "artifact_slot"));
    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "artifact"));
    assert!(envelope
        .relations
        .iter()
        .any(|relation| relation.relation_type == "targets"));
    assert!(envelope
        .relations
        .iter()
        .any(|relation| relation.relation_type == "attached_to"));
    assert!(envelope
        .relations
        .iter()
        .any(|relation| relation.relation_type == "selected"));
}

#[test]
fn task_run_emits_target_node_ref_from_init_payload() {
    let mut executor = TaskExecutor::new(
        compiled_task(),
        init_payload_with_node(),
        "repo_docs_writer",
    )
    .unwrap();

    let _ = executor
        .release_ready_invocations(CapabilityExecutionContext::default())
        .unwrap();

    let envelope = executor
        .events()
        .iter()
        .filter_map(|event| build_execution_task_envelope("session_1", event))
        .find(|event| event.event_type == "execution.task.progressed")
        .unwrap();

    assert!(envelope
        .objects
        .iter()
        .any(|object| object.domain_id == "workspace_fs" && object.object_kind == "node"));
    assert!(envelope
        .relations
        .iter()
        .any(|relation| relation.relation_type == "targets"));
}
