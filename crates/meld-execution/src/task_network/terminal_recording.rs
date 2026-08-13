//! Package-route terminal outcome recording over the command boundary.
//!
//! Owner: the task network terminal recording boundary. Aggregate package
//! publication treats the durable task-network outcome accepted through the
//! command boundary as the only terminal authority for a package run: durable
//! per-folder progress alone never produces an aggregate. The package route
//! executes outside the claim route, so without a recorder no actor ever
//! records that outcome and aggregate publication truthfully skips complete
//! runs. This module turns one durably complete package-route run into one
//! task-network task node recorded at terminality, following the flywheel
//! model: the run is one task instance whose executor fanned out per-folder
//! capability instances.
//!
//! Invariants:
//!
//! - Durable truth only. The recorder documents completion the durable
//!   executor snapshot already proves and carries the run's durable artifact
//!   records byte-intact; it never manufactures artifacts, task events, or
//!   completion. The injected node's compiled task is a minimal terminal
//!   marker naming the run's real task identity with no capability structure;
//!   the authoritative fan-out graph stays in the durable executor snapshot,
//!   which is where aggregate production validates artifact producers.
//! - Nothing is recorded for an incomplete run. Completion uses the same
//!   projection aggregate publication uses
//!   ([`crate::task_network::aggregate_publication::check_aggregate_completion`]):
//!   every known work unit in the snapshot's own compiled fan-out completed
//!   durably, and an empty fan-out is never complete.
//! - Completion only. Terminal failure recording is deferred with terminality
//!   policy (retry, backoff, and permanent-failure classification stay
//!   deferred across the dispatch routes); this recorder never records a
//!   `Failed` outcome, and a completed run whose terminal node already
//!   carries a non-success status is a conflict, not an input.
//! - Idempotent under replay. Every identity derives deterministically from
//!   the durable run id: the task instance id, the dispatch claim id, and the
//!   outcome id are pure functions of the run id and the stable worker id.
//!   Re-observing a run whose outcome is already recorded is a no-op, and a
//!   crash between the inject, claim, and outcome commands resumes from the
//!   reduced network state on the next observation. Command ids carry the
//!   base-revision suffix the dispatch actor already uses so a retry after a
//!   concurrent-writer rejection proposes a fresh command instead of
//!   colliding with the recorded rejection.
//!
//! This module does not own package execution, completion detection policy,
//! aggregate production, or publication. It submits only public commands
//! through [`TaskNetworkCommandPort`].
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::terminal_recording::package_run_task_instance_id;
//!
//! let task_instance_id = package_run_task_instance_id("package-route::plan-docs");
//! assert_eq!(task_instance_id, "package-run::package-route::plan-docs");
//! ```

use std::collections::BTreeSet;

use crate::planning::realization::TaskPackageRoutePlan;
use crate::task::{
    package_step_repo_id, ArtifactRecord, CompiledTaskRecord, TaskArtifactRepo,
    TaskExecutorSnapshot,
};
use crate::task_network::aggregate_publication::check_aggregate_completion;
use crate::task_network::command::{Command, Request as CommandRequest, Response};
use crate::task_network::dispatch::{Outcome, OutcomeStatus, Request as DispatchRequest};
use crate::task_network::dispatch_actor::{
    dispatch_claim_id, dispatch_outcome_id, DispatchPortError, TaskNetworkCommandPort,
};
use crate::task_network::mutation::{Inject, Mutation, Rejection, Set};
use crate::task_network::state::{NetworkState, TaskLineage, TaskNode, TaskStatus};

/// Prefix marking task instances that record package-route run terminality.
///
/// The claim route must never treat these nodes as invocable work: their
/// execution already happened through the package route, so dispatch code
/// recognizes them by this prefix and completes their recording from durable
/// state instead of invoking them.
pub const PACKAGE_RUN_TASK_INSTANCE_PREFIX: &str = "package-run::";

/// Producer marker the task executor stamps on init seed artifacts.
///
/// Init seeds are the run's inputs, not its products, and their producer is
/// not a work unit in the package graph, so the terminal outcome excludes
/// them exactly like the claim route's outcome projection does.
const TASK_INIT_PRODUCER: &str = "__task_init__";

/// Derives the deterministic task instance id recording one package run.
pub fn package_run_task_instance_id(task_run_id: &str) -> String {
    format!("{PACKAGE_RUN_TASK_INSTANCE_PREFIX}{task_run_id}")
}

/// Returns the package run id recorded by a task instance, when it is one.
pub fn package_run_id_for_task_instance(task_instance_id: &str) -> Option<&str> {
    task_instance_id.strip_prefix(PACKAGE_RUN_TASK_INSTANCE_PREFIX)
}

/// Failure surfaced by package-route terminal recording.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum TerminalRecordingError {
    /// Input violated recording invariants before any command was submitted.
    #[error("terminal recording input is invalid: {0}")]
    InvalidInput(String),
    /// Durable storage access failed; a later observation may succeed.
    #[error("terminal recording storage failed: {0}")]
    Storage(String),
    /// A transient command or port failure a later re-observation resolves.
    #[error("terminal recording must be retried: {0}")]
    Retryable(String),
    /// Durable state contradicts the recording; operator attention required.
    #[error("terminal recording conflicts with durable state: {0}")]
    Conflict(String),
}

impl TerminalRecordingError {
    /// True when a later re-observation of the same run may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Storage(_) | Self::Retryable(_))
    }
}

/// Result of observing one package run for terminal recording.
#[derive(Debug, Clone, PartialEq)]
pub enum PackageRunRecording {
    /// The run is not durably complete. Nothing was recorded.
    Skipped {
        /// Work units still pending in the durable snapshot.
        pending_instance_ids: Vec<String>,
    },
    /// The run has a durable command-boundary terminal outcome. Boxed
    /// because the recording carries the full outcome record.
    Recorded(Box<PackageRunTerminalRecording>),
}

/// Durable command-boundary terminal record for one package run.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageRunTerminalRecording {
    /// Task network holding the terminal record.
    pub network_id: String,
    /// Durable package run id the record describes.
    pub task_run_id: String,
    /// Deterministic task instance recording the run.
    pub task_instance_id: String,
    /// Terminal outcome as reduced network state holds it.
    pub outcome: Outcome,
    /// True when the outcome was already recorded before this observation.
    pub already_recorded: bool,
}

/// Terminal outcome query result for one package run.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageRunTerminalOutcome {
    /// Task network holding the terminal record.
    pub network_id: String,
    /// Deterministic task instance recording the run.
    pub task_instance_id: String,
    /// Terminal outcome recorded through the command boundary.
    pub outcome: Outcome,
}

/// Returns the recorded terminal outcome for one package run, when present.
///
/// This is the read surface aggregate publication integration feeds from:
/// both public command stores expose the reduced [`NetworkState`], so the
/// query serves the in-memory and sled-backed stores alike. A run without a
/// recorded terminal outcome returns `None`, which downstream production
/// truthfully treats as not terminal.
pub fn package_run_terminal_outcome(
    state: &NetworkState,
    task_run_id: &str,
) -> Option<PackageRunTerminalOutcome> {
    let task_instance_id = package_run_task_instance_id(task_run_id);
    let outcome_id = match state.statuses.get(&task_instance_id)? {
        TaskStatus::Succeeded { outcome_id } => outcome_id,
        TaskStatus::Failed { outcome_id, .. } => outcome_id,
        TaskStatus::Pending | TaskStatus::Running { .. } | TaskStatus::Cancelled { .. } => {
            return None;
        }
    };
    let outcome = state.outcomes.get(outcome_id)?.clone();
    Some(PackageRunTerminalOutcome {
        network_id: state.network_id.clone(),
        task_instance_id,
        outcome,
    })
}

/// Loads one package run's durable artifact records for its terminal outcome.
///
/// Records come byte-intact and in append order from the run's task-owned
/// durable repository under [`package_step_repo_id`]. Init seed records are
/// excluded because their producer is not a work unit of the package graph;
/// every emitted record, including expansion requests and failure summaries,
/// passes through unchanged.
pub fn load_package_run_artifact_records(
    db: sled::Db,
    task_run_id: &str,
) -> Result<Vec<ArtifactRecord>, TerminalRecordingError> {
    let repo = TaskArtifactRepo::open_sled(db, package_step_repo_id(task_run_id))
        .map_err(|error| TerminalRecordingError::Storage(error.to_string()))?;
    Ok(repo
        .record()
        .artifacts
        .iter()
        .filter(|artifact| artifact.producer.capability_instance_id != TASK_INIT_PRODUCER)
        .cloned()
        .collect())
}

/// Builds the terminal task node's lineage from one package-route handoff.
///
/// Every value is a real identity the handoff or the durable run carries:
/// the composition, goal, method, and world-state frame come from the plan;
/// the step is the afforded action the plan realized; the operator is the
/// selected built-in package; the capability type is the validated workflow
/// route; and the capability version is the compiled package task version
/// that actually ran. The plan identity itself rides the node through the
/// derived run id in its task run context, since the run id is a pure
/// function of the plan id.
pub fn package_route_task_lineage(
    plan: &TaskPackageRoutePlan,
    compiled_task_version: u32,
) -> TaskLineage {
    TaskLineage {
        composition_id: plan.composition_id.clone(),
        goal_id: plan.goal_id.clone(),
        method_id: plan.method_id.clone(),
        step_id: plan.action_id.clone(),
        operator_id: plan.package_id.clone(),
        world_state_frame_id: plan.world_state_frame.frame_id.clone(),
        capability_type_id: plan.workflow_id.clone(),
        capability_version: compiled_task_version,
        authority_decision: None,
    }
}

/// Records the durable terminal outcome for one complete package run.
///
/// The three command-boundary steps run in order, each skipped when reduced
/// network state shows it already happened, so any crash window resumes on
/// the next observation:
///
/// 1. Inject one task node for the run through `ApplyMutationSet`, with no
///    dependency edges and a minimal terminal-marker compiled task naming the
///    run's real task identity. `lineage` is required only for this step; a
///    resumed observation of an already-injected node needs none.
/// 2. Claim the node through `ClaimReadyTask` under the deterministic
///    dispatch claim id for the stable `worker_id`.
/// 3. Record the `Succeeded` outcome through `RecordTaskOutcome`, carrying
///    `artifact_records` byte-intact (see
///    [`load_package_run_artifact_records`]) and no task events, because the
///    durable snapshot carries none and the recorder fabricates nothing.
///
/// An incomplete snapshot returns [`PackageRunRecording::Skipped`] without
/// touching the command boundary. A node already fenced `Running` completes
/// under its existing claim, whichever worker fenced it, since the outcome
/// content derives from durable run state rather than from the claimant.
pub fn record_package_run_terminal_outcome<N: TaskNetworkCommandPort>(
    network: &mut N,
    worker_id: &str,
    snapshot: &TaskExecutorSnapshot,
    artifact_records: Vec<ArtifactRecord>,
    lineage: Option<TaskLineage>,
) -> Result<PackageRunRecording, TerminalRecordingError> {
    if worker_id.trim().is_empty() {
        return Err(TerminalRecordingError::InvalidInput(
            "worker id must not be empty".to_string(),
        ));
    }
    let task_run_id = snapshot.init_payload.task_run_context.task_run_id.clone();
    if task_run_id.trim().is_empty() {
        return Err(TerminalRecordingError::InvalidInput(
            "snapshot task run id must not be empty".to_string(),
        ));
    }

    let completion = check_aggregate_completion(snapshot);
    if !completion.is_complete() {
        return Ok(PackageRunRecording::Skipped {
            pending_instance_ids: completion.pending_instance_ids,
        });
    }
    validate_artifact_producers(snapshot, &artifact_records)?;

    let task_instance_id = package_run_task_instance_id(&task_run_id);
    if let Some(recording) =
        existing_terminal_recording(network.network_state(), &task_run_id, &task_instance_id)?
    {
        return Ok(PackageRunRecording::Recorded(Box::new(recording)));
    }

    if !network
        .network_state()
        .tasks
        .contains_key(&task_instance_id)
    {
        let Some(lineage) = lineage else {
            return Err(TerminalRecordingError::InvalidInput(format!(
                "run '{task_run_id}' has no recorded task node and no lineage was supplied"
            )));
        };
        inject_terminal_node(network, snapshot, &task_run_id, &task_instance_id, lineage)?;
    }

    let claim_id = match network.network_state().statuses.get(&task_instance_id) {
        Some(TaskStatus::Pending) => {
            claim_terminal_node(network, worker_id, &task_run_id, &task_instance_id)?
        }
        Some(TaskStatus::Running { claim_id }) => claim_id.clone(),
        other => {
            return Err(TerminalRecordingError::Conflict(format!(
                "terminal node '{task_instance_id}' has unexpected status {other:?}"
            )));
        }
    };
    let claim = network
        .network_state()
        .claims
        .get(&claim_id)
        .cloned()
        .ok_or_else(|| {
            TerminalRecordingError::Conflict(format!(
                "claim '{claim_id}' fences terminal node '{task_instance_id}' but is absent from state"
            ))
        })?;

    let outcome_id = dispatch_outcome_id(&claim.claim_id);
    let outcome = Outcome {
        outcome_id: outcome_id.clone(),
        task_instance_id: task_instance_id.clone(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Succeeded,
        error: None,
        artifact_records,
        task_events: vec![],
    };
    submit(
        network,
        &format!("package-terminal-outcome::{task_run_id}"),
        Command::RecordTaskOutcome(outcome),
    )?;

    let state = network.network_state();
    let recorded = state.outcomes.get(&outcome_id).cloned().ok_or_else(|| {
        TerminalRecordingError::Conflict(format!(
            "accepted terminal outcome '{outcome_id}' is absent from state"
        ))
    })?;
    Ok(PackageRunRecording::Recorded(Box::new(
        PackageRunTerminalRecording {
            network_id: state.network_id.clone(),
            task_run_id,
            task_instance_id,
            outcome: recorded,
            already_recorded: false,
        },
    )))
}

/// Rejects artifact records whose producer is outside the run's own graph.
///
/// The terminal outcome may only carry the run's durable products, and
/// aggregate production enforces the same bound against the snapshot graph;
/// catching drift here keeps an untruthful outcome out of the command
/// boundary entirely.
fn validate_artifact_producers(
    snapshot: &TaskExecutorSnapshot,
    artifact_records: &[ArtifactRecord],
) -> Result<(), TerminalRecordingError> {
    let known_units: BTreeSet<&str> = snapshot
        .compiled_task
        .capability_instances
        .iter()
        .map(|instance| instance.capability_instance_id.as_str())
        .collect();
    for artifact in artifact_records {
        let producer = artifact.producer.capability_instance_id.as_str();
        if !known_units.contains(producer) {
            return Err(TerminalRecordingError::InvalidInput(format!(
                "artifact '{}' names producer '{}' outside the run's durable graph",
                artifact.artifact_id, producer
            )));
        }
    }
    Ok(())
}

/// Returns the already-recorded terminal outcome for the node, when present.
///
/// A non-success terminal status on the node contradicts the durable
/// completion this recorder was called with, so it is a conflict rather than
/// an idempotent no-op.
fn existing_terminal_recording(
    state: &NetworkState,
    task_run_id: &str,
    task_instance_id: &str,
) -> Result<Option<PackageRunTerminalRecording>, TerminalRecordingError> {
    match state.statuses.get(task_instance_id) {
        Some(TaskStatus::Succeeded { outcome_id }) => {
            let outcome = state.outcomes.get(outcome_id).cloned().ok_or_else(|| {
                TerminalRecordingError::Conflict(format!(
                    "terminal node '{task_instance_id}' succeeded with missing outcome '{outcome_id}'"
                ))
            })?;
            Ok(Some(PackageRunTerminalRecording {
                network_id: state.network_id.clone(),
                task_run_id: task_run_id.to_string(),
                task_instance_id: task_instance_id.to_string(),
                outcome,
                already_recorded: true,
            }))
        }
        Some(TaskStatus::Failed { .. }) | Some(TaskStatus::Cancelled { .. }) => {
            Err(TerminalRecordingError::Conflict(format!(
                "terminal node '{task_instance_id}' already carries a non-success terminal status for a durably complete run"
            )))
        }
        Some(TaskStatus::Pending) | Some(TaskStatus::Running { .. }) | None => Ok(None),
    }
}

/// Injects the run's terminal task node through the command boundary.
fn inject_terminal_node<N: TaskNetworkCommandPort>(
    network: &mut N,
    snapshot: &TaskExecutorSnapshot,
    task_run_id: &str,
    task_instance_id: &str,
    lineage: TaskLineage,
) -> Result<(), TerminalRecordingError> {
    let source_composition_id = lineage.composition_id.clone();
    let node = TaskNode {
        task_instance_id: task_instance_id.to_string(),
        lifecycle_epoch: 0,
        // Minimal terminal marker: the run's real task identity with no
        // capability structure. The durable executor snapshot remains the
        // authoritative graph; copying it here would either fabricate a
        // claimable structure the claim route must never invoke or drag
        // expansion-added init contracts into node materialization.
        compiled_task: CompiledTaskRecord {
            task_id: snapshot.compiled_task.task_id.clone(),
            task_version: snapshot.compiled_task.task_version,
            init_slots: vec![],
            capability_instances: vec![],
            dependency_edges: vec![],
        },
        init_sources: vec![],
        task_run_context: snapshot.init_payload.task_run_context.clone(),
        lineage,
    };
    let set = Set::new(
        network.network_state().network_id.clone(),
        source_composition_id,
        format!("package-terminal::{task_run_id}"),
        vec![Mutation::Inject(Inject::new(node, vec![]))],
        vec![],
    );
    submit(
        network,
        &format!("package-terminal-inject::{task_run_id}"),
        Command::ApplyMutationSet(set),
    )
}

/// Claims the run's terminal task node under the deterministic claim id.
fn claim_terminal_node<N: TaskNetworkCommandPort>(
    network: &mut N,
    worker_id: &str,
    task_run_id: &str,
    task_instance_id: &str,
) -> Result<String, TerminalRecordingError> {
    let (claim_id, request) = {
        let state = network.network_state();
        let node = state.tasks.get(task_instance_id).ok_or_else(|| {
            TerminalRecordingError::Conflict(format!(
                "terminal node '{task_instance_id}' is pending but absent from state"
            ))
        })?;
        let claim_id = dispatch_claim_id(
            &state.network_id,
            task_instance_id,
            node.lifecycle_epoch,
            worker_id,
        );
        let request = DispatchRequest {
            claim_id: claim_id.clone(),
            task_instance_id: task_instance_id.to_string(),
            worker_id: worker_id.to_string(),
            idempotency_key: format!("{claim_id}::once"),
        };
        (claim_id, request)
    };
    submit(
        network,
        &format!("package-terminal-claim::{task_run_id}"),
        Command::ClaimReadyTask(request),
    )?;
    Ok(claim_id)
}

/// Submits one command with a run-derived, revision-scoped command id.
fn submit<N: TaskNetworkCommandPort>(
    network: &mut N,
    command_root: &str,
    command: Command,
) -> Result<(), TerminalRecordingError> {
    let request = {
        let state = network.network_state();
        CommandRequest {
            command_id: format!("{command_root}::rev{}", state.revision),
            network_id: state.network_id.clone(),
            base_revision: state.revision,
            base_state_hash: state.state_hash.clone(),
            read_preconditions: vec![],
            command,
        }
    };
    match network.submit_command(request) {
        Ok(Response::Accepted { .. }) | Ok(Response::Duplicate { .. }) => Ok(()),
        Ok(Response::Rejected(rejection)) => Err(rejection_error(command_root, &rejection)),
        Err(DispatchPortError { message, retryable }) => {
            if retryable {
                Err(TerminalRecordingError::Retryable(format!(
                    "'{command_root}' command failed: {message}"
                )))
            } else {
                Err(TerminalRecordingError::Conflict(format!(
                    "'{command_root}' command failed: {message}"
                )))
            }
        }
    }
}

fn rejection_error(command_root: &str, rejection: &Rejection) -> TerminalRecordingError {
    match rejection {
        // Concurrency races against other command writers resolve when the
        // run is re-observed over the advanced base.
        Rejection::StaleBase { expected, actual } => TerminalRecordingError::Retryable(format!(
            "'{command_root}' base revision {expected} is stale against revision {actual}"
        )),
        Rejection::StateHashMismatch { .. } => {
            TerminalRecordingError::Retryable(format!("'{command_root}' base state hash is stale"))
        }
        other => TerminalRecordingError::Conflict(format!(
            "'{command_root}' command rejected: {other:?}"
        )),
    }
}
