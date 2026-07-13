use std::path::Path;

use super::contracts::{ActivationLoadError, ValidatedDocsFreshnessActivation};

/// Load and validate one explicit activation path without opening product stores.
pub fn load_and_validate_activation(
    workspace_root: impl AsRef<Path>,
    activation_path: impl AsRef<Path>,
) -> Result<ValidatedDocsFreshnessActivation, ActivationLoadError> {
    super::loader::load(workspace_root.as_ref(), activation_path.as_ref())
}
