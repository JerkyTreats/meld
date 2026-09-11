//! Bounded completion with provider-owned execution provenance.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{ChatMessage, CompletionResponse};
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ProviderExecutionPort, ProviderValidationPort};

/// Resolved execution settings that may cross an owner boundary without exposing
/// provider credentials, endpoints, or an independently usable provider client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderExecutionDescription {
    pub provider_type: String,
    pub requested_model: String,
    pub configuration_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCompletion {
    pub preparation: ProviderExecutionDescription,
    pub response: CompletionResponse,
}

#[async_trait]
pub trait ProviderCompletionPort: Send + Sync {
    async fn complete_provider_request(
        &self,
        request: &GenerationOrchestrationRequest,
        messages: Vec<ChatMessage>,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<ProviderCompletion, ApiError>;
}

#[async_trait]
impl<P: ProviderValidationPort + ProviderExecutionPort + ?Sized> ProviderCompletionPort for P {
    async fn complete_provider_request(
        &self,
        request: &GenerationOrchestrationRequest,
        messages: Vec<ChatMessage>,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<ProviderCompletion, ApiError> {
        let preparation = self.prepare_provider_for_request(request)?;
        let config = &preparation.provider_config;
        let bytes = serde_json::to_vec(&(
            &preparation.provider_type,
            &config.model,
            &config.endpoint,
            &config.default_options,
        ))
        .map_err(|error| ApiError::ConfigError(error.to_string()))?;
        let description = ProviderExecutionDescription {
            provider_type: preparation.provider_type.clone(),
            requested_model: config.model.clone(),
            configuration_identity: blake3::hash(&bytes).to_hex().to_string(),
        };
        let response = self
            .execute_completion(request, &preparation, messages, event_context)
            .await?;
        Ok(ProviderCompletion {
            preparation: description,
            response,
        })
    }
}

/// Provider failures retain their native category across an owner transport.
/// The HTTP status is part of a rejected completion, not transport metadata.
#[derive(Debug, Serialize, Deserialize)]
pub enum ProviderCompletionFailure {
    ProviderError(String),
    ProviderNotConfigured(String),
    ProviderRequestFailed(String),
    ProviderRequestRejected { status: u16, message: String },
    ProviderAuthFailed(String),
    ProviderRateLimit(String),
    ProviderModelNotFound(String),
    GenerationFailed(String),
    StorageUnavailable(String),
    ConfigError(String),
}

impl From<ApiError> for ProviderCompletionFailure {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::ProviderError(message) => Self::ProviderError(message),
            ApiError::ProviderNotConfigured(message) => Self::ProviderNotConfigured(message),
            ApiError::ProviderRequestFailed(message) => Self::ProviderRequestFailed(message),
            ApiError::ProviderExecutionFailed { message, .. } => {
                Self::ProviderRequestFailed(message)
            }
            ApiError::ProviderRequestRejected { status, message } => {
                Self::ProviderRequestRejected { status, message }
            }
            ApiError::ProviderAuthFailed(message) => Self::ProviderAuthFailed(message),
            ApiError::ProviderRateLimit(message) => Self::ProviderRateLimit(message),
            ApiError::ProviderModelNotFound(message) => Self::ProviderModelNotFound(message),
            ApiError::GenerationFailed(message) => Self::GenerationFailed(message),
            ApiError::StorageError(error) => Self::StorageUnavailable(error.to_string()),
            ApiError::ConfigError(message) => Self::ConfigError(message),
            error => Self::ProviderError(error.to_string()),
        }
    }
}

impl From<ProviderCompletionFailure> for ApiError {
    fn from(error: ProviderCompletionFailure) -> Self {
        match error {
            ProviderCompletionFailure::ProviderError(message) => Self::ProviderError(message),
            ProviderCompletionFailure::ProviderNotConfigured(message) => {
                Self::ProviderNotConfigured(message)
            }
            ProviderCompletionFailure::ProviderRequestFailed(message) => {
                Self::ProviderRequestFailed(message)
            }
            ProviderCompletionFailure::ProviderRequestRejected { status, message } => {
                Self::ProviderRequestRejected { status, message }
            }
            ProviderCompletionFailure::ProviderAuthFailed(message) => {
                Self::ProviderAuthFailed(message)
            }
            ProviderCompletionFailure::ProviderRateLimit(message) => {
                Self::ProviderRateLimit(message)
            }
            ProviderCompletionFailure::ProviderModelNotFound(message) => {
                Self::ProviderModelNotFound(message)
            }
            ProviderCompletionFailure::GenerationFailed(message) => Self::GenerationFailed(message),
            ProviderCompletionFailure::StorageUnavailable(message) => Self::StorageError(
                crate::error::StorageError::EventAuthorityUnavailable(message),
            ),
            ProviderCompletionFailure::ConfigError(message) => Self::ConfigError(message),
        }
    }
}
