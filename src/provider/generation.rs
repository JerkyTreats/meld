use crate::error::ApiError;
use crate::provider::clients::ProviderClientResolver;
use crate::provider::profile::ProviderConfig;
use crate::provider::ModelProviderClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const RESERVED_PROVIDER_REQUEST_FIELD_KEYS: [&str; 9] = [
    "model",
    "messages",
    "stream",
    "temperature",
    "max_tokens",
    "top_p",
    "frequency_penalty",
    "presence_penalty",
    "stop",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRuntimeOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra_body_fields: BTreeMap<String, Value>,
}

impl ProviderRuntimeOverrides {
    pub fn new(
        model_override: Option<String>,
        extra_body_fields: BTreeMap<String, Value>,
    ) -> Result<Self, ApiError> {
        let overrides = Self {
            model_override,
            extra_body_fields,
        };
        overrides.validate()?;
        Ok(overrides)
    }

    pub fn is_empty(&self) -> bool {
        self.model_override.is_none() && self.extra_body_fields.is_empty()
    }

    pub fn extra_body_field_keys(&self) -> Vec<&str> {
        self.extra_body_fields.keys().map(String::as_str).collect()
    }

    pub fn fingerprint(&self) -> Result<String, ApiError> {
        let encoded = serde_json::to_vec(self).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to encode provider runtime overrides: {}",
                err
            ))
        })?;
        Ok(blake3::hash(&encoded).to_hex().to_string())
    }

    pub fn validate(&self) -> Result<(), ApiError> {
        if let Some(key) = self
            .extra_body_fields
            .keys()
            .find(|key| RESERVED_PROVIDER_REQUEST_FIELD_KEYS.contains(&key.as_str()))
        {
            return Err(ApiError::ConfigError(format!(
                "Provider runtime override key '{}' is reserved. Use dedicated flags for core request fields.",
                key
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderExecutionBinding {
    pub provider_name: String,
    #[serde(default)]
    pub runtime_overrides: ProviderRuntimeOverrides,
}

impl ProviderExecutionBinding {
    pub fn new(
        provider_name: impl Into<String>,
        runtime_overrides: ProviderRuntimeOverrides,
    ) -> Result<Self, ApiError> {
        let provider_name = provider_name.into();
        if provider_name.trim().is_empty() {
            return Err(ApiError::ConfigError(
                "Provider execution binding requires a non-empty provider name".to_string(),
            ));
        }
        runtime_overrides.validate()?;
        Ok(Self {
            provider_name,
            runtime_overrides,
        })
    }

    pub fn fingerprint(&self) -> Result<String, ApiError> {
        let encoded = serde_json::to_vec(self).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to encode provider execution binding: {}",
                err
            ))
        })?;
        Ok(blake3::hash(&encoded).to_hex().to_string())
    }
}

pub struct ProviderGenerationService<'a, R: ProviderClientResolver> {
    resolver: &'a R,
}

impl<'a, R: ProviderClientResolver> ProviderGenerationService<'a, R> {
    pub fn new(resolver: &'a R) -> Self {
        Self { resolver }
    }

    pub fn resolve_provider(
        &self,
        provider_name: &str,
    ) -> Result<(ProviderConfig, Box<dyn ModelProviderClient>), ApiError> {
        let config = self.resolver.resolve_provider_config(provider_name)?;
        let client = self.resolver.create_provider_client(provider_name)?;
        Ok((config, client))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn runtime_overrides_reject_reserved_request_keys() {
        let overrides = ProviderRuntimeOverrides::new(
            None,
            BTreeMap::from([("model".to_string(), json!("other-model"))]),
        );

        assert!(matches!(overrides, Err(ApiError::ConfigError(_))));
    }

    #[test]
    fn runtime_override_fingerprint_changes_with_payload() {
        let baseline = ProviderRuntimeOverrides::default();
        let tuned = ProviderRuntimeOverrides::new(
            Some("qwen3-coder-next".to_string()),
            BTreeMap::from([("lmserver_max_tool_turns".to_string(), json!(24))]),
        )
        .unwrap();

        assert_ne!(
            baseline.fingerprint().unwrap(),
            tuned.fingerprint().unwrap()
        );
    }
}
