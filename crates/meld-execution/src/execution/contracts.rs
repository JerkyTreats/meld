//! Provider execution binding contracts.

use crate::error::ApiError;
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

/// Runtime request fields that may be supplied alongside a provider binding.
///
/// Core provider request fields stay owned by execution so call sites cannot
/// bypass typed validation through extra JSON body fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRuntimeOverrides {
    /// Optional provider model replacement chosen at execution time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    /// Provider-specific JSON fields that are not part of the core request.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra_body_fields: BTreeMap<String, Value>,
}

impl ProviderRuntimeOverrides {
    /// Builds validated runtime overrides.
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

    /// Returns true when the binding carries no runtime changes.
    pub fn is_empty(&self) -> bool {
        self.model_override.is_none() && self.extra_body_fields.is_empty()
    }

    /// Returns the configured provider-specific body keys in deterministic order.
    pub fn extra_body_field_keys(&self) -> Vec<&str> {
        self.extra_body_fields.keys().map(String::as_str).collect()
    }

    /// Returns a deterministic hash of the serialized override payload.
    pub fn fingerprint(&self) -> Result<String, ApiError> {
        let encoded = serde_json::to_vec(self).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to encode provider runtime overrides: {}",
                err
            ))
        })?;
        Ok(blake3::hash(&encoded).to_hex().to_string())
    }

    /// Rejects provider fields that must remain typed execution inputs.
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

/// Validated provider binding selected for one execution request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderExecutionBinding {
    /// Stable provider name resolved by the workspace adapter.
    pub provider_name: String,
    /// Optional provider-specific runtime adjustments.
    #[serde(default)]
    pub runtime_overrides: ProviderRuntimeOverrides,
}

impl ProviderExecutionBinding {
    /// Builds a provider binding after validating the provider name and overrides.
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

    /// Returns a deterministic hash of the serialized binding.
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

    #[test]
    fn provider_binding_fingerprint_changes_with_provider_and_overrides() {
        let baseline =
            ProviderExecutionBinding::new("local-a", ProviderRuntimeOverrides::default()).unwrap();
        let renamed =
            ProviderExecutionBinding::new("local-b", ProviderRuntimeOverrides::default()).unwrap();
        let tuned = ProviderExecutionBinding::new(
            "local-a",
            ProviderRuntimeOverrides::new(
                Some("qwen3-coder-next".to_string()),
                BTreeMap::from([("lmserver_max_tool_turns".to_string(), json!(24))]),
            )
            .unwrap(),
        )
        .unwrap();

        let baseline_fingerprint = baseline.fingerprint().unwrap();

        assert!(!baseline_fingerprint.is_empty());
        assert_ne!(baseline_fingerprint, "xyzzy");
        assert_ne!(baseline_fingerprint, renamed.fingerprint().unwrap());
        assert_ne!(baseline_fingerprint, tuned.fingerprint().unwrap());
    }

    #[test]
    fn runtime_overrides_report_empty_state_and_sorted_extra_body_keys() {
        let baseline = ProviderRuntimeOverrides::default();

        assert!(baseline.is_empty());
        assert!(baseline.extra_body_field_keys().is_empty());

        let model_only =
            ProviderRuntimeOverrides::new(Some("qwen3-coder-next".to_string()), BTreeMap::new())
                .unwrap();

        assert!(!model_only.is_empty());
        assert!(model_only.extra_body_field_keys().is_empty());

        let extra_fields = ProviderRuntimeOverrides::new(
            None,
            BTreeMap::from([
                ("zeta".to_string(), json!(1)),
                ("alpha".to_string(), json!(2)),
            ]),
        )
        .unwrap();

        assert!(!extra_fields.is_empty());
        assert_eq!(extra_fields.extra_body_field_keys(), vec!["alpha", "zeta"]);
    }
}
