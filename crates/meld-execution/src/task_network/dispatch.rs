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
use crate::task::{ArtifactRecord, TaskEvent, TaskExecutor, TaskInitializationPayload};
use crate::task_network::state::{ArtifactAvailability, TaskNode};
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
    /// Artifact availability emitted by the task.
    pub artifacts: Vec<ArtifactAvailability>,
    /// Task artifact records available for publication payloads.
    pub artifact_records: Vec<ArtifactRecord>,
    /// Task events emitted by the existing task runtime.
    pub task_events: Vec<TaskEvent>,
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
    if node.task_instance_id != claim.task_instance_id
        || node.lifecycle_epoch != claim.lifecycle_epoch
    {
        return Err(ApiError::ConfigError(format!(
            "Dispatch claim '{}' does not match task instance '{}'",
            claim.claim_id, node.task_instance_id
        )));
    }

    TaskExecutor::new(node.compiled_task.clone(), init_payload, repo_id.into())
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
        artifacts: artifact_availability_from_executor(claim, executor),
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
        artifacts: Vec::new(),
        artifact_records: emitted_artifact_records(executor),
        task_events: executor.events().to_vec(),
    }
}

fn artifact_availability_from_executor(
    claim: &Claim,
    executor: &TaskExecutor,
) -> Vec<ArtifactAvailability> {
    emitted_artifact_records(executor)
        .into_iter()
        .map(|artifact| ArtifactAvailability {
            task_instance_id: claim.task_instance_id.clone(),
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
