use crate::api::ContextApi;
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::provider::{ChatMessage, CompletionResponse, ModelProviderClient, ProviderConfig};
use crate::telemetry::ProviderLifecycleEventData;
use serde_json::json;
use std::time::Instant;
use tracing::info;

pub struct ProviderPreparation {
    pub provider_config: ProviderConfig,
    pub provider_type: String,
    pub client: Box<dyn ModelProviderClient>,
}

pub fn prepare_provider(
    api: &ContextApi,
    provider_name: &str,
) -> Result<ProviderPreparation, ApiError> {
    let provider_registry = api.provider_registry().read();
    let provider_config = provider_registry.get_or_error(provider_name)?.clone();
    let provider_type =
        crate::provider::profile::provider_type_slug(provider_config.provider_type).to_string();
    let client = provider_registry.create_client(provider_name)?;
    drop(provider_registry);

    Ok(ProviderPreparation {
        provider_config,
        provider_type,
        client,
    })
}

pub async fn execute_completion(
    request: &GenerationOrchestrationRequest,
    preparation: &ProviderPreparation,
    messages: Vec<ChatMessage>,
    event_context: Option<&QueueEventContext>,
) -> Result<CompletionResponse, ApiError> {
    let completion_options = preparation.provider_config.default_options.clone();

    let start = Instant::now();
    info!(
        request_id = request.request_id,
        node_id = %hex::encode(request.node_id),
        agent_id = %request.agent_id,
        provider_name = %request.provider_name,
        frame_type = %request.frame_type,
        attempt = request.retry_count + 1,
        message_count = messages.len(),
        "Provider request sent"
    );
    emit_provider_event(
        event_context,
        "provider_request_sent",
        ProviderLifecycleEventData {
            node_id: hex::encode(request.node_id),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            duration_ms: None,
            error: None,
            retry_count: Some(request.retry_count),
        },
    );

    let response = match preparation
        .client
        .complete(messages, completion_options)
        .await
    {
        Ok(r) => Ok(r),
        Err(e) => {
            emit_provider_event(
                event_context,
                "provider_request_failed",
                ProviderLifecycleEventData {
                    node_id: hex::encode(request.node_id),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
                    duration_ms: Some(start.elapsed().as_millis()),
                    error: Some(e.to_string()),
                    retry_count: Some(request.retry_count),
                },
            );

            if let ApiError::ProviderModelNotFound(_) = e {
                match preparation.client.list_models().await {
                    Ok(available_models) => {
                        if available_models.is_empty() {
                            Err(ApiError::ProviderModelNotFound(format!(
                                "Model '{}' not found. Unable to retrieve available models list.",
                                preparation.client.model_name()
                            )))
                        } else {
                            Err(ApiError::ProviderModelNotFound(format!(
                                "Model '{}' not found. Available models: {}",
                                preparation.client.model_name(),
                                available_models.join(", ")
                            )))
                        }
                    }
                    Err(_) => Err(e),
                }
            } else {
                Err(e)
            }
        }
    }?;

    let duration = start.elapsed();
    info!(
        request_id = request.request_id,
        node_id = %hex::encode(request.node_id),
        agent_id = %request.agent_id,
        provider_name = %request.provider_name,
        frame_type = %request.frame_type,
        attempt = request.retry_count + 1,
        duration_ms = duration.as_millis(),
        response_chars = response.content.chars().count(),
        "Provider response received"
    );
    emit_provider_event(
        event_context,
        "provider_response_received",
        ProviderLifecycleEventData {
            node_id: hex::encode(request.node_id),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            duration_ms: Some(duration.as_millis()),
            error: None,
            retry_count: Some(request.retry_count),
        },
    );

    Ok(response)
}

fn emit_provider_event(
    event_context: Option<&QueueEventContext>,
    event_type: &str,
    payload: ProviderLifecycleEventData,
) {
    if let Some(ctx) = event_context {
        ctx.progress
            .emit_event_best_effort(&ctx.session_id, event_type, json!(payload));
    }
}
