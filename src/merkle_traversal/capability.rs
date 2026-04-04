//! Merkle traversal capability publication and invocation.

use crate::api::ContextApi;
use crate::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityInvocationPayload,
    CapabilityInvocationResult, CapabilityInvoker, CapabilityTypeContract, EffectKind, EffectSpec,
    ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec,
    ScopeContract, SuppliedValueRef,
};
use crate::error::ApiError;
use crate::merkle_traversal::{traverse, TraversalStrategy};
use crate::task::{ArtifactProducerRef, ArtifactRecord};
use async_trait::async_trait;
use serde_json::{json, Value};

const CAPABILITY_TYPE_ID: &str = "merkle_traversal";
const CAPABILITY_VERSION: u32 = 1;
const ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// Publishes and invokes the Merkle traversal capability.
#[derive(Debug, Clone, Default)]
pub struct MerkleTraversalCapability;

impl MerkleTraversalCapability {
    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }

    fn parse_node_id(payload: &CapabilityInvocationPayload) -> Result<[u8; 32], ApiError> {
        let input = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "resolved_node_ref")
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' is missing 'resolved_node_ref'",
                    payload.invocation_id
                ))
            })?;
        let value = match &input.value {
            SuppliedValueRef::Artifact(artifact) => &artifact.content,
            SuppliedValueRef::StructuredValue(value) => value,
        };
        let node_hex = value
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' resolved_node_ref is missing 'node_id'",
                    payload.invocation_id
                ))
            })?;
        let bytes = hex::decode(node_hex).map_err(|err| {
            ApiError::ConfigError(format!("Invalid node hex '{}': {}", node_hex, err))
        })?;
        if bytes.len() != 32 {
            return Err(ApiError::ConfigError(format!(
                "Invalid node hex '{}' length '{}'",
                node_hex,
                bytes.len()
            )));
        }
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&bytes);
        Ok(node_id)
    }

    fn parse_strategy(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
    ) -> Result<TraversalStrategy, ApiError> {
        if let Some(input) = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "traversal_strategy")
        {
            let value = match &input.value {
                SuppliedValueRef::Artifact(artifact) => &artifact.content,
                SuppliedValueRef::StructuredValue(value) => value,
            };
            if let Some(strategy) = value.get("strategy").and_then(Value::as_str) {
                return Self::strategy_from_str(strategy);
            }
        }

        runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == "strategy")
            .and_then(|binding| binding.value.as_str())
            .map(Self::strategy_from_str)
            .transpose()?
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' is missing traversal strategy",
                    payload.invocation_id
                ))
            })
    }

    fn strategy_from_str(value: &str) -> Result<TraversalStrategy, ApiError> {
        match value {
            "bottom_up" | "BottomUp" => Ok(TraversalStrategy::BottomUp),
            "top_down" | "TopDown" => Ok(TraversalStrategy::TopDown),
            other => Err(ApiError::ConfigError(format!(
                "Unsupported traversal strategy '{}'",
                other
            ))),
        }
    }
}

#[async_trait]
impl CapabilityInvoker for MerkleTraversalCapability {
    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "merkle_traversal".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: true,
            },
            binding_contract: vec![BindingSpec {
                binding_id: "strategy".to_string(),
                value_kind: BindingValueKind::Literal,
                required: false,
                affects_deterministic_identity: true,
            }],
            input_contract: vec![
                InputSlotSpec {
                    slot_id: "resolved_node_ref".to_string(),
                    accepted_artifact_type_ids: vec!["resolved_node_ref".to_string()],
                    schema_versions: ArtifactSchemaVersionRange {
                        min: ARTIFACT_SCHEMA_VERSION,
                        max: ARTIFACT_SCHEMA_VERSION,
                    },
                    required: true,
                    cardinality: InputCardinality::One,
                },
                InputSlotSpec {
                    slot_id: "traversal_strategy".to_string(),
                    accepted_artifact_type_ids: vec!["traversal_strategy".to_string()],
                    schema_versions: ArtifactSchemaVersionRange {
                        min: ARTIFACT_SCHEMA_VERSION,
                        max: ARTIFACT_SCHEMA_VERSION,
                    },
                    required: false,
                    cardinality: InputCardinality::One,
                },
            ],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "ordered_merkle_node_batches".to_string(),
                    artifact_type_id: "ordered_merkle_node_batches".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "traversal_metadata".to_string(),
                    artifact_type_id: "traversal_metadata".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
            ],
            effect_contract: vec![EffectSpec {
                effect_id: "read_tree".to_string(),
                kind: EffectKind::Read,
                target: "workspace_tree".to_string(),
                exclusive: false,
            }],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifacts".to_string(),
                retry_class: "none".to_string(),
                cancellation_supported: false,
            },
        }
    }

    async fn invoke(
        &self,
        api: &ContextApi,
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;

        let node_id = Self::parse_node_id(payload)?;
        let strategy = Self::parse_strategy(runtime_init, payload)?;
        let ordered = traverse(api, node_id, strategy)?;
        let batches = ordered
            .as_slice()
            .iter()
            .map(|batch| batch.iter().map(hex::encode).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let node_count = batches.iter().map(Vec::len).sum::<usize>();
        let strategy_slug = match strategy {
            TraversalStrategy::BottomUp => "bottom_up",
            TraversalStrategy::TopDown => "top_down",
        };
        let producer = ArtifactProducerRef {
            task_id: payload
                .upstream_lineage
                .as_ref()
                .map(|lineage| lineage.task_id.clone())
                .unwrap_or_default(),
            capability_instance_id: runtime_init.capability_instance_id.clone(),
            invocation_id: Some(payload.invocation_id.clone()),
            output_slot_id: None,
        };

        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![
                ArtifactRecord {
                    artifact_id: Self::artifact_id(
                        &payload.invocation_id,
                        "ordered_merkle_node_batches",
                    ),
                    artifact_type_id: "ordered_merkle_node_batches".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "strategy": strategy_slug,
                        "batches": batches,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("ordered_merkle_node_batches".to_string()),
                        ..producer.clone()
                    },
                },
                ArtifactRecord {
                    artifact_id: Self::artifact_id(&payload.invocation_id, "traversal_metadata"),
                    artifact_type_id: "traversal_metadata".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "root_node_id": hex::encode(node_id),
                        "batch_count": ordered.as_slice().len(),
                        "node_count": node_count,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("traversal_metadata".to_string()),
                        ..producer
                    },
                },
            ],
        })
    }
}
