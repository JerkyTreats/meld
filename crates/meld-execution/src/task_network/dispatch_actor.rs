//! Bounded execution dispatch actor over public execution ports.
//!
//! Owner: the task network dispatch boundary. One actor tick claims and
//! executes a bounded amount of ready work through injected ports and stops
//! at recorded task outcomes. The actor never sequences Task admission,
//! publication, or evidence.
//!
//! Ready tasks selected in deterministic ready-set order are
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
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::dispatch_actor::DispatchTickRequest;
//!
//! let request = DispatchTickRequest {
//!     sequence: 7,
//!     max_items: 2,
//! };
//!
//! assert_eq!(request.max_items, 2);
//! ```

use crate::authority::revalidate_action_authority;
use crate::task::{ArtifactRecord, TaskArtifactRepo, TaskInitializationPayload};
use crate::task_network::command::{Command, Request as CommandRequest, Response};
use crate::task_network::dispatch::{Claim, Outcome, OutcomeStatus, Request as DispatchRequest};
use crate::task_network::initialization::materialize_task_initialization;
use crate::task_network::mutation::Rejection;
use crate::task_network::readiness::compute_ready_set;
use crate::task_network::state::ReadinessDiagnosticCode;
use crate::task_network::state::{
    validate_task_admission_attribution, NetworkState, TaskNode, TaskStatus,
};
use crate::task_network::store::{InMemoryTaskNetworkStore, SledTaskNetworkStore};
use crate::waiting::{conditions, StructuralWakeAddress, WaitingOnDeclaration};
use async_trait::async_trait;
use meld_lang::AuthorityPolicyBinding;
use std::sync::Arc;

const DISPATCH_ACTOR_ID: &str = "execution.task_network.dispatch.runtime";

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
    /// Observe an existing claim's completed effects without executing new work.
    /// A closed admission fence permits this return path but never invocation.
    async fn recover_claimed_task(
        &self,
        _node: &TaskNode,
        _claim: &Claim,
        _init_payload: &TaskInitializationPayload,
    ) -> Result<Option<ClaimedInvocationOutcome>, DispatchPortError> {
        Ok(None)
    }

    /// Executes one bounded invocation for a claimed task node.
    async fn invoke_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError>;
}

/// Read-only observer for the Agent activation generation fencing dispatch.
///
/// Implementations live outside Execution and must read the current generation
/// on every call. The dispatch actor keeps no generation cache when this port
/// is bound.
pub trait AdmissionGenerationObserver: Send + Sync {
    /// Return the currently active generation for one attributed Agent.
    fn active_generation(&self, agent_id: &str) -> Result<Option<String>, String>;

    /// Recheck the whole admission fence in one owner observation.
    fn validates_admission(
        &self,
        attribution: &crate::task_network::TaskAdmissionAttribution,
    ) -> Result<bool, String> {
        Ok(attribution.admission_epoch.is_none()
            && self.active_generation(&attribution.agent_id)?.as_deref()
                == Some(attribution.activation_generation.as_str()))
    }
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
    /// Maximum bounded claim invocations one tick may release.
    pub max_items: usize,
}

/// Durable progress checkpoint reached by one dispatch actor tick.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchCheckpoint {
    /// A claimed task outcome was recorded through the command boundary.
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
    /// Task instance id the issue concerns, when known.
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
    /// Bounded claim invocations released by this tick.
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
/// The actor owns claim identity derivation, tick budget accounting, and the
/// persist-before-outcome order. It does not own Task admission, capability
/// execution, command reduction, publication, or retry policy.
pub struct DispatchRuntimeActor<CI> {
    actor_id: String,
    worker_id: String,
    db: sled::Db,
    claim_invoker: CI,
    authority_policy: Option<AuthorityPolicyBinding>,
    admission_generation: Option<String>,
    admission_generation_observer: Option<Arc<dyn AdmissionGenerationObserver>>,
    lifecycle: crate::lifecycle::NativeLifecycle,
}

impl<CI> DispatchRuntimeActor<CI>
where
    CI: ClaimedTaskInvoker,
{
    /// Creates a dispatch actor over a caller-owned execution database.
    ///
    /// The database holds claim artifact repositories, so a fresh actor opened
    /// over the same database resumes every durable claim.
    pub fn new(
        worker_id: impl Into<String>,
        db: sled::Db,
        claim_invoker: CI,
    ) -> Result<Self, DispatchActorError> {
        let worker_id = worker_id.into();
        if worker_id.is_empty() {
            return Err(DispatchActorError::InvalidConstruction(
                "worker id must not be empty".to_string(),
            ));
        }
        Ok(Self {
            actor_id: DISPATCH_ACTOR_ID.to_string(),
            worker_id,
            db,
            claim_invoker,
            authority_policy: None,
            admission_generation: None,
            admission_generation_observer: None,
            lifecycle: crate::lifecycle::NativeLifecycle::new(DISPATCH_ACTOR_ID),
        })
    }

    /// Resolve readiness and artifacts against the exact network and contained Task.
    pub fn resolves_wake(
        &self,
        network: &crate::task_network::store::SledTaskNetworkStore,
        wake: &crate::waiting::StructuralWakeAddress,
    ) -> bool {
        let state = network.state();
        match wake {
            StructuralWakeAddress::OwnerRevision(value) => crate::waiting::after_position(
                value,
                &format!("task-network::{}", state.network_id),
            ),
            StructuralWakeAddress::DurableOperation(value) => value
                .strip_prefix(&format!("task-artifact::{}::", state.network_id))
                .and_then(|tail| tail.strip_suffix("::available"))
                .is_some_and(|id| state.tasks.contains_key(id)),
            _ => false,
        }
    }

    /// Account for the native durable work in the borrowed, exclusively bound network.
    pub fn lifecycle_evidence(
        &self,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        network.flush().map_err(|error| error.to_string())?;
        let state = network.state();
        self.db.flush().map_err(|error| error.to_string())?;
        let checkpoint_ref = format!("task-network::{}::{}", state.network_id, state.revision);
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: vec![
                format!("task-network-state::{}", state.state_hash),
                crate::lifecycle::evidence_ref("dispatch-authority", &self.authority_policy)?,
            ],
            binding_refs: vec![
                format!("dispatch-worker::{}", self.worker_id),
                format!("task-network::{}", state.network_id),
            ],
            subscription_refs: vec![format!("ready-task-set::{}", state.network_id)],
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: crate::lifecycle::evidence_ref(
                "dispatch-durable-claims",
                &state.claims,
            )?,
        })
    }

    /// Author native start evidence while the network is borrowed.
    pub fn lifecycle_start(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .start(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native safe point evidence while the network is borrowed.
    pub fn lifecycle_safe_point(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .safe_point(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native stop evidence while the network is borrowed.
    pub fn lifecycle_stop(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .stop(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native release evidence while the network is borrowed.
    pub fn lifecycle_release(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .release(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Bind dispatch to one exact authority policy for independent enforcement.
    pub fn with_authority_policy(mut self, policy: AuthorityPolicyBinding) -> Self {
        self.authority_policy = Some(policy);
        self
    }

    /// Bind dispatch to the live Agent activation fencing admitted Tasks.
    pub fn with_admission_generation(mut self, generation: impl Into<String>) -> Self {
        self.admission_generation = Some(generation.into());
        self
    }

    /// Bind a live generation observer used before claims and invocations.
    pub fn with_admission_generation_observer(
        mut self,
        observer: Arc<dyn AdmissionGenerationObserver>,
    ) -> Self {
        self.admission_generation_observer = Some(observer);
        self
    }

    fn validate_task_authority(&self, state: &NetworkState, node: &TaskNode) -> Result<(), String> {
        validate_task_admission_attribution(state, node)?;
        let observed_generation = match (
            self.admission_generation_observer.as_ref(),
            node.lineage.admission.as_ref(),
        ) {
            (Some(observer), Some(attribution)) => {
                if !observer.validates_admission(attribution)? {
                    return Err(
                        "admitted Task activation generation or admission epoch is no longer open"
                            .to_string(),
                    );
                }
                Some(attribution.activation_generation.clone())
            }
            _ => self.admission_generation.clone(),
        };
        validate_task_authority(
            self.authority_policy.as_ref(),
            observed_generation.as_deref(),
            node,
        )
    }

    /// Returns the stable actor id used in reports.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Returns the worker id fencing this actor's dispatch claims.
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Runs one bounded dispatch tick over durable Task Network claims.
    ///
    /// Interrupted claims resume before new ready tasks are claimed. The tick
    /// returns only after every resolved item has a task outcome recorded
    /// through the command boundary.
    pub async fn tick<N: TaskNetworkCommandPort>(
        &self,
        network: &mut N,
        request: DispatchTickRequest,
    ) -> Result<DispatchTickReport, DispatchActorError> {
        let input_revision = network.network_state().revision;
        let mut report = DispatchTickReport::new(&self.actor_id, request.sequence, input_revision);
        let mut remaining = request.max_items;

        self.drive_claim_route(network, &mut remaining, &mut report)
            .await;

        report.output_revision = network.network_state().revision;
        Ok(report)
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
        // outcomes wait for a later tick, and a task failed this tick can
        // never re-enter the snapshot.
        let ready = compute_ready_set(network.network_state());
        // The hardened DBG-016 rule: the readiness diagnostics the snapshot
        // already computed become declarations instead of being discarded,
        // and an empty ready set states that dispatch waits on one.
        for diagnostic in &ready.diagnostics {
            report.waiting_on.push(WaitingOnDeclaration {
                condition: readiness_condition(&diagnostic.code).to_string(),
                subject_key: diagnostic.task_instance_id.clone(),
                detail: diagnostic.message.clone(),
                wake_addresses: readiness_wakes(
                    &diagnostic.code,
                    &ready.network_id,
                    ready.revision,
                    diagnostic.task_instance_id.as_deref(),
                ),
            });
        }
        if ready.task_instance_ids.is_empty() && ready.diagnostics.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                conditions::NO_READY_TASKS,
                format!("no claimable task at network revision {}", ready.revision),
                vec![StructuralWakeAddress::OwnerRevision(format!(
                    "task-network::{}::after::{}",
                    ready.network_id, ready.revision
                ))],
            ));
        }
        for task_instance_id in &ready.task_instance_ids {
            if *remaining == 0 {
                report.budget_exhausted = true;
                return;
            }
            let state = network.network_state();
            let authority_check = state
                .tasks
                .get(task_instance_id)
                .ok_or_else(|| format!("ready task '{task_instance_id}' is absent"))
                .and_then(|node| self.validate_task_authority(state, node));
            if let Err(error) = authority_check {
                report.fatal(
                    Some(task_instance_id.clone()),
                    "effective_authority_denied",
                    error,
                );
                continue;
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

        if let Err(error) = validate_task_admission_attribution(network.network_state(), &node) {
            report.fatal(
                Some(claim.task_instance_id.clone()),
                "effective_authority_denied",
                error,
            );
            return;
        }
        let invocation = match self.validate_task_authority(network.network_state(), &node) {
            Ok(()) => {
                self.claim_invoker
                    .invoke_claimed_task(&node, claim, &init_payload)
                    .await
            }
            Err(error) if resumed => match self
                .claim_invoker
                .recover_claimed_task(&node, claim, &init_payload)
                .await
            {
                Ok(Some(outcome)) => Ok(outcome),
                Ok(None) => {
                    report.fatal(
                        Some(claim.task_instance_id.clone()),
                        "effective_authority_denied",
                        error,
                    );
                    return;
                }
                Err(error) => Err(error),
            },
            Err(error) => {
                report.fatal(
                    Some(claim.task_instance_id.clone()),
                    "effective_authority_denied",
                    error,
                );
                return;
            }
        };
        let outcome = match invocation {
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

fn validate_task_authority(
    active_policy: Option<&AuthorityPolicyBinding>,
    active_generation: Option<&str>,
    node: &TaskNode,
) -> Result<(), String> {
    if node.lineage.authority_decision.is_some() && node.lineage.admission.is_none() {
        return Err(
            "Agent Task authority requires a durable Task admission attribution".to_string(),
        );
    }
    if let Some(admission) = &node.lineage.admission {
        let generation = active_generation.ok_or_else(|| {
            "admitted Task cannot be checked without a live activation generation".to_string()
        })?;
        if generation != admission.activation_generation {
            return Err(format!(
                "admitted Task activation generation '{}' is stale against '{}'",
                admission.activation_generation, generation
            ));
        }
        let policy = active_policy.ok_or_else(|| {
            "admitted Task authority cannot be checked without an active policy".to_string()
        })?;
        if policy.content_hash != admission.authority_policy_content_hash {
            return Err(
                "admitted Task authority policy content identity is no longer active".to_string(),
            );
        }
    }
    match (active_policy, &node.lineage.authority_decision) {
        (Some(policy), Some(decision)) => {
            revalidate_action_authority(policy, decision, &node.lineage.capability_type_id)
                .map_err(|error| error.to_string())
        }
        (Some(_), None) => Err("task has no authority decision under an active policy".to_string()),
        (None, Some(_)) => {
            Err("task authority cannot be checked without an active policy".to_string())
        }
        (None, None) => Ok(()),
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

fn readiness_wakes(
    code: &ReadinessDiagnosticCode,
    network_id: &str,
    revision: u64,
    task_instance_id: Option<&str>,
) -> Vec<StructuralWakeAddress> {
    match code {
        ReadinessDiagnosticCode::MissingEndpoint | ReadinessDiagnosticCode::ConditionalDeferred => {
            vec![StructuralWakeAddress::OwnerRevision(format!(
                "task-network::{network_id}::after::{revision}"
            ))]
        }
        ReadinessDiagnosticCode::CycleDetected => {
            vec![StructuralWakeAddress::OperatorAction(format!(
                "task-network-cycle::{network_id}"
            ))]
        }
        ReadinessDiagnosticCode::ArtifactUnavailable => {
            vec![StructuralWakeAddress::DurableOperation(format!(
                "task-artifact::{network_id}::{}::available",
                task_instance_id.unwrap_or(network_id)
            ))]
        }
    }
}
