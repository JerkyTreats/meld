use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use super::super::{
    provider::{CallbackProviderClient, OwnerProviderCallbacks},
    NoOwnerCallbacks,
};
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::error::ApiError;
use crate::execution::ExecutionEventContext;
use crate::provider::{
    ChatMessage, CompletionResponse, ProviderCompletion, ProviderCompletionPort,
    ProviderExecutionBinding, ProviderExecutionDescription, ProviderRuntimeOverrides, TokenUsage,
};
use async_trait::async_trait;

#[derive(Default)]
struct RecordingCompletion {
    contexts: Mutex<Vec<Option<ExecutionEventContext>>>,
}

#[async_trait]
impl ProviderCompletionPort for RecordingCompletion {
    async fn complete_provider_request(
        &self,
        _: &GenerationOrchestrationRequest,
        _: Vec<ChatMessage>,
        context: Option<&ExecutionEventContext>,
    ) -> Result<ProviderCompletion, ApiError> {
        self.contexts.lock().unwrap().push(context.cloned());
        Ok(ProviderCompletion {
            preparation: ProviderExecutionDescription {
                provider_type: "local".into(),
                requested_model: "selected-model".into(),
                configuration_identity: "selected-profile".into(),
            },
            response: CompletionResponse {
                content: "owner-result".into(),
                model: "local".into(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 2,
                    total_tokens: 3,
                },
                finish_reason: Some("stop".into()),
            },
        })
    }
}

#[test]
fn provider_callbacks_keep_the_native_selection_context_and_provenance() {
    let native = Arc::new(RecordingCompletion::default());
    let binding = ProviderExecutionBinding::new(
        "selected-provider",
        ProviderRuntimeOverrides {
            extra_body_fields: std::collections::BTreeMap::from([(
                "top_k".into(),
                serde_json::json!(0),
            )]),
            ..Default::default()
        },
    )
    .unwrap();
    let context = ExecutionEventContext {
        session_id: "native-session".into(),
        effect_authority: None,
    };
    let callbacks = OwnerProviderCallbacks::new(
        Arc::new(NoOwnerCallbacks),
        native.clone(),
        binding.clone(),
        "assigned-agent".into(),
        BTreeSet::from(["owner-judgment".into()]),
        Some(context.clone()),
    )
    .unwrap();
    let client = CallbackProviderClient::new(Arc::new(callbacks));
    let mut request = GenerationOrchestrationRequest {
        request_id: 1,
        node_id: [0; 32],
        agent_id: "assigned-agent".into(),
        provider: binding,
        frame_type: "owner-judgment".into(),
        retry_count: 0,
        force: false,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    request.provider.runtime_overrides.extra_body_fields.insert(
        "response_format".into(),
        serde_json::json!({"type":"json_schema","json_schema":{"name":"owner_result"}}),
    );
    let completed = runtime
        .block_on(client.complete_provider_request(&request, vec![], None))
        .unwrap();
    assert_eq!(
        completed.preparation.configuration_identity,
        "selected-profile"
    );
    assert_eq!(completed.preparation.requested_model, "selected-model");
    assert_eq!(completed.response.content, "owner-result");
    assert_eq!(*native.contexts.lock().unwrap(), vec![Some(context)]);
    let forged = ExecutionEventContext {
        session_id: "another-session".into(),
        effect_authority: None,
    };
    assert!(runtime
        .block_on(client.complete_provider_request(&request, vec![], Some(&forged)))
        .is_err());
    let allowed = request.provider.clone();
    request.provider.runtime_overrides.model_override = Some("other-model".into());
    assert!(runtime
        .block_on(client.complete_provider_request(&request, vec![], None))
        .is_err());
    request.provider = allowed.clone();
    request
        .provider
        .runtime_overrides
        .extra_body_fields
        .insert("top_k".into(), serde_json::json!(1));
    assert!(runtime
        .block_on(client.complete_provider_request(&request, vec![], None))
        .is_err());
    request.provider = allowed;
    request.provider.provider_name = "another-provider".into();
    assert!(runtime
        .block_on(client.complete_provider_request(&request, vec![], None))
        .is_err());
    assert_eq!(native.contexts.lock().unwrap().len(), 1);
}

struct FailingCompletion(ApiError);

#[async_trait]
impl ProviderCompletionPort for FailingCompletion {
    async fn complete_provider_request(
        &self,
        _: &GenerationOrchestrationRequest,
        _: Vec<ChatMessage>,
        _: Option<&ExecutionEventContext>,
    ) -> Result<ProviderCompletion, ApiError> {
        Err(self.0.clone())
    }
}

#[test]
fn provider_callback_failures_preserve_native_categories_messages_and_status() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let binding =
        ProviderExecutionBinding::new("selected-provider", ProviderRuntimeOverrides::default())
            .unwrap();
    let request = GenerationOrchestrationRequest {
        request_id: 1,
        node_id: [0; 32],
        agent_id: "assigned-agent".into(),
        provider: binding.clone(),
        frame_type: "owner-judgment".into(),
        retry_count: 0,
        force: false,
    };
    for failure in [
        ApiError::ProviderError("upstream error".into()),
        ApiError::ProviderNotConfigured("missing binding".into()),
        ApiError::ProviderRequestFailed("connection reset".into()),
        ApiError::ProviderRequestRejected {
            status: 413,
            message: "request too large".into(),
        },
        ApiError::ProviderAuthFailed("rejected credential".into()),
        ApiError::ProviderRateLimit("try later".into()),
        ApiError::ProviderModelNotFound("unknown model".into()),
        ApiError::GenerationFailed("incomplete output".into()),
        ApiError::ConfigError("invalid semantic response".into()),
    ] {
        let callbacks = OwnerProviderCallbacks::new(
            Arc::new(NoOwnerCallbacks),
            Arc::new(FailingCompletion(failure.clone())),
            binding.clone(),
            "assigned-agent".into(),
            BTreeSet::from(["owner-judgment".into()]),
            None,
        )
        .unwrap();
        let client = CallbackProviderClient::new(Arc::new(callbacks));
        let actual = runtime
            .block_on(client.complete_provider_request(&request, vec![], None))
            .unwrap_err();
        assert_eq!(
            std::mem::discriminant(&actual),
            std::mem::discriminant(&failure)
        );
        assert_eq!(actual.to_string(), failure.to_string());
        if let ApiError::ProviderRequestRejected { status, message } = actual {
            assert_eq!(status, 413);
            assert_eq!(message, "request too large");
        }
    }
}
