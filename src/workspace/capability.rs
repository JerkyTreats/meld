//! Workspace capability publication and invocation.

use crate::api::ContextApi;
use crate::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityInvocationPayload,
    CapabilityInvocationResult, CapabilityInvoker, CapabilityTypeContract, EffectKind, EffectSpec,
    ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec,
    ScopeContract, SuppliedValueRef,
};
use crate::error::ApiError;
use crate::task::{ArtifactProducerRef, ArtifactRecord};
use crate::workspace::commands::resolve_workspace_node_id;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;

const CAPABILITY_TYPE_ID: &str = "workspace_resolve_node_id";
const CAPABILITY_VERSION: u32 = 1;
const ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// Publishes and invokes the workspace target resolution capability.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceResolveNodeIdCapability;

impl WorkspaceResolveNodeIdCapability {
    fn extract_selector(
        payload: &CapabilityInvocationPayload,
    ) -> Result<(Option<PathBuf>, Option<String>, String), ApiError> {
        let supplied = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "target_selector")
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' is missing 'target_selector'",
                    payload.invocation_id
                ))
            })?;
        let value = match &supplied.value {
            SuppliedValueRef::Artifact(artifact) => &artifact.content,
            SuppliedValueRef::StructuredValue(value) => value,
        };

        if let Some(path) = value.get("path").and_then(Value::as_str) {
            return Ok((Some(PathBuf::from(path)), None, "path".to_string()));
        }
        if let Some(node_id) = value.get("node_id").and_then(Value::as_str) {
            return Ok((None, Some(node_id.to_string()), "node_id".to_string()));
        }

        Err(ApiError::ConfigError(format!(
            "Capability invocation '{}' target_selector must contain 'path' or 'node_id'",
            payload.invocation_id
        )))
    }

    fn include_tombstoned(runtime_init: &crate::capability::CapabilityRuntimeInit) -> bool {
        runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == "include_tombstoned")
            .and_then(|binding| binding.value.as_bool())
            .unwrap_or(false)
    }

    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }
}

#[async_trait]
impl CapabilityInvoker for WorkspaceResolveNodeIdCapability {
    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "workspace".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "workspace".to_string(),
                scope_ref_kind: "workspace_root".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![BindingSpec {
                binding_id: "include_tombstoned".to_string(),
                value_kind: BindingValueKind::Literal,
                required: false,
                affects_deterministic_identity: true,
            }],
            input_contract: vec![InputSlotSpec {
                slot_id: "target_selector".to_string(),
                accepted_artifact_type_ids: vec!["target_selector".to_string()],
                schema_versions: ArtifactSchemaVersionRange {
                    min: ARTIFACT_SCHEMA_VERSION,
                    max: ARTIFACT_SCHEMA_VERSION,
                },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "resolved_node_ref".to_string(),
                    artifact_type_id: "resolved_node_ref".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "target_resolution_summary".to_string(),
                    artifact_type_id: "target_resolution_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
            ],
            effect_contract: vec![EffectSpec {
                effect_id: "read_workspace_tree".to_string(),
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

        let workspace_root = api.workspace_root().ok_or_else(|| {
            ApiError::ConfigError(
                "Workspace resolve capability requires workspace root context".to_string(),
            )
        })?;
        let include_tombstoned = Self::include_tombstoned(runtime_init);
        let (path, node_hex, selector_kind) = Self::extract_selector(payload)?;
        let node_id = resolve_workspace_node_id(
            api,
            workspace_root,
            path.as_deref(),
            node_hex.as_deref(),
            include_tombstoned,
        )?;
        let record = api
            .node_store()
            .get(&node_id)
            .map_err(ApiError::from)?
            .ok_or(ApiError::NodeNotFound(node_id))?;

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
                    artifact_id: Self::artifact_id(&payload.invocation_id, "resolved_node_ref"),
                    artifact_type_id: "resolved_node_ref".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "node_id": hex::encode(node_id),
                        "path": record.path.to_string_lossy(),
                        "include_tombstoned": include_tombstoned,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("resolved_node_ref".to_string()),
                        ..producer.clone()
                    },
                },
                ArtifactRecord {
                    artifact_id: Self::artifact_id(
                        &payload.invocation_id,
                        "target_resolution_summary",
                    ),
                    artifact_type_id: "target_resolution_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "selector_kind": selector_kind,
                        "lookup_mode": "workspace_path_then_canonical_fallback",
                        "resolved": true,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("target_resolution_summary".to_string()),
                        ..producer
                    },
                },
            ],
        })
    }
}
