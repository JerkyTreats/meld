//! Operational ports available inside a package capability invocation.

use std::sync::Arc;

use async_trait::async_trait;
use meld_events::EventAppendCapability;

use super::provider::CallbackProviderClient;
use super::OwnerCallbackPort;
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use crate::provider::{ChatMessage, ProviderCompletion, ProviderCompletionPort};

/// Packages execute provider work and publish through the existing operational
/// authorities. Context mutation, Task admission and scheduling stay in core.
pub trait OwnerExecutionPorts: ProviderCompletionPort {
    fn owner_events(&self) -> Option<EventAppendCapability>;
}

impl<T: ExecutionRuntimeContext + ?Sized> OwnerExecutionPorts for T {
    fn owner_events(&self) -> Option<EventAppendCapability> {
        self.durable_event_append()
    }
}

pub struct CallbackExecutionPorts {
    provider: CallbackProviderClient,
    events: EventAppendCapability,
}

impl CallbackExecutionPorts {
    pub fn new(events: EventAppendCapability, callbacks: Arc<dyn OwnerCallbackPort>) -> Self {
        Self {
            provider: CallbackProviderClient::new(callbacks),
            events,
        }
    }
}

#[async_trait]
impl ProviderCompletionPort for CallbackExecutionPorts {
    async fn complete_provider_request(
        &self,
        request: &GenerationOrchestrationRequest,
        messages: Vec<ChatMessage>,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<ProviderCompletion, ApiError> {
        self.provider
            .complete_provider_request(request, messages, event_context)
            .await
    }
}

impl OwnerExecutionPorts for CallbackExecutionPorts {
    fn owner_events(&self) -> Option<EventAppendCapability> {
        Some(self.events.clone())
    }
}
