use crate::error::ApiError;
use crate::provider::profile::{ProviderConfig, ProviderType, ValidationResult};
use crate::provider::ProviderRegistry;

pub struct ProviderDiagnosticsService;

impl ProviderDiagnosticsService {
    fn load_provider_for_validation(
        registry: &ProviderRegistry,
        provider_name: &str,
        config_path: &std::path::Path,
        result: &mut ValidationResult,
    ) -> Result<Option<ProviderConfig>, ApiError> {
        if let Some(provider) = registry.get(provider_name) {
            return Ok(Some(provider.clone()));
        }

        if !config_path.exists() {
            result.add_error("Provider not found in registry".to_string());
            return Ok(None);
        }

        let content = match std::fs::read_to_string(config_path) {
            Ok(content) => content,
            Err(e) => {
                result.add_error(format!(
                    "Failed to read config file {}: {}",
                    config_path.display(),
                    e
                ));
                return Ok(None);
            }
        };

        let mut provider: ProviderConfig = match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                result.add_error(format!(
                    "Failed to parse config file {}: {}",
                    config_path.display(),
                    e
                ));
                return Ok(None);
            }
        };

        if provider.provider_name.is_none() {
            provider.provider_name = Some(provider_name.to_string());
        }

        Ok(Some(provider))
    }

    pub fn resolve_api_key_status(provider: &ProviderConfig) -> String {
        match provider.provider_type {
            ProviderType::OpenAI => {
                if provider.api_key.is_some() {
                    "Set (from config)".to_string()
                } else if std::env::var("OPENAI_API_KEY").is_ok() {
                    "Set (from environment)".to_string()
                } else {
                    "Not set".to_string()
                }
            }
            ProviderType::Anthropic => {
                if provider.api_key.is_some() {
                    "Set (from config)".to_string()
                } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                    "Set (from environment)".to_string()
                } else {
                    "Not set".to_string()
                }
            }
            ProviderType::Ollama | ProviderType::LocalCustom => "Not required".to_string(),
        }
    }

    pub fn validate_provider(
        registry: &ProviderRegistry,
        provider_name: &str,
    ) -> Result<ValidationResult, ApiError> {
        let mut result = ValidationResult::new(provider_name.to_string());

        let config_path = registry.provider_config_path(provider_name)?;
        let provider = match Self::load_provider_for_validation(
            registry,
            provider_name,
            &config_path,
            &mut result,
        )? {
            Some(provider) => provider,
            None => return Ok(result),
        };

        if !config_path.exists() {
            result.add_error(format!("Config file not found: {}", config_path.display()));
            return Ok(result);
        }

        let expected_filename = format!("{}.toml", provider_name);
        if config_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == expected_filename)
            .unwrap_or(false)
        {
            result.add_check("Provider name matches filename", true);
        } else {
            result.add_error(format!(
                "Provider name '{}' doesn't match filename '{}'",
                provider_name,
                config_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            ));
        }

        result.add_check("Provider type is valid", true);

        if provider.model.trim().is_empty() {
            result.add_error("Model name cannot be empty".to_string());
        } else {
            result.add_check("Model is not empty", true);
        }

        match provider.provider_type {
            ProviderType::OpenAI | ProviderType::Anthropic => {
                let api_key_available = provider.api_key.is_some()
                    || match provider.provider_type {
                        ProviderType::OpenAI => std::env::var("OPENAI_API_KEY").is_ok(),
                        ProviderType::Anthropic => std::env::var("ANTHROPIC_API_KEY").is_ok(),
                        _ => false,
                    };

                if api_key_available {
                    let source = if provider.api_key.is_some() {
                        "from config"
                    } else {
                        "from environment"
                    };
                    result.add_check(&format!("API key available ({})", source), true);
                } else {
                    let env_var = match provider.provider_type {
                        ProviderType::OpenAI => "OPENAI_API_KEY",
                        ProviderType::Anthropic => "ANTHROPIC_API_KEY",
                        _ => unreachable!(),
                    };
                    result.add_error(format!(
                        "API key not found (set {} or add to config)",
                        env_var
                    ));
                }
            }
            ProviderType::Ollama | ProviderType::LocalCustom => {
                result.add_check("API key not required for local provider", true);
            }
        }

        if let Some(endpoint) = &provider.endpoint {
            if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
                result.add_check("Endpoint URL is valid", true);
            } else {
                result.add_error(format!("Invalid endpoint URL: {}", endpoint));
            }
        } else if provider.provider_type == ProviderType::LocalCustom {
            result.add_error("Endpoint is required for local custom provider".to_string());
        } else {
            result.add_check("Endpoint URL (optional)", true);
        }

        if let Some(temp) = provider.default_options.temperature {
            if (0.0..=2.0).contains(&temp) {
                result.add_check("Temperature is in valid range (0.0-2.0)", true);
            } else {
                result.add_error(format!(
                    "Temperature must be between 0.0 and 2.0, got {}",
                    temp
                ));
            }
        }

        if let Some(max_tokens) = provider.default_options.max_tokens {
            if max_tokens > 0 {
                result.add_check("Max tokens is positive", true);
            } else {
                result.add_error("Max tokens must be positive".to_string());
            }
        }

        if let Some(top_p) = provider.default_options.top_p {
            if (0.0..=1.0).contains(&top_p) {
                result.add_check("Top-p is in valid range (0.0-1.0)", true);
            } else {
                result.add_error(format!("Top-p must be between 0.0 and 1.0, got {}", top_p));
            }
        }

        Ok(result)
    }

    pub fn list_available_models(
        registry: &ProviderRegistry,
        provider_name: &str,
    ) -> Result<Vec<String>, ApiError> {
        let client = registry.create_client(provider_name)?;
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ApiError::ProviderError(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(client.list_models())
    }

    pub fn list_available_models_with_timeout(
        registry: &ProviderRegistry,
        provider_name: &str,
        timeout_secs: u64,
    ) -> Result<Vec<String>, ApiError> {
        let client = registry.create_client(provider_name)?;
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ApiError::ProviderError(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                client.list_models(),
            )
            .await
            .map_err(|_| {
                ApiError::ProviderError(format!("API connectivity timeout ({}s)", timeout_secs))
            })?
        })
    }
}
