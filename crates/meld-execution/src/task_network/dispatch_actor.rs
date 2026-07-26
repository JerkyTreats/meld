//! Bounded execution dispatch actor over public execution ports.
//!
//! Owner: the task network dispatch boundary. One actor tick claims and
//! executes a bounded amount of ready work through injected ports and stops
//! at recorded task outcomes and durable package progress. The actor never
//! sequences planning, publication, or evidence.
//!
//! Two dispatch routes share one tick budget, measured in released bounded
//! invocations:
//!
//! - Package route: planning hands off a [`TaskPackageRoutePlan`]. The
//!   handoff carries no durable record by design, so dedupe derives from the
//!   package progress store keyed by a run id derived from the deterministic
//!   plan identity. A plan already driven to durable completion is never
//!   restarted; a plan with durable progress resumes the same run. A live
//!   plan advances by exactly one bounded package step per tick.
//! - Claim route: ready tasks selected in deterministic ready-set order are
//!   claimed through the existing command boundary under deterministic claim
//!   identity, invoked through the claimed-task port, and resolved by an
//!   outcome recorded through the same command boundary before the tick
//!   returns. Task artifacts persist to the task-owned durable repository
//!   before the terminal outcome is accepted, so a crash between the two
//!   replays without duplicating domain outputs.
//!
//! Failed bounded work is recorded through the command boundary, never counts
//! as committed progress, and cannot re-enter the same tick: a recorded
//! outcome moves the task out of `Pending` and the ready-set snapshot is
//! taken once per tick. Retry, backoff, and terminality policy stay deferred.
//!
//! When the actor observes a package run's durable completion — a step report
//! with `package_complete` or an already-complete durable snapshot — it
//! idempotently records the run's terminal outcome through the command
//! boundary via [`crate::task_network::terminal_recording`], the durable
//! terminal authority aggregate publication requires. Terminal recording is
//! command-boundary bookkeeping over work the package route already released,
//! so it consumes no tick budget. The claim route never invokes the terminal
//! nodes this recording injects: it recognizes them by their deterministic
//! instance-id prefix and completes their recording from durable state
//! instead, so a crash window inside recording can never re-execute a
//! completed package.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::dispatch_actor::DispatchTickRequest;
//!
//! let request = DispatchTickRequest {
//!     sequence: 7,
//!     max_items: 2,
//!     package_plans: vec![],
//! };
//!
//! assert_eq!(request.max_items, 2);
//! ```

use crate::capability::{
    BoundCapabilityInstance, CapabilityInvocationPayload, CapabilityInvocationResult,
};
use crate::error::ApiError;
use crate::planning::realization::TaskPackageRoutePlan;
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use crate::task::{
    ArtifactRecord, CompiledTaskRecord, DurablePackageExecution, PackageStepInvoker,
    TaskArtifactRepo, TaskExecutorSnapshot, TaskInitializationPayload, TaskProgressStore,
};
use crate::task_network::command::{Command, Request as CommandRequest, Response};
use crate::task_network::dispatch::{Claim, Outcome, OutcomeStatus, Request as DispatchRequest};
use crate::task_network::initialization::materialize_task_initialization;
use crate::task_network::mutation::Rejection;
use crate::task_network::package_step::{PackageStep, PackageStepReport, PackageStepRequest};
use crate::task_network::readiness::compute_ready_set;
use crate::task_network::state::ReadinessDiagnosticCode;
use crate::task_network::state::{NetworkState, TaskLineage, TaskNode, TaskStatus};
use crate::task_network::store::{InMemoryTaskNetworkStore, SledTaskNetworkStore};
use crate::task_network::terminal_recording::{
    load_package_run_artifact_records, package_route_task_lineage,
    package_run_id_for_task_instance, record_package_run_terminal_outcome, PackageRunRecording,
};
use crate::waiting::{conditions, WaitingOnDeclaration};
use async_trait::async_trait;
use std::collections::BTreeSet;
use std::sync::Arc;

const DISPATCH_ACTOR_ID: &str = "execution.task_network.dispatch.runtime";

/// Derives the durable package run id for one package-route plan handoff.
///
/// The handoff itself carries no durable record, so this derivation is the
/// only bridge between plan identity and durable package progress. Every
/// consumer of the run's progress or artifacts must derive the same id from
/// the same deterministic `plan_id`.
pub fn package_route_run_id(plan_id: &str) -> String {
    format!("package-route::{plan_id}")
}

/// Derives the deterministic dispatch claim id for one ready task.
///
/// The identity covers the task network, task instance, lifecycle epoch, and
/// worker, so duplicate dispatch of the same worker converges on one claim
/// while a reopened task epoch or another worker fences a distinct claim.
pub fn dispatch_claim_id(
    network_id: &str,
    task_instance_id: &str,
    lifecycle_epoch: u64,
    worker_id: &str,
) -> String {
    format!("dispatch-claim::{network_id}::{task_instance_id}::{lifecycle_epoch}::{worker_id}")
}

/// Derives the deterministic outcome id recorded for one dispatch claim.
pub fn dispatch_outcome_id(claim_id: &str) -> String {
    format!("dispatch-outcome::{claim_id}")
}

/// Returns the artifact repo id holding one claim's task-owned artifacts.
///
/// Hosts reopen the task artifact repository under this id in the same
/// database to read the canonical artifact records after dispatch.
pub fn dispatch_claim_repo_id(claim_id: &str) -> String {
    format!("dispatch_claim::{claim_id}")
}

/// Failure surfaced by an injected dispatch port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchPortError {
    /// Stable summary suitable for actor reporting.
    pub message: String,
    /// True when a later actor tick may succeed without operator action.
    pub retryable: bool,
}

impl DispatchPortError {
    /// Build a retryable port error.
    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    /// Build a fatal port error.
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}

/// Prepared package run supplied by the package-run preparation port.
#[derive(Debug, Clone)]
pub struct PreparedPackageRun {
    /// Compiled package task lowered by the owning package machinery.
    pub compiled_task: CompiledTaskRecord,
    /// Initialization payload for the run. Its run context must carry the
    /// actor-derived task run id so durable dedupe and progress stay keyed by
    /// plan identity; the actor rejects a drifted run id before opening.
    pub init_payload: TaskInitializationPayload,
}

/// Package-run preparation port for plan handoffs.
///
/// Root adapters own how a validated plan resolves into a compiled package
/// task and initialization payload. The actor owns run identity, durable
/// dedupe, and bounded stepping. Preparation must be deterministic for one
/// plan id: the durable execution rejects reopening a run under drifted
/// compiled-task identity or initialization payload.
pub trait PackageRunPreparer {
    /// Resolves one plan handoff into a compiled task and run payload.
    fn prepare_package_run(
        &self,
        plan: &TaskPackageRoutePlan,
        task_run_id: &str,
    ) -> Result<PreparedPackageRun, DispatchPortError>;
}

/// One bounded claimed-task invocation resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaimedInvocationOutcome {
    /// The invocation completed and emitted these task artifact records.
    Completed(Vec<ArtifactRecord>),
    /// The invocation failed within bounded semantics and is recordable as a
    /// failed task outcome through the command boundary.
    Failed {
        /// Stable failure summary carried into the recorded outcome.
        error: String,
    },
}

/// Capability invocation port for one fenced dispatch claim.
///
/// The actor owns claim identity, artifact persistence order, and outcome
/// recording. The port owns how one claimed task node executes; it must be
/// deterministic for one claim so crash replay re-emits byte-identical
/// artifacts. A port `Err` means the invocation never resolved: the claim
/// stays fenced and a later tick resumes it. A `Failed` outcome means the
/// invocation resolved to a bounded, recordable failure.
#[async_trait]
pub trait ClaimedTaskInvoker: Send + Sync {
    /// Executes one bounded invocation for a claimed task node.
    async fn invoke_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError>;
}

/// Narrow task-network access port for the dispatch actor.
///
/// Both public command stores implement this port. It exists so the actor
/// binds to the serialized command boundary without naming a concrete store,
/// and so tests can interpose crash windows around command submission.
pub trait TaskNetworkCommandPort {
    /// Returns the latest reduced network state.
    fn network_state(&self) -> &NetworkState;

    /// Submits one command through the single-writer command boundary.
    fn submit_command(&mut self, request: CommandRequest) -> Result<Response, DispatchPortError>;
}

impl TaskNetworkCommandPort for InMemoryTaskNetworkStore {
    fn network_state(&self) -> &NetworkState {
        self.state()
    }

    fn submit_command(&mut self, request: CommandRequest) -> Result<Response, DispatchPortError> {
        Ok(self.submit(request))
    }
}

impl TaskNetworkCommandPort for SledTaskNetworkStore {
    fn network_state(&self) -> &NetworkState {
        self.state()
    }

    fn submit_command(&mut self, request: CommandRequest) -> Result<Response, DispatchPortError> {
        self.submit(request)
            .map_err(|error| DispatchPortError::retryable(error.to_string()))
    }
}

/// Bounded dispatch actor tick request.
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchTickRequest {
    /// Injected tick sequence carried into the report for supervision.
    /// Dispatch identities derive from durable plan and claim identity, not
    /// from this sequence, so replayed ticks converge on the same records.
    pub sequence: u64,
    /// Maximum bounded invocations one tick may release across both routes.
    pub max_items: usize,
    /// Package-route handoffs delivered by the caller for this tick, in the
    /// caller's deterministic order. Duplicate plan ids dedupe to one run.
    pub package_plans: Vec<TaskPackageRoutePlan>,
}

/// Durable progress checkpoint reached by one dispatch actor tick.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchCheckpoint {
    /// A plan handoff deduped against a durably completed package run.
    PackageRunAlreadyComplete {
        /// Deterministic plan identity of the handoff.
        plan_id: String,
        /// Durable run id derived from the plan identity.
        task_run_id: String,
    },
    /// One bounded package step advanced a durable package run.
    PackageStepDriven {
        /// Deterministic plan identity of the handoff.
        plan_id: String,
        /// Durable run id derived from the plan identity.
        task_run_id: String,
        /// Bounded step report projected from durable run state.
        step: PackageStepReport,
    },
    /// A package run's terminal outcome was recorded through the command
    /// boundary. Emitted only when the outcome became durable this tick;
    /// re-observing an already-recorded run reaches no new checkpoint.
    PackageTerminalOutcomeRecorded {
        /// Durable run id derived from the plan identity.
        task_run_id: String,
        /// Deterministic task instance recording the run.
        task_instance_id: String,
        /// Deterministic outcome id accepted by the command boundary.
        outcome_id: String,
    },
    /// A claim-route task outcome was recorded through the command boundary.
    TaskOutcomeRecorded {
        /// Task instance resolved by the outcome.
        task_instance_id: String,
        /// Claim that fenced the invocation.
        claim_id: String,
        /// Deterministic outcome id recorded for the claim.
        outcome_id: String,
        /// Terminal status accepted by the command boundary.
        status: OutcomeStatus,
        /// True when the claim was resumed from a prior interrupted tick.
        resumed: bool,
    },
}

/// Diagnostic issue emitted by one dispatch actor tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchIssue {
    /// Plan id or task instance id the issue concerns, when known.
    pub item_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Report returned by one bounded dispatch actor tick.
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchTickReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Injected sequence supplied by the request.
    pub input_sequence: u64,
    /// Task network revision before actor work began.
    pub input_revision: u64,
    /// Task network revision after actor work completed.
    pub output_revision: u64,
    /// Bounded invocations released by this tick across both routes.
    pub items_attempted: usize,
    /// Bounded invocations resolved to durable committed progress. Failed
    /// bounded work is recorded but never counted here.
    pub items_committed: usize,
    /// Durable checkpoints reached by this tick in processing order.
    pub checkpoints: Vec<DispatchCheckpoint>,
    /// Retryable diagnostics observed during the tick.
    pub retryable_errors: Vec<DispatchIssue>,
    /// Fatal diagnostics observed during the tick.
    pub fatal_errors: Vec<DispatchIssue>,
    /// True when eligible work remained beyond this tick's budget.
    pub budget_exhausted: bool,
    /// What would make quiet or blocked dispatch work eligible (DBG-016).
    ///
    /// Derived from the ready-set snapshot this tick already computed —
    /// including its readiness diagnostics, which previously never left
    /// the domain; emission never gates or reorders dispatch.
    pub waiting_on: Vec<WaitingOnDeclaration>,
}

impl DispatchTickReport {
    fn new(actor_id: &str, input_sequence: u64, input_revision: u64) -> Self {
        Self {
            actor_id: actor_id.to_string(),
            input_sequence,
            input_revision,
            output_revision: input_revision,
            items_attempted: 0,
            items_committed: 0,
            checkpoints: Vec::new(),
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: Vec::new(),
        }
    }

    fn retryable(&mut self, item_id: Option<String>, code: &str, message: impl Into<String>) {
        self.retryable_errors.push(DispatchIssue {
            item_id,
            code: code.to_string(),
            message: message.into(),
        });
    }

    fn fatal(&mut self, item_id: Option<String>, code: &str, message: impl Into<String>) {
        self.fatal_errors.push(DispatchIssue {
            item_id,
            code: code.to_string(),
            message: message.into(),
        });
    }

    fn issue(&mut self, item_id: Option<String>, code: &str, error: DispatchPortError) {
        if error.retryable {
            self.retryable(item_id, code, error.message);
        } else {
            self.fatal(item_id, code, error.message);
        }
    }
}

/// Error that prevents a dispatch actor from being constructed.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum DispatchActorError {
    /// Actor construction input violated dispatch invariants.
    #[error("dispatch runtime actor construction is invalid: {0}")]
    InvalidConstruction(String),
    /// Durable execution storage failed before the actor could operate.
    #[error("dispatch runtime actor storage failed: {0}")]
    Storage(String),
}

/// Shares one package capability invocation port across per-plan runs.
///
/// The durable package execution takes its invoker by value, so the actor
/// hands each opened run a shared handle instead of requiring `Clone` on the
/// bound port.
struct SharedPackageInvoker<I>(Arc<I>);

#[async_trait]
impl<I: PackageStepInvoker> PackageStepInvoker for SharedPackageInvoker<I> {
    async fn invoke_capability(
        &self,
        instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        self.0.invoke_capability(instance, payload).await
    }

    fn compile_expansion(
        &self,
        compiled_task: &CompiledTaskRecord,
        request: &TaskExpansionRequest,
    ) -> Result<CompiledTaskDelta, ApiError> {
        self.0.compile_expansion(compiled_task, request)
    }
}

/// Distinguishes claim-route artifact persistence failures.
///
/// Drift under an existing artifact id is a bounded, recordable failure;
/// storage failure leaves the invocation unresolved so the fenced claim
/// replays on a later tick.
enum ArtifactPersistFailure {
    Drift(String),
    Storage(String),
}

/// Bounded execution dispatch actor over injected ports.
///
/// The actor owns run and claim identity derivation, durable package-route
/// dedupe, tick budget accounting, and the persist-before-outcome order. It
/// does not own package lowering, capability execution, command reduction,
/// aggregate outcome publication, or retry policy.
pub struct DispatchRuntimeActor<P, PI, CI> {
    actor_id: String,
    worker_id: String,
    db: sled::Db,
    progress: TaskProgressStore,
    preparer: P,
    package_invoker: Arc<PI>,
    claim_invoker: CI,
}

impl<P, PI, CI> DispatchRuntimeActor<P, PI, CI>
where
    P: PackageRunPreparer,
    PI: PackageStepInvoker,
    CI: ClaimedTaskInvoker,
{
    /// Creates a dispatch actor over a caller-owned execution database.
    ///
    /// The database holds the package progress store, package-run artifact
    /// repositories, and claim-route artifact repositories, so a fresh actor
    /// opened over the same database resumes every durable run and claim.
    pub fn new(
        worker_id: impl Into<String>,
        db: sled::Db,
        preparer: P,
        package_invoker: PI,
        claim_invoker: CI,
    ) -> Result<Self, DispatchActorError> {
        let worker_id = worker_id.into();
        if worker_id.is_empty() {
            return Err(DispatchActorError::InvalidConstruction(
                "worker id must not be empty".to_string(),
            ));
        }
        let progress = TaskProgressStore::open(db.clone())
            .map_err(|error| DispatchActorError::Storage(error.to_string()))?;
        Ok(Self {
            actor_id: DISPATCH_ACTOR_ID.to_string(),
            worker_id,
            db,
            progress,
            preparer,
            package_invoker: Arc::new(package_invoker),
            claim_invoker,
        })
    }

    /// Returns the stable actor id used in reports.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Returns the worker id fencing this actor's dispatch claims.
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Runs one bounded dispatch tick over both routes.
    ///
    /// The package route runs first because handoffs are explicit tick input;
    /// the claim route consumes the remaining budget, resuming this worker's
    /// interrupted claims before claiming new ready tasks. The tick returns
    /// only after every attempted item reached a durable resolution: package
    /// progress persisted by the step contract, or a task outcome recorded
    /// through the command boundary.
    pub async fn tick<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        request: DispatchTickRequest,
    ) -> Result<DispatchTickReport, DispatchActorError> {
        let input_revision = network.network_state().revision;
        let mut report = DispatchTickReport::new(&self.actor_id, request.sequence, input_revision);
        let mut remaining = request.max_items;

        self.drive_package_plans(network, &request.package_plans, &mut remaining, &mut report)
            .await;
        self.drive_claim_route(network, &mut remaining, &mut report)
            .await;

        report.output_revision = network.network_state().revision;
        Ok(report)
    }

    /// Drives at most one bounded package step per deduped plan handoff.
    async fn drive_package_plans<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        plans: &[TaskPackageRoutePlan],
        remaining: &mut usize,
        report: &mut DispatchTickReport,
    ) {
        let mut seen_plan_ids = BTreeSet::new();
        for plan in plans {
            // The handoff carries no durable record by design, so in-tick
            // dedupe keys on the deterministic plan identity and durable
            // dedupe keys on the derived run id below.
            if !seen_plan_ids.insert(plan.plan_id.as_str()) {
                continue;
            }
            let task_run_id = package_route_run_id(&plan.plan_id);
            match self.progress.load(&task_run_id) {
                Ok(Some(snapshot)) if snapshot_is_complete(&snapshot) => {
                    // Completed runs are never restarted and consume no budget.
                    report
                        .checkpoints
                        .push(DispatchCheckpoint::PackageRunAlreadyComplete {
                            plan_id: plan.plan_id.clone(),
                            task_run_id: task_run_id.clone(),
                        });
                    // Re-observation keeps terminal recording idempotently
                    // converged: a crash window inside a prior recording
                    // resumes here, and a recorded run is a no-op.
                    let lineage =
                        package_route_task_lineage(plan, snapshot.compiled_task.task_version);
                    self.record_package_run_completion(
                        network,
                        &task_run_id,
                        &snapshot,
                        Some(lineage),
                        report,
                    );
                    continue;
                }
                Ok(_) => {}
                Err(error) => {
                    report.retryable(
                        Some(plan.plan_id.clone()),
                        "package_progress_unavailable",
                        error.to_string(),
                    );
                    continue;
                }
            }
            if *remaining == 0 {
                report.budget_exhausted = true;
                continue;
            }

            let prepared = match self.preparer.prepare_package_run(plan, &task_run_id) {
                Ok(prepared) => prepared,
                Err(error) => {
                    report.issue(
                        Some(plan.plan_id.clone()),
                        "package_run_preparation_failed",
                        error,
                    );
                    continue;
                }
            };
            if prepared.init_payload.task_run_context.task_run_id != task_run_id {
                // A drifted run id would fork durable dedupe away from the plan
                // identity, so it is unconstructible rather than tolerated.
                report.fatal(
                    Some(plan.plan_id.clone()),
                    "package_run_identity_drift",
                    format!(
                        "prepared run id '{}' does not match derived run id '{}'",
                        prepared.init_payload.task_run_context.task_run_id, task_run_id
                    ),
                );
                continue;
            }

            let mut run = match DurablePackageExecution::open(
                self.db.clone(),
                prepared.compiled_task,
                prepared.init_payload,
                SharedPackageInvoker(Arc::clone(&self.package_invoker)),
            ) {
                Ok(run) => run,
                Err(error) => {
                    report.fatal(
                        Some(plan.plan_id.clone()),
                        "package_run_open_failed",
                        error.to_string(),
                    );
                    continue;
                }
            };
            if run.is_complete() {
                report
                    .checkpoints
                    .push(DispatchCheckpoint::PackageRunAlreadyComplete {
                        plan_id: plan.plan_id.clone(),
                        task_run_id: task_run_id.clone(),
                    });
                self.record_observed_package_completion(network, plan, &task_run_id, report);
                continue;
            }

            let granted = *remaining;
            match run
                .step(&PackageStepRequest {
                    max_ready_invocations: granted,
                })
                .await
            {
                Ok(step) => {
                    report.items_attempted += step.items_attempted;
                    report.items_committed += step.items_committed;
                    *remaining -= step.items_attempted.min(granted);
                    if step.budget_exhausted {
                        report.budget_exhausted = true;
                    }
                    let package_complete = step.package_complete;
                    report
                        .checkpoints
                        .push(DispatchCheckpoint::PackageStepDriven {
                            plan_id: plan.plan_id.clone(),
                            task_run_id: task_run_id.clone(),
                            step,
                        });
                    if package_complete {
                        self.record_observed_package_completion(
                            network,
                            plan,
                            &task_run_id,
                            report,
                        );
                    }
                }
                Err(error) => {
                    // The step persisted its recorded failures durably but the
                    // released count is lost with the report. Treat the whole
                    // granted wave as spent so a failing plan cannot hot-loop
                    // this tick's budget; failed work never counts as committed.
                    report.items_attempted += granted;
                    *remaining = 0;
                    report.retryable(
                        Some(plan.plan_id.clone()),
                        "package_step_failed",
                        error.to_string(),
                    );
                }
            }
        }
    }

    /// Records terminal outcome for a completion observed with its plan.
    ///
    /// Loads the durable snapshot the completion claim rests on; a missing
    /// snapshot for an observed-complete run is a broken invariant, not a
    /// retry condition.
    fn record_observed_package_completion<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        plan: &TaskPackageRoutePlan,
        task_run_id: &str,
        report: &mut DispatchTickReport,
    ) {
        match self.progress.load(task_run_id) {
            Ok(Some(snapshot)) => {
                let lineage = package_route_task_lineage(plan, snapshot.compiled_task.task_version);
                self.record_package_run_completion(
                    network,
                    task_run_id,
                    &snapshot,
                    Some(lineage),
                    report,
                );
            }
            Ok(None) => {
                report.fatal(
                    Some(plan.plan_id.clone()),
                    "terminal_recording_missing_progress",
                    format!("complete run '{task_run_id}' has no durable progress snapshot"),
                );
            }
            Err(error) => {
                report.retryable(
                    Some(plan.plan_id.clone()),
                    "package_progress_unavailable",
                    error.to_string(),
                );
            }
        }
    }

    /// Records one complete package run's terminal outcome idempotently.
    ///
    /// Terminal recording is command-boundary bookkeeping over already
    /// released package work, so it never consumes tick budget and never
    /// counts as a committed bounded invocation. A checkpoint is emitted only
    /// when the outcome became durable this tick.
    fn record_package_run_completion<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        task_run_id: &str,
        snapshot: &TaskExecutorSnapshot,
        lineage: Option<TaskLineage>,
        report: &mut DispatchTickReport,
    ) {
        let artifact_records = match load_package_run_artifact_records(self.db.clone(), task_run_id)
        {
            Ok(artifact_records) => artifact_records,
            Err(error) => {
                report.retryable(
                    Some(task_run_id.to_string()),
                    "terminal_recording_artifacts_unavailable",
                    error.to_string(),
                );
                return;
            }
        };
        match record_package_run_terminal_outcome(
            network,
            &self.worker_id,
            snapshot,
            artifact_records,
            lineage,
        ) {
            Ok(PackageRunRecording::Recorded(recording)) => {
                if !recording.already_recorded {
                    report
                        .checkpoints
                        .push(DispatchCheckpoint::PackageTerminalOutcomeRecorded {
                            task_run_id: recording.task_run_id,
                            task_instance_id: recording.task_instance_id,
                            outcome_id: recording.outcome.outcome_id,
                        });
                }
            }
            Ok(PackageRunRecording::Skipped {
                pending_instance_ids,
            }) => {
                // Callers only reach here after observing durable completion,
                // so a skip means the durable stores contradict each other.
                report.fatal(
                    Some(task_run_id.to_string()),
                    "terminal_recording_incomplete_run",
                    format!(
                        "run '{task_run_id}' observed complete but snapshot has {} pending units",
                        pending_instance_ids.len()
                    ),
                );
            }
            Err(error) if error.is_retryable() => {
                report.retryable(
                    Some(task_run_id.to_string()),
                    "terminal_recording_failed",
                    error.to_string(),
                );
            }
            Err(error) => {
                report.fatal(
                    Some(task_run_id.to_string()),
                    "terminal_recording_failed",
                    error.to_string(),
                );
            }
        }
    }

    /// Completes recording for a stranded terminal node from durable state.
    ///
    /// The claim route routes package-run terminal nodes here instead of
    /// invoking them: their work already ran through the package route, so
    /// the only legitimate remaining action is finishing the claim-and-record
    /// sequence a crash window interrupted. Needs no plan handoff because the
    /// node, and therefore its lineage, is already durable in network state.
    fn recover_package_run_terminal<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        task_run_id: &str,
        report: &mut DispatchTickReport,
    ) {
        match self.progress.load(task_run_id) {
            Ok(Some(snapshot)) => {
                self.record_package_run_completion(network, task_run_id, &snapshot, None, report);
            }
            Ok(None) => {
                report.fatal(
                    Some(task_run_id.to_string()),
                    "terminal_recording_missing_progress",
                    format!(
                        "terminal node exists for run '{task_run_id}' with no durable progress"
                    ),
                );
            }
            Err(error) => {
                report.retryable(
                    Some(task_run_id.to_string()),
                    "package_progress_unavailable",
                    error.to_string(),
                );
            }
        }
    }

    /// Resumes this worker's fenced claims, then claims new ready tasks.
    async fn drive_claim_route<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        remaining: &mut usize,
        report: &mut DispatchTickReport,
    ) {
        // Resume before claiming new work so a crash between artifact
        // persistence and outcome recording replays ahead of fresh claims.
        // Claim map order is deterministic by claim id.
        let resumable: Vec<Claim> = {
            let state = network.network_state();
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
        };
        for claim in resumable {
            // A running package-run terminal node is a recording crash
            // window, never invocable work: complete its recording without
            // charging bounded-invocation budget.
            if let Some(task_run_id) = package_run_id_for_task_instance(&claim.task_instance_id) {
                let task_run_id = task_run_id.to_string();
                self.recover_package_run_terminal(network, &task_run_id, report);
                continue;
            }
            if *remaining == 0 {
                report.budget_exhausted = true;
                return;
            }
            *remaining -= 1;
            report.items_attempted += 1;
            self.execute_claimed_task(network, &claim, true, report)
                .await;
        }

        // One ready-set snapshot per tick: dependents readied by this tick's
        // outcomes wait for a later tick, mirroring the package-step wave
        // rule, and a task failed this tick can never re-enter the snapshot.
        let ready = compute_ready_set(network.network_state());
        // The hardened DBG-016 rule: the readiness diagnostics the snapshot
        // already computed become declarations instead of being discarded,
        // and an empty ready set states that dispatch waits on one.
        for diagnostic in &ready.diagnostics {
            report.waiting_on.push(WaitingOnDeclaration {
                condition: readiness_condition(&diagnostic.code).to_string(),
                subject_key: diagnostic.task_instance_id.clone(),
                detail: diagnostic.message.clone(),
            });
        }
        if ready.task_instance_ids.is_empty() && ready.diagnostics.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                conditions::NO_READY_TASKS,
                format!("no claimable task at network revision {}", ready.revision),
            ));
        }
        for task_instance_id in &ready.task_instance_ids {
            // Same protection for a pending terminal node stranded before
            // its claim: recover the recording instead of dispatching it.
            if let Some(task_run_id) = package_run_id_for_task_instance(task_instance_id) {
                let task_run_id = task_run_id.to_string();
                self.recover_package_run_terminal(network, &task_run_id, report);
                continue;
            }
            if *remaining == 0 {
                report.budget_exhausted = true;
                return;
            }
            *remaining -= 1;
            report.items_attempted += 1;
            let Some(claim) = self.claim_ready_task(network, task_instance_id, report) else {
                continue;
            };
            self.execute_claimed_task(network, &claim, false, report)
                .await;
        }
    }

    /// Claims one ready task through the command boundary.
    fn claim_ready_task<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        task_instance_id: &str,
        report: &mut DispatchTickReport,
    ) -> Option<Claim> {
        let (claim_id, command) = {
            let state = network.network_state();
            let Some(node) = state.tasks.get(task_instance_id) else {
                report.fatal(
                    Some(task_instance_id.to_string()),
                    "ready_task_missing",
                    format!("ready task '{task_instance_id}' is absent from network state"),
                );
                return None;
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

        match network.submit_command(command) {
            Ok(Response::Accepted { .. }) | Ok(Response::Duplicate { .. }) => {
                let claim = network.network_state().claims.get(&claim_id).cloned();
                if claim.is_none() {
                    report.fatal(
                        Some(task_instance_id.to_string()),
                        "claim_record_missing",
                        format!("accepted claim '{claim_id}' is absent from network state"),
                    );
                }
                claim
            }
            Ok(Response::Rejected(rejection)) => {
                report.issue(
                    Some(task_instance_id.to_string()),
                    "claim_rejected",
                    rejection_error(&rejection),
                );
                None
            }
            Err(error) => {
                report.issue(
                    Some(task_instance_id.to_string()),
                    "claim_submit_failed",
                    error,
                );
                None
            }
        }
    }

    /// Invokes one fenced claim and records its outcome.
    ///
    /// Ordering invariant: emitted artifacts persist to the task-owned
    /// durable repository before the terminal outcome command is submitted.
    /// A crash between the two leaves a fenced `Running` claim that a later
    /// tick resumes; byte-identical re-emitted artifacts are accepted as
    /// durable replay so duplicate dispatch never duplicates domain outputs.
    async fn execute_claimed_task<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        claim: &Claim,
        resumed: bool,
        report: &mut DispatchTickReport,
    ) {
        let (node, init_payload) = {
            let state = network.network_state();
            let Some(node) = state.tasks.get(&claim.task_instance_id) else {
                report.fatal(
                    Some(claim.task_instance_id.clone()),
                    "claimed_task_missing",
                    format!(
                        "claimed task '{}' is absent from network state",
                        claim.task_instance_id
                    ),
                );
                return;
            };
            if node.lifecycle_epoch != claim.lifecycle_epoch {
                report.fatal(
                    Some(claim.task_instance_id.clone()),
                    "claim_epoch_stale",
                    format!(
                        "claim '{}' fences epoch {} but node is at epoch {}",
                        claim.claim_id, claim.lifecycle_epoch, node.lifecycle_epoch
                    ),
                );
                return;
            }
            let init_payload = match materialize_task_initialization(state, &claim.task_instance_id)
            {
                Ok(materialized) => materialized.payload,
                Err(error) => {
                    report.fatal(
                        Some(claim.task_instance_id.clone()),
                        "task_initialization_failed",
                        error
                            .diagnostics
                            .iter()
                            .map(|diagnostic| diagnostic.message.clone())
                            .collect::<Vec<_>>()
                            .join("; "),
                    );
                    return;
                }
            };
            (node.clone(), init_payload)
        };

        let outcome = match self
            .claim_invoker
            .invoke_claimed_task(&node, claim, &init_payload)
            .await
        {
            Ok(ClaimedInvocationOutcome::Completed(artifacts)) => {
                // Persist the task-owned canonical artifacts before the
                // terminal outcome so a crash in between replays instead of
                // losing or duplicating outputs.
                match self.persist_claim_artifacts(claim, &artifacts) {
                    Ok(()) => succeeded_outcome(claim, artifacts),
                    Err(ArtifactPersistFailure::Drift(message)) => {
                        // Drift under an existing artifact id is a bounded
                        // failure recorded through the command boundary.
                        failed_outcome(claim, message)
                    }
                    Err(ArtifactPersistFailure::Storage(message)) => {
                        report.retryable(
                            Some(claim.task_instance_id.clone()),
                            "artifact_persist_failed",
                            message,
                        );
                        return;
                    }
                }
            }
            Ok(ClaimedInvocationOutcome::Failed { error }) => failed_outcome(claim, error),
            Err(error) => {
                report.issue(
                    Some(claim.task_instance_id.clone()),
                    "claimed_invocation_unresolved",
                    error,
                );
                return;
            }
        };

        let status = outcome.status.clone();
        let outcome_id = outcome.outcome_id.clone();
        let command = {
            let state = network.network_state();
            command_request(
                state,
                format!("{outcome_id}::rev{}", state.revision),
                Command::RecordTaskOutcome(outcome),
            )
        };
        match network.submit_command(command) {
            Ok(Response::Accepted { .. }) | Ok(Response::Duplicate { .. }) => {
                if status == OutcomeStatus::Succeeded {
                    report.items_committed += 1;
                }
                report
                    .checkpoints
                    .push(DispatchCheckpoint::TaskOutcomeRecorded {
                        task_instance_id: claim.task_instance_id.clone(),
                        claim_id: claim.claim_id.clone(),
                        outcome_id,
                        status,
                        resumed,
                    });
            }
            Ok(Response::Rejected(rejection)) => {
                report.issue(
                    Some(claim.task_instance_id.clone()),
                    "outcome_rejected",
                    rejection_error(&rejection),
                );
            }
            Err(error) => {
                report.issue(
                    Some(claim.task_instance_id.clone()),
                    "outcome_submit_failed",
                    error,
                );
            }
        }
    }

    /// Persists claim-route artifacts durably with replay tolerance.
    ///
    /// A byte-identical artifact already present under the same id is
    /// accepted as durable replay from an interrupted tick, mirroring the
    /// package executor's crash-window rule. Content drift is rejected as a
    /// bounded failure.
    fn persist_claim_artifacts(
        &self,
        claim: &Claim,
        artifacts: &[ArtifactRecord],
    ) -> Result<(), ArtifactPersistFailure> {
        let mut repo =
            TaskArtifactRepo::open_sled(self.db.clone(), dispatch_claim_repo_id(&claim.claim_id))
                .map_err(|error| ArtifactPersistFailure::Storage(error.to_string()))?;
        for artifact in artifacts {
            match repo.get_artifact(&artifact.artifact_id) {
                Some(existing) if existing == artifact => {}
                Some(_) => {
                    return Err(ArtifactPersistFailure::Drift(format!(
                        "claim '{}' re-emitted artifact '{}' with drifted content",
                        claim.claim_id, artifact.artifact_id
                    )));
                }
                None => {
                    repo.append_artifact(artifact.clone())
                        .map_err(|error| ArtifactPersistFailure::Storage(error.to_string()))?;
                }
            }
        }
        repo.flush()
            .map_err(|error| ArtifactPersistFailure::Storage(error.to_string()))?;
        Ok(())
    }
}

/// True when every known work unit in the durable snapshot completed.
///
/// The snapshot's compiled task already contains every applied expansion, so
/// this comparison covers expanded units, not only the base graph.
fn snapshot_is_complete(snapshot: &TaskExecutorSnapshot) -> bool {
    snapshot.completed_instance_ids.len() == snapshot.compiled_task.capability_instances.len()
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
    }
}

fn rejection_error(rejection: &Rejection) -> DispatchPortError {
    match rejection {
        // Concurrency races against other writers resolve on a later tick.
        Rejection::StaleBase { expected, actual } => DispatchPortError::retryable(format!(
            "command base revision {expected} is stale against revision {actual}"
        )),
        Rejection::StateHashMismatch { .. } => {
            DispatchPortError::retryable("command base state hash is stale".to_string())
        }
        other => DispatchPortError::fatal(format!("command rejected: {other:?}")),
    }
}

/// Stable waiting-on condition vocabulary for readiness diagnostics.
fn readiness_condition(code: &ReadinessDiagnosticCode) -> &'static str {
    match code {
        ReadinessDiagnosticCode::MissingEndpoint => conditions::DEPENDENCY_ENDPOINT_MISSING,
        ReadinessDiagnosticCode::CycleDetected => conditions::DEPENDENCY_CYCLE,
        ReadinessDiagnosticCode::ConditionalDeferred => conditions::CONDITIONAL_EDGE_DEFERRED,
        ReadinessDiagnosticCode::ArtifactUnavailable => conditions::UPSTREAM_ARTIFACT_UNAVAILABLE,
    }
}
