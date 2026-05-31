//! Capability runtime initialization and invocation payload records.

use crate::capability::contracts::{
    BoundBindingValue, EffectSpec, ExecutionContract, InputCardinality, InputSlotSpec,
    OutputSlotSpec,
};
use crate::error::ExecutionInvariantError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Structured runtime initialization package for one capability instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityRuntimeInit {
    pub capability_instance_id: String,
    pub capability_type_id: String,
    pub capability_version: u32,
    pub scope_ref: String,
    pub scope_kind: String,
    pub binding_values: Vec<BoundBindingValue>,
    pub input_contract: Vec<InputSlotSpec>,
    pub output_contract: Vec<OutputSlotSpec>,
    pub effect_contract: Vec<EffectSpec>,
    pub execution_contract: ExecutionContract,
}

/// Per-call invocation payload delivered to capability runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityInvocationPayload {
    pub invocation_id: String,
    pub capability_instance_id: String,
    pub supplied_inputs: Vec<SuppliedInputValue>,
    pub upstream_lineage: Option<UpstreamLineage>,
    pub execution_context: CapabilityExecutionContext,
}

impl CapabilityInvocationPayload {
    /// Validates one invocation payload against runtime initialization data.
    pub fn validate_against(
        &self,
        runtime_init: &CapabilityRuntimeInit,
    ) -> Result<(), ExecutionInvariantError> {
        if self.invocation_id.trim().is_empty() {
            return Err(ExecutionInvariantError::ConfigError(
                "Capability invocation_id must not be empty".to_string(),
            ));
        }
        if self.capability_instance_id != runtime_init.capability_instance_id {
            return Err(ExecutionInvariantError::ConfigError(format!(
                "Capability invocation '{}' targets instance '{}' but runtime init is '{}'",
                self.invocation_id,
                self.capability_instance_id,
                runtime_init.capability_instance_id
            )));
        }

        let input_contracts: HashMap<&str, &InputSlotSpec> = runtime_init
            .input_contract
            .iter()
            .map(|slot| (slot.slot_id.as_str(), slot))
            .collect();
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for supplied_input in &self.supplied_inputs {
            let slot = input_contracts
                .get(supplied_input.slot_id.as_str())
                .ok_or_else(|| {
                    ExecutionInvariantError::ConfigError(format!(
                        "Capability invocation '{}' supplies unknown slot '{}'",
                        self.invocation_id, supplied_input.slot_id
                    ))
                })?;
            *counts.entry(slot.slot_id.as_str()).or_insert(0) += 1;
            match &supplied_input.value {
                SuppliedValueRef::Artifact(artifact) => {
                    if !slot
                        .accepted_artifact_type_ids
                        .iter()
                        .any(|artifact_type_id| artifact_type_id == &artifact.artifact_type_id)
                    {
                        return Err(ExecutionInvariantError::ConfigError(format!(
                            "Capability invocation '{}' slot '{}' rejects artifact type '{}'",
                            self.invocation_id, supplied_input.slot_id, artifact.artifact_type_id
                        )));
                    }
                    if !slot.schema_versions.accepts(artifact.schema_version) {
                        return Err(ExecutionInvariantError::ConfigError(format!(
                            "Capability invocation '{}' slot '{}' rejects schema version '{}'",
                            self.invocation_id, supplied_input.slot_id, artifact.schema_version
                        )));
                    }
                }
                SuppliedValueRef::StructuredValue(_) => {}
            }
            if slot.cardinality == InputCardinality::One && counts[slot.slot_id.as_str()] > 1 {
                return Err(ExecutionInvariantError::ConfigError(format!(
                    "Capability invocation '{}' supplied slot '{}' more than once",
                    self.invocation_id, supplied_input.slot_id
                )));
            }
        }

        for slot in &runtime_init.input_contract {
            if slot.required && counts.get(slot.slot_id.as_str()).copied().unwrap_or(0) == 0 {
                return Err(ExecutionInvariantError::ConfigError(format!(
                    "Capability invocation '{}' is missing required slot '{}'",
                    self.invocation_id, slot.slot_id
                )));
            }
        }

        Ok(())
    }
}

/// Source family for one supplied input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputValueSource {
    InitPayload,
    ArtifactHandoff,
}

/// One slot-keyed supplied value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuppliedInputValue {
    pub slot_id: String,
    pub source: InputValueSource,
    pub value: SuppliedValueRef,
}

/// Structured artifact envelope supplied to a capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactValueRef {
    pub artifact_id: String,
    pub artifact_type_id: String,
    pub schema_version: u32,
    pub content: Value,
}

/// Durable structured input value reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SuppliedValueRef {
    StructuredValue(Value),
    Artifact(ArtifactValueRef),
}

/// Optional lineage context supplied by task and control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpstreamLineage {
    pub task_id: String,
    pub task_run_id: String,
    pub capability_path: Vec<String>,
    pub batch_index: Option<usize>,
    pub node_index: Option<usize>,
    pub repair_scope: Option<String>,
}

/// Ephemeral execution metadata for one invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityExecutionContext {
    pub attempt: u32,
    pub trace_id: Option<String>,
    pub deadline_ms: Option<u64>,
    pub cancellation_key: Option<String>,
    pub dispatch_priority: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::contracts::{
        ArtifactSchemaVersionRange, ExecutionClass, InputSlotSpec, OutputSlotSpec,
    };

    type PayloadMutation = Box<dyn FnOnce(&mut CapabilityInvocationPayload)>;

    fn runtime_init() -> CapabilityRuntimeInit {
        CapabilityRuntimeInit {
            capability_instance_id: "capinst_provider_execute_chat".to_string(),
            capability_type_id: "provider_execute_chat".to_string(),
            capability_version: 1,
            scope_ref: "node_a".to_string(),
            scope_kind: "node".to_string(),
            binding_values: Vec::new(),
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
            effect_contract: Vec::new(),
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Queued,
                completion_semantics: "result_or_failure".to_string(),
                retry_class: "provider_io".to_string(),
                cancellation_supported: true,
            },
        }
    }

    fn valid_artifact_value() -> SuppliedValueRef {
        SuppliedValueRef::Artifact(ArtifactValueRef {
            artifact_id: "artifact_request".to_string(),
            artifact_type_id: "provider_execute_request".to_string(),
            schema_version: 1,
            content: Value::Object(Default::default()),
        })
    }

    fn valid_payload(runtime_init: &CapabilityRuntimeInit) -> CapabilityInvocationPayload {
        CapabilityInvocationPayload {
            invocation_id: "invk_1".to_string(),
            capability_instance_id: runtime_init.capability_instance_id.clone(),
            supplied_inputs: vec![SuppliedInputValue {
                slot_id: "provider_request".to_string(),
                source: InputValueSource::ArtifactHandoff,
                value: valid_artifact_value(),
            }],
            upstream_lineage: None,
            execution_context: CapabilityExecutionContext {
                attempt: 1,
                ..CapabilityExecutionContext::default()
            },
        }
    }

    #[test]
    fn invocation_validation_accepts_matching_artifact_input() {
        let runtime_init = runtime_init();
        let payload = valid_payload(&runtime_init);

        payload.validate_against(&runtime_init).unwrap();
    }

    #[test]
    fn invocation_validation_rejects_unknown_slot() {
        let runtime_init = runtime_init();
        let mut payload = valid_payload(&runtime_init);
        payload.supplied_inputs[0].slot_id = "unknown".to_string();

        let error = payload.validate_against(&runtime_init).unwrap_err();

        assert!(matches!(error, ExecutionInvariantError::ConfigError(_)));
        assert!(error.to_string().contains("unknown slot"));
    }

    #[test]
    fn invocation_validation_rejects_missing_required_slot() {
        let runtime_init = runtime_init();
        let mut payload = valid_payload(&runtime_init);
        payload.supplied_inputs.clear();

        let error = payload.validate_against(&runtime_init).unwrap_err();

        assert!(matches!(error, ExecutionInvariantError::ConfigError(_)));
        assert!(error.to_string().contains("missing required slot"));
    }

    #[test]
    fn invocation_validation_rejects_payload_contract_mismatches() {
        let cases: Vec<(&str, PayloadMutation, &str)> = vec![
            (
                "wrong capability instance",
                Box::new(|payload| payload.capability_instance_id = "other".to_string()),
                "targets instance",
            ),
            (
                "artifact schema below range",
                Box::new(|payload| {
                    let SuppliedValueRef::Artifact(artifact) =
                        &mut payload.supplied_inputs[0].value
                    else {
                        unreachable!("fixture uses artifact value");
                    };
                    artifact.schema_version = 0;
                }),
                "rejects schema version",
            ),
            (
                "artifact schema above range",
                Box::new(|payload| {
                    let SuppliedValueRef::Artifact(artifact) =
                        &mut payload.supplied_inputs[0].value
                    else {
                        unreachable!("fixture uses artifact value");
                    };
                    artifact.schema_version = 2;
                }),
                "rejects schema version",
            ),
            (
                "artifact type mismatch",
                Box::new(|payload| {
                    let SuppliedValueRef::Artifact(artifact) =
                        &mut payload.supplied_inputs[0].value
                    else {
                        unreachable!("fixture uses artifact value");
                    };
                    artifact.artifact_type_id = "wrong_type".to_string();
                }),
                "rejects artifact type",
            ),
            (
                "duplicate one value",
                Box::new(|payload| {
                    payload
                        .supplied_inputs
                        .push(payload.supplied_inputs[0].clone())
                }),
                "more than once",
            ),
        ];

        for (case_name, mutate, expected) in cases {
            let runtime_init = runtime_init();
            let mut payload = valid_payload(&runtime_init);
            mutate(&mut payload);

            let error = match payload.validate_against(&runtime_init) {
                Ok(()) => panic!("{case_name} should fail validation"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(expected),
                "{case_name} expected error containing '{expected}', got '{error}'"
            );
        }
    }

    #[test]
    fn invocation_validation_accepts_optional_many_and_structured_values() {
        let mut runtime_init = runtime_init();
        runtime_init.input_contract.push(InputSlotSpec {
            slot_id: "notes".to_string(),
            accepted_artifact_type_ids: vec!["note".to_string()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 2 },
            required: false,
            cardinality: InputCardinality::Many,
        });
        let mut payload = valid_payload(&runtime_init);
        payload.supplied_inputs.push(SuppliedInputValue {
            slot_id: "notes".to_string(),
            source: InputValueSource::InitPayload,
            value: SuppliedValueRef::StructuredValue(serde_json::json!({
                "kind": "operator_note",
                "body": "preserved",
            })),
        });
        payload.supplied_inputs.push(SuppliedInputValue {
            slot_id: "notes".to_string(),
            source: InputValueSource::InitPayload,
            value: SuppliedValueRef::StructuredValue(serde_json::json!({
                "kind": "operator_note",
                "body": "also preserved",
            })),
        });

        payload.validate_against(&runtime_init).unwrap();

        assert_eq!(
            payload.supplied_inputs[1].value,
            SuppliedValueRef::StructuredValue(serde_json::json!({
                "kind": "operator_note",
                "body": "preserved",
            }))
        );
    }
}
