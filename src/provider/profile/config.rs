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
    fn endpoint_has_scheme(endpoint: &str) -> bool {
        endpoint.starts_with("http://") || endpoint.starts_with("https://")
    }

    fn infer_endpoint_scheme(provider_type: ProviderType, endpoint: &str) -> String {
        let endpoint = endpoint.trim();
        if provider_type == ProviderType::LocalCustom && !Self::endpoint_has_scheme(endpoint) {
            format!("https://{}", endpoint)
        } else {
            endpoint.to_string()
        }
    }

    pub fn normalized_endpoint(&self) -> Option<String> {
        self.endpoint
            .as_deref()
            .map(|endpoint| Self::infer_endpoint_scheme(self.provider_type, endpoint))
    }

    pub fn normalize_endpoint_in_place(&mut self) {
        self.endpoint = self.normalized_endpoint();
    }

    pub fn endpoint_url_is_valid(provider_type: ProviderType, endpoint: &str) -> bool {
        let endpoint = Self::infer_endpoint_scheme(provider_type, endpoint);
        if !Self::endpoint_has_scheme(&endpoint) {
            return false;
        }

        let Some(rest) = endpoint.split_once("://").map(|(_, rest)| rest) else {
            return false;
        };

        if rest.is_empty() || rest.chars().any(char::is_whitespace) {
            return false;
        }

        let authority = rest.split('/').next().unwrap_or_default();
        if authority.is_empty() {
            return false;
        }

        let host_port = authority.rsplit('@').next().unwrap_or(authority);
        if host_port.is_empty() {
            return false;
        }

        let host = if host_port.starts_with('[') {
            let Some(end_bracket) = host_port.find(']') else {
                return false;
            };
            &host_port[1..end_bracket]
        } else {
            host_port.split(':').next().unwrap_or_default()
        };

        if host.is_empty() {
            return false;
        }

        host == "localhost" || host.contains('.') || host.parse::<std::net::IpAddr>().is_ok()
    }

    /// Validate provider configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.model.trim().is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        if let Some(endpoint) = &self.endpoint {
            if !Self::endpoint_url_is_valid(self.provider_type, endpoint) {
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
                let endpoint = self.normalized_endpoint().ok_or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::CompletionOptions;
    use crate::provider::ModelProvider;

    #[test]
    fn local_custom_endpoint_validation_infers_https() {
        let provider = ProviderConfig {
            provider_name: Some("local".to_string()),
            provider_type: ProviderType::LocalCustom,
            model: "llama3".to_string(),
            api_key: None,
            endpoint: Some("chat.internal.jerkytreats.dev".to_string()),
            default_options: CompletionOptions::default(),
        };

        assert!(provider.validate().is_ok());
        assert_eq!(
            provider.normalized_endpoint().as_deref(),
            Some("https://chat.internal.jerkytreats.dev")
        );
    }

    #[test]
    fn local_custom_to_model_provider_infers_https() {
        let provider = ProviderConfig {
            provider_name: Some("local".to_string()),
            provider_type: ProviderType::LocalCustom,
            model: "llama3".to_string(),
            api_key: Some("test-key".to_string()),
            endpoint: Some("chat.internal.jerkytreats.dev".to_string()),
            default_options: CompletionOptions::default(),
        };

        let model_provider = provider.to_model_provider().unwrap();
        match model_provider {
            ModelProvider::LocalCustom {
                endpoint, api_key, ..
            } => {
                assert_eq!(endpoint, "https://chat.internal.jerkytreats.dev");
                assert_eq!(api_key.as_deref(), Some("test-key"));
            }
            other => panic!("Expected local custom provider, got {:?}", other),
        }
    }
}
