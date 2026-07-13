use std::path::Path;

use super::contracts::{
    ActivationLoadError, ValidatedDocsFreshnessActivation, ValidatedProductActivationPreflight,
};
use super::preflight::{preflight_validated_activation, ActivationPreflightError};

/// Load and validate one explicit activation path without opening product stores.
pub fn load_and_validate_activation(
    workspace_root: impl AsRef<Path>,
    activation_path: impl AsRef<Path>,
) -> Result<ValidatedDocsFreshnessActivation, ActivationLoadError> {
    super::loader::load(workspace_root.as_ref(), activation_path.as_ref())
}

/// Load repository configuration and complete store-free product preflight.
///
/// An explicit configuration path follows the same file plus environment
/// overlay contract as `RunContext`. Without one, the standard global,
/// workspace, and environment merge is loaded. Both paths complete before
/// any product or execution semantic store can be opened.
pub fn load_and_preflight_activation(
    workspace_root: impl AsRef<Path>,
    activation_path: impl AsRef<Path>,
    config_path: Option<&Path>,
) -> Result<ValidatedProductActivationPreflight, ActivationPreflightError> {
    let workspace_root = workspace_root.as_ref();
    let activation = load_and_validate_activation(workspace_root, activation_path)?;
    let config = match config_path {
        Some(path) => crate::config::ConfigLoader::load_from_file(path),
        None => crate::config::ConfigLoader::load(workspace_root),
    }
    .map_err(|error| ActivationPreflightError::Configuration(error.to_string()))?;

    preflight_validated_activation(activation, &config)
}
