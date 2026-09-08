//! Bounded composition-path stepping over committed task-network graphs.
//!
//! Owner: the task network dispatch boundary. This module is the second
//! consumer of the bounded package-step contract: it advances one committed
//! task-network composition run by at most one bounded ready wave per step.
//! The existing task package executor is the compatibility-scoped first
//! implementor; this implementor rides the task network instead of the
//! package executor and invokes no package-expansion machinery.
//!
//! Inputs are a durable task-network command store holding a committed,
//! lowered composition graph, a bounded step budget, and a caller-bound
//! capability invocation port. Outputs are fenced claims, recorded task
//! outcomes, durably persisted task artifacts, and frozen package-step
//! reports projected from reduced network state.
//!
//! One step resumes this worker's interrupted `Running` claims first, then
//! claims new ready tasks in `compute_ready_set` order under deterministic
//! claim identity, resolving each released invocation through the command
//! boundary before the step returns. Emitted artifacts persist to the
//! task-owned durable repository before the terminal outcome is recorded, and
//! a byte-identical re-emitted artifact is accepted as durable replay, so a
//! fresh implementor over the same durable stores resumes exactly without
//! repeating completed invocations.
//!
//! This module does not own composition lowering, graph mutation reduction,
//! capability execution, aggregate publication, or retry policy.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::composition_step::CompositionInvocationOutcome;
//!
//! let outcome = CompositionInvocationOutcome::Failed {
//!     error: "provider unavailable".to_string(),
//! };
//!
//! assert!(matches!(
//!     outcome,
//!     CompositionInvocationOutcome::Failed { .. }
//! ));
//! ```

use crate::task::{ArtifactRecord, TaskArtifactRepo, TaskInitializationPayload};
use crate::task_network::command::{Command, Request as CommandRequest, Response};
use crate::task_network::dispatch::{Claim, Outcome, OutcomeStatus, Request as DispatchRequest};
use crate::task_network::dispatch_actor::{
    dispatch_claim_id, dispatch_claim_repo_id, dispatch_outcome_id, TaskNetworkCommandPort,
};
use crate::task_network::initialization::materialize_task_initialization;
use crate::task_network::package_step::{
    PackageStep, PackageStepProgress, PackageStepReport, PackageStepRequest,
};
use crate::task_network::readiness::compute_ready_set;
use crate::task_network::state::{NetworkState, TaskNode, TaskStatus};
use async_trait::async_trait;

/// Error surfaced by one failed composition-path step.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CompositionStepError {
    /// Construction input violated composition-step invariants.
    #[error("composition network step construction is invalid: {0}")]
    InvalidConstruction(String),
    /// The task-network command port failed before a command resolved.
    #[error("composition network step command failed: {0}")]
    CommandPort(String),
    /// The command boundary rejected a claim or outcome command.
    #[error("composition network step command was rejected: {0}")]
    CommandRejected(String),
    /// A claimed task no longer matches its fenced claim.
    #[error("claimed task '{task_instance_id}' is invalid for this step: {message}")]
    InvalidClaimedTask {
        /// Task instance the claim fences.
        task_instance_id: String,
        /// Stable mismatch summary.
        message: String,
    },
    /// Dispatch initialization could not be materialized for a claimed task.
    #[error("task initialization for '{task_instance_id}' failed: {message}")]
    InitializationFailed {
        /// Task instance whose initialization failed.
        task_instance_id: String,
        /// Joined initialization diagnostics.
        message: String,
    },
    /// An invocation never resolved; its fenced claim resumes on a later step.
    #[error("composition invocation for '{task_instance_id}' did not resolve: {message}")]
    InvocationUnresolved {
        /// Task instance whose invocation did not resolve.
        task_instance_id: String,
        /// Port failure summary.
        message: String,
    },
    /// Durable artifact storage failed; the fenced claim resumes later.
    #[error("artifact persistence for claim '{claim_id}' failed: {message}")]
    ArtifactStorage {
        /// Claim whose artifacts could not persist.
        claim_id: String,
        /// Storage failure summary.
        message: String,
    },
}

/// Failure surfaced by the composition task invocation port.
///
/// A port error means the invocation never resolved: the fenced claim stays
/// `Running` and a later step resumes it. A resolved bounded failure is a
/// [`CompositionInvocationOutcome::Failed`] value instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionInvocationError {
    /// Stable failure summary suitable for step error reporting.
    pub message: String,
}

impl CompositionInvocationError {
    /// Creates a port error from a stable failure summary.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// One bounded composition-path invocation resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum CompositionInvocationOutcome {
    /// The invocation completed and emitted these task artifact records.
    Completed(Vec<ArtifactRecord>),
    /// The invocation resolved to a bounded failure recordable as a failed
    /// task outcome through the command boundary.
    Failed {
        /// Stable failure summary carried into the recorded outcome.
        error: String,
    },
}

/// Capability invocation port for one fenced composition-path claim.
///
/// The implementor owns claim identity, artifact persistence order, and
/// outcome recording. The port owns how one claimed task node executes; it
/// must be deterministic for one claim so crash replay re-emits
/// byte-identical artifacts. Tests bind deterministic fixtures; production
/// binding is a root adapter concern.
#[async_trait]
pub trait CompositionTaskInvoker: Send + Sync {
    /// Executes one bounded invocation for a claimed task node.
    async fn invoke_composition_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<CompositionInvocationOutcome, CompositionInvocationError>;
}

/// Distinguishes replay-tolerant artifact persistence results.
///
/// Drift under an existing artifact id is a bounded, recordable failure;
/// storage failure leaves the invocation unresolved so the fenced claim
/// resumes on a later step.
enum ArtifactPersistence {
    Persisted,
    Drift(String),
}

/// Bounded, resumable package stepping over one committed task-network graph.
///
/// This is the second implementor of the frozen [`PackageStep`] contract. It
/// owns claim identity derivation, the persist-before-outcome order, and step
/// budget accounting over the serialized command boundary. Progress counters
/// are projected from reduced network state: known units are the committed
/// task count including later-injected nodes, completed units are the
/// terminal-succeeded count, and ready units come from the ready set. The
/// graph grows only through committed mutation sets, so applied expansions
/// stay zero on this path.
pub struct CompositionNetworkExecution<N, I> {
    worker_id: String,
    db: sled::Db,
    network: N,
    invoker: I,
}

impl<N, I> CompositionNetworkExecution<N, I>
where
    N: TaskNetworkCommandPort + Send,
    I: CompositionTaskInvoker,
{
    /// Creates a composition-path stepper over a committed network store.
    ///
    /// The database holds the claim-scoped task artifact repositories, so a
    /// fresh stepper opened over the same database and network store resumes
    /// every durable claim without repeating completed invocations.
    pub fn new(
        worker_id: impl Into<String>,
        db: sled::Db,
        network: N,
        invoker: I,
    ) -> Result<Self, CompositionStepError> {
        let worker_id = worker_id.into();
        if worker_id.is_empty() {
            return Err(CompositionStepError::InvalidConstruction(
                "worker id must not be empty".to_string(),
            ));
        }
        Ok(Self {
            worker_id,
            db,
            network,
            invoker,
        })
    }

    /// Returns the worker id fencing this stepper's dispatch claims.
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Returns the underlying network store for state inspection.
    pub fn network(&self) -> &N {
        &self.network
    }

    /// Releases the underlying network store.
    pub fn into_network(self) -> N {
        self.network
    }

    /// Returns this worker's fenced claims still marked `Running`.
    ///
    /// Claim map order is deterministic by claim id, mirroring the dispatch
    /// actor's resume order.
    fn resumable_claims(&self) -> Vec<Claim> {
        let state = self.network.network_state();
        state
            .claims
            .values()
            .filter(|claim| {
                claim.worker_id == self.worker_id
                    && matches!(
                        state.statuses.get(&claim.task_instance_id),
                        Some(TaskStatus::Running { claim_id }) if *claim_id == claim.claim_id
                    )
            })
            .cloned()
            .collect()
    }

    /// Claims one ready task through the command boundary.
    fn claim_ready_task(&mut self, task_instance_id: &str) -> Result<Claim, CompositionStepError> {
        let (claim_id, command) = {
            let state = self.network.network_state();
            let Some(node) = state.tasks.get(task_instance_id) else {
                return Err(CompositionStepError::InvalidClaimedTask {
                    task_instance_id: task_instance_id.to_string(),
                    message: "ready task is absent from network state".to_string(),
                });
            };
            let claim_id = dispatch_claim_id(
                &state.network_id,
                task_instance_id,
                node.lifecycle_epoch,
                &self.worker_id,
            );
            let request = DispatchRequest {
                claim_id: claim_id.clone(),
                task_instance_id: task_instance_id.to_string(),
                worker_id: self.worker_id.clone(),
                idempotency_key: format!("{claim_id}::once"),
            };
            let command = command_request(
                state,
                format!("{claim_id}::rev{}", state.revision),
                Command::ClaimReadyTask(request),
            );
            (claim_id, command)
        };

        match self.network.submit_command(command) {
            Ok(Response::Accepted { .. }) | Ok(Response::Duplicate { .. }) => self
                .network
                .network_state()
                .claims
                .get(&claim_id)
                .cloned()
                .ok_or_else(|| CompositionStepError::InvalidClaimedTask {
                    task_instance_id: task_instance_id.to_string(),
                    message: format!("accepted claim '{claim_id}' is absent from network state"),
                }),
            Ok(Response::Rejected(rejection)) => Err(CompositionStepError::CommandRejected(
                format!("claim for '{task_instance_id}' was rejected: {rejection:?}"),
            )),
            Err(error) => Err(CompositionStepError::CommandPort(error.message)),
        }
    }

    /// Invokes one fenced claim and records its terminal outcome.
    ///
    /// Ordering invariant: emitted artifacts persist to the task-owned
    /// durable repository before the terminal outcome command is submitted. A
    /// failure between the two leaves a fenced `Running` claim that a later
    /// step resumes; byte-identical re-emitted artifacts are accepted as
    /// durable replay so a resumed claim never duplicates domain outputs.
    async fn resolve_claim(
        &mut self,
        claim: &Claim,
    ) -> Result<OutcomeStatus, CompositionStepError> {
        let (node, init_payload) = {
            let state = self.network.network_state();
            let Some(node) = state.tasks.get(&claim.task_instance_id) else {
                return Err(CompositionStepError::InvalidClaimedTask {
                    task_instance_id: claim.task_instance_id.clone(),
                    message: "claimed task is absent from network state".to_string(),
                });
            };
            if node.lifecycle_epoch != claim.lifecycle_epoch {
                return Err(CompositionStepError::InvalidClaimedTask {
                    task_instance_id: claim.task_instance_id.clone(),
                    message: format!(
                        "claim '{}' fences epoch {} but node is at epoch {}",
                        claim.claim_id, claim.lifecycle_epoch, node.lifecycle_epoch
                    ),
                });
            }
            let init_payload = materialize_task_initialization(state, &claim.task_instance_id)
                .map_err(|error| CompositionStepError::InitializationFailed {
                    task_instance_id: claim.task_instance_id.clone(),
                    message: error
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.message.clone())
                        .collect::<Vec<_>>()
                        .join("; "),
                })?;
            (node.clone(), init_payload.payload)
        };

        let invocation = if let Some(refusal) = &claim.refusal {
            Ok(CompositionInvocationOutcome::Failed {
                error: refusal.reason().into(),
            })
        } else {
            self.invoker
                .invoke_composition_task(&node, claim, &init_payload)
                .await
        };
        let outcome = match invocation {
            Ok(CompositionInvocationOutcome::Completed(artifacts)) => {
                match self.persist_claim_artifacts(claim, &artifacts)? {
                    ArtifactPersistence::Persisted => succeeded_outcome(claim, artifacts),
                    // Drift under an existing artifact id is a bounded
                    // failure recorded through the command boundary.
                    ArtifactPersistence::Drift(message) => failed_outcome(claim, message),
                }
            }
            Ok(CompositionInvocationOutcome::Failed { error }) => failed_outcome(claim, error),
            Err(error) => {
                return Err(CompositionStepError::InvocationUnresolved {
                    task_instance_id: claim.task_instance_id.clone(),
                    message: error.message,
                });
            }
        };

        let status = outcome.status.clone();
        let command = {
            let state = self.network.network_state();
            command_request(
                state,
                format!("{}::rev{}", outcome.outcome_id, state.revision),
                Command::RecordTaskOutcome(outcome),
            )
        };
        match self.network.submit_command(command) {
            Ok(Response::Accepted { .. }) | Ok(Response::Duplicate { .. }) => Ok(status),
            Ok(Response::Rejected(rejection)) => {
                Err(CompositionStepError::CommandRejected(format!(
                    "outcome for '{}' was rejected: {rejection:?}",
                    claim.task_instance_id
                )))
            }
            Err(error) => Err(CompositionStepError::CommandPort(error.message)),
        }
    }

    /// Persists claim artifacts durably with replay tolerance.
    ///
    /// A byte-identical artifact already present under the same id is
    /// accepted as durable replay from an interrupted step, per the
    /// established crash-window rule shared with the first implementor and
    /// the dispatch actor's claim route. Content drift is a bounded failure.
    fn persist_claim_artifacts(
        &self,
        claim: &Claim,
        artifacts: &[ArtifactRecord],
    ) -> Result<ArtifactPersistence, CompositionStepError> {
        let storage = |message: String| CompositionStepError::ArtifactStorage {
            claim_id: claim.claim_id.clone(),
            message,
        };
        let mut repo =
            TaskArtifactRepo::open_sled(self.db.clone(), dispatch_claim_repo_id(&claim.claim_id))
                .map_err(|error| storage(error.to_string()))?;
        for artifact in artifacts {
            match repo.get_artifact(&artifact.artifact_id) {
                Some(existing) if existing == artifact => {}
                Some(_) => {
                    return Ok(ArtifactPersistence::Drift(format!(
                        "claim '{}' re-emitted artifact '{}' with drifted content",
                        claim.claim_id, artifact.artifact_id
                    )));
                }
                None => {
                    repo.append_artifact(artifact.clone())
                        .map_err(|error| storage(error.to_string()))?;
                }
            }
        }
        repo.flush().map_err(|error| storage(error.to_string()))?;
        Ok(ArtifactPersistence::Persisted)
    }
}

#[async_trait]
impl<N, I> PackageStep for CompositionNetworkExecution<N, I>
where
    N: TaskNetworkCommandPort + Send,
    I: CompositionTaskInvoker,
{
    type Error = CompositionStepError;

    fn progress(&self) -> PackageStepProgress {
        let state = self.network.network_state();
        let ready = compute_ready_set(state);
        PackageStepProgress {
            known_units: state.tasks.len(),
            completed_units: state
                .statuses
                .values()
                .filter(|status| matches!(status, TaskStatus::Succeeded { .. }))
                .count(),
            ready_units: ready.task_instance_ids.len(),
            // No package-expansion machinery runs on this path; committed
            // graph growth appears in the known-unit count instead.
            applied_expansions: 0,
            persisted_artifacts: state.artifact_availability.len(),
        }
    }

    async fn step(
        &mut self,
        request: &PackageStepRequest,
    ) -> Result<PackageStepReport, Self::Error> {
        let input_progress = self.progress();
        let mut items_attempted = 0;
        let mut items_committed = 0;
        let mut budget_exhausted = false;

        if input_progress.completed_units != input_progress.known_units {
            let mut remaining = request.max_ready_invocations;

            // Resume before claiming new work so a failure between artifact
            // persistence and outcome recording replays ahead of fresh
            // claims, mirroring the dispatch actor's crash semantics.
            for claim in self.resumable_claims() {
                if remaining == 0 {
                    budget_exhausted = true;
                    break;
                }
                remaining -= 1;
                items_attempted += 1;
                if self.resolve_claim(&claim).await? == OutcomeStatus::Succeeded {
                    items_committed += 1;
                }
            }

            if !budget_exhausted {
                // One ready-set snapshot per step: dependents readied by this
                // step's outcomes wait for a later step, preserving the
                // bounded ready-wave rule of the first implementor.
                let ready = compute_ready_set(self.network.network_state());
                for task_instance_id in &ready.task_instance_ids {
                    if remaining == 0 {
                        budget_exhausted = true;
                        break;
                    }
                    remaining -= 1;
                    items_attempted += 1;
                    let claim = self.claim_ready_task(task_instance_id)?;
                    if self.resolve_claim(&claim).await? == OutcomeStatus::Succeeded {
                        items_committed += 1;
                    }
                }
            }
        }

        let output_progress = self.progress();
        let package_complete = output_progress.completed_units == output_progress.known_units;
        Ok(PackageStepReport {
            items_attempted,
            items_committed,
            input_progress,
            output_progress,
            budget_exhausted,
            package_complete,
        })
    }
}

fn command_request(state: &NetworkState, command_id: String, command: Command) -> CommandRequest {
    CommandRequest {
        command_id,
        network_id: state.network_id.clone(),
        base_revision: state.revision,
        base_state_hash: state.state_hash.clone(),
        read_preconditions: vec![],
        command,
    }
}

fn succeeded_outcome(claim: &Claim, artifact_records: Vec<ArtifactRecord>) -> Outcome {
    Outcome {
        outcome_id: dispatch_outcome_id(&claim.claim_id),
        task_instance_id: claim.task_instance_id.clone(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Succeeded,
        error: None,
        artifact_records,
        task_events: vec![],
        admission: claim.admission.clone(),
    }
}

fn failed_outcome(claim: &Claim, error: String) -> Outcome {
    Outcome {
        outcome_id: dispatch_outcome_id(&claim.claim_id),
        task_instance_id: claim.task_instance_id.clone(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Failed,
        error: Some(error),
        artifact_records: vec![],
        task_events: vec![],
        admission: claim.admission.clone(),
    }
}
