//! Pure task package output mapping and policy contracts.
//!
//! Wave 2 resolves authored workflow outputs into typed artifact values in
//! memory. Durable task artifact persistence and replay remain Wave 4 work.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{PackageExpansionSpec, TaskPackageOutputArtifactSpec, TaskPackageSpec};

/// Output policy for one authored turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnOutputPolicySpec {
    /// True when turn output should be persisted as a frame.
    pub persist_frame: bool,
}

/// Pure typed artifact value resolved from completed workflow outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedTaskPackageOutputArtifact {
    /// Artifact type declared by the authored package mapping.
    pub artifact_type_id: String,
    /// Exact schema version declared by the authored package mapping.
    pub schema_version: u32,
    /// Source workflow output type used by the mapping.
    pub source_output_type: String,
    /// In-memory workflow output content selected by the mapping.
    pub content: String,
}

/// Fail-closed error from pure task package output mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPackageOutputMappingError {
    /// Stable package field or runtime output associated with the failure.
    pub field: String,
    /// Human-readable mapping detail.
    pub message: String,
}

impl fmt::Display for TaskPackageOutputMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl Error for TaskPackageOutputMappingError {}

/// Resolve one typed artifact mapping from authored package semantics.
pub fn resolve_task_package_output_mapping<'a>(
    package: &'a TaskPackageSpec,
    artifact_type_id: &str,
    schema_version: u32,
) -> Result<&'a TaskPackageOutputArtifactSpec, TaskPackageOutputMappingError> {
    let mappings = package
        .output_artifacts
        .iter()
        .filter(|mapping| {
            mapping.artifact_type_id == artifact_type_id && mapping.schema_version == schema_version
        })
        .collect::<Vec<_>>();
    if mappings.len() != 1 {
        return Err(invalid(
            "task_package.output_artifacts",
            "must contain exactly one mapping for the requested artifact and schema",
        ));
    }

    let mapping = mappings[0];
    let source_turns = package
        .expansions
        .iter()
        .flat_map(|expansion| match expansion {
            PackageExpansionSpec::TraversalPrerequisite(spec) => spec.repeated_region.turns.iter(),
        })
        .filter(|turn| {
            turn.output_type == mapping.source_output_type && turn.output_policy.persist_frame
        })
        .count();
    if source_turns != 1 {
        return Err(invalid(
            "task_package.output_artifacts.source_output_type",
            "must identify exactly one persisted authored workflow output",
        ));
    }

    Ok(mapping)
}

/// Map completed workflow output content into a typed in-memory artifact.
///
/// This function performs no store access and does not persist the returned
/// value. The restart-safe task artifact commit boundary remains Wave 4 work.
pub fn map_task_package_output_artifact(
    package: &TaskPackageSpec,
    artifact_type_id: &str,
    schema_version: u32,
    completed_outputs: &BTreeMap<String, String>,
) -> Result<MappedTaskPackageOutputArtifact, TaskPackageOutputMappingError> {
    let mapping = resolve_task_package_output_mapping(package, artifact_type_id, schema_version)?;
    let content = completed_outputs
        .get(&mapping.source_output_type)
        .ok_or_else(|| {
            invalid(
                "completed_outputs",
                format!(
                    "missing mapped workflow output {}",
                    mapping.source_output_type
                ),
            )
        })?;

    Ok(MappedTaskPackageOutputArtifact {
        artifact_type_id: mapping.artifact_type_id.clone(),
        schema_version: mapping.schema_version,
        source_output_type: mapping.source_output_type.clone(),
        content: content.clone(),
    })
}

fn invalid(field: impl Into<String>, message: impl Into<String>) -> TaskPackageOutputMappingError {
    TaskPackageOutputMappingError {
        field: field.into(),
        message: message.into(),
    }
}
