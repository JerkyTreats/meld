//! Source-neutral binding from activation selection to execution assets.

use std::error::Error;
use std::fmt;

use super::{
    ExecutionActivationInput, ExecutionActivationSelection, ExecutionActivationValidationContext,
    MethodTaskPackageBinding,
};
use crate::capability::CapabilityCatalog;
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

/// Execution-owned runtime assets resolved from the same verified registry.
///
/// Activation identity continues to cover [`ExecutionActivationAssets`]
/// exactly as before. The additional catalog is the in-memory semantic input
/// used to verify the method library and later construct planning and lowering
/// runtimes without root rebuilding execution policy.
#[derive(Debug, Clone)]
pub struct ExecutionRuntimeAssets {
    /// Durable activation assets whose existing digests remain authoritative.
    pub activation: ExecutionActivationAssets,
    /// Exact capability catalog used to verify the selected method library.
    pub capability_catalog: CapabilityCatalog,
}

impl ExecutionRuntimeAssets {
    /// Consume runtime assets and retain the unchanged activation product.
    pub fn into_activation_assets(self) -> ExecutionActivationAssets {
        self.activation
    }
}

impl ExecutionActivationAssets {
    /// Derive the source-neutral semantic seal for all registry-owned assets.
    pub fn semantic_digests(&self) -> ExecutionActivationAssetDigests {
        let encoded_binding = serde_json::to_vec(&self.method_binding)
            .expect("method binding must remain serializable for activation identity");
        ExecutionActivationAssetDigests {
            method_library_digest: self.method_library.semantic_digest(),
            method_binding_digest: blake3::hash(&encoded_binding).to_hex().to_string(),
            task_package_digest: self.task_package.semantic_digest(),
        }
    }
}

/// Canonical source-neutral semantic seal for registry-owned activation assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionActivationAssetDigests {
    /// Complete authored method semantics plus normalized verification.
    pub method_library_digest: String,
    /// Complete explicit method-to-package binding semantics.
    pub method_binding_digest: String,
    /// Complete typed package semantics including seed and expansion authoring.
    pub task_package_digest: String,
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
    validation_context: ExecutionActivationValidationContext,
) -> Result<ExecutionActivationInput, ExecutionActivationBindingError> {
    if !validation_context.contains_provider_binding(&selection.provider_binding_ref) {
        return Err(invalid(
            "selection.provider_binding_ref",
            "must identify a configured repository provider binding",
        ));
    }
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
        validation_context,
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
