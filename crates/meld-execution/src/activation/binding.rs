//! Source-neutral binding from activation selection to execution assets.

use std::error::Error;
use std::fmt;

use super::{ExecutionActivationInput, ExecutionActivationSelection, MethodTaskPackageBinding};
use crate::planning::MethodLibrary;
use crate::task::package::TaskPackageSpec;

/// Typed execution assets resolved independently from activation source format.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionActivationAssets {
    /// Verified semantic method library selected for activation.
    pub method_library: MethodLibrary,
    /// Explicit selected-method step to task-package binding.
    pub method_binding: MethodTaskPackageBinding,
    /// Typed authored task package selected by the binding.
    pub task_package: TaskPackageSpec,
}

/// Fail-closed error returned while resolving or binding execution assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionActivationBindingError {
    /// Stable selection or asset field associated with the failure.
    pub field: String,
    /// Human-readable binding detail.
    pub message: String,
}

impl fmt::Display for ExecutionActivationBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl Error for ExecutionActivationBindingError {}

/// Bind one frozen selection to already resolved typed execution assets.
///
/// This boundary checks identity agreement only. Call
/// [`super::validate_execution_activation`] on the returned input to perform
/// complete semantic validation and derive the deterministic receipt.
pub fn bind_execution_activation(
    selection: ExecutionActivationSelection,
    assets: ExecutionActivationAssets,
) -> Result<ExecutionActivationInput, ExecutionActivationBindingError> {
    if !assets.method_library.invalid.is_empty() {
        return Err(invalid(
            "assets.method_library.invalid",
            "resolved method assets must contain no invalid entries",
        ));
    }

    let selected_methods = assets
        .method_library
        .entries
        .iter()
        .filter(|entry| entry.method.method_id == selection.method_id)
        .collect::<Vec<_>>();
    if selected_methods.len() != 1 {
        return Err(invalid(
            "assets.method_library.entries",
            "must contain exactly one method matching selection.method_id",
        ));
    }
    if assets.method_library.entries.len() != 1 {
        return Err(invalid(
            "assets.method_library.entries",
            "built-in activation assets must contain exactly one method",
        ));
    }
    let selected_method = selected_methods[0];
    if assets.method_binding.method_id != selection.method_id {
        return Err(invalid(
            "assets.method_binding.method_id",
            "must equal selection.method_id",
        ));
    }
    if assets.method_binding.package_id != selection.task_package_id {
        return Err(invalid(
            "assets.method_binding.package_id",
            "must equal selection.task_package_id",
        ));
    }
    if assets.method_binding.workflow_id != selection.workflow_id {
        return Err(invalid(
            "assets.method_binding.workflow_id",
            "must equal selection.workflow_id",
        ));
    }
    if !selected_method
        .method
        .composition
        .steps
        .iter()
        .any(|step| step.step_id == selection.workspace_scan_step_id)
    {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            "must select a step in the resolved method",
        ));
    }
    if !selected_method
        .method
        .composition
        .steps
        .iter()
        .any(|step| step.step_id == assets.method_binding.package_step_id)
    {
        return Err(invalid(
            "assets.method_binding.package_step_id",
            "must select a step in the resolved method",
        ));
    }
    if assets.task_package.package_id != selection.task_package_id {
        return Err(invalid(
            "assets.task_package.package_id",
            "must equal selection.task_package_id",
        ));
    }
    if assets.task_package.workflow_id != selection.workflow_id {
        return Err(invalid(
            "assets.task_package.workflow_id",
            "must equal selection.workflow_id",
        ));
    }

    Ok(ExecutionActivationInput {
        selection,
        method_library: assets.method_library,
        method_binding: assets.method_binding,
        task_package: assets.task_package,
    })
}

pub(crate) fn invalid(
    field: impl Into<String>,
    message: impl Into<String>,
) -> ExecutionActivationBindingError {
    ExecutionActivationBindingError {
        field: field.into(),
        message: message.into(),
    }
}
