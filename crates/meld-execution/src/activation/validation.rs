//! Pure validation and receipt derivation for execution activation.

use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::path::{Component, Path};

use meld_lang::{Composition, StepKind, Term};
use serde::Serialize;

use super::{ExecutionActivationInput, ExecutionActivationValidationReceipt};
use crate::planning::OperatorResolutionStatus;
use crate::task::package::{PackageExpansionSpec, TargetSelectorKind};
use crate::task_network::store::network_storage_key;

const SUCCESS_EVENT_TYPE: &str = "execution.task.succeeded";
const FAILURE_EVENT_TYPE: &str = "execution.task.failed";
const CONTENT_SOURCE_KIND: &str = "content_written";
const WORKSPACE_SCAN_CAPABILITY_TYPE_ID: &str = "workspace_scan";
const WORKSPACE_SCAN_CAPABILITY_VERSION: u32 = 1;
const DOCS_WRITER_PACKAGE_ID: &str = "docs_writer";
const DOCS_WRITER_WORKFLOW_ID: &str = "docs_writer_thread_v1";
const REFRESH_DOCS_METHOD_ID: &str = "refresh_docs_v1";
const WORKSPACE_SCAN_STEP_ID: &str = "scan_workspace";
const DOCS_WRITER_STEP_ID: &str = "run_docs_writer";
const DOCS_WRITER_BINDING_ID: &str = "binding.refresh_docs_v1.docs_writer";
const DOCS_WRITER_CAPABILITY_TYPE_ID: &str = "task_package.docs_writer";
const DOCS_WRITER_CAPABILITY_VERSION: u32 = 1;
const DOCS_PATCH_ARTIFACT_TYPE_ID: &str = "docs_patch";
const DOCS_PATCH_SCHEMA_VERSION: u32 = 1;
const WORKSPACE_SNAPSHOT_ARTIFACT_TYPE_ID: &str = "workspace_snapshot_ref";
const DOCS_WRITER_REQUIRED_FIELDS: [&str; 4] =
    ["agent_id", "provider_binding", "frame_type", "force"];

/// Structured failure returned before execution semantic stores are opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionActivationValidationError {
    /// Stable field path associated with the failure.
    pub field: String,
    /// Human-readable validation detail.
    pub message: String,
}

impl fmt::Display for ExecutionActivationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl Error for ExecutionActivationValidationError {}

/// Validate typed execution inputs without opening or mutating semantic stores.
pub fn validate_execution_activation(
    input: &ExecutionActivationInput,
) -> Result<ExecutionActivationValidationReceipt, ExecutionActivationValidationError> {
    require_blake3_hash(
        "selection.activation_hash",
        &input.selection.activation_hash,
    )?;
    require_text("selection.activation_id", &input.selection.activation_id)?;
    require_text("selection.method_id", &input.selection.method_id)?;
    require_text(
        "selection.task_package_id",
        &input.selection.task_package_id,
    )?;
    require_text("selection.workflow_id", &input.selection.workflow_id)?;
    require_text(
        "selection.workspace_scan_step_id",
        &input.selection.workspace_scan_step_id,
    )?;
    require_text(
        "selection.provider_binding_ref",
        &input.selection.provider_binding_ref,
    )?;
    require_text("selection.frame_type", &input.selection.frame_type)?;
    require_text(
        "selection.target.canonical_value",
        &input.selection.target.canonical_value,
    )?;
    require_structural_id(
        "selection.provider_binding_ref",
        &input.selection.provider_binding_ref,
    )?;
    require_structural_id("selection.frame_type", &input.selection.frame_type)?;
    validate_target(input)?;
    require_text(
        "method_binding.binding_id",
        &input.method_binding.binding_id,
    )?;
    require_text("method_binding.method_id", &input.method_binding.method_id)?;
    require_text(
        "method_binding.package_step_id",
        &input.method_binding.package_step_id,
    )?;
    require_text(
        "method_binding.package_id",
        &input.method_binding.package_id,
    )?;
    require_text(
        "method_binding.workflow_id",
        &input.method_binding.workflow_id,
    )?;
    require_text(
        "selection.task_network_id",
        &input.selection.task_network_id,
    )?;
    require_structural_id(
        "selection.task_network_id",
        &input.selection.task_network_id,
    )?;
    require_text(
        "selection.required_artifact.artifact_type_id",
        &input.selection.required_artifact.artifact_type_id,
    )?;
    if input.selection.required_artifact.schema_version == 0 {
        return Err(invalid(
            "selection.required_artifact.schema_version",
            "must be greater than zero",
        ));
    }
    validate_publication_mapping(input)?;
    validate_package(input)?;
    validate_method(input)?;

    let task_network_storage_key = network_storage_key(&input.selection.task_network_id)
        .map_err(|error| invalid("selection.task_network_id", error.to_string()))?;
    let method_library_digest = input.method_library.semantic_digest();
    let method_binding_digest = semantic_digest(&input.method_binding)?;
    let task_package_digest = input.task_package.semantic_digest();
    let artifact_contract_digest = semantic_digest(&input.selection.required_artifact)?;
    let workspace_scan_contract_digest = semantic_digest(&input.selection.workspace_scan)?;
    let publication_mapping_digest = semantic_digest(&input.selection.publication)?;

    #[derive(Serialize)]
    struct ExecutionCoordinatesProjection<'a> {
        provider_binding_ref: &'a str,
        frame_type: &'a str,
        force_policy: &'a super::ExecutionForcePolicy,
        target: &'a super::ExecutionTargetSelector,
    }

    let execution_coordinates_digest = semantic_digest(&ExecutionCoordinatesProjection {
        provider_binding_ref: &input.selection.provider_binding_ref,
        frame_type: &input.selection.frame_type,
        force_policy: &input.selection.force_policy,
        target: &input.selection.target,
    })?;

    #[derive(Serialize)]
    struct TaskNetworkIdentityProjection<'a> {
        task_network_id: &'a str,
        task_network_storage_key: &'a str,
    }

    let task_network_identity_digest = semantic_digest(&TaskNetworkIdentityProjection {
        task_network_id: &input.selection.task_network_id,
        task_network_storage_key: &task_network_storage_key,
    })?;

    #[derive(Serialize)]
    struct IdentityProjection<'a> {
        selection: &'a super::ExecutionActivationSelection,
        method_binding: &'a super::MethodTaskPackageBinding,
        method_library_digest: &'a str,
        task_package_digest: &'a str,
        task_network_identity_digest: &'a str,
    }

    let input_hash = semantic_digest(&IdentityProjection {
        selection: &input.selection,
        method_binding: &input.method_binding,
        method_library_digest: &method_library_digest,
        task_package_digest: &task_package_digest,
        task_network_identity_digest: &task_network_identity_digest,
    })?;

    Ok(ExecutionActivationValidationReceipt {
        receipt_id: format!("execution-activation-{input_hash}"),
        activation_hash: input.selection.activation_hash.clone(),
        activation_id: input.selection.activation_id.clone(),
        input_hash,
        method_binding_id: input.method_binding.binding_id.clone(),
        method_binding_digest,
        method_id: input.method_binding.method_id.clone(),
        method_library_digest,
        task_package_id: input.task_package.package_id.clone(),
        task_package_digest,
        task_network_id: input.selection.task_network_id.clone(),
        task_network_storage_key,
        task_network_identity_digest,
        artifact_contract_digest,
        workspace_scan_contract_digest,
        execution_coordinates_digest,
        publication_mapping_id: input.selection.publication.mapping_id.clone(),
        publication_mapping_digest,
    })
}

fn validate_package(
    input: &ExecutionActivationInput,
) -> Result<(), ExecutionActivationValidationError> {
    if input.selection.required_artifact.artifact_type_id != DOCS_PATCH_ARTIFACT_TYPE_ID {
        return Err(invalid(
            "selection.required_artifact.artifact_type_id",
            format!("must equal {DOCS_PATCH_ARTIFACT_TYPE_ID}"),
        ));
    }
    if input.selection.required_artifact.schema_version != DOCS_PATCH_SCHEMA_VERSION {
        return Err(invalid(
            "selection.required_artifact.schema_version",
            format!("must equal {DOCS_PATCH_SCHEMA_VERSION}"),
        ));
    }
    if input.selection.task_package_id != DOCS_WRITER_PACKAGE_ID {
        return Err(invalid(
            "selection.task_package_id",
            format!("must equal {DOCS_WRITER_PACKAGE_ID}"),
        ));
    }
    if input.task_package.workflow_id != DOCS_WRITER_WORKFLOW_ID {
        return Err(invalid(
            "task_package.workflow_id",
            format!("must equal {DOCS_WRITER_WORKFLOW_ID}"),
        ));
    }
    if input.selection.workflow_id != DOCS_WRITER_WORKFLOW_ID {
        return Err(invalid(
            "selection.workflow_id",
            format!("must equal {DOCS_WRITER_WORKFLOW_ID}"),
        ));
    }
    if input.task_package.package_id != DOCS_WRITER_PACKAGE_ID {
        return Err(invalid(
            "task_package.package_id",
            format!("must equal {DOCS_WRITER_PACKAGE_ID}"),
        ));
    }
    if input.method_binding.package_id != input.task_package.package_id {
        return Err(invalid(
            "method_binding.package_id",
            "must equal task_package.package_id",
        ));
    }
    if input.method_binding.package_id != input.selection.task_package_id {
        return Err(invalid(
            "method_binding.package_id",
            "must equal selection.task_package_id",
        ));
    }
    if input.method_binding.workflow_id != input.task_package.workflow_id {
        return Err(invalid(
            "method_binding.workflow_id",
            "must equal task_package.workflow_id",
        ));
    }
    if input.method_binding.workflow_id != input.selection.workflow_id {
        return Err(invalid(
            "method_binding.workflow_id",
            "must equal selection.workflow_id",
        ));
    }
    if input.task_package.expansions.is_empty() {
        return Err(invalid(
            "task_package.expansions",
            "must contain at least one authored expansion",
        ));
    }
    let required_fields = input
        .task_package
        .trigger
        .required_runtime_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if required_fields.len() != input.task_package.trigger.required_runtime_fields.len()
        || required_fields != BTreeSet::from(DOCS_WRITER_REQUIRED_FIELDS)
    {
        return Err(invalid(
            "task_package.trigger.required_runtime_fields",
            "must equal the canonical docs-writer runtime field set",
        ));
    }

    let accepted_target = match input.selection.target.kind {
        super::ExecutionTargetKind::Path => TargetSelectorKind::Path,
        super::ExecutionTargetKind::NodeId => TargetSelectorKind::NodeId,
    };
    if !input
        .task_package
        .trigger
        .accepted_targets
        .contains(&accepted_target)
    {
        return Err(invalid(
            "selection.target.kind",
            "must be accepted by the docs-writer task package",
        ));
    }

    let output_mappings = input
        .task_package
        .output_artifacts
        .iter()
        .filter(|mapping| {
            mapping.artifact_type_id == input.selection.required_artifact.artifact_type_id
                && mapping.schema_version == input.selection.required_artifact.schema_version
        })
        .collect::<Vec<_>>();
    if input.task_package.output_artifacts.len() != 1 || output_mappings.len() != 1 {
        return Err(invalid(
            "task_package.output_artifacts",
            "must contain exactly one mapping for the required artifact and schema",
        ));
    }
    let source_output_type = output_mappings[0].source_output_type.as_str();
    let source_turns = input
        .task_package
        .expansions
        .iter()
        .flat_map(|expansion| match expansion {
            PackageExpansionSpec::TraversalPrerequisite(spec) => spec.repeated_region.turns.iter(),
        })
        .filter(|turn| turn.output_type == source_output_type && turn.output_policy.persist_frame)
        .count();
    if source_turns != 1 {
        return Err(invalid(
            "task_package.output_artifacts.source_output_type",
            "must identify exactly one persisted authored workflow output",
        ));
    }
    Ok(())
}

fn validate_method(
    input: &ExecutionActivationInput,
) -> Result<(), ExecutionActivationValidationError> {
    if input.selection.method_id != REFRESH_DOCS_METHOD_ID {
        return Err(invalid(
            "selection.method_id",
            format!("must equal {REFRESH_DOCS_METHOD_ID}"),
        ));
    }
    if input.selection.workspace_scan_step_id != WORKSPACE_SCAN_STEP_ID {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            format!("must equal {WORKSPACE_SCAN_STEP_ID}"),
        ));
    }
    if input.method_binding.binding_id != DOCS_WRITER_BINDING_ID {
        return Err(invalid(
            "method_binding.binding_id",
            format!("must equal {DOCS_WRITER_BINDING_ID}"),
        ));
    }
    if input.method_binding.package_step_id != DOCS_WRITER_STEP_ID {
        return Err(invalid(
            "method_binding.package_step_id",
            format!("must equal {DOCS_WRITER_STEP_ID}"),
        ));
    }
    if !input.method_library.invalid.is_empty() {
        return Err(invalid(
            "method_library.invalid",
            "must be empty for product activation",
        ));
    }
    if input.method_library.entries.len() != 1 {
        return Err(invalid(
            "method_library.entries",
            "must contain exactly one product activation method",
        ));
    }
    let matching = input
        .method_library
        .entries
        .iter()
        .filter(|entry| entry.method.method_id == input.method_binding.method_id)
        .collect::<Vec<_>>();
    if input.method_binding.method_id != input.selection.method_id {
        return Err(invalid(
            "method_binding.method_id",
            "must equal selection.method_id",
        ));
    }
    if matching.len() != 1 {
        return Err(invalid(
            "method_binding.method_id",
            "must select exactly one verified method",
        ));
    }
    let entry = matching[0];
    let package_step = entry
        .method
        .composition
        .steps
        .iter()
        .find(|step| step.step_id == input.method_binding.package_step_id)
        .ok_or_else(|| {
            invalid(
                "method_binding.package_step_id",
                "must select a step in the activated method",
            )
        })?;
    let StepKind::Op(package_operator) = &package_step.kind else {
        return Err(invalid(
            "method_binding.package_step_id",
            "must select an operator step",
        ));
    };
    if package_operator
        .resolution
        .specific
        .as_ref()
        .map(|reference| {
            (
                reference.capability_type_id.as_str(),
                reference.capability_version,
            )
        })
        != Some((
            DOCS_WRITER_CAPABILITY_TYPE_ID,
            DOCS_WRITER_CAPABILITY_VERSION,
        ))
    {
        return Err(invalid(
            "method_binding.package_step_id",
            "must resolve the canonical docs-writer package capability",
        ));
    }
    let package_resolutions = entry
        .verification
        .operator_resolutions
        .iter()
        .filter(|report| report.operator_id == package_operator.operator_id)
        .collect::<Vec<_>>();
    if package_resolutions.len() != 1
        || package_resolutions[0].status != OperatorResolutionStatus::Resolved
        || package_resolutions[0].capability_type_id.as_deref()
            != Some(DOCS_WRITER_CAPABILITY_TYPE_ID)
        || package_resolutions[0].capability_version != Some(DOCS_WRITER_CAPABILITY_VERSION)
    {
        return Err(invalid(
            "method_binding.package_step_id",
            "package step must have one normalized docs-writer capability resolution",
        ));
    }
    let required_artifact =
        Term::ArtifactType(input.selection.required_artifact.artifact_type_id.clone());
    if !package_operator
        .resolution
        .requires_outputs
        .iter()
        .any(|slot| slot.required && slot.artifact_type == required_artifact)
    {
        return Err(invalid(
            "selection.required_artifact.artifact_type_id",
            "must be required by the package-bound method step",
        ));
    }

    if input.selection.workspace_scan.capability_type_id != WORKSPACE_SCAN_CAPABILITY_TYPE_ID {
        return Err(invalid(
            "selection.workspace_scan.capability_type_id",
            "must identify the real workspace_scan capability",
        ));
    }
    if input.selection.workspace_scan.capability_version != WORKSPACE_SCAN_CAPABILITY_VERSION {
        return Err(invalid(
            "selection.workspace_scan.capability_version",
            format!("must equal {WORKSPACE_SCAN_CAPABILITY_VERSION}"),
        ));
    }

    let scan_step = entry
        .method
        .composition
        .steps
        .iter()
        .find(|step| step.step_id == input.selection.workspace_scan_step_id)
        .ok_or_else(|| {
            invalid(
                "selection.workspace_scan_step_id",
                "must select a step in the activated method",
            )
        })?;
    let StepKind::Op(scan_operator) = &scan_step.kind else {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            "must select an operator step",
        ));
    };
    if scan_operator.resolution.specific.as_ref().map(|reference| {
        (
            reference.capability_type_id.as_str(),
            reference.capability_version,
        )
    }) != Some((
        WORKSPACE_SCAN_CAPABILITY_TYPE_ID,
        WORKSPACE_SCAN_CAPABILITY_VERSION,
    )) {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            "must name an operator authored against workspace_scan version one",
        ));
    }
    let produces_snapshot = scan_operator
        .resolution
        .requires_outputs
        .iter()
        .any(|slot| {
            slot.required
                && slot.artifact_type
                    == Term::ArtifactType(WORKSPACE_SNAPSHOT_ARTIFACT_TYPE_ID.to_string())
        });
    let consumes_snapshot = package_operator
        .resolution
        .requires_inputs
        .iter()
        .any(|slot| {
            slot.required
                && slot.artifact_type
                    == Term::ArtifactType(WORKSPACE_SNAPSHOT_ARTIFACT_TYPE_ID.to_string())
        });
    if !produces_snapshot || !consumes_snapshot {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            "scan output must feed the package through workspace_snapshot_ref",
        ));
    }
    let matching_resolutions = entry
        .verification
        .operator_resolutions
        .iter()
        .filter(|report| report.operator_id == scan_operator.operator_id)
        .collect::<Vec<_>>();
    if matching_resolutions.len() != 1 {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            "must identify an operator with exactly one verification report",
        ));
    }
    let scan_resolution = matching_resolutions[0];
    if scan_resolution.status != OperatorResolutionStatus::Resolved
        || scan_resolution.capability_type_id.as_deref()
            != Some(input.selection.workspace_scan.capability_type_id.as_str())
        || scan_resolution.capability_version
            != Some(input.selection.workspace_scan.capability_version)
    {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            "selected step must resolve the configured workspace scan contract",
        ));
    }
    if !has_dependency_path(
        &entry.method.composition,
        &input.selection.workspace_scan_step_id,
        &input.method_binding.package_step_id,
    ) {
        return Err(invalid(
            "selection.workspace_scan_step_id",
            "selected scan step must have a dependency path to the package-bound step",
        ));
    }
    Ok(())
}

fn validate_target(
    input: &ExecutionActivationInput,
) -> Result<(), ExecutionActivationValidationError> {
    match input.selection.target.kind {
        super::ExecutionTargetKind::Path => {
            let path = Path::new(&input.selection.target.canonical_value);
            if !path.is_absolute()
                || path
                    .components()
                    .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
            {
                return Err(invalid(
                    "selection.target.canonical_value",
                    "path targets must be absolute and lexically canonical",
                ));
            }
        }
        super::ExecutionTargetKind::NodeId => require_structural_id(
            "selection.target.canonical_value",
            &input.selection.target.canonical_value,
        )?,
    }
    Ok(())
}

fn has_dependency_path(composition: &Composition, from: &str, to: &str) -> bool {
    let mut pending = VecDeque::from([from]);
    let mut visited = BTreeSet::new();

    while let Some(step_id) = pending.pop_front() {
        if !visited.insert(step_id) {
            continue;
        }
        for edge in composition.edges.iter().filter(|edge| edge.from == step_id) {
            if edge.to == to {
                return true;
            }
            pending.push_back(edge.to.as_str());
        }
    }
    false
}

fn validate_publication_mapping(
    input: &ExecutionActivationInput,
) -> Result<(), ExecutionActivationValidationError> {
    require_text(
        "selection.publication.mapping_id",
        &input.selection.publication.mapping_id,
    )?;
    require_structural_id(
        "selection.publication.mapping_id",
        &input.selection.publication.mapping_id,
    )?;
    require_text(
        "selection.publication.content_source_kind",
        &input.selection.publication.content_source_kind,
    )?;
    if input.selection.publication.content_source_kind != CONTENT_SOURCE_KIND {
        return Err(invalid(
            "selection.publication.content_source_kind",
            format!("must equal {CONTENT_SOURCE_KIND}"),
        ));
    }
    if input.selection.publication.success_event_type != SUCCESS_EVENT_TYPE {
        return Err(invalid(
            "selection.publication.success_event_type",
            format!("must equal {SUCCESS_EVENT_TYPE}"),
        ));
    }
    if input.selection.publication.failure_event_type != FAILURE_EVENT_TYPE {
        return Err(invalid(
            "selection.publication.failure_event_type",
            format!("must equal {FAILURE_EVENT_TYPE}"),
        ));
    }
    Ok(())
}

fn semantic_digest(value: &impl Serialize) -> Result<String, ExecutionActivationValidationError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| invalid("activation_input", error.to_string()))?;
    Ok(blake3::hash(&encoded).to_hex().to_string())
}

fn require_blake3_hash(field: &str, value: &str) -> Result<(), ExecutionActivationValidationError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(invalid(
            field,
            "must be a lowercase BLAKE3 hexadecimal digest",
        ))
    }
}

fn require_text(field: &str, value: &str) -> Result<(), ExecutionActivationValidationError> {
    if value.trim().is_empty() {
        Err(invalid(field, "must be non-empty"))
    } else {
        Ok(())
    }
}

fn require_structural_id(
    field: &str,
    value: &str,
) -> Result<(), ExecutionActivationValidationError> {
    let bytes = value.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 128
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        });
    if valid {
        Ok(())
    } else {
        Err(invalid(
            field,
            "must be a lowercase structural identifier of at most 128 bytes",
        ))
    }
}

fn invalid(
    field: impl Into<String>,
    message: impl Into<String>,
) -> ExecutionActivationValidationError {
    ExecutionActivationValidationError {
        field: field.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use meld_lang::{CostEstimate, Edge, EdgeKind, Operator, Resolution, Step};

    use super::*;
    use crate::activation::{
        bind_builtin_execution_activation, CapabilityContractRef, ExecutionActivationSelection,
        ExecutionForcePolicy, ExecutionTargetKind, ExecutionTargetSelector, PublicationMapping,
        RequiredArtifactContract,
    };
    use crate::planning::MethodSourceRef;

    fn selection() -> ExecutionActivationSelection {
        ExecutionActivationSelection {
            activation_hash: "a".repeat(64),
            activation_id: "activation.docs_freshness".to_string(),
            method_id: "refresh_docs_v1".to_string(),
            task_package_id: "docs_writer".to_string(),
            workflow_id: "docs_writer_thread_v1".to_string(),
            workspace_scan_step_id: "scan_workspace".to_string(),
            workspace_scan: CapabilityContractRef {
                capability_type_id: "workspace_scan".to_string(),
                capability_version: 1,
            },
            task_network_id: "network-docs".to_string(),
            required_artifact: RequiredArtifactContract {
                artifact_type_id: "docs_patch".to_string(),
                schema_version: 1,
            },
            provider_binding_ref: "provider.docs".to_string(),
            frame_type: "analysis".to_string(),
            force_policy: ExecutionForcePolicy::ReuseExisting,
            target: ExecutionTargetSelector {
                kind: ExecutionTargetKind::Path,
                canonical_value: "/workspace".to_string(),
            },
            publication: PublicationMapping {
                mapping_id: "publication.docs_freshness".to_string(),
                success_event_type: "execution.task.succeeded".to_string(),
                failure_event_type: "execution.task.failed".to_string(),
                content_source_kind: "content_written".to_string(),
            },
        }
    }

    fn input() -> ExecutionActivationInput {
        bind_builtin_execution_activation(selection()).unwrap()
    }

    #[test]
    fn validation_returns_a_deterministic_complete_receipt() {
        let input = input();
        let first = validate_execution_activation(&input).unwrap();
        let second = validate_execution_activation(&input).unwrap();

        #[derive(Serialize)]
        struct NetworkProjection<'a> {
            task_network_id: &'a str,
            task_network_storage_key: &'a str,
        }

        assert_eq!(first, second);
        assert_eq!(first.activation_id, "activation.docs_freshness");
        assert_eq!(first.method_id, "refresh_docs_v1");
        assert_eq!(first.task_package_id, "docs_writer");
        assert_eq!(first.task_network_storage_key, "network-docs");
        assert_eq!(first.task_network_identity_digest.len(), 64);
        assert_eq!(
            first.task_network_identity_digest,
            semantic_digest(&NetworkProjection {
                task_network_id: "network-docs",
                task_network_storage_key: "network-docs",
            })
            .unwrap()
        );
        assert_eq!(first.input_hash.len(), 64);
    }

    #[test]
    fn semantic_method_digest_excludes_source_provenance() {
        let first = input();
        let mut second = first.clone();
        second.method_library.entries[0].source_ref = MethodSourceRef::File {
            path: "/different/source/method.json".to_string(),
        };

        assert_eq!(
            validate_execution_activation(&first)
                .unwrap()
                .method_library_digest,
            validate_execution_activation(&second)
                .unwrap()
                .method_library_digest
        );
        assert_eq!(
            validate_execution_activation(&first).unwrap().input_hash,
            validate_execution_activation(&second).unwrap().input_hash
        );
    }

    #[test]
    fn semantic_method_digest_binds_resolution_status() {
        let mut value = input();
        let baseline = value.method_library.semantic_digest();
        let scan_resolution = value.method_library.entries[0]
            .verification
            .operator_resolutions
            .iter_mut()
            .find(|report| report.operator_id == "scan_workspace")
            .unwrap();
        scan_resolution.status = OperatorResolutionStatus::Unresolved;

        assert_ne!(value.method_library.semantic_digest(), baseline);
    }

    #[test]
    fn semantic_method_digest_binds_capability_identity_and_version() {
        let baseline = input().method_library.semantic_digest();

        let mut type_drift = input();
        let scan_resolution = type_drift.method_library.entries[0]
            .verification
            .operator_resolutions
            .iter_mut()
            .find(|report| report.operator_id == "scan_workspace")
            .unwrap();
        scan_resolution.capability_type_id = Some("workspace_scan_other".to_string());
        assert_ne!(type_drift.method_library.semantic_digest(), baseline);

        let mut version_drift = input();
        let scan_resolution = version_drift.method_library.entries[0]
            .verification
            .operator_resolutions
            .iter_mut()
            .find(|report| report.operator_id == "scan_workspace")
            .unwrap();
        scan_resolution.capability_version = Some(2);
        assert_ne!(version_drift.method_library.semantic_digest(), baseline);
    }

    #[test]
    fn semantic_method_digest_normalizes_verification_report_order() {
        let first = input();
        let mut second = first.clone();
        second.method_library.entries[0]
            .verification
            .operator_resolutions
            .reverse();

        assert_eq!(
            first.method_library.semantic_digest(),
            second.method_library.semantic_digest()
        );
    }

    #[test]
    fn validation_requires_real_workspace_scan_resolution() {
        let mut value = input();
        value.selection.workspace_scan.capability_version = 2;

        let error = validate_execution_activation(&value).unwrap_err();
        assert_eq!(error.field, "selection.workspace_scan.capability_version");
    }

    #[test]
    fn validation_rejects_a_disconnected_workspace_scan_step() {
        let mut value = input();
        value.method_library.entries[0]
            .method
            .composition
            .edges
            .clear();

        let error = validate_execution_activation(&value).unwrap_err();
        assert_eq!(error.field, "selection.workspace_scan_step_id");
        assert!(error.message.contains("dependency path"));
    }

    #[test]
    fn validation_rejects_a_decoy_scan_step() {
        let mut value = input();
        let composition = &mut value.method_library.entries[0].method.composition;
        composition.steps[0].step_id = "actual_scan".to_string();
        composition.edges[0].from = "actual_scan".to_string();
        composition.steps.push(Step {
            step_id: "scan_workspace".to_string(),
            kind: StepKind::Op(Operator {
                operator_id: "decoy".to_string(),
                preconditions: Vec::new(),
                effects: Vec::new(),
                cost: CostEstimate::zero(),
                resolution: Resolution {
                    requires_inputs: Vec::new(),
                    requires_outputs: Vec::new(),
                    scope_kind: Some("repository".to_string()),
                    tags: Vec::new(),
                    specific: None,
                },
            }),
        });
        composition.edges.push(Edge {
            from: "scan_workspace".to_string(),
            to: "run_docs_writer".to_string(),
            kind: EdgeKind::Ordering,
        });

        let error = validate_execution_activation(&value).unwrap_err();
        assert_eq!(error.field, "selection.workspace_scan_step_id");
        assert!(error.message.contains("workspace_scan version one"));
    }

    #[test]
    fn validation_rejects_publication_drift() {
        let mut value = input();
        value.selection.publication.success_event_type = "task_succeeded".to_string();

        let error = validate_execution_activation(&value).unwrap_err();
        assert_eq!(error.field, "selection.publication.success_event_type");
    }

    #[test]
    fn root_selection_contains_no_bound_assets_or_source_provenance() {
        let value = serde_json::to_value(input().selection).unwrap();
        let object = value.as_object().unwrap();

        assert!(!object.contains_key("method_library"));
        assert!(!object.contains_key("task_package"));
        assert!(!object.contains_key("source_path"));
        assert!(!object.contains_key("source_format"));
    }

    #[test]
    fn validation_rejects_execution_binder_drift_from_selection() {
        let mut value = input();
        value.method_binding.method_id = "other_method".to_string();

        let error = validate_execution_activation(&value).unwrap_err();
        assert_eq!(error.field, "method_binding.method_id");
    }

    #[test]
    fn contracts_reject_unknown_fields() {
        let mut value = serde_json::to_value(input()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("source_format".to_string(), serde_json::json!("toml"));

        assert!(serde_json::from_value::<ExecutionActivationInput>(value).is_err());
    }
}
