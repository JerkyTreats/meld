//! Dispatch claim and task outcome contracts.
//!
//! Owner: task network.
//! Inputs: ready task claim requests and task runtime outcomes.
//! Outputs: fenced claim records, outcome records, and task executor bridge
//! helpers.
//! Does not own: this module does not invoke capabilities or publish events.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::dispatch::Request;
//!
//! let request = Request {
//!     claim_id: "claim-a".to_string(),
//!     task_instance_id: "task-a".to_string(),
//!     worker_id: "worker-a".to_string(),
//!     idempotency_key: "once".to_string(),
//! };
//!
//! assert_eq!(request.task_instance_id, "task-a");
//! ```

use crate::error::ApiError;
use crate::task::{
    ArtifactRecord, TaskArtifactRepo, TaskEvent, TaskExecutor, TaskInitializationPayload,
};
use crate::task_network::state::{ArtifactAvailability, TaskNode};
use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

/// Request to claim one ready task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// Stable claim id supplied by the caller.
    pub claim_id: String,
    /// Task instance to claim.
    pub task_instance_id: String,
    /// Worker or runtime instance claiming the task.
    pub worker_id: String,
    /// Caller supplied idempotency key.
    pub idempotency_key: String,
}

/// Fenced dispatch claim persisted before task execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    /// Stable claim id.
    pub claim_id: String,
    /// Stable task network identifier.
    pub network_id: String,
    /// Claimed task instance id.
    pub task_instance_id: String,
    /// Lifecycle epoch of the claimed task.
    pub lifecycle_epoch: u64,
    /// Revision that accepted the claim.
    pub claim_revision: u64,
    /// Worker or runtime instance that holds the claim.
    pub worker_id: String,
    /// Caller supplied idempotency key.
    pub idempotency_key: String,
}

impl Claim {
    /// Builds the claim record for an accepted claim command.
    pub fn accepted(
        network_id: impl Into<String>,
        request: &Request,
        lifecycle_epoch: u64,
        claim_revision: u64,
    ) -> Self {
        Self {
            claim_id: request.claim_id.clone(),
            network_id: network_id.into(),
            task_instance_id: request.task_instance_id.clone(),
            lifecycle_epoch,
            claim_revision,
            worker_id: request.worker_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
        }
    }
}

/// Terminal status carried by a task outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeStatus {
    /// Task runtime completed successfully.
    Succeeded,
    /// Task runtime completed with an error.
    Failed,
}

/// Task runtime outcome accepted through the task network command boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    /// Stable outcome id.
    pub outcome_id: String,
    /// Task instance that produced the outcome.
    pub task_instance_id: String,
    /// Lifecycle epoch observed by the worker.
    pub lifecycle_epoch: u64,
    /// Claim id observed by the worker.
    pub claim_id: String,
    /// Claim revision observed by the worker.
    pub claim_revision: u64,
    /// Terminal task status.
    pub status: OutcomeStatus,
    /// Optional failure summary.
    pub error: Option<String>,
    /// Task artifact records available for publication payloads.
    pub artifact_records: Vec<ArtifactRecord>,
    /// Task events emitted by the existing task runtime.
    pub task_events: Vec<TaskEvent>,
}

/// Semantic attribution missing from the task-local outcome product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeSemanticLineage {
    /// Execution goal that authorized planning.
    pub goal: DomainObjectRef,
    /// Method selected by planning.
    pub method: DomainObjectRef,
    /// Durable projection frame used by planning.
    pub projection_frame: DomainObjectRef,
    /// Full typed subject preserved from the goal and lowering boundary.
    pub subject: DomainObjectRef,
}

/// Self-contained publication input that preserves the canonical task outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributedOutcome {
    /// Canonical task-network outcome product.
    pub outcome: Outcome,
    /// Additional semantic lineage owned by execution planning.
    pub semantic_lineage: OutcomeSemanticLineage,
}

/// Creates a task executor for a fenced claim.
///
/// The caller still owns capability invocation through the existing task
/// runtime. This helper only validates that the claim names the node epoch and
/// builds the task-local executor.
pub fn build_executor_for_claim(
    node: &TaskNode,
    claim: &Claim,
    init_payload: TaskInitializationPayload,
    repo_id: impl Into<String>,
) -> Result<TaskExecutor, ApiError> {
    validate_claim_matches_node(node, claim)?;

    TaskExecutor::new(node.compiled_task.clone(), init_payload, repo_id.into())
}

/// Creates a task executor for a fenced claim with a caller supplied artifact repo.
///
/// Runtime hosts use this boundary after opening the task-owned durable repo.
/// Claim validation still happens before task-local execution starts.
pub fn build_executor_for_claim_with_artifact_repo(
    node: &TaskNode,
    claim: &Claim,
    init_payload: TaskInitializationPayload,
    artifact_repo: TaskArtifactRepo,
) -> Result<TaskExecutor, ApiError> {
    validate_claim_matches_node(node, claim)?;

    TaskExecutor::new_with_artifact_repo(node.compiled_task.clone(), init_payload, artifact_repo)
}

fn validate_claim_matches_node(node: &TaskNode, claim: &Claim) -> Result<(), ApiError> {
    if node.task_instance_id != claim.task_instance_id
        || node.lifecycle_epoch != claim.lifecycle_epoch
    {
        return Err(ApiError::ConfigError(format!(
            "Dispatch claim '{}' does not match task instance '{}'",
            claim.claim_id, node.task_instance_id
        )));
    }
    Ok(())
}

/// Converts a completed task executor into a successful fenced outcome.
pub fn succeeded_outcome_from_executor(
    outcome_id: impl Into<String>,
    claim: &Claim,
    executor: &TaskExecutor,
) -> Outcome {
    Outcome {
        outcome_id: outcome_id.into(),
        task_instance_id: claim.task_instance_id.clone(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Succeeded,
        error: None,
        artifact_records: emitted_artifact_records(executor),
        task_events: executor.events().to_vec(),
    }
}

/// Converts a failed task executor into a failed fenced outcome.
pub fn failed_outcome_from_executor(
    outcome_id: impl Into<String>,
    claim: &Claim,
    executor: &TaskExecutor,
    error: impl Into<String>,
) -> Outcome {
    Outcome {
        outcome_id: outcome_id.into(),
        task_instance_id: claim.task_instance_id.clone(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Failed,
        error: Some(error.into()),
        artifact_records: emitted_artifact_records(executor),
        task_events: executor.events().to_vec(),
    }
}

/// Derives the receiver-owned artifact availability projection for an outcome.
pub fn artifact_availability_for_outcome(outcome: &Outcome) -> Vec<ArtifactAvailability> {
    if outcome.status != OutcomeStatus::Succeeded {
        return Vec::new();
    }

    outcome
        .artifact_records
        .iter()
        .cloned()
        .map(|artifact| ArtifactAvailability {
            task_instance_id: outcome.task_instance_id.clone(),
            artifact_type_id: artifact.artifact_type_id,
            artifact_id: artifact.artifact_id,
            schema_version: artifact.schema_version,
        })
        .collect()
}

fn emitted_artifact_records(executor: &TaskExecutor) -> Vec<ArtifactRecord> {
    executor
        .artifact_repo()
        .record()
        .artifacts
        .iter()
        .filter(|artifact| artifact.producer.capability_instance_id != "__task_init__")
        .cloned()
        .collect()
}

#[cfg(test)]
mod contract_freeze_tests {
    use super::*;

    #[test]
    fn semantic_lineage_preserves_full_object_coordinates() {
        let lineage = OutcomeSemanticLineage {
            goal: DomainObjectRef::new("execution", "goal", "goal-a").unwrap(),
            method: DomainObjectRef::new("execution", "method", "method-a").unwrap(),
            projection_frame: DomainObjectRef::new("world_model", "projection_frame", "frame-a")
                .unwrap(),
            subject: DomainObjectRef::new("workspace_fs", "node", "shared-id").unwrap(),
        };

        let encoded = serde_json::to_vec(&lineage).unwrap();
        let decoded: OutcomeSemanticLineage = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, lineage);
        assert_eq!(decoded.subject.domain_id, "workspace_fs");
        assert_eq!(decoded.subject.object_kind, "node");
    }
}
