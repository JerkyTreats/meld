use crate::error::ApiError;
use crate::provider::{CompletionOptions, ModelProvider};
use serde::{Deserialize, Serialize};

/// Model provider configuration owned by the provider domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name unique identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,

    /// Provider type.
    pub provider_type: ProviderType,

    /// Model identifier.
    pub model: String,

    /// API key optional and can be loaded from environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Base URL or endpoint provider specific.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Default completion options for this provider.
    #[serde(default)]
    pub default_options: CompletionOptions,
}

/// Provider type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "local")]
    LocalCustom,
}

impl ProviderConfig {
    /// Validate provider configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.model.trim().is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        if let Some(endpoint) = &self.endpoint {
            if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
                return Err(format!("Invalid endpoint URL: {}", endpoint));
            }
        }

        if let Some(temp) = self.default_options.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(format!(
                    "Temperature must be between 0.0 and 2.0, got {}",
                    temp
                ));
            }
        }

        Ok(())
    }

    /// Convert ProviderConfig to ModelProvider.
    pub fn to_model_provider(&self) -> Result<ModelProvider, ApiError> {
        let api_key = self.api_key.clone().or_else(|| match self.provider_type {
            ProviderType::OpenAI => std::env::var("OPENAI_API_KEY").ok(),
            ProviderType::Anthropic => std::env::var("ANTHROPIC_API_KEY").ok(),
            _ => None,
        });

        match self.provider_type {
            ProviderType::OpenAI => {
                let api_key = api_key.ok_or_else(|| {
                    ApiError::ProviderNotConfigured(
                        "OpenAI API key required (set in config or OPENAI_API_KEY env var)"
                            .to_string(),
                    )
                })?;
                Ok(ModelProvider::OpenAI {
                    model: self.model.clone(),
                    api_key,
                    base_url: self.endpoint.clone(),
                })
            }
            ProviderType::Anthropic => {
                let api_key = api_key.ok_or_else(|| {
                    ApiError::ProviderNotConfigured(
                        "Anthropic API key required (set in config or ANTHROPIC_API_KEY env var)"
                            .to_string(),
                    )
                })?;
                Ok(ModelProvider::Anthropic {
                    model: self.model.clone(),
                    api_key,
                })
            }
            ProviderType::Ollama => Ok(ModelProvider::Ollama {
                model: self.model.clone(),
                base_url: self.endpoint.clone(),
            }),
            ProviderType::LocalCustom => {
                let endpoint = self.endpoint.clone().ok_or_else(|| {
                    ApiError::ProviderNotConfigured(
                        "LocalCustom provider requires endpoint".to_string(),
                    )
                })?;
                Ok(ModelProvider::LocalCustom {
                    model: self.model.clone(),
                    endpoint,
                    api_key,
                })
            }
        }
    }
}
