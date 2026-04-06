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
use crate::task::{
    TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND,
};
use crate::workspace::commands::resolve_workspace_node_id;
use crate::workspace::publish::{
    decode_node_id, evaluate_publish_target, node_from_ref_content, publish_output_path,
    record_published_head, validate_directory_node, FrameHeadWriteExpansionContent,
    PublishFilterDecision,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const RESOLVE_CAPABILITY_TYPE_ID: &str = "workspace_resolve_node_id";
const FILTER_PUBLISH_CAPABILITY_TYPE_ID: &str = "workspace_filter_frame_head_publish";
const WRITE_FRAME_HEAD_CAPABILITY_TYPE_ID: &str = "workspace_write_frame_head";
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
            capability_type_id: RESOLVE_CAPABILITY_TYPE_ID.to_string(),
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

/// Plans per-node frame-head publish work and emits write-task expansions only for actionable targets.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceFilterFrameHeadPublishCapability;

impl WorkspaceFilterFrameHeadPublishCapability {
    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }

    fn parse_node_ref(payload: &CapabilityInvocationPayload) -> Result<(String, PathBuf), ApiError> {
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
        let node = node_from_ref_content(value)?;
        Ok((node.node_id, PathBuf::from(node.path)))
    }

    fn string_binding(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        binding_id: &str,
    ) -> Result<String, ApiError> {
        runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == binding_id)
            .and_then(|binding| binding.value.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability '{}' is missing required binding '{}'",
                    FILTER_PUBLISH_CAPABILITY_TYPE_ID, binding_id
                ))
            })
    }
}

#[async_trait]
impl CapabilityInvoker for WorkspaceFilterFrameHeadPublishCapability {
    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: FILTER_PUBLISH_CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "workspace".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![
                BindingSpec {
                    binding_id: "frame_type".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "file_name".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "publish_strategy".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
            ],
            input_contract: vec![InputSlotSpec {
                slot_id: "resolved_node_ref".to_string(),
                accepted_artifact_type_ids: vec!["resolved_node_ref".to_string()],
                schema_versions: ArtifactSchemaVersionRange {
                    min: ARTIFACT_SCHEMA_VERSION,
                    max: ARTIFACT_SCHEMA_VERSION,
                },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "publish_filter_result".to_string(),
                    artifact_type_id: "publish_filter_result".to_string(),
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
            effect_contract: vec![
                EffectSpec {
                    effect_id: "read_workspace_tree".to_string(),
                    kind: EffectKind::Read,
                    target: "workspace_tree".to_string(),
                    exclusive: false,
                },
                EffectSpec {
                    effect_id: "read_frame_heads".to_string(),
                    kind: EffectKind::Read,
                    target: "frame_head".to_string(),
                    exclusive: false,
                },
            ],
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

        let (node_hex, node_path) = Self::parse_node_ref(payload)?;
        let node_id = decode_node_id(&node_hex)?;
        validate_directory_node(api, &node_id)?;
        let frame_type = Self::string_binding(runtime_init, "frame_type")?;
        let file_name = Self::string_binding(runtime_init, "file_name")?;
        let publish_strategy = Self::string_binding(runtime_init, "publish_strategy")?;
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

        let decision = evaluate_publish_target(
            api,
            node_id,
            &node_path,
            &frame_type,
            &file_name,
            &publish_strategy,
        )?;

        let mut emitted_artifacts = vec![ArtifactRecord {
            artifact_id: Self::artifact_id(&payload.invocation_id, "publish_filter_result"),
            artifact_type_id: "publish_filter_result".to_string(),
            schema_version: ARTIFACT_SCHEMA_VERSION,
            content: match &decision {
                PublishFilterDecision::MissingHead => json!({
                    "status": "missing_head",
                    "node_id": node_hex.clone(),
                    "path": node_path.to_string_lossy().to_string(),
                    "frame_type": frame_type.clone(),
                    "file_name": file_name.clone(),
                    "publish_strategy": publish_strategy.clone(),
                }),
                PublishFilterDecision::SkipCurrentHeadAlreadyPublished => json!({
                    "status": "skipped_up_to_date",
                    "node_id": node_hex.clone(),
                    "path": node_path.to_string_lossy().to_string(),
                    "frame_type": frame_type.clone(),
                    "file_name": file_name.clone(),
                    "publish_strategy": publish_strategy.clone(),
                }),
                PublishFilterDecision::WriteCurrentHead {
                    frame_id,
                    output_path,
                    file_missing,
                } => json!({
                    "status": "actionable",
                    "node_id": node_hex.clone(),
                    "path": node_path.to_string_lossy().to_string(),
                    "frame_id": hex::encode(frame_id),
                    "output_path": output_path.to_string_lossy().to_string(),
                    "file_missing": file_missing,
                    "frame_type": frame_type.clone(),
                    "file_name": file_name.clone(),
                    "publish_strategy": publish_strategy.clone(),
                }),
            },
            producer: ArtifactProducerRef {
                output_slot_id: Some("publish_filter_result".to_string()),
                ..producer.clone()
            },
        }];

        if matches!(decision, PublishFilterDecision::WriteCurrentHead { .. }) {
            emitted_artifacts.push(ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "task_expansion_request"),
                artifact_type_id: TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID.to_string(),
                schema_version: TASK_EXPANSION_SCHEMA_VERSION,
                content: serde_json::to_value(crate::task::TaskExpansionRequest {
                    expansion_id: format!(
                        "{}::{}",
                        runtime_init.capability_instance_id, WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND
                    ),
                    expansion_kind: WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND.to_string(),
                    content: serde_json::to_value(FrameHeadWriteExpansionContent {
                        node_id: node_hex.clone(),
                        path: node_path.to_string_lossy().to_string(),
                        frame_type: frame_type.clone(),
                        file_name: file_name.clone(),
                    })
                    .map_err(|err| {
                        ApiError::ConfigError(format!(
                            "Capability invocation '{}' failed to encode publish expansion: {}",
                            payload.invocation_id, err
                        ))
                    })?,
                })
                .map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Capability invocation '{}' failed to encode expansion request: {}",
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

/// Writes the latest frame head for one node to a workspace file.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceWriteFrameHeadCapability;

impl WorkspaceWriteFrameHeadCapability {
    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }

    fn parse_node_ref(payload: &CapabilityInvocationPayload) -> Result<(String, PathBuf), ApiError> {
        WorkspaceFilterFrameHeadPublishCapability::parse_node_ref(payload)
    }

    fn string_binding(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        binding_id: &str,
    ) -> Result<String, ApiError> {
        runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == binding_id)
            .and_then(|binding| binding.value.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability '{}' is missing required binding '{}'",
                    WRITE_FRAME_HEAD_CAPABILITY_TYPE_ID, binding_id
                ))
            })
    }
}

#[async_trait]
impl CapabilityInvoker for WorkspaceWriteFrameHeadCapability {
    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: WRITE_FRAME_HEAD_CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "workspace".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![
                BindingSpec {
                    binding_id: "frame_type".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "file_name".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
            ],
            input_contract: vec![InputSlotSpec {
                slot_id: "resolved_node_ref".to_string(),
                accepted_artifact_type_ids: vec!["resolved_node_ref".to_string()],
                schema_versions: ArtifactSchemaVersionRange {
                    min: ARTIFACT_SCHEMA_VERSION,
                    max: ARTIFACT_SCHEMA_VERSION,
                },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "publish_result".to_string(),
                artifact_type_id: "publish_result".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                guaranteed: true,
            }],
            effect_contract: vec![EffectSpec {
                effect_id: "write_workspace_file".to_string(),
                kind: EffectKind::Write,
                target: "workspace_tree".to_string(),
                exclusive: true,
            }],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifacts".to_string(),
                retry_class: "workspace_io".to_string(),
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

        let (node_hex, node_path) = Self::parse_node_ref(payload)?;
        let node_id = decode_node_id(&node_hex)?;
        validate_directory_node(api, &node_id)?;
        let frame_type = Self::string_binding(runtime_init, "frame_type")?;
        let file_name = Self::string_binding(runtime_init, "file_name")?;
        let Some(workspace_root) = api.workspace_root() else {
            return Err(ApiError::ConfigError(
                "Publish capability requires workspace root context".to_string(),
            ));
        };
        let output_path = publish_output_path(workspace_root, &node_path, &file_name);
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

        let content = match api.get_head(&node_id, &frame_type)? {
            Some(frame_id) => {
                let frame = api
                    .frame_storage()
                    .get(&frame_id)
                    .map_err(ApiError::from)?
                    .ok_or_else(|| {
                        ApiError::ConfigError(format!(
                            "Missing frame '{}' for publish target '{}'",
                            hex::encode(frame_id),
                            node_hex
                        ))
                    })?;
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent).map_err(|err| {
                        ApiError::StorageError(crate::error::StorageError::IoError(
                            std::io::Error::other(format!(
                                "Failed to create publish parent '{}' : {}",
                                parent.display(),
                                err
                            )),
                        ))
                    })?;
                }
                fs::write(&output_path, &frame.content).map_err(|err| {
                    ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                        format!("Failed to write publish file '{}' : {}", output_path.display(), err),
                    )))
                })?;
                record_published_head(workspace_root, &output_path, frame_id)?;
                json!({
                    "status": "written",
                    "node_id": node_hex,
                    "frame_id": hex::encode(frame_id),
                    "output_path": output_path.to_string_lossy(),
                })
            }
            None => json!({
                "status": "missing_head",
                "node_id": node_hex,
                "output_path": output_path.to_string_lossy(),
            }),
        };

        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "publish_result"),
                artifact_type_id: "publish_result".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content,
                producer: ArtifactProducerRef {
                    output_slot_id: Some("publish_result".to_string()),
                    ..producer
                },
            }],
        })
    }
}
