//! Provider capability publication and invocation.

use crate::api::ContextApi;
use crate::capability::{
    ArtifactSchemaVersionRange, CapabilityInvocationPayload, CapabilityInvocationResult,
    CapabilityInvoker, CapabilityTypeContract, EffectKind, EffectSpec, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
    SuppliedValueRef,
};
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::error::ApiError;
use crate::provider::executor::{execute_completion, prepare_provider_for_request};
use crate::provider::ChatMessage;
use crate::task::{ArtifactProducerRef, ArtifactRecord};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

const CAPABILITY_TYPE_ID: &str = "provider_execute_chat";
const CAPABILITY_VERSION: u32 = 1;
const ARTIFACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderExecuteRequestArtifact {
    request: GenerationOrchestrationRequest,
    messages: Vec<ChatMessage>,
    request_kind: String,
}

/// Publishes and invokes the provider execution capability.
#[derive(Debug, Clone, Default)]
pub struct ProviderExecuteChatCapability;

impl ProviderExecuteChatCapability {
    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }

    fn parse_request(
        payload: &CapabilityInvocationPayload,
    ) -> Result<ProviderExecuteRequestArtifact, ApiError> {
        let input = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "provider_execute_request")
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' is missing 'provider_execute_request'",
                    payload.invocation_id
                ))
            })?;
        let value = match &input.value {
            SuppliedValueRef::Artifact(artifact) => artifact.content.clone(),
            SuppliedValueRef::StructuredValue(value) => value.clone(),
        };
        serde_json::from_value(value).map_err(|err| {
            ApiError::ConfigError(format!(
                "Capability invocation '{}' provider_execute_request decode failed: {}",
                payload.invocation_id, err
            ))
        })
    }
}

#[async_trait]
impl CapabilityInvoker for ProviderExecuteChatCapability {
    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "provider".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![InputSlotSpec {
                slot_id: "provider_execute_request".to_string(),
                accepted_artifact_type_ids: vec!["provider_execute_request".to_string()],
                schema_versions: ArtifactSchemaVersionRange {
                    min: ARTIFACT_SCHEMA_VERSION,
                    max: ARTIFACT_SCHEMA_VERSION,
                },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "provider_execute_result".to_string(),
                    artifact_type_id: "provider_execute_result".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "provider_usage_summary".to_string(),
                    artifact_type_id: "provider_usage_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "provider_timing_summary".to_string(),
                    artifact_type_id: "provider_timing_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
            ],
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

    async fn invoke(
        &self,
        api: &ContextApi,
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;

        let request_artifact = Self::parse_request(payload)?;
        let preparation = prepare_provider_for_request(api, &request_artifact.request)?;
        let started = Instant::now();
        let response = execute_completion(
            &request_artifact.request,
            &preparation,
            request_artifact.messages,
            None,
        )
        .await?;
        let duration_ms = started.elapsed().as_millis();
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
                        "provider_execute_result",
                    ),
                    artifact_type_id: "provider_execute_result".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "provider_name": request_artifact.request.provider.provider_name,
                        "model": response.model,
                        "finish_reason": response.finish_reason,
                        "content": response.content,
                        "normalized_status": "succeeded",
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("provider_execute_result".to_string()),
                        ..producer.clone()
                    },
                },
                ArtifactRecord {
                    artifact_id: Self::artifact_id(
                        &payload.invocation_id,
                        "provider_usage_summary",
                    ),
                    artifact_type_id: "provider_usage_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "prompt_tokens": response.usage.prompt_tokens,
                        "completion_tokens": response.usage.completion_tokens,
                        "total_tokens": response.usage.total_tokens,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("provider_usage_summary".to_string()),
                        ..producer.clone()
                    },
                },
                ArtifactRecord {
                    artifact_id: Self::artifact_id(
                        &payload.invocation_id,
                        "provider_timing_summary",
                    ),
                    artifact_type_id: "provider_timing_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "duration_ms": duration_ms,
                        "lane_id": format!(
                            "{}:{}:{}",
                            request_artifact.request.provider.provider_name,
                            preparation.client.model_name(),
                            request_artifact.request_kind,
                        ),
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("provider_timing_summary".to_string()),
                        ..producer
                    },
                },
            ],
        })
    }
}
