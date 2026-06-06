//! Task initialization source validation and materialization.
//!
//! Owner: task network.
//! Inputs: reduced network state and task node initialization source records.
//! Outputs: validated task initialization payloads for dispatch.
//! Does not own: this module does not execute task-local capability graphs.

use crate::error::ApiError;
use crate::task::{validate_task_initialization, InitArtifactValue, TaskInitializationPayload};
use crate::task_network::state::{
    DependencyKind, NetworkState, StaticSeedInitSource, TaskInitSource, TaskNode, TaskStatus,
    UpstreamArtifactInitSource,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Fully materialized dispatch initialization payload with source provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterializedTaskInitialization {
    /// Final payload accepted by the task executor.
    pub payload: TaskInitializationPayload,
    /// Source provenance for every materialized init artifact.
    pub provenance: Vec<MaterializedInitSource>,
}

/// Source provenance for one materialized init artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializedInitSource {
    /// The init artifact came from a static task node seed.
    StaticSeed {
        /// Task initialization slot that received the artifact.
        init_slot_id: String,
    },
    /// The init artifact came from an upstream task artifact.
    UpstreamArtifact {
        /// Task initialization slot that received the artifact.
        init_slot_id: String,
        /// Task instance that produced the upstream artifact.
        upstream_task_instance_id: String,
        /// Artifact selected from the upstream task outcome.
        artifact_id: String,
    },
}

/// Error wrapper for failed task initialization materialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInitializationMaterializationError {
    /// Typed diagnostics explaining why materialization failed.
    pub diagnostics: Vec<TaskInitializationDiagnostic>,
}

/// Typed task initialization diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInitializationDiagnostic {
    /// Machine-readable diagnostic code.
    pub code: TaskInitializationDiagnosticCode,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Task instance associated with the diagnostic.
    pub task_instance_id: String,
    /// Init slot associated with the diagnostic when available.
    pub init_slot_id: Option<String>,
}

impl TaskInitializationDiagnostic {
    fn new(
        code: TaskInitializationDiagnosticCode,
        task_instance_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            task_instance_id: task_instance_id.into(),
            init_slot_id: None,
        }
    }

    fn with_slot(mut self, init_slot_id: impl Into<String>) -> Self {
        self.init_slot_id = Some(init_slot_id.into());
        self
    }
}

/// Stable task initialization diagnostic code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskInitializationDiagnosticCode {
    /// Required init slot has no source.
    MissingRequiredSource,
    /// More than one source targets the same init slot.
    DuplicateSource,
    /// Source targets a slot that the compiled task does not declare.
    UnknownSourceSlot,
    /// Source type or schema does not match the compiled task slot.
    SourceContractMismatch,
    /// Task data flow edges and upstream init sources do not match.
    DataFlowSourceMismatch,
    /// Upstream task instance does not exist in the network state.
    MissingUpstreamTask,
    /// Upstream task has not completed successfully.
    UpstreamTaskNotSucceeded,
    /// Upstream task has no matching available artifact.
    MissingUpstreamArtifact,
    /// More than one upstream artifact matches the source selector.
    AmbiguousUpstreamArtifact,
    /// Availability points at an artifact outside the current accepted outcome.
    StaleUpstreamArtifact,
    /// Final payload failed task initialization validation.
    ValidationFailed,
}

/// Validates source records against the compiled task init contract.
pub fn validate_task_init_sources(node: &TaskNode) -> Vec<TaskInitializationDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut source_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for source in &node.init_sources {
        *source_counts.entry(source.init_slot_id()).or_default() += 1;
        let Some(slot) = node
            .compiled_task
            .init_slots
            .iter()
            .find(|slot| slot.init_slot_id == source.init_slot_id())
        else {
            diagnostics.push(
                TaskInitializationDiagnostic::new(
                    TaskInitializationDiagnosticCode::UnknownSourceSlot,
                    node.task_instance_id.clone(),
                    format!(
                        "init source targets unknown slot '{}'",
                        source.init_slot_id()
                    ),
                )
                .with_slot(source.init_slot_id().to_string()),
            );
            continue;
        };

        if slot.artifact_type_id != source.artifact_type_id()
            || slot.schema_version != source.schema_version()
        {
            diagnostics.push(
                TaskInitializationDiagnostic::new(
                    TaskInitializationDiagnosticCode::SourceContractMismatch,
                    node.task_instance_id.clone(),
                    format!(
                        "init source for slot '{}' does not match compiled task contract",
                        source.init_slot_id()
                    ),
                )
                .with_slot(source.init_slot_id().to_string()),
            );
        }

        if let TaskInitSource::UpstreamArtifact(source) = source {
            if source.upstream_task_instance_id.trim().is_empty()
                || source.upstream_artifact_type_id.trim().is_empty()
            {
                diagnostics.push(
                    TaskInitializationDiagnostic::new(
                        TaskInitializationDiagnosticCode::SourceContractMismatch,
                        node.task_instance_id.clone(),
                        format!(
                            "upstream source for slot '{}' is missing upstream identity",
                            source.init_slot_id
                        ),
                    )
                    .with_slot(source.init_slot_id.clone()),
                );
            }
        }
    }

    for slot in &node.compiled_task.init_slots {
        let count = source_counts
            .get(slot.init_slot_id.as_str())
            .copied()
            .unwrap_or(0);
        if slot.required && count == 0 {
            diagnostics.push(
                TaskInitializationDiagnostic::new(
                    TaskInitializationDiagnosticCode::MissingRequiredSource,
                    node.task_instance_id.clone(),
                    format!("required init slot '{}' has no source", slot.init_slot_id),
                )
                .with_slot(slot.init_slot_id.clone()),
            );
        }
        if count > 1 {
            diagnostics.push(
                TaskInitializationDiagnostic::new(
                    TaskInitializationDiagnosticCode::DuplicateSource,
                    node.task_instance_id.clone(),
                    format!("init slot '{}' has more than one source", slot.init_slot_id),
                )
                .with_slot(slot.init_slot_id.clone()),
            );
        }
    }

    diagnostics
}

/// Validates data flow edges against task init source records.
pub fn validate_task_init_graph_sources(state: &NetworkState) -> Vec<TaskInitializationDiagnostic> {
    let mut diagnostics = Vec::new();

    for edge in &state.edges {
        let DependencyKind::DataFlow { artifact_type_id } = &edge.kind else {
            continue;
        };
        let Some(target) = state.tasks.get(&edge.to) else {
            continue;
        };
        let matching_source_count = target
            .init_sources
            .iter()
            .filter(|source| {
                matches!(
                    source,
                    TaskInitSource::UpstreamArtifact(source)
                        if source.upstream_task_instance_id == edge.from
                            && source.upstream_artifact_type_id == *artifact_type_id
                )
            })
            .count();
        if matching_source_count != 1 {
            diagnostics.push(
                TaskInitializationDiagnostic::new(
                    TaskInitializationDiagnosticCode::DataFlowSourceMismatch,
                    edge.to.clone(),
                    format!(
                        "data flow edge from '{}' to '{}' for artifact '{}' does not map to exactly one upstream init source",
                        edge.from, edge.to, artifact_type_id
                    ),
                ),
            );
        }
    }

    for (task_instance_id, node) in &state.tasks {
        for source in &node.init_sources {
            let TaskInitSource::UpstreamArtifact(source) = source else {
                continue;
            };
            let has_edge = state.edges.iter().any(|edge| {
                edge.from == source.upstream_task_instance_id
                    && edge.to == *task_instance_id
                    && matches!(
                        &edge.kind,
                        DependencyKind::DataFlow { artifact_type_id }
                            if artifact_type_id == &source.upstream_artifact_type_id
                    )
            });
            if !has_edge {
                diagnostics.push(
                    TaskInitializationDiagnostic::new(
                        TaskInitializationDiagnosticCode::DataFlowSourceMismatch,
                        task_instance_id.clone(),
                        format!(
                            "upstream init source for slot '{}' does not have a matching data flow edge",
                            source.init_slot_id
                        ),
                    )
                    .with_slot(source.init_slot_id.clone()),
                );
            }
        }
    }

    diagnostics
}

/// Materializes the final dispatch initialization payload for one task node.
pub fn materialize_task_initialization(
    state: &NetworkState,
    task_instance_id: &str,
) -> Result<MaterializedTaskInitialization, TaskInitializationMaterializationError> {
    let Some(node) = state.tasks.get(task_instance_id) else {
        return Err(TaskInitializationMaterializationError {
            diagnostics: vec![TaskInitializationDiagnostic::new(
                TaskInitializationDiagnosticCode::MissingUpstreamTask,
                task_instance_id,
                format!("task '{}' does not exist", task_instance_id),
            )],
        });
    };

    let mut diagnostics = validate_task_init_sources(node);
    if !diagnostics.is_empty() {
        return Err(TaskInitializationMaterializationError { diagnostics });
    }

    let mut init_artifacts = Vec::new();
    let mut provenance = Vec::new();
    for source in &node.init_sources {
        match source {
            TaskInitSource::StaticSeed(source) => {
                init_artifacts.push(static_seed_artifact(source));
                provenance.push(MaterializedInitSource::StaticSeed {
                    init_slot_id: source.init_slot_id.clone(),
                });
            }
            TaskInitSource::UpstreamArtifact(source) => {
                match upstream_artifact(state, node, source) {
                    Ok(artifact) => {
                        provenance.push(MaterializedInitSource::UpstreamArtifact {
                            init_slot_id: source.init_slot_id.clone(),
                            upstream_task_instance_id: source.upstream_task_instance_id.clone(),
                            artifact_id: artifact.artifact_id.clone(),
                        });
                        init_artifacts.push(InitArtifactValue {
                            init_slot_id: source.init_slot_id.clone(),
                            artifact_type_id: source.artifact_type_id.clone(),
                            schema_version: source.schema_version,
                            content: artifact.content.clone(),
                        });
                    }
                    Err(diagnostic) => diagnostics.push(diagnostic),
                }
            }
        }
    }
    if !diagnostics.is_empty() {
        return Err(TaskInitializationMaterializationError { diagnostics });
    }

    let payload = TaskInitializationPayload {
        task_id: node.compiled_task.task_id.clone(),
        compiled_task_ref: format!(
            "{}@{}",
            node.compiled_task.task_id, node.compiled_task.task_version
        ),
        init_artifacts,
        task_run_context: node.task_run_context.clone(),
    };
    if let Err(error) = validate_task_initialization(&node.compiled_task, &payload) {
        return Err(TaskInitializationMaterializationError {
            diagnostics: vec![TaskInitializationDiagnostic::new(
                TaskInitializationDiagnosticCode::ValidationFailed,
                node.task_instance_id.clone(),
                error.to_string(),
            )],
        });
    }

    Ok(MaterializedTaskInitialization {
        payload,
        provenance,
    })
}

fn static_seed_artifact(source: &StaticSeedInitSource) -> InitArtifactValue {
    InitArtifactValue {
        init_slot_id: source.init_slot_id.clone(),
        artifact_type_id: source.artifact_type_id.clone(),
        schema_version: source.schema_version,
        content: source.content.clone(),
    }
}

fn upstream_artifact<'a>(
    state: &'a NetworkState,
    node: &TaskNode,
    source: &UpstreamArtifactInitSource,
) -> Result<&'a crate::task::ArtifactRecord, TaskInitializationDiagnostic> {
    if !state.tasks.contains_key(&source.upstream_task_instance_id) {
        return Err(TaskInitializationDiagnostic::new(
            TaskInitializationDiagnosticCode::MissingUpstreamTask,
            node.task_instance_id.clone(),
            format!(
                "upstream task '{}' does not exist",
                source.upstream_task_instance_id
            ),
        )
        .with_slot(source.init_slot_id.clone()));
    }

    let Some(TaskStatus::Succeeded { outcome_id }) =
        state.statuses.get(&source.upstream_task_instance_id)
    else {
        return Err(TaskInitializationDiagnostic::new(
            TaskInitializationDiagnosticCode::UpstreamTaskNotSucceeded,
            node.task_instance_id.clone(),
            format!(
                "upstream task '{}' has not succeeded",
                source.upstream_task_instance_id
            ),
        )
        .with_slot(source.init_slot_id.clone()));
    };
    let Some(outcome) = state.outcomes.get(outcome_id) else {
        return Err(TaskInitializationDiagnostic::new(
            TaskInitializationDiagnosticCode::StaleUpstreamArtifact,
            node.task_instance_id.clone(),
            format!(
                "upstream task '{}' current outcome '{}' is missing",
                source.upstream_task_instance_id, outcome_id
            ),
        )
        .with_slot(source.init_slot_id.clone()));
    };

    let availability = state
        .artifact_availability
        .iter()
        .filter(|artifact| {
            artifact.task_instance_id == source.upstream_task_instance_id
                && artifact.artifact_type_id == source.upstream_artifact_type_id
                && artifact.schema_version == source.schema_version
        })
        .map(|artifact| artifact.artifact_id.as_str())
        .collect::<BTreeSet<_>>();
    if availability.is_empty() {
        return Err(TaskInitializationDiagnostic::new(
            TaskInitializationDiagnosticCode::MissingUpstreamArtifact,
            node.task_instance_id.clone(),
            format!(
                "upstream task '{}' has no artifact '{}' schema '{}'",
                source.upstream_task_instance_id,
                source.upstream_artifact_type_id,
                source.schema_version
            ),
        )
        .with_slot(source.init_slot_id.clone()));
    }

    let mut candidates = outcome
        .artifact_records
        .iter()
        .filter(|artifact| {
            availability.contains(artifact.artifact_id.as_str())
                && artifact.artifact_type_id == source.upstream_artifact_type_id
                && artifact.schema_version == source.schema_version
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));

    match candidates.as_slice() {
        [] => Err(TaskInitializationDiagnostic::new(
            TaskInitializationDiagnosticCode::StaleUpstreamArtifact,
            node.task_instance_id.clone(),
            format!(
                "available upstream artifact for task '{}' is not in current outcome '{}'",
                source.upstream_task_instance_id, outcome_id
            ),
        )
        .with_slot(source.init_slot_id.clone())),
        [artifact] => Ok(artifact),
        _ => Err(TaskInitializationDiagnostic::new(
            TaskInitializationDiagnosticCode::AmbiguousUpstreamArtifact,
            node.task_instance_id.clone(),
            format!(
                "upstream task '{}' has multiple matching artifacts for '{}'",
                source.upstream_task_instance_id, source.upstream_artifact_type_id
            ),
        )
        .with_slot(source.init_slot_id.clone())),
    }
}

impl From<TaskInitializationMaterializationError> for ApiError {
    fn from(error: TaskInitializationMaterializationError) -> Self {
        let message = error
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>()
            .join("; ");
        ApiError::ConfigError(message)
    }
}
