use meld::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, BoundBindingValue,
    BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityCatalog,
    CapabilityExecutionContext, CapabilityInvocationPayload, CapabilityRuntimeInit,
    CapabilityTypeContract, EffectKind, EffectSpec, ExecutionClass, ExecutionContract,
    InputCardinality, InputSlotSpec, InputValueSource, OutputSlotSpec, ScopeContract,
    SuppliedInputValue, SuppliedValueRef,
};
use serde_json::json;

fn provider_execute_chat_contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: "provider_execute_chat".to_string(),
        capability_version: 1,
        owning_domain: "provider".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "node".to_string(),
            scope_ref_kind: "node_id".to_string(),
            allow_fan_out: false,
        },
        binding_contract: vec![BindingSpec {
            binding_id: "provider".to_string(),
            value_kind: BindingValueKind::ProviderRef,
            required: true,
            affects_deterministic_identity: true,
        }],
        input_contract: vec![InputSlotSpec {
            slot_id: "provider_request".to_string(),
            accepted_artifact_type_ids: vec!["provider_execute_request".to_string()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
            required: true,
            cardinality: InputCardinality::One,
        }],
        output_contract: vec![OutputSlotSpec {
            slot_id: "provider_result".to_string(),
            artifact_type_id: "provider_execute_result".to_string(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![EffectSpec {
            effect_id: "provider_transport".to_string(),
            kind: EffectKind::Emit,
            target: "provider_service".to_string(),
            exclusive: false,
        }],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Queued,
            completion_semantics: "result_or_failure".to_string(),
            retry_class: "provider_io".to_string(),
            cancellation_supported: true,
        },
    }
}

#[test]
fn capability_catalog_registers_and_retrieves_contracts() {
    let mut catalog = CapabilityCatalog::new();
    let contract = provider_execute_chat_contract();

    catalog.register(contract.clone()).unwrap();

    assert!(catalog.contains("provider_execute_chat", 1));
    assert_eq!(catalog.get("provider_execute_chat", 1), Some(&contract));
}

#[test]
fn bound_capability_instance_validates_against_registered_contract() {
    let contract = provider_execute_chat_contract();
    let instance = BoundCapabilityInstance {
        capability_instance_id: "capinst_provider_execute_chat".to_string(),
        capability_type_id: contract.capability_type_id.clone(),
        capability_version: contract.capability_version,
        scope_ref: "node_abc".to_string(),
        scope_kind: contract.scope_contract.scope_kind.clone(),
        binding_values: vec![BoundBindingValue {
            binding_id: "provider".to_string(),
            value: json!("provider-test"),
        }],
        input_wiring: vec![BoundInputWiring {
            slot_id: "provider_request".to_string(),
            sources: vec![BoundInputWiringSource::TaskInitSlot {
                init_slot_id: "request".to_string(),
                artifact_type_id: "provider_execute_request".to_string(),
                schema_version: 1,
            }],
        }],
    };

    instance.validate_against(&contract).unwrap();
}

#[test]
fn invocation_payload_validates_against_runtime_init() {
    let contract = provider_execute_chat_contract();
    let runtime_init = CapabilityRuntimeInit {
        capability_instance_id: "capinst_provider_execute_chat".to_string(),
        capability_type_id: contract.capability_type_id.clone(),
        capability_version: contract.capability_version,
        scope_ref: "node_abc".to_string(),
        scope_kind: contract.scope_contract.scope_kind.clone(),
        binding_values: vec![BoundBindingValue {
            binding_id: "provider".to_string(),
            value: json!("provider-test"),
        }],
        input_contract: contract.input_contract.clone(),
        output_contract: contract.output_contract.clone(),
        effect_contract: contract.effect_contract.clone(),
        execution_contract: contract.execution_contract.clone(),
    };
    let payload = CapabilityInvocationPayload {
        invocation_id: "invk_provider_execute_chat".to_string(),
        capability_instance_id: runtime_init.capability_instance_id.clone(),
        supplied_inputs: vec![SuppliedInputValue {
            slot_id: "provider_request".to_string(),
            source: InputValueSource::ArtifactHandoff,
            value: SuppliedValueRef::Artifact(meld::capability::ArtifactValueRef {
                artifact_id: "artifact_request".to_string(),
                artifact_type_id: "provider_execute_request".to_string(),
                schema_version: 1,
                content: json!({
                    "messages": [{"role": "user", "content": "hello"}],
                }),
            }),
        }],
        upstream_lineage: None,
        execution_context: CapabilityExecutionContext {
            attempt: 1,
            trace_id: Some("trace_1".to_string()),
            deadline_ms: Some(30_000),
            cancellation_key: Some("cancel_1".to_string()),
            dispatch_priority: Some("normal".to_string()),
        },
    };

    payload.validate_against(&runtime_init).unwrap();
}
