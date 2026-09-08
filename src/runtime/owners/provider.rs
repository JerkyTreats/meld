//! Forward bounded provider requests through the native operation's selected port.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::{
    encode_owner_result, OwnerCallbackPort, OwnerCallbackV1, OwnerDiagnosticV1, OwnerResult,
};
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::error::ApiError;
use crate::execution::ExecutionEventContext;
use crate::provider::{
    ChatMessage, ProviderCompletion, ProviderCompletionFailure, ProviderCompletionPort,
    ProviderExecutionBinding,
};
use async_trait::async_trait;

pub struct CallbackProviderClient {
    callbacks: Arc<dyn OwnerCallbackPort>,
}

impl CallbackProviderClient {
    pub fn new(callbacks: Arc<dyn OwnerCallbackPort>) -> Self {
        Self { callbacks }
    }
}

#[async_trait]
impl ProviderCompletionPort for CallbackProviderClient {
    async fn complete_provider_request(
        &self,
        request: &GenerationOrchestrationRequest,
        messages: Vec<ChatMessage>,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<ProviderCompletion, ApiError> {
        let value = self
            .callbacks
            .call(OwnerCallbackV1::Provider {
                request: request.clone(),
                messages,
                event_context: event_context.cloned(),
            })
            .map_err(decode_transport_failure)?;
        serde_json::from_value::<Result<ProviderCompletion, ProviderCompletionFailure>>(value)
            .map_err(|error| ApiError::ProviderRequestFailed(error.to_string()))?
            .map_err(ApiError::from)
    }
}

/// The parent resolves these selections from its prepared assignment and admitted
/// operation. Child-supplied event context cannot replace that native authority.
pub struct OwnerProviderCallbacks {
    events: Arc<dyn OwnerCallbackPort>,
    provider: Arc<dyn ProviderCompletionPort>,
    binding: ProviderExecutionBinding,
    agent_id: String,
    frame_types: BTreeSet<String>,
    event_context: Option<ExecutionEventContext>,
}

impl OwnerProviderCallbacks {
    pub fn new(
        events: Arc<dyn OwnerCallbackPort>,
        provider: Arc<dyn ProviderCompletionPort>,
        binding: ProviderExecutionBinding,
        agent_id: String,
        frame_types: BTreeSet<String>,
        event_context: Option<ExecutionEventContext>,
    ) -> Result<Self, OwnerDiagnosticV1> {
        let binding =
            ProviderExecutionBinding::new(binding.provider_name, binding.runtime_overrides)
                .map_err(|error| OwnerDiagnosticV1::new("owner_callback_not_granted", error))?;
        if agent_id.trim().is_empty()
            || frame_types.is_empty()
            || frame_types.iter().any(|frame| frame.trim().is_empty())
            || event_context.as_ref().is_some_and(|context| {
                context.session_id.trim().is_empty()
                    || context
                        .effect_authority
                        .as_ref()
                        .is_some_and(|authority| authority.issuer_ref != agent_id)
            })
        {
            return Err(OwnerDiagnosticV1::new(
                "owner_callback_not_granted",
                "provider grant lacks an exact native scope",
            ));
        }
        Ok(Self {
            events,
            provider,
            binding,
            agent_id,
            frame_types,
            event_context,
        })
    }
}

impl OwnerCallbackPort for OwnerProviderCallbacks {
    fn call(&self, callback: OwnerCallbackV1) -> OwnerResult {
        let OwnerCallbackV1::Provider {
            request,
            messages,
            event_context,
        } = callback
        else {
            return self.events.call(callback);
        };
        if request.provider != self.binding
            || request.agent_id != self.agent_id
            || !self.frame_types.contains(&request.frame_type)
            || event_context
                .as_ref()
                .is_some_and(|context| Some(context) != self.event_context.as_ref())
        {
            return Err(OwnerDiagnosticV1::new(
                "owner_callback_not_granted",
                "provider request differs from the selected binding or native operation scope",
            ));
        }
        // A callback is synchronous even when its caller is inside Tokio. The
        // scoped thread completes this same operation before transport can return.
        let completion = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| ApiError::ProviderRequestFailed(error.to_string()))?;
                    runtime.block_on(self.provider.complete_provider_request(
                        &request,
                        messages,
                        self.event_context.as_ref(),
                    ))
                })
                .join()
        })
        .map_err(|_| {
            OwnerDiagnosticV1::new(
                "provider_request_failed",
                "native provider callback panicked",
            )
        })?
        .map_err(ProviderCompletionFailure::from);
        encode_owner_result(completion)
    }
}

fn decode_transport_failure(error: OwnerDiagnosticV1) -> ApiError {
    match error.code.as_str() {
        "owner_callback_not_granted" => ApiError::ConfigError(error.to_string()),
        _ => ApiError::ProviderRequestFailed(error.to_string()),
    }
}
