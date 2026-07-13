//! Store-free root preflight for domain-owned activation products.

use meld_execution::activation::{
    bind_builtin_execution_activation, validate_execution_activation,
    ExecutionActivationBindingError, ExecutionActivationSelection,
    ExecutionActivationValidationContext, ExecutionActivationValidationError,
};
use thiserror::Error;

use crate::config::MerkleConfig;

use super::contracts::{
    ActivationLoadError, ExecutionActivationPreflight, ValidatedDocsFreshnessActivation,
    ValidatedProductActivationPreflight,
};

/// Fail-closed error returned before product semantic stores are opened.
#[derive(Debug, Error)]
pub enum ActivationPreflightError {
    /// Activation source or an owner package failed root validation.
    #[error(transparent)]
    Activation(#[from] ActivationLoadError),
    /// Merged repository configuration could not be loaded or validated.
    #[error("activation repository configuration is invalid: {0}")]
    Configuration(String),
    /// Execution could not resolve or bind the selected authored assets.
    #[error("execution activation binding failed: {0}")]
    ExecutionBinding(#[from] ExecutionActivationBindingError),
    /// Execution rejected the fully bound source-neutral input.
    #[error("execution activation validation failed: {0}")]
    ExecutionValidation(#[from] ExecutionActivationValidationError),
}

/// Bind and validate a source-neutral execution selection without opening stores.
///
/// Provider binding identities are derived only from repository provider map
/// keys. Activation content and provider display names cannot introduce an
/// identity into this validation context.
pub fn preflight_execution_activation(
    execution_selection: ExecutionActivationSelection,
    repository_config: &MerkleConfig,
) -> Result<ExecutionActivationPreflight, ActivationPreflightError> {
    repository_config.validate().map_err(|errors| {
        ActivationPreflightError::Configuration(
            errors
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;

    let validation_context = ExecutionActivationValidationContext::from_provider_binding_refs(
        repository_config.providers.keys().cloned(),
    );
    let execution_input =
        bind_builtin_execution_activation(execution_selection, validation_context)?;
    let execution_receipt = validate_execution_activation(&execution_input)?;

    Ok(ExecutionActivationPreflight {
        execution_input,
        execution_receipt,
    })
}

/// Compose validated root activation data with source-neutral execution preflight.
pub fn preflight_validated_activation(
    activation: ValidatedDocsFreshnessActivation,
    repository_config: &MerkleConfig,
) -> Result<ValidatedProductActivationPreflight, ActivationPreflightError> {
    let execution = preflight_execution_activation(
        activation.runtime_inputs.execution.clone(),
        repository_config,
    )?;

    Ok(ValidatedProductActivationPreflight {
        activation,
        execution_input: execution.execution_input,
        execution_receipt: execution.execution_receipt,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::{MerkleConfig, ProviderConfig, ProviderType};
    use crate::provider::CompletionOptions;

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/runtime/docs_freshness_activation.toml"
    ));

    fn activation() -> ValidatedDocsFreshnessActivation {
        let workspace = tempfile::tempdir().unwrap();
        let source = workspace.path().join("activation.toml");
        std::fs::write(&source, FIXTURE).unwrap();
        super::super::load_and_validate_activation(workspace.path(), &source).unwrap()
    }

    fn config_with_provider(key: &str, provider_name: Option<&str>) -> MerkleConfig {
        let mut config = MerkleConfig::default();
        config.providers.insert(
            key.to_string(),
            ProviderConfig {
                provider_name: provider_name.map(str::to_string),
                provider_type: ProviderType::Ollama,
                model: "test-model".to_string(),
                api_key: None,
                endpoint: None,
                default_options: CompletionOptions::default(),
            },
        );
        config
    }

    #[test]
    fn configured_provider_key_completes_execution_preflight() {
        let result = preflight_validated_activation(
            activation(),
            &config_with_provider("docs-writer", Some("display-name")),
        )
        .unwrap();

        assert_eq!(
            result.execution_input.selection.provider_binding_ref,
            "docs-writer"
        );
        assert!(result
            .execution_input
            .validation_context
            .contains_provider_binding("docs-writer"));
        assert_eq!(
            result.execution_receipt.activation_hash,
            result.activation.activation_hash.as_str()
        );
        assert!(result.passive_description().application_ready);
    }

    #[test]
    fn provider_display_name_cannot_synthesize_binding_identity() {
        let error = preflight_validated_activation(
            activation(),
            &config_with_provider("different-key", Some("docs-writer")),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ActivationPreflightError::ExecutionBinding(ref error)
                if error.field == "selection.provider_binding_ref"
        ));
    }

    #[test]
    fn repeated_preflight_returns_deterministic_execution_receipt() {
        let workspace = tempfile::tempdir().unwrap();
        let source = workspace.path().join("activation.toml");
        std::fs::write(&source, FIXTURE).unwrap();
        let first_activation =
            super::super::load_and_validate_activation(workspace.path(), &source).unwrap();
        let second_activation =
            super::super::load_and_validate_activation(workspace.path(), &source).unwrap();
        let config = config_with_provider("docs-writer", None);

        let first = preflight_validated_activation(first_activation, &config).unwrap();
        let second = preflight_validated_activation(second_activation, &config).unwrap();

        assert_eq!(first.execution_input, second.execution_input);
        assert_eq!(first.execution_receipt, second.execution_receipt);
    }

    #[test]
    fn preflight_does_not_create_configured_product_root() {
        let product_parent = tempfile::tempdir().unwrap();
        let product_root: PathBuf = product_parent.path().join("runtime-product");
        let mut config = config_with_provider("different-key", None);
        config.system.storage.product_root = Some(product_root.clone());

        assert!(preflight_validated_activation(activation(), &config).is_err());
        assert!(!product_root.exists());
    }
}
