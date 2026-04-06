//! Context capability publication and invocation.

use crate::agent::profile::prompt_contract::PromptContract;
use crate::api::ContextApi;
use crate::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityInvocationPayload,
    CapabilityInvocationResult, CapabilityInvoker, CapabilityTypeContract, EffectKind, EffectSpec,
    ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec,
    ScopeContract, SuppliedValueRef,
};
use crate::context::frame::{Basis, Frame};
use crate::context::generation::contracts::{GenerationOrchestrationRequest, PromptAssemblyOutput};
use crate::context::generation::metadata_construction::{
    build_and_validate_generated_metadata, load_previous_metadata_snapshot,
};
use crate::context::generation::prompt_collection::build_prompt_messages;
use crate::error::ApiError;
use crate::metadata::frame_write_contract::{
    build_generated_metadata, GeneratedFrameMetadataInput,
};
use crate::prompt_context::{prepare_generated_lineage, PromptContextLineageInput};
use crate::provider::{ChatMessage, MessageRole, ProviderExecutionBinding};
use crate::task::{ArtifactProducerRef, ArtifactRecord};
use crate::telemetry::{FrameMetadataValidationEventData, PromptContextLineageEventData};
use crate::workflow::gates::evaluate_gate;
use crate::workflow::profile::WorkflowGate;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

const PREPARE_CAPABILITY_TYPE_ID: &str = "context_generate_prepare";
const FINALIZE_CAPABILITY_TYPE_ID: &str = "context_generate_finalize";
const CAPABILITY_VERSION: u32 = 1;
const ARTIFACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderExecuteRequestArtifact {
    request: GenerationOrchestrationRequest,
    messages: Vec<ChatMessage>,
    request_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreparationSummaryArtifact {
    request: GenerationOrchestrationRequest,
    metadata_input: GeneratedFrameMetadataInput,
    agent_id: String,
    frame_type: String,
    output_type: String,
    turn_id: String,
    workflow_id: Option<String>,
    gate: Option<WorkflowGate>,
    gate_inputs: HashMap<String, String>,
    prompt_output: PromptAssemblyOutput,
}

/// Publishes and invokes the context-side generation preparation capability.
#[derive(Debug, Clone, Default)]
pub struct ContextGeneratePrepareCapability;

/// Publishes and invokes the context-side generation finalization capability.
#[derive(Debug, Clone, Default)]
pub struct ContextGenerateFinalizeCapability;

impl ContextGeneratePrepareCapability {
    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
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
                    "Capability '{}' is missing string binding '{}'",
                    runtime_init.capability_instance_id, binding_id
                ))
            })
    }

    fn bool_binding(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        binding_id: &str,
    ) -> bool {
        runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == binding_id)
            .and_then(|binding| binding.value.as_bool())
            .unwrap_or(false)
    }

    fn json_binding<T: for<'de> Deserialize<'de>>(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        binding_id: &str,
    ) -> Result<T, ApiError> {
        let value = runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == binding_id)
            .map(|binding| binding.value.clone())
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability '{}' is missing binding '{}'",
                    runtime_init.capability_instance_id, binding_id
                ))
            })?;
        serde_json::from_value(value).map_err(|err| {
            ApiError::ConfigError(format!(
                "Capability '{}' failed to decode binding '{}': {}",
                runtime_init.capability_instance_id, binding_id, err
            ))
        })
    }

    fn request_id(invocation_id: &str) -> u64 {
        let digest = blake3::hash(invocation_id.as_bytes());
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest.as_bytes()[..8]);
        u64::from_le_bytes(bytes)
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

    fn supporting_inputs(
        payload: &CapabilityInvocationPayload,
    ) -> Vec<(String, serde_json::Value)> {
        payload
            .supplied_inputs
            .iter()
            .filter(|input| input.slot_id == "upstream_artifact")
            .map(|input| {
                let (artifact_type_id, value) = match &input.value {
                    SuppliedValueRef::Artifact(artifact) => {
                        (artifact.artifact_type_id.clone(), artifact.content.clone())
                    }
                    SuppliedValueRef::StructuredValue(value) => {
                        ("structured_value".to_string(), value.clone())
                    }
                };
                (artifact_type_id, value)
            })
            .collect()
    }

    fn append_supporting_context(
        prompt_output: &mut PromptAssemblyOutput,
        supporting_inputs: &[(String, Value)],
    ) {
        if supporting_inputs.is_empty() {
            return;
        }

        let appended = supporting_inputs
            .iter()
            .map(|(artifact_type_id, value)| {
                let rendered = if let Some(string) = value.as_str() {
                    string.to_string()
                } else {
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
                };
                format!("Type: {artifact_type_id}\nContent:\n{rendered}")
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        prompt_output.context_payload = if prompt_output.context_payload.trim().is_empty() {
            appended
        } else {
            format!("{}\n\n---\n\n{}", prompt_output.context_payload, appended)
        };
        prompt_output.messages = vec![
            ChatMessage {
                role: MessageRole::System,
                content: prompt_output.system_prompt.clone(),
            },
            ChatMessage {
                role: MessageRole::User,
                content: format!(
                    "Context:\n{}\n\nTask: {}",
                    prompt_output.context_payload, prompt_output.rendered_prompt
                ),
            },
        ];
    }
}

#[async_trait]
impl CapabilityInvoker for ContextGeneratePrepareCapability {
    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: PREPARE_CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "context".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![
                BindingSpec {
                    binding_id: "agent_id".to_string(),
                    value_kind: BindingValueKind::AgentRef,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "provider_binding".to_string(),
                    value_kind: BindingValueKind::ProviderRef,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "frame_type".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "prompt_text".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "turn_id".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "workflow_id".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: false,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "output_type".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "gate".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: false,
                    affects_deterministic_identity: true,
                },
            ],
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
                    slot_id: "force_posture".to_string(),
                    accepted_artifact_type_ids: vec!["force_posture".to_string()],
                    schema_versions: ArtifactSchemaVersionRange {
                        min: ARTIFACT_SCHEMA_VERSION,
                        max: ARTIFACT_SCHEMA_VERSION,
                    },
                    required: false,
                    cardinality: InputCardinality::One,
                },
                InputSlotSpec {
                    slot_id: "upstream_artifact".to_string(),
                    accepted_artifact_type_ids: vec![
                        "evidence_map".to_string(),
                        "verification_report".to_string(),
                        "readme_struct".to_string(),
                        "readme_final".to_string(),
                    ],
                    schema_versions: ArtifactSchemaVersionRange {
                        min: ARTIFACT_SCHEMA_VERSION,
                        max: ARTIFACT_SCHEMA_VERSION,
                    },
                    required: false,
                    cardinality: InputCardinality::Many,
                },
            ],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "provider_execute_request".to_string(),
                    artifact_type_id: "provider_execute_request".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "preparation_summary".to_string(),
                    artifact_type_id: "preparation_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "prompt_context_lineage_summary".to_string(),
                    artifact_type_id: "prompt_context_lineage_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
            ],
            effect_contract: vec![EffectSpec {
                effect_id: "read_context".to_string(),
                kind: EffectKind::Read,
                target: "context_view".to_string(),
                exclusive: false,
            }],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifacts".to_string(),
                retry_class: "context_prepare".to_string(),
                cancellation_supported: false,
            },
        }
    }

    async fn invoke(
        &self,
        api: &ContextApi,
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&crate::context::queue::QueueEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;

        let node_id = Self::parse_node_id(payload)?;
        let agent_id = Self::string_binding(runtime_init, "agent_id")?;
        let provider_binding: ProviderExecutionBinding =
            Self::json_binding(runtime_init, "provider_binding")?;
        let frame_type = Self::string_binding(runtime_init, "frame_type")?;
        let prompt_text = Self::string_binding(runtime_init, "prompt_text")?;
        let turn_id = Self::string_binding(runtime_init, "turn_id")?;
        let workflow_id = runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == "workflow_id")
            .and_then(|binding| binding.value.as_str())
            .map(ToString::to_string);
        let output_type = Self::string_binding(runtime_init, "output_type")?;
        let gate = runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == "gate")
            .map(|binding| {
                serde_json::from_value::<WorkflowGate>(binding.value.clone()).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Capability '{}' failed to decode gate binding: {}",
                        runtime_init.capability_instance_id, err
                    ))
                })
            })
            .transpose()?;
        let force = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "force_posture")
            .and_then(|input| match &input.value {
                SuppliedValueRef::Artifact(artifact) => {
                    artifact.content.get("force").and_then(Value::as_bool)
                }
                SuppliedValueRef::StructuredValue(value) => {
                    value.get("force").and_then(Value::as_bool)
                }
            })
            .unwrap_or_else(|| Self::bool_binding(runtime_init, "force"));
        let request = GenerationOrchestrationRequest {
            request_id: Self::request_id(&payload.invocation_id),
            node_id,
            agent_id: agent_id.clone(),
            provider: provider_binding.clone(),
            frame_type: frame_type.clone(),
            retry_count: payload.execution_context.attempt.saturating_sub(1) as usize,
            force,
        };

        let agent = api.get_agent(&agent_id)?;
        let mut prompt_contract = PromptContract::from_agent(&agent)?;
        prompt_contract.user_prompt_file = prompt_text.clone();
        prompt_contract.user_prompt_directory = prompt_text;
        let node_record = api
            .node_store()
            .get(&node_id)
            .map_err(ApiError::from)?
            .ok_or(ApiError::NodeNotFound(node_id))?;
        let mut prompt_output =
            build_prompt_messages(api, &request, &node_record, &prompt_contract)?;
        let supporting_inputs = Self::supporting_inputs(payload);
        Self::append_supporting_context(&mut prompt_output, &supporting_inputs);

        let prepared_lineage = prepare_generated_lineage(
            api.prompt_context_storage(),
            &PromptContextLineageInput {
                system_prompt: prompt_output.system_prompt.clone(),
                user_prompt_template: prompt_output.user_prompt_template.clone(),
                rendered_prompt: prompt_output.rendered_prompt.clone(),
                context_payload: prompt_output.context_payload.clone(),
            },
            &agent_id,
            &provider_binding.provider_name,
            &provider_binding
                .runtime_overrides
                .model_override
                .clone()
                .unwrap_or_else(|| "resolved_at_provider".to_string()),
            "task_capability",
        )?;
        if let Some(ctx) = event_context {
            ctx.progress.emit_event_best_effort(
                &ctx.session_id,
                "prompt_context_lineage_prepared",
                json!(PromptContextLineageEventData {
                    node_id: hex::encode(node_id),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
                    prompt_link_id: prepared_lineage.prompt_link_contract.prompt_link_id.clone(),
                    prompt_digest: prepared_lineage.prompt_link_contract.prompt_digest.clone(),
                    context_digest: prepared_lineage.prompt_link_contract.context_digest.clone(),
                    system_prompt_artifact_id: prepared_lineage
                        .prompt_link_contract
                        .system_prompt_artifact_id
                        .clone(),
                    user_prompt_template_artifact_id: prepared_lineage
                        .prompt_link_contract
                        .user_prompt_template_artifact_id
                        .clone(),
                    rendered_prompt_artifact_id: prepared_lineage
                        .prompt_link_contract
                        .rendered_prompt_artifact_id
                        .clone(),
                    context_artifact_id: prepared_lineage
                        .prompt_link_contract
                        .context_artifact_id
                        .clone(),
                    lineage_failure_policy: "deterministic_orphan_keep".to_string(),
                }),
            );
        }
        let previous_metadata = load_previous_metadata_snapshot(api, &request)?;
        if let Some(ctx) = event_context {
            ctx.progress.emit_event_best_effort(
                &ctx.session_id,
                "frame_metadata_validation_started",
                json!(FrameMetadataValidationEventData {
                    node_id: hex::encode(node_id),
                    path: node_record.path.to_string_lossy().to_string(),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
                    prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
                    context_digest: prepared_lineage.metadata_input.context_digest.clone(),
                    prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
                    previous_frame_id: previous_metadata.frame_id.clone(),
                    previous_prompt_digest: previous_metadata.prompt_digest.clone(),
                    previous_context_digest: previous_metadata.context_digest.clone(),
                    previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
                    workflow_id: workflow_id.clone(),
                    thread_id: None,
                    turn_id: Some(turn_id.clone()),
                    turn_seq: None,
                    attempt: Some(request.retry_count + 1),
                    plan_id: None,
                    level_index: None,
                    error: None,
                }),
            );
        }
        build_and_validate_generated_metadata(
            api,
            &request,
            &prepared_lineage.metadata_input,
            &build_generated_metadata,
        )?;
        if let Some(ctx) = event_context {
            ctx.progress.emit_event_best_effort(
                &ctx.session_id,
                "frame_metadata_validation_succeeded",
                json!(FrameMetadataValidationEventData {
                    node_id: hex::encode(node_id),
                    path: node_record.path.to_string_lossy().to_string(),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
                    prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
                    context_digest: prepared_lineage.metadata_input.context_digest.clone(),
                    prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
                    previous_frame_id: previous_metadata.frame_id.clone(),
                    previous_prompt_digest: previous_metadata.prompt_digest.clone(),
                    previous_context_digest: previous_metadata.context_digest.clone(),
                    previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
                    workflow_id: workflow_id.clone(),
                    thread_id: None,
                    turn_id: Some(turn_id.clone()),
                    turn_seq: None,
                    attempt: Some(request.retry_count + 1),
                    plan_id: None,
                    level_index: None,
                    error: None,
                }),
            );
        }

        let gate_inputs = supporting_inputs
            .into_iter()
            .map(|(artifact_type_id, value)| {
                let rendered = if let Some(string) = value.as_str() {
                    string.to_string()
                } else {
                    serde_json::to_string(&value).unwrap_or_else(|_| value.to_string())
                };
                (artifact_type_id, rendered)
            })
            .collect::<HashMap<_, _>>();
        let summary = PreparationSummaryArtifact {
            request: request.clone(),
            metadata_input: prepared_lineage.metadata_input.clone(),
            agent_id,
            frame_type,
            output_type,
            turn_id,
            workflow_id,
            gate,
            gate_inputs,
            prompt_output: prompt_output.clone(),
        };
        let provider_request = ProviderExecuteRequestArtifact {
            request: request.clone(),
            messages: prompt_output.messages.clone(),
            request_kind: "text_completion".to_string(),
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
                        "provider_execute_request",
                    ),
                    artifact_type_id: "provider_execute_request".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: serde_json::to_value(provider_request).map_err(|err| {
                        ApiError::ConfigError(format!(
                            "Failed to encode provider execute request artifact: {}",
                            err
                        ))
                    })?,
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("provider_execute_request".to_string()),
                        ..producer.clone()
                    },
                },
                ArtifactRecord {
                    artifact_id: Self::artifact_id(&payload.invocation_id, "preparation_summary"),
                    artifact_type_id: "preparation_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: serde_json::to_value(summary).map_err(|err| {
                        ApiError::ConfigError(format!(
                            "Failed to encode preparation summary artifact: {}",
                            err
                        ))
                    })?,
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("preparation_summary".to_string()),
                        ..producer.clone()
                    },
                },
                ArtifactRecord {
                    artifact_id: Self::artifact_id(
                        &payload.invocation_id,
                        "prompt_context_lineage_summary",
                    ),
                    artifact_type_id: "prompt_context_lineage_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "prompt_link_id": prepared_lineage.prompt_link_contract.prompt_link_id,
                        "prompt_digest": prepared_lineage.prompt_link_contract.prompt_digest,
                        "context_digest": prepared_lineage.prompt_link_contract.context_digest,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("prompt_context_lineage_summary".to_string()),
                        ..producer
                    },
                },
            ],
        })
    }
}

impl ContextGenerateFinalizeCapability {
    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }

    fn string_binding(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        binding_id: &str,
    ) -> Result<String, ApiError> {
        ContextGeneratePrepareCapability::string_binding(runtime_init, binding_id)
    }

    fn bool_binding(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        binding_id: &str,
    ) -> bool {
        ContextGeneratePrepareCapability::bool_binding(runtime_init, binding_id)
    }

    fn parse_preparation_summary(
        payload: &CapabilityInvocationPayload,
    ) -> Result<PreparationSummaryArtifact, ApiError> {
        let input = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "preparation_summary")
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' is missing 'preparation_summary'",
                    payload.invocation_id
                ))
            })?;
        let value = match &input.value {
            SuppliedValueRef::Artifact(artifact) => artifact.content.clone(),
            SuppliedValueRef::StructuredValue(value) => value.clone(),
        };
        serde_json::from_value(value).map_err(|err| {
            ApiError::ConfigError(format!(
                "Capability invocation '{}' failed to decode preparation summary: {}",
                payload.invocation_id, err
            ))
        })
    }

    fn parse_provider_result(payload: &CapabilityInvocationPayload) -> Result<Value, ApiError> {
        let input = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "provider_execute_result")
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' is missing 'provider_execute_result'",
                    payload.invocation_id
                ))
            })?;
        Ok(match &input.value {
            SuppliedValueRef::Artifact(artifact) => artifact.content.clone(),
            SuppliedValueRef::StructuredValue(value) => value.clone(),
        })
    }

    fn shaped_output_value(output_type: &str, output_text: &str) -> Result<Value, ApiError> {
        if output_type == "readme_final" {
            return Ok(json!({
                "content": output_text,
                "format": "markdown",
            }));
        }
        let trimmed = output_text.trim();
        let mut candidates = vec![trimmed.to_string()];
        if let Some(fenced) = Self::extract_fenced_block(trimmed) {
            if !candidates.iter().any(|candidate| candidate == fenced) {
                candidates.push(fenced.to_string());
            }
        }
        if let Some(json_slice) = Self::extract_first_json_slice(trimmed) {
            if !candidates.iter().any(|candidate| candidate == json_slice) {
                candidates.push(json_slice.to_string());
            }
        }

        let mut last_error = None;
        for candidate in candidates {
            match serde_json::from_str(&candidate) {
                Ok(value) => return Ok(value),
                Err(err) => last_error = Some(err),
            }
        }

        Err(ApiError::GenerationFailed(format!(
            "Failed to decode '{}' output as JSON: {}",
            output_type,
            last_error
                .map(|err| err.to_string())
                .unwrap_or_else(|| "no JSON content found".to_string())
        )))
    }

    fn extract_fenced_block(output_text: &str) -> Option<&str> {
        let fence_start = output_text.find("```")?;
        let after_start = &output_text[fence_start + 3..];
        let newline_index = after_start.find('\n')?;
        let content_start = fence_start + 3 + newline_index + 1;
        let fence_end = output_text[content_start..].find("```")?;
        Some(output_text[content_start..content_start + fence_end].trim())
    }

    fn extract_first_json_slice(output_text: &str) -> Option<&str> {
        let mut start = None;
        let mut curly_depth = 0usize;
        let mut square_depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;

        for (index, ch) in output_text.char_indices() {
            if start.is_none() {
                match ch {
                    '{' => {
                        start = Some(index);
                        curly_depth = 1;
                    }
                    '[' => {
                        start = Some(index);
                        square_depth = 1;
                    }
                    _ => {}
                }
                continue;
            }

            if escaped {
                escaped = false;
                continue;
            }

            if in_string {
                match ch {
                    '\\' => escaped = true,
                    '"' => in_string = false,
                    _ => {}
                }
                continue;
            }

            match ch {
                '"' => in_string = true,
                '{' => curly_depth += 1,
                '}' => curly_depth = curly_depth.saturating_sub(1),
                '[' => square_depth += 1,
                ']' => square_depth = square_depth.saturating_sub(1),
                _ => {}
            }

            if curly_depth == 0 && square_depth == 0 {
                let start_index = start?;
                let end_index = index + ch.len_utf8();
                return Some(&output_text[start_index..end_index]);
            }
        }

        None
    }
}

/// Returns true if `text` can be decoded as JSON using the same extraction
/// strategies that `ContextGenerateFinalizeCapability` applies at finalize time.
/// Use this in upstream stages to avoid retrying responses that finalize can
/// already handle (e.g. fenced blocks, prose-prefixed JSON).
pub(crate) fn json_output_is_decodable(text: &str) -> bool {
    let trimmed = text.trim();
    let candidates: Vec<&str> = {
        let mut v = vec![trimmed];
        if let Some(fenced) = ContextGenerateFinalizeCapability::extract_fenced_block(trimmed) {
            if !v.contains(&fenced) {
                v.push(fenced);
            }
        }
        if let Some(slice) = ContextGenerateFinalizeCapability::extract_first_json_slice(trimmed) {
            if !v.contains(&slice) {
                v.push(slice);
            }
        }
        v
    };
    candidates
        .iter()
        .any(|c| serde_json::from_str::<serde_json::Value>(c).is_ok())
}

#[async_trait]
impl CapabilityInvoker for ContextGenerateFinalizeCapability {
    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: FINALIZE_CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "context".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![
                BindingSpec {
                    binding_id: "persist_frame".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: false,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "output_type".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: true,
                    affects_deterministic_identity: true,
                },
            ],
            input_contract: vec![
                InputSlotSpec {
                    slot_id: "provider_execute_result".to_string(),
                    accepted_artifact_type_ids: vec!["provider_execute_result".to_string()],
                    schema_versions: ArtifactSchemaVersionRange {
                        min: ARTIFACT_SCHEMA_VERSION,
                        max: ARTIFACT_SCHEMA_VERSION,
                    },
                    required: true,
                    cardinality: InputCardinality::One,
                },
                InputSlotSpec {
                    slot_id: "preparation_summary".to_string(),
                    accepted_artifact_type_ids: vec!["preparation_summary".to_string()],
                    schema_versions: ArtifactSchemaVersionRange {
                        min: ARTIFACT_SCHEMA_VERSION,
                        max: ARTIFACT_SCHEMA_VERSION,
                    },
                    required: true,
                    cardinality: InputCardinality::One,
                },
            ],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "generation_output".to_string(),
                    artifact_type_id: "generation_output".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "frame_ref".to_string(),
                    artifact_type_id: "frame_ref".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: false,
                },
                OutputSlotSpec {
                    slot_id: "effect_summary".to_string(),
                    artifact_type_id: "effect_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: false,
                },
            ],
            effect_contract: Vec::new(),
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifacts".to_string(),
                retry_class: "context_finalize".to_string(),
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

        let output_type = Self::string_binding(runtime_init, "output_type")?;
        let persist_frame = Self::bool_binding(runtime_init, "persist_frame");
        let summary = Self::parse_preparation_summary(payload)?;
        let provider_result = Self::parse_provider_result(payload)?;
        let output_text = provider_result
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ApiError::GenerationFailed(
                    "Provider execute result is missing completion content".to_string(),
                )
            })?;

        if let Some(gate) = summary.gate.as_ref() {
            let gate_result = evaluate_gate(gate, output_text, Some(&summary.gate_inputs));
            if !gate_result.is_pass() && gate.fail_on_violation {
                return Err(ApiError::GenerationFailed(format!(
                    "Workflow gate '{}' failed: {}",
                    gate.gate_id,
                    gate_result.reasons.join(" | ")
                )));
            }
        }

        let output_value = Self::shaped_output_value(&output_type, output_text)?;
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
        let mut emitted_artifacts = vec![ArtifactRecord {
            artifact_id: Self::artifact_id(&payload.invocation_id, "generation_output"),
            artifact_type_id: output_type,
            schema_version: ARTIFACT_SCHEMA_VERSION,
            content: output_value,
            producer: ArtifactProducerRef {
                output_slot_id: Some("generation_output".to_string()),
                ..producer.clone()
            },
        }];

        if persist_frame {
            let metadata = build_and_validate_generated_metadata(
                api,
                &summary.request,
                &summary.metadata_input,
                &build_generated_metadata,
            )?;
            let frame = Frame::new(
                Basis::Node(summary.request.node_id),
                output_text.as_bytes().to_vec(),
                summary.frame_type.clone(),
                summary.agent_id.clone(),
                metadata,
            )?;
            let frame_id =
                api.put_frame(summary.request.node_id, frame, summary.agent_id.clone())?;
            emitted_artifacts.push(ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "frame_ref"),
                artifact_type_id: "frame_ref".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content: json!({
                    "frame_id": hex::encode(frame_id),
                    "node_id": hex::encode(summary.request.node_id),
                    "frame_type": summary.frame_type,
                }),
                producer: ArtifactProducerRef {
                    output_slot_id: Some("frame_ref".to_string()),
                    ..producer.clone()
                },
            });
            emitted_artifacts.push(ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "effect_summary"),
                artifact_type_id: "effect_summary".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content: json!({
                    "writes": [
                        {
                            "effect_target": "frame_store",
                            "kind": "exclusive_write",
                        },
                        {
                            "effect_target": "active_head",
                            "kind": "exclusive_write",
                        }
                    ]
                }),
                producer: ArtifactProducerRef {
                    output_slot_id: Some("effect_summary".to_string()),
                    ..producer
                },
            });
        }

        Ok(CapabilityInvocationResult { emitted_artifacts })
    }
}

#[cfg(test)]
mod tests {
    use super::ContextGenerateFinalizeCapability;

    #[test]
    fn shaped_output_value_accepts_plain_json() {
        let value = ContextGenerateFinalizeCapability::shaped_output_value(
            "evidence_map",
            r#"{"claims":[{"statement":"ok"}]}"#,
        )
        .unwrap();

        assert_eq!(value["claims"][0]["statement"], "ok");
    }

    #[test]
    fn shaped_output_value_accepts_fenced_json() {
        let value = ContextGenerateFinalizeCapability::shaped_output_value(
            "verification_report",
            "```json\n{\"verified_claims\":[{\"statement\":\"ok\"}]}\n```",
        )
        .unwrap();

        assert_eq!(value["verified_claims"][0]["statement"], "ok");
    }

    #[test]
    fn shaped_output_value_accepts_prefixed_json() {
        let value = ContextGenerateFinalizeCapability::shaped_output_value(
            "readme_struct",
            "Here is the JSON you requested.\n{\"title\":\"Doc\",\"purpose\":\"ok\"}",
        )
        .unwrap();

        assert_eq!(value["title"], "Doc");
    }

    #[test]
    fn shaped_output_value_rejects_missing_json() {
        let err = ContextGenerateFinalizeCapability::shaped_output_value(
            "evidence_map",
            "No structured output available.",
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("Failed to decode 'evidence_map' output as JSON"));
    }
}
