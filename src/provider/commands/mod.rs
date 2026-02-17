use crate::error::ApiError;
use crate::provider::profile::{ProviderConfig, ProviderType};
use crate::provider::ProviderRegistry;
use std::path::{Path, PathBuf};

pub struct ProviderCommandService;

impl ProviderCommandService {
    pub fn parse_provider_type(type_str: &str) -> Result<ProviderType, ApiError> {
        match type_str {
            "openai" => Ok(ProviderType::OpenAI),
            "anthropic" => Ok(ProviderType::Anthropic),
            "ollama" => Ok(ProviderType::Ollama),
            "local" => Ok(ProviderType::LocalCustom),
            _ => Err(ApiError::ConfigError(format!(
                "Invalid provider type: {}. Must be openai, anthropic, ollama, or local",
                type_str
            ))),
        }
    }

    pub fn default_endpoint(provider_type: ProviderType) -> Option<String> {
        match provider_type {
            ProviderType::OpenAI => Some("https://api.openai.com/v1".to_string()),
            ProviderType::Ollama => Some("http://localhost:11434".to_string()),
            ProviderType::LocalCustom | ProviderType::Anthropic => None,
        }
    }

    pub fn required_api_key_env_var(provider_type: ProviderType) -> Option<&'static str> {
        match provider_type {
            ProviderType::OpenAI => Some("OPENAI_API_KEY"),
            ProviderType::Anthropic => Some("ANTHROPIC_API_KEY"),
            ProviderType::Ollama | ProviderType::LocalCustom => None,
        }
    }

    pub fn build_provider_config(
        provider_name: &str,
        provider_type: ProviderType,
        model: String,
        endpoint: Option<String>,
        api_key: Option<String>,
        default_options: crate::provider::CompletionOptions,
    ) -> ProviderConfig {
        ProviderConfig {
            provider_name: Some(provider_name.to_string()),
            provider_type,
            model,
            api_key,
            endpoint,
            default_options,
        }
    }

    pub fn provider_config_path(
        registry: &ProviderRegistry,
        provider_name: &str,
    ) -> Result<PathBuf, ApiError> {
        registry.provider_config_path(provider_name)
    }

    pub fn persist_provider_config(
        registry: &mut ProviderRegistry,
        provider_name: &str,
        config: &ProviderConfig,
    ) -> Result<PathBuf, ApiError> {
        let path = registry.provider_config_path(provider_name)?;
        registry.save_provider_config(provider_name, config)?;
        registry.load_from_xdg()?;
        Ok(path)
    }

    pub fn delete_provider_config(
        registry: &mut ProviderRegistry,
        provider_name: &str,
    ) -> Result<PathBuf, ApiError> {
        let path = registry.provider_config_path(provider_name)?;
        registry.delete_provider_config(provider_name)?;
        registry.load_from_xdg()?;
        Ok(path)
    }

    pub fn load_provider_config_from_path(path: &Path) -> Result<ProviderConfig, ApiError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ApiError::ConfigError(format!("Failed to read config: {}", e)))?;
        toml::from_str(&content)
            .map_err(|e| ApiError::ConfigError(format!("Failed to parse config: {}", e)))
    }
}
