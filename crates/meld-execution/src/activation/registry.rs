//! Built-in execution activation asset registry.

use meld_lang::Method;

use super::binding::{invalid, ExecutionActivationAssets, ExecutionActivationBindingError};
use super::{
    bind_execution_activation, ExecutionActivationInput, ExecutionActivationSelection,
    MethodTaskPackageBinding,
};
use crate::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityCatalog,
    CapabilityTypeContract, ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec,
    OutputSlotSpec, ScopeContract,
};
use crate::planning::MethodLibrary;
use crate::task::package::load_builtin_task_package_spec;

const REFRESH_DOCS_METHOD_ID: &str = "refresh_docs_v1";
const REFRESH_DOCS_METHOD_SOURCE: &str = include_str!("refresh_docs_v1.json");
const WORKSPACE_SCAN_STEP_ID: &str = "scan_workspace";
const DOCS_WRITER_STEP_ID: &str = "run_docs_writer";
const DOCS_WRITER_PACKAGE_ID: &str = "docs_writer";
const DOCS_WRITER_WORKFLOW_ID: &str = "docs_writer_thread_v1";
const DOCS_WRITER_BINDING_ID: &str = "binding.refresh_docs_v1.docs_writer";

/// Source-format-neutral registry for product-owned built-in execution assets.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltInExecutionActivationRegistry;

impl BuiltInExecutionActivationRegistry {
    /// Create the immutable built-in registry.
    pub fn new() -> Self {
        Self
    }

    /// Resolve typed assets for one frozen activation selection.
    pub fn resolve(
        &self,
        selection: &ExecutionActivationSelection,
    ) -> Result<ExecutionActivationAssets, ExecutionActivationBindingError> {
        require_supported(
            "selection.method_id",
            &selection.method_id,
            REFRESH_DOCS_METHOD_ID,
        )?;
        require_supported(
            "selection.workspace_scan_step_id",
            &selection.workspace_scan_step_id,
            WORKSPACE_SCAN_STEP_ID,
        )?;
        require_supported(
            "selection.task_package_id",
            &selection.task_package_id,
            DOCS_WRITER_PACKAGE_ID,
        )?;
        require_supported(
            "selection.workflow_id",
            &selection.workflow_id,
            DOCS_WRITER_WORKFLOW_ID,
        )?;

        let method =
            serde_json::from_str::<Method>(REFRESH_DOCS_METHOD_SOURCE).map_err(|error| {
                invalid(
                    "builtin.refresh_docs_v1",
                    format!("failed to decode authored method: {error}"),
                )
            })?;
        let catalog = activation_verification_catalog()?;
        let method_library = MethodLibrary::from_methods(vec![method], &catalog);
        if !method_library.invalid.is_empty() || method_library.entries.len() != 1 {
            return Err(invalid(
                "builtin.refresh_docs_v1",
                "authored method failed execution verification",
            ));
        }

        let task_package =
            load_builtin_task_package_spec(DOCS_WRITER_PACKAGE_ID).map_err(|error| {
                invalid(
                    "builtin.docs_writer",
                    format!("failed to decode authored task package: {error}"),
                )
            })?;

        Ok(ExecutionActivationAssets {
            method_library,
            method_binding: MethodTaskPackageBinding {
                binding_id: DOCS_WRITER_BINDING_ID.to_string(),
                method_id: REFRESH_DOCS_METHOD_ID.to_string(),
                package_step_id: DOCS_WRITER_STEP_ID.to_string(),
                package_id: DOCS_WRITER_PACKAGE_ID.to_string(),
                workflow_id: DOCS_WRITER_WORKFLOW_ID.to_string(),
            },
            task_package,
        })
    }
}

/// Resolve built-in assets and bind them into a typed activation input.
pub fn bind_builtin_execution_activation(
    selection: ExecutionActivationSelection,
) -> Result<ExecutionActivationInput, ExecutionActivationBindingError> {
    let assets = BuiltInExecutionActivationRegistry::new().resolve(&selection)?;
    bind_execution_activation(selection, assets)
}

fn require_supported(
    field: &str,
    actual: &str,
    expected: &str,
) -> Result<(), ExecutionActivationBindingError> {
    if actual == expected {
        Ok(())
    } else {
        Err(invalid(field, format!("unsupported built-in id {actual}")))
    }
}

fn activation_verification_catalog() -> Result<CapabilityCatalog, ExecutionActivationBindingError> {
    let mut catalog = CapabilityCatalog::new();
    for contract in [workspace_scan_contract(), docs_writer_package_contract()] {
        catalog.register(contract).map_err(|error| {
            invalid(
                "builtin.capability_catalog",
                format!("invalid authored capability contract: {error}"),
            )
        })?;
    }
    Ok(catalog)
}

fn workspace_scan_contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: "workspace_scan".to_string(),
        capability_version: 1,
        owning_domain: "workspace".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "workspace".to_string(),
            scope_ref_kind: "workspace_root".to_string(),
            allow_fan_out: false,
        },
        binding_contract: Vec::new(),
        input_contract: Vec::new(),
        output_contract: vec![OutputSlotSpec {
            slot_id: "workspace_snapshot_ref".to_string(),
            artifact_type_id: "workspace_snapshot_ref".to_string(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: Vec::new(),
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "artifacts".to_string(),
            retry_class: "workspace_io".to_string(),
            cancellation_supported: false,
        },
    }
}

fn docs_writer_package_contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: "task_package.docs_writer".to_string(),
        capability_version: 1,
        owning_domain: "execution".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "filesystem".to_string(),
            scope_ref_kind: "target_selector".to_string(),
            allow_fan_out: true,
        },
        binding_contract: vec![
            BindingSpec {
                binding_id: "provider_binding".to_string(),
                value_kind: BindingValueKind::ProviderRef,
                required: true,
                affects_deterministic_identity: true,
            },
            BindingSpec {
                binding_id: "frame_type".to_string(),
                value_kind: BindingValueKind::Literal,
                required: true,
                affects_deterministic_identity: true,
            },
            BindingSpec {
                binding_id: "force".to_string(),
                value_kind: BindingValueKind::Literal,
                required: true,
                affects_deterministic_identity: true,
            },
        ],
        input_contract: vec![InputSlotSpec {
            slot_id: "workspace_snapshot_ref".to_string(),
            accepted_artifact_type_ids: vec!["workspace_snapshot_ref".to_string()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
            required: true,
            cardinality: InputCardinality::One,
        }],
        output_contract: vec![OutputSlotSpec {
            slot_id: "docs_patch".to_string(),
            artifact_type_id: "docs_patch".to_string(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: Vec::new(),
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Queued,
            completion_semantics: "result_or_failure".to_string(),
            retry_class: "provider_io".to_string(),
            cancellation_supported: true,
        },
    }
}
