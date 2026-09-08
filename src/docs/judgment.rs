//! Execution provenance authored by the Docs provider adapter, never by model output.

use crate::error::ApiError;
use crate::provider::executor::ProviderPreparation;
use crate::provider::CompletionResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsJudgmentExecution {
    pub execution_id: String,
    pub request_identity: String,
    pub policy_identity: String,
    pub provider_name: String,
    pub provider_type: String,
    pub requested_model: String,
    /// The provider's reported model may be an alias. It is not a claim about
    /// the service administrator's current backing weights.
    pub reported_model: String,
    pub configuration_identity: String,
    pub response_identity: String,
    pub finish_reason: Option<String>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl DocsJudgmentExecution {
    pub(crate) fn capture(
        policy_identity: String,
        request: &crate::context::generation::contracts::GenerationOrchestrationRequest,
        preparation: &ProviderPreparation,
        response: &CompletionResponse,
    ) -> Result<Self, ApiError> {
        let config = &preparation.provider_config;
        // Hash the resolved settings; do not persist endpoints or credentials.
        let configuration_identity = digest(&(
            &preparation.provider_type,
            &config.model,
            &config.endpoint,
            &config.default_options,
        ))?;
        let mut execution = Self {
            execution_id: String::new(),
            request_identity: hex::encode(request.node_id),
            policy_identity,
            provider_name: request.provider.provider_name.clone(),
            provider_type: preparation.provider_type.clone(),
            requested_model: config.model.clone(),
            reported_model: response.model.clone(),
            configuration_identity,
            response_identity: blake3::hash(response.content.as_bytes())
                .to_hex()
                .to_string(),
            finish_reason: response.finish_reason.clone(),
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
        };
        execution.execution_id = digest(&execution)?;
        Ok(execution)
    }

    pub(crate) fn validate(&self, policy: Option<&str>) -> Result<(), ApiError> {
        let mut basis = self.clone();
        basis.execution_id.clear();
        if digest(&basis)? != self.execution_id
            || policy.is_some_and(|policy| policy != self.policy_identity)
        {
            return Err(ApiError::ConfigError(
                "Docs judgment execution identity or policy changed".into(),
            ));
        }
        Ok(())
    }
}

fn digest(value: &impl Serialize) -> Result<String, ApiError> {
    Ok(blake3::hash(
        &serde_json::to_vec(value).map_err(|error| ApiError::ConfigError(error.to_string()))?,
    )
    .to_hex()
    .to_string())
}
