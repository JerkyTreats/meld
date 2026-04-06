//! Merkle traversal capability publication and invocation.

use crate::api::ContextApi;
use crate::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityInvocationPayload,
    CapabilityInvocationResult, CapabilityInvoker, CapabilityTypeContract, EffectKind, EffectSpec,
    ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec,
    ScopeContract, SuppliedValueRef,
};
use crate::error::ApiError;
use crate::merkle_traversal::expansion::{
    TraversalExpansionNode, TraversalExpansionRelation, TraversalPrerequisiteExpansionContent,
    TraversalPrerequisiteExpansionTemplate,
};
use crate::merkle_traversal::{traverse, TraversalStrategy};
use crate::task::{
    ArtifactProducerRef, ArtifactRecord, TaskExpansionRequest, TaskExpansionTemplate,
    TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID, TRAVERSAL_PREREQUISITE_EXPANSION_KIND,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

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
            "directories_bottom_up" | "DirectoriesBottomUp" => {
                Ok(TraversalStrategy::DirectoriesBottomUp)
            }
            other => Err(ApiError::ConfigError(format!(
                "Unsupported traversal strategy '{}'",
                other
            ))),
        }
    }

    fn parse_expansion_template(
        payload: &CapabilityInvocationPayload,
    ) -> Result<Option<TaskExpansionTemplate>, ApiError> {
        let Some(input) = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "task_expansion_template")
        else {
            return Ok(None);
        };
        let artifact = match &input.value {
            SuppliedValueRef::Artifact(artifact) => artifact,
            SuppliedValueRef::StructuredValue(_) => {
                return Err(ApiError::ConfigError(format!(
                    "Capability invocation '{}' task_expansion_template must be an artifact",
                    payload.invocation_id
                )))
            }
        };
        if artifact.artifact_type_id != TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID {
            return Err(ApiError::ConfigError(format!(
                "Capability invocation '{}' received unexpected expansion template type '{}'",
                payload.invocation_id, artifact.artifact_type_id
            )));
        }
        if artifact.schema_version != TASK_EXPANSION_SCHEMA_VERSION {
            return Err(ApiError::ConfigError(format!(
                "Capability invocation '{}' received unsupported expansion template schema '{}'",
                payload.invocation_id, artifact.schema_version
            )));
        }

        serde_json::from_value(artifact.content.clone())
            .map(Some)
            .map_err(|err| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' failed to decode expansion template: {}",
                    payload.invocation_id, err
                ))
            })
    }

    fn collect_nodes_and_relations(
        api: &ContextApi,
        ordered: &[Vec<[u8; 32]>],
    ) -> Result<
        (
            Vec<Vec<TraversalExpansionNode>>,
            Vec<TraversalExpansionRelation>,
        ),
        ApiError,
    > {
        let traversed = ordered
            .iter()
            .flat_map(|batch| batch.iter().copied())
            .collect::<HashSet<_>>();
        let mut nodes_by_id = BTreeMap::new();

        for batch in ordered {
            for node_id in batch {
                let record = api
                    .node_store()
                    .get(node_id)
                    .map_err(ApiError::from)?
                    .ok_or(ApiError::NodeNotFound(*node_id))?;
                nodes_by_id.insert(
                    *node_id,
                    TraversalExpansionNode {
                        node_id: hex::encode(node_id),
                        path: record.path.to_string_lossy().to_string(),
                    },
                );
            }
        }

        let node_batches = ordered
            .iter()
            .map(|batch| {
                batch
                    .iter()
                    .filter_map(|node_id| nodes_by_id.get(node_id).cloned())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut relations = Vec::new();
        for node_id in nodes_by_id.keys() {
            let record = api
                .node_store()
                .get(node_id)
                .map_err(ApiError::from)?
                .ok_or(ApiError::NodeNotFound(*node_id))?;
            for child in &record.children {
                if traversed.contains(child) {
                    relations.push(TraversalExpansionRelation {
                        upstream_node_id: hex::encode(child),
                        downstream_node_id: hex::encode(node_id),
                    });
                }
            }
        }

        Ok((node_batches, relations))
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
                InputSlotSpec {
                    slot_id: "task_expansion_template".to_string(),
                    accepted_artifact_type_ids: vec![
                        TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID.to_string()
                    ],
                    schema_versions: ArtifactSchemaVersionRange {
                        min: TASK_EXPANSION_SCHEMA_VERSION,
                        max: TASK_EXPANSION_SCHEMA_VERSION,
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
                OutputSlotSpec {
                    slot_id: "task_expansion_request".to_string(),
                    artifact_type_id: TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID.to_string(),
                    schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                    guaranteed: false,
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
        _event_context: Option<&crate::context::queue::QueueEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;

        let node_id = Self::parse_node_id(payload)?;
        let strategy = Self::parse_strategy(runtime_init, payload)?;
        let expansion_template = Self::parse_expansion_template(payload)?;
        let ordered = traverse(api, node_id, strategy)?;
        let (node_batches, relations) = Self::collect_nodes_and_relations(api, ordered.as_slice())?;
        let batches = ordered
            .as_slice()
            .iter()
            .map(|batch| batch.iter().map(hex::encode).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let node_count = batches.iter().map(Vec::len).sum::<usize>();
        let strategy_slug = match strategy {
            TraversalStrategy::BottomUp => "bottom_up",
            TraversalStrategy::TopDown => "top_down",
            TraversalStrategy::DirectoriesBottomUp => "directories_bottom_up",
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
        let mut emitted_artifacts = vec![
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
                    ..producer.clone()
                },
            },
        ];

        if let Some(template) = expansion_template {
            if template.expansion_kind != TRAVERSAL_PREREQUISITE_EXPANSION_KIND {
                return Err(ApiError::ConfigError(format!(
                    "Capability invocation '{}' received unsupported expansion kind '{}'",
                    payload.invocation_id, template.expansion_kind
                )));
            }
            let template_content: TraversalPrerequisiteExpansionTemplate =
                serde_json::from_value(template.content).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Capability invocation '{}' failed to decode traversal prerequisite template: {}",
                        payload.invocation_id, err
                    ))
                })?;
            let expansion_content = serde_json::to_value(TraversalPrerequisiteExpansionContent {
                traversal_strategy: strategy_slug.to_string(),
                node_batches,
                relations,
                repeated_region: template_content.repeated_region,
                prerequisite_template: template_content.prerequisite_template,
                publish: template_content.publish,
            })
            .map_err(|err| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' failed to encode expansion content: {}",
                    payload.invocation_id, err
                ))
            })?;
            emitted_artifacts.push(ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "task_expansion_request"),
                artifact_type_id: TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID.to_string(),
                schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                content: serde_json::to_value(TaskExpansionRequest {
                    expansion_id: format!(
                        "{}::{}",
                        runtime_init.capability_instance_id, TRAVERSAL_PREREQUISITE_EXPANSION_KIND
                    ),
                    expansion_kind: TRAVERSAL_PREREQUISITE_EXPANSION_KIND.to_string(),
                    content: expansion_content,
                })
                .map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Capability invocation '{}' failed to encode task expansion request: {}",
                        payload.invocation_id, err
                    ))
                })?,
                producer: ArtifactProducerRef {
                    output_slot_id: Some("task_expansion_request".to_string()),
                    ..producer
                },
            });
        }

        Ok(CapabilityInvocationResult { emitted_artifacts })
    }
}
