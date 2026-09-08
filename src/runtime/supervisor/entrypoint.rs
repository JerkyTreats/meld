//! Explicit root supervisor lifecycle entrypoint.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use thiserror::Error;

use crate::runtime::assembly::{
    DesiredRuntimeState, RuntimeHandleFlushReport, RuntimeHandleStopReport, RuntimeLeaseContext,
    SupervisorStartupPackage,
};
use crate::runtime::contracts::{
    ActorBoundedStep, RuntimeActionOutcome, RuntimeActionRecord, RuntimeStatusPublisher,
    WorkBudget, WorkerTickReport,
};
use crate::runtime::error::RuntimeAssemblyError;
use crate::runtime::lifecycle::{
    ActivationGenerationStatus, ActivationLifecycleRequestV1, ActivationLifecycleStore,
    AssignmentLifecycleProjectionV1, LifecycleAcceptanceOutcome, LifecycleAction, LifecycleError,
    OwnerReadinessReceiptV1, OwnerWaitReceiptV1, ParticipantIncarnationV1,
    ParticipantLifecycleContextV1,
};
use crate::runtime::registration::{RegistrationKind, RegistrationLifecycle, RegistrationSet};
use crate::theory::{ParticipantKind, PreparedActivationClosureV1};
use crate::workspace::lifecycle::WorkspaceSourceLifecycle;

use super::contracts::{
    RestartCause, RestartPolicy, RuntimeDesiredState, RuntimeDiagnosticSummary, RuntimeHealth,
    RuntimeHealthSnapshot, RuntimeHealthStatus, RuntimeHeartbeat, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLease, RuntimeLeaseOwner, RuntimeRestartRecord,
    RuntimeShutdownState, RuntimeShutdownStatus, SupervisorContractError, SupervisorLifecycleEvent,
    SupervisorLifecycleEventType,
};
use super::reports::SupervisorReportStore;
use super::stepping::BoundedActorHandle;
use super::store::{SupervisorStore, SupervisorStoreError};

const DEFAULT_RESTART_ATTEMPT_LIMIT: u64 = 3;
const DEFAULT_RESTART_BACKOFF_MS: u64 = 0;
const INITIAL_EVENT_SEQUENCE: u64 = 0;
const NO_RESTART_ATTEMPTS: u64 = 0;
const NO_RUNTIME_ERRORS: u64 = 0;
const LIFECYCLE_EVENT_SEQUENCE_WIDTH: usize = 20;

/// Error returned by the root supervisor lifecycle entrypoint.
#[derive(Debug, Error)]
pub enum SupervisorRuntimeError {
    /// Supervisor storage failed.
    #[error("supervisor store error: {0}")]
    Store(#[from] SupervisorStoreError),
    /// Supervisor contract validation failed.
    #[error("supervisor contract error: {0}")]
    Contract(#[from] SupervisorContractError),
    /// Runtime assembly handoff failed.
    #[error("runtime assembly error: {0}")]
    Assembly(#[from] RuntimeAssemblyError),
    /// Canonical activation lifecycle transition failed.
    #[error("activation lifecycle error: {0}")]
    Lifecycle(#[from] LifecycleError),
    /// The caller supplied an invalid lifecycle command.
    #[error("invalid supervisor command: {0}")]
    InvalidCommand(String),
}

/// Command used to register and start one supervisor instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorStartCommand {
    /// Stable supervisor instance id for this process run.
    pub instance_id: String,
    /// Process start time in milliseconds.
    pub started_at_ms: u64,
    /// Restart policy assigned to desired runtime state from assembly.
    pub default_restart_policy: RestartPolicy,
    /// Maximum restart attempts recorded for one runtime.
    pub restart_attempt_limit: u64,
    /// Backoff recorded for each restart attempt.
    pub restart_backoff_ms: u64,
    /// Explicit registration classification for this composition.
    ///
    /// When present, declared kinds win: passive services are never leased,
    /// never ticked, and receive no actor health. When absent the supervisor
    /// classifies conservatively from whether the handle factory binds a
    /// semantic body. The actor-binding workstream replaces the conservative
    /// seam with owner-scoped registration production.
    pub registration_set: Option<RegistrationSet>,
}

/// Operational snapshot for a supervisor instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorStatusSnapshot {
    /// Supervisor instance id.
    pub instance_id: String,
    /// Product root owned operationally by this supervisor.
    pub product_root: PathBuf,
    /// Current supervisor instance status.
    pub instance_status: RuntimeInstanceStatus,
    /// Runtime status rows in stable runtime id order.
    pub runtimes: Vec<SupervisorRuntimeStatus>,
    /// Activation-wide lifecycle state derived from durable owner waits and resolvers.
    pub activation_liveness: Option<AssignmentLifecycleProjectionV1>,
}

/// Operator status row for one supervised runtime id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorRuntimeStatus {
    /// Stable runtime id.
    pub runtime_id: String,
    /// Whether desired state asks the supervisor to start the runtime.
    pub desired_enabled: bool,
    /// Whether assembly provided a factory for the runtime.
    pub factory_available: bool,
    /// Whether this process currently has a local started handle.
    pub handle_started: bool,
    /// Root classification for this runtime id.
    pub registration_kind: RegistrationKind,
    /// Truthful lifecycle projection derived from durable reports.
    ///
    /// `None` means this participant has no actor lifecycle: passive
    /// services are never assigned actor health.
    pub lifecycle: Option<RegistrationLifecycle>,
    /// Active lease id when present.
    pub active_lease_id: Option<String>,
    /// Active lease status when present.
    pub lease_status: Option<super::contracts::RuntimeLeaseStatus>,
    /// Active lease expiry time when present.
    pub lease_expires_at_ms: Option<u64>,
    /// Last heartbeat time when present.
    pub last_heartbeat_at_ms: Option<u64>,
    /// Age of the last heartbeat at the requested status time.
    pub heartbeat_age_ms: Option<u64>,
    /// Derived health status.
    pub health_status: RuntimeHealthStatus,
    /// Restart attempts visible to the supervisor.
    pub restart_count: u64,
    /// Last restart cause when one exists.
    pub last_restart_cause: Option<RestartCause>,
    /// Retryable issue count from the latest health signal.
    pub retryable_error_count: u64,
    /// Fatal issue count from the latest health signal.
    pub fatal_error_count: u64,
    /// Whether the latest diagnostic exhausted its budget.
    pub budget_exhausted: bool,
    /// Actor id from the latest diagnostic summary.
    pub last_diagnostic_actor_id: Option<String>,
    /// Latest lifecycle event type for this runtime.
    pub last_lifecycle_event: Option<SupervisorLifecycleEventType>,
}

/// Result of an explicit shutdown request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorShutdownReport {
    /// Stable shutdown id.
    pub shutdown_id: String,
    /// Runtime ids stopped by this request.
    pub stopped_runtime_ids: Vec<String>,
    /// Stop reports returned by runtime handles.
    pub stop_reports: Vec<RuntimeHandleStopReport>,
    /// Flush reports returned by runtime handles.
    pub flush_reports: Vec<RuntimeHandleFlushReport>,
}

/// Result of one restart policy evaluation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorRestartEvaluation {
    /// Runtime ids whose leases expired during recovery.
    pub expired_runtime_ids: Vec<String>,
    /// Runtime ids restarted during this pass.
    pub restarted_runtime_ids: Vec<String>,
}

/// Result of one foreground supervisor maintenance tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorTickReport {
    /// Runtime ids whose active leases were renewed.
    pub renewed_runtime_ids: Vec<String>,
    /// Runtime ids whose healthy heartbeat was accepted.
    pub heartbeat_runtime_ids: Vec<String>,
    /// Restart policy evaluation result for this tick.
    pub restart_evaluation: SupervisorRestartEvaluation,
    /// Full per-tick action records, one per bounded actor invocation.
    ///
    /// Each record is also persisted through the report store before the
    /// lifecycle summary is written; this copy exists so callers see the
    /// same durable truth without a read-back.
    pub actions: Vec<RuntimeActionRecord>,
}

/// Explicit root runtime supervisor.
///
/// This entrypoint owns operational lifecycle records only. Domain runtimes
/// still resume and validate semantic progress through their own stores.
pub struct RuntimeSupervisor<'a> {
    product_root: PathBuf,
    stores: &'a crate::runtime::storage::OpenProductStores,
    supervisor_store: &'a SupervisorStore,
    handle_factories: &'a crate::runtime::assembly::RuntimeHandleFactoryRegistry,
    lifecycle_config: crate::runtime::assembly::RuntimeLifecycleConfig,
    default_work_budget: WorkBudget,
    instance_id: String,
    started_at_ms: u64,
    desired: BTreeMap<String, SupervisorRuntimeDesired>,
    handles: BTreeMap<String, SupervisedRuntimeHandle>,
    startup_acquisitions: Option<Vec<RuntimeLeaseOwner>>,
    report_store: SupervisorReportStore,
    event_sequence: u64,
    action_sequence: u64,
    next_tick_start: usize,
    restart_attempt_limit: u64,
    restart_backoff_ms: u64,
    shutdown_completed: bool,
    registration_set: Option<RegistrationSet>,
    lifecycle_store: Option<&'a ActivationLifecycleStore>,
    prepared_activation: Option<&'a PreparedActivationClosureV1>,
    generation_id: Option<String>,
    incarnations: BTreeMap<String, ParticipantIncarnationV1>,
    passive_sources: BTreeMap<String, WorkspaceSourceLifecycle>,
    event_append: crate::runtime::ports::ProductEventAppendPort,
    retirement_recovery: &'a dyn crate::runtime::assembly::RetirementRuntimeRecovery,
}

/// Root classification for one desired runtime id.
///
/// Classification prefers an explicit registration kind. Where assembly does
/// not yet supply registrations the supervisor classifies conservatively
/// from whether the handle factory binds a semantic body; that conservative
/// arm is the seam the actor-binding workstream replaces with owner-scoped
/// registration production.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeClassification {
    /// Active actor with a bound semantic body: leased and ticked.
    ActiveBound,
    /// Required active binding that cannot resolve: never leased and never
    /// projected healthy.
    ActiveUnresolved,
    /// Passive service: never leased, never ticked, no actor health rows.
    Passive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivationStartMode {
    None,
    Publish,
    Recover,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SupervisorRuntimeDesired {
    runtime_id: RuntimeId,
    enabled: bool,
    factory_available: bool,
    restart_policy: RestartPolicy,
    classification: RuntimeClassification,
}

struct SupervisedRuntimeHandle {
    actor: BoundedActorHandle,
    owner: RuntimeLeaseOwner,
}

impl SupervisorStartCommand {
    /// Build a start command with conservative restart defaults.
    pub fn new(instance_id: impl Into<String>, started_at_ms: u64) -> Self {
        Self {
            instance_id: instance_id.into(),
            started_at_ms,
            default_restart_policy: RestartPolicy::OnHeartbeatExpiry,
            restart_attempt_limit: DEFAULT_RESTART_ATTEMPT_LIMIT,
            restart_backoff_ms: DEFAULT_RESTART_BACKOFF_MS,
            registration_set: None,
        }
    }
}

impl<'a> RuntimeSupervisor<'a> {
    /// Register a supervisor instance and start enabled runtime handles.
    pub fn start(
        package: SupervisorStartupPackage<'a>,
        command: SupervisorStartCommand,
    ) -> Result<Self, SupervisorRuntimeError> {
        if command.instance_id.trim().is_empty() {
            return Err(SupervisorRuntimeError::InvalidCommand(
                "instance id must not be empty".to_string(),
            ));
        }

        let registration_set = command
            .registration_set
            .clone()
            .or_else(|| package.registration_set.cloned());
        if package.prepared_activation.is_some()
            && registration_set.as_ref() != package.registration_set
        {
            return Err(SupervisorRuntimeError::InvalidCommand(
                "supervisor registrations differ from the prepared participant projection"
                    .to_string(),
            ));
        }
        let desired = desired_runtime_map(
            package.desired_runtime_state,
            command.default_restart_policy,
            registration_set.as_ref(),
            package.handle_factories,
        )?;
        let report_store = SupervisorReportStore::open(package.supervisor_store)?;
        let mut supervisor = Self {
            product_root: package.product_root.to_path_buf(),
            stores: package.stores,
            supervisor_store: package.supervisor_store,
            handle_factories: package.handle_factories,
            lifecycle_config: package.lifecycle_config.clone(),
            default_work_budget: package.default_work_budget.clone(),
            instance_id: command.instance_id,
            started_at_ms: command.started_at_ms,
            desired,
            handles: BTreeMap::new(),
            startup_acquisitions: Some(Vec::new()),
            report_store,
            event_sequence: INITIAL_EVENT_SEQUENCE,
            action_sequence: 0,
            next_tick_start: 0,
            restart_attempt_limit: command.restart_attempt_limit,
            restart_backoff_ms: command.restart_backoff_ms,
            shutdown_completed: false,
            registration_set,
            lifecycle_store: package.lifecycle_store,
            prepared_activation: package.prepared_activation,
            generation_id: None,
            incarnations: BTreeMap::new(),
            passive_sources: BTreeMap::new(),
            event_append: package.ports.event_append().clone(),
            retirement_recovery: package.retirement_recovery,
        };

        supervisor.register_instance()?;
        let started = (|| {
            supervisor.persist_desired_state()?;
            supervisor.recover_expired_leases(command.started_at_ms)?;
            supervisor.check_startup_ownership()?;
            let activation_mode = supervisor.prepare_activation()?;
            supervisor.start_enabled_runtimes(command.started_at_ms)?;
            supervisor.publish_activation(activation_mode)?;
            supervisor.retire_predecessors()?;
            supervisor.mark_instance(
                RuntimeInstanceStatus::Running,
                None,
                command.started_at_ms,
            )?;
            supervisor.supervisor_store.flush()?;
            Ok::<_, SupervisorRuntimeError>(())
        })();
        if let Err(error) = started {
            if let Err(cleanup) = supervisor.abort_start(command.started_at_ms) {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "{error}; startup acquisition cleanup also failed: {cleanup}"
                )));
            }
            return Err(error);
        }
        supervisor.startup_acquisitions = None;
        Ok(supervisor)
    }

    /// A prepared activation requires its whole participant set. Detect a live
    /// predecessor before changing its admission fence or acquiring partial ownership.
    fn check_startup_ownership(&self) -> Result<(), SupervisorRuntimeError> {
        if self.prepared_activation.is_none() {
            return Ok(());
        }
        for desired in self.desired.values().filter(|desired| {
            desired.enabled && desired.classification == RuntimeClassification::ActiveBound
        }) {
            if let Some(lease) = self
                .supervisor_store
                .get_active_runtime_lease(&desired.runtime_id)?
            {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{}' is owned by instance '{}' under lease '{}' until {} ms UTC",
                    desired.runtime_id, lease.instance_id, lease.lease_id, lease.expires_at_ms,
                )));
            }
        }
        Ok(())
    }

    /// No bounded step runs during startup. Unwind this attempt's operational
    /// acquisitions without retiring semantic history or releasing predecessor leases.
    fn abort_start(&mut self, now_ms: u64) -> Result<(), SupervisorRuntimeError> {
        if let (Some(store), Some(prepared), Some(generation_id)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) {
            if store
                .current_generation(&prepared.assignment.assignment_id)?
                .is_some_and(|generation| {
                    generation.generation_id == generation_id && generation.admission_open()
                })
            {
                store.interrupt(&prepared.assignment.assignment_id, generation_id)?;
            }
        }
        let mut failures = Vec::new();
        for owner in self
            .startup_acquisitions
            .take()
            .unwrap_or_default()
            .into_iter()
            .rev()
        {
            let context = self.lifecycle_context(owner.runtime_id.as_str());
            if let Some(mut runtime) = self.handles.remove(owner.runtime_id.as_str()) {
                let stopped = (|| {
                    if runtime.actor.is_started() {
                        if let Some(context) = context? {
                            let safe = runtime.actor.wait_for_lifecycle_safe_point(&context)?;
                            if !safe.safe_for_flush {
                                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                                    "startup runtime '{}' is not quiescent",
                                    owner.runtime_id
                                )));
                            }
                            runtime.actor.flush_resources()?;
                            runtime.actor.request_lifecycle_stop(&context)?;
                            runtime.actor.release_lifecycle(&context)?;
                        } else {
                            runtime.actor.request_stop();
                            runtime.actor.flush_resources()?;
                        }
                    }
                    Ok::<_, SupervisorRuntimeError>(())
                })();
                if let Err(error) = stopped {
                    failures.push(error.to_string());
                    continue;
                }
            }
            if let Err(error) = self.supervisor_store.release_runtime_lease(&owner, now_ms) {
                failures.push(error.to_string());
            }
        }
        self.passive_sources.clear();
        self.mark_instance(RuntimeInstanceStatus::Failed, Some(now_ms), now_ms)?;
        self.supervisor_store.flush()?;
        if failures.is_empty() {
            Ok(())
        } else {
            Err(SupervisorRuntimeError::InvalidCommand(failures.join("; ")))
        }
    }

    /// Return this supervisor instance id.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Borrow the durable report store for `RuntimeStatusReader` access.
    pub fn report_store(&self) -> &SupervisorReportStore {
        &self.report_store
    }

    /// Build an operator status snapshot without reading domain internals.
    pub fn status_snapshot(
        &self,
        now_ms: u64,
    ) -> Result<SupervisorStatusSnapshot, SupervisorRuntimeError> {
        let instance = self
            .supervisor_store
            .get_runtime_instance(&self.instance_id)?
            .ok_or_else(|| SupervisorRuntimeError::InvalidCommand("instance is missing".into()))?;
        let mut runtimes = Vec::with_capacity(self.desired.len());

        for desired in self.desired.values() {
            let active_lease = self
                .supervisor_store
                .get_active_runtime_lease(&desired.runtime_id)?;
            let heartbeat = self
                .supervisor_store
                .get_runtime_heartbeat(&desired.runtime_id)?;
            let health = self
                .supervisor_store
                .get_health_snapshot(&desired.runtime_id)?;
            let event = self
                .supervisor_store
                .latest_lifecycle_event_for_runtime(&desired.runtime_id)?;
            let last_heartbeat_at_ms = heartbeat.as_ref().map(|record| record.observed_at_ms);
            let heartbeat_age_ms =
                last_heartbeat_at_ms.map(|observed| now_ms.saturating_sub(observed));
            let default_health_status = if !desired.enabled {
                RuntimeHealthStatus::Stopped
            } else if desired.classification == RuntimeClassification::ActiveUnresolved {
                // A required active binding without a resolvable body is
                // truthfully unavailable, never defaulted toward health.
                RuntimeHealthStatus::Unhealthy
            } else {
                RuntimeHealthStatus::Unknown
            };
            let health_status = health
                .as_ref()
                .map(|record| record.status)
                .or_else(|| heartbeat.as_ref().map(|record| record.health.status))
                .unwrap_or(default_health_status);
            let restart_count = health
                .as_ref()
                .map(|record| record.restart_count)
                .unwrap_or(0);
            let last_restart_cause = health
                .as_ref()
                .and_then(|record| record.last_restart_cause.clone());
            let retryable_error_count = health
                .as_ref()
                .map(|record| record.retryable_error_count)
                .or_else(|| {
                    heartbeat
                        .as_ref()
                        .map(|record| record.health.retryable_error_count)
                })
                .unwrap_or(0);
            let fatal_error_count = health
                .as_ref()
                .map(|record| record.fatal_error_count)
                .or_else(|| {
                    heartbeat
                        .as_ref()
                        .map(|record| record.health.fatal_error_count)
                })
                .unwrap_or(0);
            let budget_exhausted = health
                .as_ref()
                .map(|record| record.budget_exhausted)
                .or_else(|| {
                    heartbeat
                        .as_ref()
                        .map(|record| record.health.budget_exhausted)
                })
                .unwrap_or(false);
            let last_diagnostic_actor_id = heartbeat
                .as_ref()
                .and_then(|record| record.diagnostic.as_ref())
                .map(|diagnostic| diagnostic.actor_id.clone());
            // Idle-versus-working detail comes from the preserved durable
            // report, not from a supervisor-side shadow of domain meaning.
            let mut latest_action = self
                .report_store
                .latest_action_for_runtime(desired.runtime_id.as_str())?;
            if let (Some(generation_id), Some(incarnation)) = (
                self.generation_id.as_deref(),
                self.incarnations.get(desired.runtime_id.as_str()),
            ) {
                latest_action = latest_action.filter(|record| {
                    action_matches_lifecycle(record, generation_id, &incarnation.incarnation_id)
                });
            }
            let registration_kind = match desired.classification {
                RuntimeClassification::Passive => RegistrationKind::PassiveService,
                _ => RegistrationKind::ActiveActor,
            };

            runtimes.push(SupervisorRuntimeStatus {
                runtime_id: desired.runtime_id.to_string(),
                desired_enabled: desired.enabled,
                factory_available: desired.factory_available,
                handle_started: self
                    .handles
                    .get(desired.runtime_id.as_str())
                    .is_some_and(|runtime| runtime.actor.is_started()),
                registration_kind,
                lifecycle: lifecycle_projection(
                    desired.classification,
                    desired.enabled,
                    health_status,
                    latest_action.as_ref(),
                ),
                active_lease_id: active_lease.as_ref().map(|record| record.lease_id.clone()),
                lease_status: active_lease.as_ref().map(|record| record.status),
                lease_expires_at_ms: active_lease.as_ref().map(|record| record.expires_at_ms),
                last_heartbeat_at_ms,
                heartbeat_age_ms,
                health_status,
                restart_count,
                last_restart_cause,
                retryable_error_count,
                fatal_error_count,
                budget_exhausted,
                last_diagnostic_actor_id,
                last_lifecycle_event: event.map(|record| record.event_type),
            });
        }

        let activation_liveness = self.activation_liveness_projection()?;
        Ok(SupervisorStatusSnapshot {
            instance_id: instance.instance_id,
            product_root: instance.product_root,
            instance_status: instance.status,
            runtimes,
            activation_liveness,
        })
    }

    fn activation_liveness_projection(
        &self,
    ) -> Result<Option<AssignmentLifecycleProjectionV1>, SupervisorRuntimeError> {
        let (Some(store), Some(prepared), Some(generation_id)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) else {
            return Ok(None);
        };
        let generation = store
            .generation(&prepared.assignment.assignment_id, generation_id)?
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(
                    "activation generation is absent during liveness projection".to_string(),
                )
            })?;
        let mut working_participants = BTreeSet::new();
        let mut idle_participants = BTreeSet::new();
        let mut failed_participants = BTreeSet::new();
        for participant_id in self.handles.keys() {
            let current_incarnation = generation.incarnations.get(participant_id);
            let action = self
                .report_store
                .latest_action_for_runtime(participant_id)?;
            let activity = current_incarnation
                .map(|incarnation| {
                    current_owner_activity(
                        action.as_ref(),
                        generation_id,
                        &incarnation.incarnation_id,
                    )
                })
                .unwrap_or(CurrentOwnerActivity::Missing);
            match activity {
                CurrentOwnerActivity::Working => {
                    working_participants.insert(participant_id.clone());
                }
                CurrentOwnerActivity::Idle => {
                    idle_participants.insert(participant_id.clone());
                }
                CurrentOwnerActivity::Failed => {
                    failed_participants.insert(participant_id.clone());
                }
                CurrentOwnerActivity::Missing => {}
            }
        }
        let mut resolvable_wakes = BTreeSet::new();
        for wait in generation.waits.values() {
            for wake_ref in &wait.wake_refs {
                let mut active_resolver = false;
                for (participant_id, runtime) in &self.handles {
                    let Some(incarnation) = generation.incarnations.get(participant_id) else {
                        continue;
                    };
                    if runtime.actor.resolves_lifecycle_wake(
                        generation_id,
                        &incarnation.incarnation_id,
                        wake_ref,
                    )? {
                        active_resolver = true;
                        break;
                    }
                }
                let mut passive_resolver = false;
                if !active_resolver {
                    for (participant_id, source) in &self.passive_sources {
                        if self.incarnations.get(participant_id)
                            != generation.incarnations.get(participant_id)
                        {
                            continue;
                        }
                        if source.resolves_wake(wake_ref)? {
                            passive_resolver = true;
                            break;
                        }
                    }
                }
                if active_resolver || passive_resolver {
                    resolvable_wakes.insert(wake_ref.clone());
                }
            }
        }
        store
            .project_liveness(
                &prepared.assignment.assignment_id,
                generation_id,
                &working_participants,
                &idle_participants,
                &failed_participants,
                &resolvable_wakes,
            )
            .map(Some)
            .map_err(Into::into)
    }

    /// Evaluate conservative restart policy for expired or retryable runtimes.
    pub fn evaluate_restart_policies(
        &mut self,
        now_ms: u64,
    ) -> Result<SupervisorRestartEvaluation, SupervisorRuntimeError> {
        let expired = self.recover_expired_leases(now_ms)?;
        let mut expired_runtime_ids = Vec::new();
        let mut restarted_runtime_ids = Vec::new();

        for lease in expired {
            let runtime_id = lease.runtime_id.to_string();
            expired_runtime_ids.push(runtime_id.clone());
            self.handles.remove(&runtime_id);
            if self.desired.get(&runtime_id).is_some_and(|desired| {
                desired.enabled && desired.restart_policy == RestartPolicy::OnHeartbeatExpiry
            }) {
                if self.restart_runtime(
                    &runtime_id,
                    Some(lease.lease_id),
                    RestartCause::HeartbeatExpired,
                    now_ms,
                )? {
                    restarted_runtime_ids.push(runtime_id);
                }
            } else {
                self.write_health_snapshot(
                    &lease.runtime_id,
                    None,
                    RuntimeHealthStatus::Unhealthy,
                    now_ms,
                    NO_RESTART_ATTEMPTS,
                    Some(RestartCause::HeartbeatExpired),
                )?;
            }
        }

        let retryable_ids = self
            .desired
            .values()
            .filter(|desired| {
                desired.enabled && desired.restart_policy == RestartPolicy::OnRetryableFailure
            })
            .map(|desired| desired.runtime_id.to_string())
            .collect::<Vec<_>>();

        for runtime_id in retryable_ids {
            let runtime = RuntimeId::new(runtime_id.clone())?;
            let Some(heartbeat) = self.supervisor_store.get_runtime_heartbeat(&runtime)? else {
                continue;
            };
            if !is_retryable_restart_signal(&heartbeat) {
                continue;
            }
            let Some(active_lease) = self.supervisor_store.get_active_runtime_lease(&runtime)?
            else {
                continue;
            };
            let owner = active_lease.owner();
            self.supervisor_store
                .release_runtime_lease(&owner, now_ms)?;
            self.write_lifecycle_event(
                Some(runtime.clone()),
                Some(owner.lease_id.clone()),
                now_ms,
                SupervisorLifecycleEventType::LeaseReleased,
                Some("lease released before retryable restart".to_string()),
            )?;
            if self.restart_runtime(
                &runtime_id,
                Some(active_lease.lease_id),
                RestartCause::RetryableFailure,
                now_ms,
            )? {
                restarted_runtime_ids.push(runtime_id);
            }
        }

        self.supervisor_store.flush()?;
        Ok(SupervisorRestartEvaluation {
            expired_runtime_ids,
            restarted_runtime_ids,
        })
    }

    /// Renew local leases, step each active actor once, and evaluate restarts.
    ///
    /// Every started actor is stepped exactly once per maintenance pass
    /// through the bounded-step contract and always yields a real report:
    /// a failed step becomes a truthful fatal report. The full report is
    /// persisted as an action record before the lifecycle summary derives
    /// from it.
    #[cfg(test)]
    pub(crate) fn step_owner_for_test(
        &mut self,
        runtime_id: &str,
        budget: WorkBudget,
    ) -> WorkerTickReport {
        // Schedule an already-started native incarnation while deliberately
        // holding other participants at their current durable positions.
        self.handles
            .get_mut(runtime_id)
            .unwrap()
            .actor
            .bounded_step(self.started_at_ms, &budget)
            .unwrap()
    }

    pub fn tick(&mut self, now_ms: u64) -> Result<SupervisorTickReport, SupervisorRuntimeError> {
        if let (Some(store), Some(prepared), Some(generation_id)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) {
            let generation = store
                .generation(&prepared.assignment.assignment_id, generation_id)?
                .ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(
                        "activation generation is absent before tick".to_string(),
                    )
                })?;
            if !generation.admission_open() {
                return Err(SupervisorRuntimeError::InvalidCommand(
                    "activation admission is closed".to_string(),
                ));
            }
        }
        let mut owners = self
            .handles
            .values()
            .map(|runtime| runtime.owner.clone())
            .collect::<Vec<_>>();
        // Rotate the first actor so continuous producers cannot always invalidate
        // a consumer's inputs immediately before its bounded invocation.
        if !owners.is_empty() {
            let start = self.next_tick_start % owners.len();
            owners.rotate_left(start);
            self.next_tick_start = (start + 1) % owners.len();
        }
        let mut renewed_runtime_ids = Vec::new();
        let mut heartbeat_runtime_ids = Vec::new();
        let mut actions = Vec::new();

        for owner in owners {
            let active_lease = self
                .supervisor_store
                .get_active_runtime_lease(&owner.runtime_id)?;
            if !active_lease
                .as_ref()
                .is_some_and(|lease| lease.is_owned_by(&owner) && lease.is_active_at(now_ms))
            {
                continue;
            }
            self.supervisor_store.renew_runtime_lease(
                &owner,
                now_ms,
                self.lifecycle_config.lease_duration_ms,
            )?;
            renewed_runtime_ids.push(owner.runtime_id.to_string());

            let budget = self.default_work_budget.clone();
            let lifecycle_context = self.lifecycle_context(owner.runtime_id.as_str())?;
            let Some(runtime) = self.handles.get_mut(owner.runtime_id.as_str()) else {
                continue;
            };
            // Exactly one bounded invocation per active actor per pass.
            let report = runtime
                .actor
                .bounded_step(now_ms, &budget)
                .unwrap_or_else(|error| {
                    WorkerTickReport::fatal(
                        owner.runtime_id.as_str(),
                        runtime_domain_id(owner.runtime_id.as_str()),
                        None,
                        "bounded_step",
                        "actor_bounded_step_failed",
                        error.message,
                    )
                });
            let health_status = health_status_from_tick_report(&report);
            let native_wait = if !report.made_progress()
                && report.retryable_errors.is_empty()
                && report.fatal_errors.is_empty()
                && !report.budget_exhausted
            {
                lifecycle_context
                    .as_ref()
                    .map(|context| runtime.actor.lifecycle_wait(context, &report))
                    .transpose()?
            } else {
                None
            };
            self.record_tick_liveness(owner.runtime_id.as_str(), &report, native_wait)?;

            // Persist the full report first; heartbeat and health snapshot
            // are summaries derived from this durable record.
            self.action_sequence += 1;
            let mut action = RuntimeActionRecord::from_worker_tick(
                format!(
                    "action:{}:{}:{}:{:020}",
                    self.instance_id, owner.runtime_id, now_ms, self.action_sequence
                ),
                owner.runtime_id.to_string(),
                now_ms,
                report.clone(),
            );
            if let Some(context) = lifecycle_context.as_ref() {
                action = action.bind_lifecycle(
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                );
            }
            self.report_store.publish_action(&action)?;
            actions.push(action);

            self.write_runtime_heartbeat(&owner, health_status, now_ms, Some(&report))?;
            self.write_lifecycle_event(
                Some(owner.runtime_id.clone()),
                Some(owner.lease_id.clone()),
                now_ms,
                SupervisorLifecycleEventType::HeartbeatAccepted,
                Some("heartbeat accepted".to_string()),
            )?;
            heartbeat_runtime_ids.push(owner.runtime_id.to_string());

            let existing_health = self
                .supervisor_store
                .get_health_snapshot(&owner.runtime_id)?;
            self.write_health_snapshot(
                &owner.runtime_id,
                Some(owner.lease_id.clone()),
                health_status,
                now_ms,
                existing_health
                    .as_ref()
                    .map(|snapshot| snapshot.restart_count)
                    .unwrap_or(0),
                existing_health.and_then(|snapshot| snapshot.last_restart_cause),
            )?;
        }

        let restart_evaluation = self.evaluate_restart_policies(now_ms)?;
        // One store flush covers the lifecycle trees and the sibling report
        // trees; preserved reports are durable before the tick returns.
        self.supervisor_store.flush()?;
        Ok(SupervisorTickReport {
            renewed_runtime_ids,
            heartbeat_runtime_ids,
            restart_evaluation,
            actions,
        })
    }

    /// Coordinate graceful shutdown and flush supervisor lifecycle records.
    pub fn request_shutdown(
        &mut self,
        now_ms: u64,
    ) -> Result<SupervisorShutdownReport, SupervisorRuntimeError> {
        let shutdown_id = format!("shutdown:{}:{}", self.instance_id, self.started_at_ms);
        if self.shutdown_completed {
            return Ok(SupervisorShutdownReport {
                shutdown_id,
                stopped_runtime_ids: Vec::new(),
                stop_reports: Vec::new(),
                flush_reports: Vec::new(),
            });
        }

        self.mark_instance(RuntimeInstanceStatus::Stopping, None, now_ms)?;
        self.supervisor_store
            .put_shutdown_state(&RuntimeShutdownState {
                shutdown_id: shutdown_id.clone(),
                instance_id: self.instance_id.clone(),
                requested_at_ms: now_ms,
                completed_at_ms: None,
                status: RuntimeShutdownStatus::Requested,
            })?;
        self.write_lifecycle_event(
            None,
            None,
            now_ms,
            SupervisorLifecycleEventType::ShutdownRequested,
            Some("shutdown requested".to_string()),
        )?;

        let activation_context = match (
            self.lifecycle_store.cloned(),
            self.prepared_activation.cloned(),
            self.generation_id.clone(),
        ) {
            (Some(store), Some(prepared), Some(generation_id)) => {
                store.begin_drain(&prepared.assignment.assignment_id, &generation_id)?;
                Some((store, prepared, generation_id))
            }
            _ => None,
        };
        let stop_order = activation_context
            .as_ref()
            .map(|(_, prepared, _)| reverse_participant_order(prepared))
            .unwrap_or_else(|| self.handles.keys().cloned().collect());

        let mut stop_reports = Vec::new();
        let mut flush_reports = Vec::new();
        for runtime_id in &stop_order {
            let lifecycle_context = self.lifecycle_context(runtime_id)?;
            if let Some(source) = self.passive_sources.get_mut(runtime_id) {
                let context = lifecycle_context.as_ref().ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(format!(
                        "passive source '{runtime_id}' has no lifecycle context"
                    ))
                })?;
                let fence = source.fence(context)?;
                if let Some((store, prepared, generation_id)) = &activation_context {
                    store.record_passive_fence(
                        &prepared.assignment.assignment_id,
                        generation_id,
                        fence,
                    )?;
                }
                continue;
            }
            let Some(runtime) = self.handles.get_mut(runtime_id) else {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "participant '{runtime_id}' has no native lifecycle owner"
                )));
            };
            let safe_point = match lifecycle_context.as_ref() {
                Some(context) => runtime.actor.wait_for_lifecycle_safe_point(context)?,
                None => {
                    let stop_report = runtime.actor.request_stop();
                    let safe_point = runtime.actor.wait_for_safe_point();
                    stop_reports.push(stop_report);
                    safe_point
                }
            };
            if !safe_point.safe_for_flush {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{}' did not reach a safe point",
                    safe_point.runtime_id
                )));
            }
            if let Some((store, prepared, generation_id)) = &activation_context {
                let receipt = safe_point.owner_safe_point.clone().ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(format!(
                        "runtime '{runtime_id}' supplied no native safe point"
                    ))
                })?;
                store.record_safe_point(
                    &prepared.assignment.assignment_id,
                    generation_id,
                    receipt,
                )?;
            }
            flush_reports.push(runtime.actor.flush_resources()?);
        }

        self.stores
            .flush_boundary()
            .map_err(RuntimeAssemblyError::from)?;

        if let Some((store, prepared, generation_id)) = &activation_context {
            store.commit_fenced_quiescence(&prepared.assignment.assignment_id, generation_id)?;

            for runtime_id in &stop_order {
                let context = self.lifecycle_context(runtime_id)?.ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(format!(
                        "participant '{runtime_id}' has no lifecycle context at stop"
                    ))
                })?;
                if let Some(source) = self.passive_sources.get_mut(runtime_id) {
                    let stop = source.stop(&context)?;
                    store.record_stop(&prepared.assignment.assignment_id, generation_id, stop)?;
                    continue;
                }
                let runtime = self.handles.get_mut(runtime_id).ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(format!(
                        "participant '{runtime_id}' has no native stop hook"
                    ))
                })?;
                let stop_report = runtime.actor.request_lifecycle_stop(&context)?;
                let stop = stop_report.owner_stop.clone().ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(format!(
                        "runtime '{runtime_id}' supplied no native stop receipt"
                    ))
                })?;
                store.record_stop(&prepared.assignment.assignment_id, generation_id, stop)?;
                stop_reports.push(stop_report);
            }
        }

        let stopped = stop_reports
            .iter()
            .map(|report| report.runtime_id.clone())
            .collect::<Vec<_>>();
        let owners = stop_order
            .iter()
            .filter_map(|runtime_id| {
                self.handles
                    .get(runtime_id)
                    .map(|runtime| runtime.owner.clone())
            })
            .collect::<Vec<_>>();
        for owner in owners {
            self.write_runtime_heartbeat(&owner, RuntimeHealthStatus::Stopped, now_ms, None)?;
            self.supervisor_store
                .release_runtime_lease(&owner, now_ms)?;
            self.write_lifecycle_event(
                Some(owner.runtime_id.clone()),
                Some(owner.lease_id.clone()),
                now_ms,
                SupervisorLifecycleEventType::LeaseReleased,
                Some("lease released during graceful shutdown".to_string()),
            )?;
            self.write_health_snapshot(
                &owner.runtime_id,
                Some(owner.lease_id.clone()),
                RuntimeHealthStatus::Stopped,
                now_ms,
                NO_RESTART_ATTEMPTS,
                None,
            )?;
            if let Some((store, prepared, generation_id)) = &activation_context {
                let context = self
                    .lifecycle_context(owner.runtime_id.as_str())?
                    .ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' has no lifecycle context at release",
                            owner.runtime_id
                        ))
                    })?;
                let receipt = self
                    .handles
                    .get_mut(owner.runtime_id.as_str())
                    .ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' disappeared before native release",
                            owner.runtime_id
                        ))
                    })?
                    .actor
                    .release_lifecycle(&context)?;
                store.record_release(&prepared.assignment.assignment_id, generation_id, receipt)?;
            }
        }

        if let Some((store, prepared, generation_id)) = &activation_context {
            for runtime_id in &stop_order {
                let context = self.lifecycle_context(runtime_id)?.ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(format!(
                        "passive source '{runtime_id}' has no lifecycle context at release"
                    ))
                })?;
                let Some(source) = self.passive_sources.get(runtime_id) else {
                    continue;
                };
                store.record_release(
                    &prepared.assignment.assignment_id,
                    generation_id,
                    source.release(&context)?,
                )?;
            }
        }

        if let Some((store, prepared, generation_id)) = &activation_context {
            store.retire(&prepared.assignment.assignment_id, generation_id)?;
        }

        self.handles.clear();
        self.supervisor_store
            .put_shutdown_state(&RuntimeShutdownState {
                shutdown_id: shutdown_id.clone(),
                instance_id: self.instance_id.clone(),
                requested_at_ms: now_ms,
                completed_at_ms: Some(now_ms),
                status: RuntimeShutdownStatus::Completed,
            })?;
        self.mark_instance(RuntimeInstanceStatus::Stopped, Some(now_ms), now_ms)?;
        self.write_lifecycle_event(
            None,
            None,
            now_ms,
            SupervisorLifecycleEventType::InstanceStopped,
            Some("supervisor instance stopped".to_string()),
        )?;
        self.supervisor_store.flush()?;
        self.shutdown_completed = true;

        Ok(SupervisorShutdownReport {
            shutdown_id,
            stopped_runtime_ids: stopped,
            stop_reports,
            flush_reports,
        })
    }

    /// Recover only native lifecycle hooks from a predecessor's exact preparation.
    /// Its pending semantic work remains in the existing owner stores under original lineage.
    fn retire_predecessors(&mut self) -> Result<(), SupervisorRuntimeError> {
        let (Some(store), Some(current), Some(current_id)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) else {
            return Ok(());
        };
        let assignment_id = &current.assignment.assignment_id;
        let history = store.assignment(assignment_id)?.ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand("assignment history is absent".into())
        })?;
        for prior in history.generations.values().filter(|generation| {
            generation.generation_id != current_id
                && matches!(
                    generation.status,
                    ActivationGenerationStatus::Draining
                        | ActivationGenerationStatus::FencedQuiescent
                )
        }) {
            let prepared = self
                .stores
                .pds_products
                .prepared_closure(&prior.prepared_id)
                .map_err(|error| SupervisorRuntimeError::InvalidCommand(error.to_string()))?
                .ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(
                        "predecessor preparation is absent".into(),
                    )
                })?;
            let mut handles = self
                .retirement_recovery
                .recover_retirement_handles(&prepared)?;
            // The current process must own every physical actor before it can reconstruct
            // a stopped predecessor. No second actor scheduler or lease is created.
            for participant in prepared
                .participant_plan
                .participants
                .iter()
                .filter(|participant| participant.kind != ParticipantKind::PassiveSource)
            {
                let owner = self
                    .handles
                    .get(&participant.participant_id)
                    .ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(
                            "predecessor owner has no current lease".into(),
                        )
                    })?;
                let lease = self
                    .supervisor_store
                    .get_active_runtime_lease(&owner.owner.runtime_id)?;
                if lease.as_ref().map(RuntimeLease::owner) != Some(owner.owner.clone()) {
                    return Err(SupervisorRuntimeError::InvalidCommand(
                        "predecessor recovery lost its native owner lease".into(),
                    ));
                }
            }
            store.recover_drain(assignment_id, &prior.generation_id)?;
            let mut contexts = BTreeMap::new();
            let mut passive = BTreeMap::new();
            for participant in &prepared.participant_plan.participants {
                let lease_ref = if participant.kind == ParticipantKind::PassiveSource {
                    if participant.participant_id != "workspace.source" {
                        return Err(SupervisorRuntimeError::InvalidCommand(
                            "unknown predecessor passive owner".into(),
                        ));
                    }
                    let source = WorkspaceSourceLifecycle::new(
                        self.event_append.clone(),
                        format!("workspace-source::{}", self.product_root.display()),
                    );
                    let binding = source.binding_ref().to_string();
                    passive.insert(participant.participant_id.clone(), source);
                    binding
                } else {
                    self.handles[&participant.participant_id]
                        .owner
                        .lease_id
                        .clone()
                };
                // This releases a recovered generation session, never the current
                // process's physical lease or passive-source binding.
                let lease_ref = format!("retirement-session::{}::{lease_ref}", prior.generation_id);
                let incarnation = store.create_incarnation(
                    assignment_id,
                    &prior.generation_id,
                    &participant.participant_id,
                    lease_ref.clone(),
                )?;
                let context = ParticipantLifecycleContextV1::new(
                    participant,
                    &prior.realizations[&participant.participant_id],
                    &incarnation,
                )?;
                let readiness = if let Some(source) = passive.get(&participant.participant_id) {
                    source.readiness(&context)?
                } else {
                    handles
                        .get_mut(&participant.participant_id)
                        .unwrap()
                        .start_after_lifecycle_lease(
                            RuntimeLeaseContext {
                                runtime_id: participant.participant_id.clone(),
                                lease_id: lease_ref,
                            },
                            &context,
                        )?
                        .owner_readiness
                        .ok_or_else(|| {
                            SupervisorRuntimeError::InvalidCommand(
                                "native retirement readiness is absent".into(),
                            )
                        })?
                };
                store.record_readiness(
                    assignment_id,
                    &prior.generation_id,
                    &participant.participant_id,
                    readiness,
                    &prepared,
                )?;
                contexts.insert(participant.participant_id.clone(), context);
            }
            let order = reverse_participant_order(&prepared);
            for id in &order {
                let context = &contexts[id];
                if let Some(source) = passive.get_mut(id) {
                    store.record_passive_fence(
                        assignment_id,
                        &prior.generation_id,
                        source.fence(context)?,
                    )?;
                } else {
                    let handle = handles.get_mut(id).unwrap();
                    let safe = handle.wait_for_lifecycle_safe_point(context)?;
                    let receipt = safe.owner_safe_point.ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(
                            "native retirement safe point is absent".into(),
                        )
                    })?;
                    store.record_safe_point(assignment_id, &prior.generation_id, receipt)?;
                    handle.flush_resources()?;
                }
            }
            store.commit_fenced_quiescence(assignment_id, &prior.generation_id)?;
            for id in &order {
                let context = &contexts[id];
                let receipt = if let Some(source) = passive.get_mut(id) {
                    source.stop(context)?
                } else {
                    handles
                        .get_mut(id)
                        .unwrap()
                        .request_lifecycle_stop(context)?
                        .owner_stop
                        .ok_or_else(|| {
                            SupervisorRuntimeError::InvalidCommand(
                                "native retirement stop is absent".into(),
                            )
                        })?
                };
                store.record_stop(assignment_id, &prior.generation_id, receipt)?;
            }
            for id in &order {
                let context = &contexts[id];
                let receipt = if let Some(source) = passive.get(id) {
                    source.release(context)?
                } else {
                    handles.get_mut(id).unwrap().release_lifecycle(context)?
                };
                store.record_release(assignment_id, &prior.generation_id, receipt)?;
            }
            store.retire(assignment_id, &prior.generation_id)?;
        }
        Ok(())
    }

    fn prepare_activation(&mut self) -> Result<ActivationStartMode, SupervisorRuntimeError> {
        let (Some(store), Some(prepared), Some(registrations)) = (
            self.lifecycle_store.cloned(),
            self.prepared_activation.cloned(),
            self.registration_set.clone(),
        ) else {
            return Ok(ActivationStartMode::None);
        };
        let assignment_id = prepared.assignment.assignment_id.clone();
        let current = store.current_generation(&assignment_id)?;
        let (action, expected_prior_generation, mode) = match current.as_ref() {
            Some(current) if current.prepared_id == prepared.prepared_id => {
                if current.admission_open() {
                    store.interrupt(&assignment_id, &current.generation_id)?;
                }
                (
                    LifecycleAction::Recover,
                    Some(current.generation_id.clone()),
                    ActivationStartMode::Recover,
                )
            }
            Some(current) => (
                LifecycleAction::Replace,
                Some(current.generation_id.clone()),
                ActivationStartMode::Publish,
            ),
            None => (
                LifecycleAction::Activate,
                None,
                ActivationStartMode::Publish,
            ),
        };
        let acceptance = store.accept(ActivationLifecycleRequestV1::new(
            format!(
                "supervisor-start::{}::{}::{}::{}",
                self.instance_id,
                lifecycle_action_key(action),
                prepared.prepared_id,
                expected_prior_generation.as_deref().unwrap_or("initial")
            ),
            action,
            prepared.clone(),
            expected_prior_generation,
        )?)?;
        if !matches!(
            acceptance.outcome,
            LifecycleAcceptanceOutcome::Accepted | LifecycleAcceptanceOutcome::Duplicate
        ) {
            return Err(SupervisorRuntimeError::InvalidCommand(format!(
                "activation lifecycle request was {:?}",
                acceptance.outcome
            )));
        }
        let generation_id = acceptance.decision.generation_id.ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand(
                "accepted activation has no generation identity".to_string(),
            )
        })?;
        let available_implementations = registrations
            .registrations
            .iter()
            .filter(|registration| {
                registration.kind == RegistrationKind::PassiveService
                    || self
                        .handle_factories
                        .get(&registration.runtime_id)
                        .is_some_and(|factory| factory.has_semantic_body())
            })
            .map(|registration| registration.runtime_id.clone())
            .collect();
        store.realize(
            &prepared,
            &generation_id,
            &registrations,
            &available_implementations,
        )?;
        self.generation_id = Some(generation_id.clone());

        for participant in prepared
            .participant_plan
            .participants
            .iter()
            .filter(|participant| participant.kind == ParticipantKind::PassiveSource)
        {
            if participant.participant_id != "workspace.source" {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "passive participant '{}' has no native lifecycle owner",
                    participant.participant_id
                )));
            }
            let source = WorkspaceSourceLifecycle::new(
                self.event_append.clone(),
                format!("workspace-source::{}", self.product_root.display()),
            );
            let incarnation = store.create_incarnation(
                &assignment_id,
                &generation_id,
                &participant.participant_id,
                source.binding_ref().to_string(),
            )?;
            let generation = store
                .generation(&assignment_id, &generation_id)?
                .ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(
                        "activation generation disappeared during realization".to_string(),
                    )
                })?;
            let realization = &generation.realizations[&participant.participant_id];
            let context =
                ParticipantLifecycleContextV1::new(participant, realization, &incarnation)?;
            store.record_readiness(
                &assignment_id,
                &generation_id,
                &participant.participant_id,
                source.readiness(&context)?,
                &prepared,
            )?;
            store.record_wait(
                &assignment_id,
                &generation_id,
                &participant.participant_id,
                source.wait(&context)?,
            )?;
            self.incarnations
                .insert(participant.participant_id.clone(), incarnation);
            self.passive_sources
                .insert(participant.participant_id.clone(), source);
        }
        Ok(mode)
    }

    fn publish_activation(
        &mut self,
        mode: ActivationStartMode,
    ) -> Result<(), SupervisorRuntimeError> {
        if mode == ActivationStartMode::None {
            return Ok(());
        }
        let store = self.lifecycle_store.ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand("activation store is absent".to_string())
        })?;
        let prepared = self.prepared_activation.ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand("prepared activation is absent".to_string())
        })?;
        let generation_id = self.generation_id.as_deref().ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand("activation generation is absent".to_string())
        })?;
        let generation = store
            .generation(&prepared.assignment.assignment_id, generation_id)?
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(
                    "activation generation disappeared before publication".to_string(),
                )
            })?;
        match mode {
            ActivationStartMode::Publish
                if generation.status == ActivationGenerationStatus::Ready =>
            {
                store.publish_current(&prepared.assignment.assignment_id, generation_id)?;
            }
            ActivationStartMode::Recover
                if generation.status == ActivationGenerationStatus::Interrupted =>
            {
                store.reopen(&prepared.assignment.assignment_id, generation_id, prepared)?;
            }
            _ => {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "activation generation is not publishable from {:?}",
                    generation.status
                )));
            }
        }
        Ok(())
    }

    fn create_runtime_lifecycle_context(
        &mut self,
        runtime_id: &str,
        lease_ref: &str,
    ) -> Result<Option<ParticipantLifecycleContextV1>, SupervisorRuntimeError> {
        let (Some(store), Some(prepared), Some(generation_id)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) else {
            return Ok(None);
        };
        let participant = prepared
            .participant_plan
            .participants
            .iter()
            .find(|participant| participant.participant_id == runtime_id)
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{runtime_id}' is absent from the participant plan"
                ))
            })?;
        let incarnation = store.create_incarnation(
            &prepared.assignment.assignment_id,
            generation_id,
            runtime_id,
            lease_ref.to_string(),
        )?;
        let generation = store
            .generation(&prepared.assignment.assignment_id, generation_id)?
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(
                    "activation generation disappeared during readiness".to_string(),
                )
            })?;
        let realization = &generation.realizations[runtime_id];
        let context = ParticipantLifecycleContextV1::new(participant, realization, &incarnation)?;
        self.incarnations
            .insert(runtime_id.to_string(), incarnation);
        Ok(Some(context))
    }

    fn record_runtime_readiness(
        &self,
        runtime_id: &str,
        receipt: Option<OwnerReadinessReceiptV1>,
    ) -> Result<(), SupervisorRuntimeError> {
        let (Some(store), Some(prepared), Some(generation_id)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) else {
            return Ok(());
        };
        let receipt = receipt.ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand(format!(
                "runtime '{runtime_id}' supplied no native owner readiness"
            ))
        })?;
        store.record_readiness(
            &prepared.assignment.assignment_id,
            generation_id,
            runtime_id,
            receipt,
            prepared,
        )?;
        Ok(())
    }

    fn lifecycle_context(
        &self,
        participant_id: &str,
    ) -> Result<Option<ParticipantLifecycleContextV1>, SupervisorRuntimeError> {
        let (Some(store), Some(prepared), Some(generation_id)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) else {
            return Ok(None);
        };
        let participant = prepared
            .participant_plan
            .participants
            .iter()
            .find(|participant| participant.participant_id == participant_id)
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "participant '{participant_id}' is absent from the accepted plan"
                ))
            })?;
        let generation = store
            .generation(&prepared.assignment.assignment_id, generation_id)?
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(
                    "activation generation disappeared during lifecycle transition".to_string(),
                )
            })?;
        let realization = generation.realizations.get(participant_id).ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand(format!(
                "participant '{participant_id}' has no realization"
            ))
        })?;
        let incarnation = self.incarnations.get(participant_id).ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand(format!(
                "participant '{participant_id}' has no current incarnation"
            ))
        })?;
        Ok(Some(ParticipantLifecycleContextV1::new(
            participant,
            realization,
            incarnation,
        )?))
    }

    fn register_instance(&mut self) -> Result<(), SupervisorRuntimeError> {
        self.supervisor_store
            .put_runtime_instance(&RuntimeInstance {
                instance_id: self.instance_id.clone(),
                product_root: self.product_root.clone(),
                started_at_ms: self.started_at_ms,
                stopped_at_ms: None,
                status: RuntimeInstanceStatus::Starting,
            })?;
        self.write_lifecycle_event(
            None,
            None,
            self.started_at_ms,
            SupervisorLifecycleEventType::InstanceRegistered,
            Some("supervisor instance registered".to_string()),
        )
    }

    fn mark_instance(
        &self,
        status: RuntimeInstanceStatus,
        stopped_at_ms: Option<u64>,
        _now_ms: u64,
    ) -> Result<(), SupervisorRuntimeError> {
        self.supervisor_store
            .put_runtime_instance(&RuntimeInstance {
                instance_id: self.instance_id.clone(),
                product_root: self.product_root.clone(),
                started_at_ms: self.started_at_ms,
                stopped_at_ms,
                status,
            })?;
        Ok(())
    }

    fn persist_desired_state(&self) -> Result<(), SupervisorRuntimeError> {
        for desired in self.desired.values() {
            self.supervisor_store
                .put_desired_runtime_state(&RuntimeDesiredState {
                    runtime_id: desired.runtime_id.clone(),
                    enabled: desired.enabled,
                    restart_policy: desired.restart_policy,
                })?;
        }
        Ok(())
    }

    fn start_enabled_runtimes(&mut self, now_ms: u64) -> Result<(), SupervisorRuntimeError> {
        let runtime_ids = self.desired.keys().cloned().collect::<Vec<_>>();
        for runtime_id in runtime_ids {
            let Some(desired) = self.desired.get(&runtime_id) else {
                continue;
            };
            // Passive services are not actors: no start, no lease, and no
            // actor health rows regardless of the enabled flag.
            if desired.classification == RuntimeClassification::Passive {
                continue;
            }
            if !desired.enabled {
                self.write_health_snapshot(
                    &desired.runtime_id.clone(),
                    None,
                    RuntimeHealthStatus::Stopped,
                    now_ms,
                    NO_RESTART_ATTEMPTS,
                    None,
                )?;
                continue;
            }
            self.start_runtime(&runtime_id, now_ms, NO_RESTART_ATTEMPTS, None)?;
        }
        Ok(())
    }

    fn start_runtime(
        &mut self,
        runtime_id: &str,
        now_ms: u64,
        restart_count: u64,
        last_restart_cause: Option<RestartCause>,
    ) -> Result<Option<RuntimeLeaseOwner>, SupervisorRuntimeError> {
        let desired = self
            .desired
            .get(runtime_id)
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{}' is not in desired state",
                    runtime_id
                ))
            })?
            .clone();
        let runtime = desired.runtime_id.clone();
        // Passive services never reach the start path from the supervisor's
        // own loops; guard defensively without writing actor health.
        if desired.classification == RuntimeClassification::Passive {
            return Ok(None);
        }
        self.write_lifecycle_event(
            Some(runtime.clone()),
            None,
            now_ms,
            SupervisorLifecycleEventType::RuntimeStartRequested,
            Some("runtime start requested".to_string()),
        )?;

        // A required active binding without a resolvable semantic body is
        // truthfully unavailable: no lease and never a healthy projection.
        if desired.classification != RuntimeClassification::ActiveBound
            || !desired.factory_available
        {
            self.write_health_snapshot(
                &runtime,
                None,
                RuntimeHealthStatus::Unhealthy,
                now_ms,
                restart_count,
                last_restart_cause,
            )?;
            return Ok(None);
        }

        let Some(factory) = self.handle_factories.get(runtime_id) else {
            self.write_health_snapshot(
                &runtime,
                None,
                RuntimeHealthStatus::Unhealthy,
                now_ms,
                restart_count,
                last_restart_cause,
            )?;
            return Ok(None);
        };

        let lease_id = format!(
            "lease:{}:{}:{}:{}",
            self.instance_id, runtime_id, restart_count, now_ms
        );
        let lease = match self.supervisor_store.acquire_runtime_lease(
            runtime.clone(),
            lease_id,
            self.instance_id.clone(),
            now_ms,
            self.lifecycle_config.lease_duration_ms,
        ) {
            Ok(lease) => lease,
            Err(error @ SupervisorStoreError::DuplicateActiveLease { .. }) => {
                if self.prepared_activation.is_some() {
                    return Err(error.into());
                }
                self.write_health_snapshot(
                    &runtime,
                    None,
                    RuntimeHealthStatus::Unhealthy,
                    now_ms,
                    restart_count,
                    last_restart_cause,
                )?;
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        if let Some(acquired) = &mut self.startup_acquisitions {
            acquired.push(lease.owner());
        }
        self.write_lifecycle_event(
            Some(runtime.clone()),
            Some(lease.lease_id.clone()),
            now_ms,
            SupervisorLifecycleEventType::LeaseAcquired,
            Some("lease acquired".to_string()),
        )?;

        let mut handle = factory.build_handle();
        let owner = lease.owner();
        let lifecycle_context =
            self.create_runtime_lifecycle_context(runtime_id, &owner.lease_id)?;
        let lease_context = RuntimeLeaseContext {
            runtime_id: runtime_id.to_string(),
            lease_id: lease.lease_id.clone(),
        };
        let start_report = match lifecycle_context.as_ref() {
            Some(context) => handle.start_after_lifecycle_lease(lease_context, context),
            None => handle.start_after_lease(lease_context),
        };
        self.handles.insert(
            runtime_id.to_string(),
            SupervisedRuntimeHandle {
                actor: BoundedActorHandle::new(handle),
                owner: owner.clone(),
            },
        );
        let start_report = start_report?;
        self.record_runtime_readiness(runtime_id, start_report.owner_readiness)?;
        // A started actor has not proven health yet: only a real bounded
        // tick report may promote it past starting.
        self.write_runtime_heartbeat(&owner, RuntimeHealthStatus::Starting, now_ms, None)?;
        self.write_lifecycle_event(
            Some(runtime.clone()),
            Some(owner.lease_id.clone()),
            now_ms,
            SupervisorLifecycleEventType::HeartbeatAccepted,
            Some("initial heartbeat accepted".to_string()),
        )?;
        self.write_health_snapshot(
            &runtime,
            Some(owner.lease_id.clone()),
            RuntimeHealthStatus::Starting,
            now_ms,
            restart_count,
            last_restart_cause,
        )?;
        Ok(Some(owner))
    }

    fn restart_runtime(
        &mut self,
        runtime_id: &str,
        previous_lease_id: Option<String>,
        cause: RestartCause,
        now_ms: u64,
    ) -> Result<bool, SupervisorRuntimeError> {
        let runtime = RuntimeId::new(runtime_id.to_string())?;
        let prior_restart_count = self
            .supervisor_store
            .get_health_snapshot(&runtime)?
            .map(|snapshot| snapshot.restart_count)
            .unwrap_or(0);
        let attempt = prior_restart_count + 1;
        if attempt > self.restart_attempt_limit {
            self.write_health_snapshot(
                &runtime,
                None,
                RuntimeHealthStatus::Unhealthy,
                now_ms,
                prior_restart_count,
                Some(cause),
            )?;
            return Ok(false);
        }

        let restart_id = format!(
            "restart:{}:{}:{}:{}",
            self.instance_id, runtime_id, attempt, now_ms
        );
        self.supervisor_store
            .put_restart_record(&RuntimeRestartRecord {
                restart_id,
                runtime_id: runtime.clone(),
                instance_id: self.instance_id.clone(),
                previous_lease_id,
                cause: cause.clone(),
                attempt,
                requested_at_ms: now_ms,
                backoff_ms: self.restart_backoff_ms,
            })?;
        self.write_lifecycle_event(
            Some(runtime),
            None,
            now_ms,
            SupervisorLifecycleEventType::RestartScheduled,
            Some("restart scheduled".to_string()),
        )?;
        let activation_recovery = match (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
        ) {
            (Some(store), Some(prepared), Some(generation_id)) => {
                let generation = store
                    .generation(&prepared.assignment.assignment_id, generation_id)?
                    .ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(
                            "activation generation is absent before restart".to_string(),
                        )
                    })?;
                if generation.admission_open() {
                    store.interrupt(&prepared.assignment.assignment_id, generation_id)?;
                }
                true
            }
            _ => false,
        };
        self.handles.remove(runtime_id);
        let restarted = self
            .start_runtime(runtime_id, now_ms, attempt, Some(cause))?
            .is_some();
        if restarted && activation_recovery {
            let store = self
                .lifecycle_store
                .expect("activation store checked above");
            let prepared = self
                .prepared_activation
                .expect("prepared activation checked above");
            let generation_id = self
                .generation_id
                .as_deref()
                .expect("generation checked above");
            store.reopen(&prepared.assignment.assignment_id, generation_id, prepared)?;
        }
        Ok(restarted)
    }

    fn recover_expired_leases(
        &mut self,
        now_ms: u64,
    ) -> Result<Vec<RuntimeLease>, SupervisorRuntimeError> {
        let expired = self.supervisor_store.recover_expired_leases(now_ms)?;
        for lease in &expired {
            self.write_lifecycle_event(
                Some(lease.runtime_id.clone()),
                Some(lease.lease_id.clone()),
                now_ms,
                SupervisorLifecycleEventType::LeaseExpired,
                Some("expired lease recovered".to_string()),
            )?;
        }
        Ok(expired)
    }

    fn write_runtime_heartbeat(
        &self,
        owner: &RuntimeLeaseOwner,
        status: RuntimeHealthStatus,
        now_ms: u64,
        tick_report: Option<&WorkerTickReport>,
    ) -> Result<(), SupervisorRuntimeError> {
        let heartbeat = RuntimeHeartbeat {
            runtime_id: owner.runtime_id.clone(),
            lease_id: owner.lease_id.clone(),
            instance_id: owner.instance_id.clone(),
            observed_at_ms: now_ms,
            health: RuntimeHealth {
                status,
                retryable_error_count: tick_report
                    .map(|report| report.retryable_errors.len() as u64)
                    .unwrap_or(NO_RUNTIME_ERRORS),
                fatal_error_count: tick_report
                    .map(|report| report.fatal_errors.len() as u64)
                    .unwrap_or(NO_RUNTIME_ERRORS),
                budget_exhausted: tick_report
                    .map(|report| report.budget_exhausted)
                    .unwrap_or(false),
                last_successful_tick_at_ms: match status {
                    RuntimeHealthStatus::Healthy => Some(now_ms),
                    _ => None,
                },
                last_error_code: tick_report.and_then(last_error_code),
            },
            diagnostic: Some(RuntimeDiagnosticSummary {
                actor_id: tick_report
                    .map(|report| report.actor_id.clone())
                    .unwrap_or_else(|| owner.runtime_id.to_string()),
                retryable_issue_count: tick_report
                    .map(|report| report.retryable_errors.len() as u64)
                    .unwrap_or(NO_RUNTIME_ERRORS),
                fatal_issue_count: tick_report
                    .map(|report| report.fatal_errors.len() as u64)
                    .unwrap_or(NO_RUNTIME_ERRORS),
                budget_exhausted: tick_report
                    .map(|report| report.budget_exhausted)
                    .unwrap_or(false),
                last_error_code: tick_report.and_then(last_error_code),
            }),
        };
        self.supervisor_store.write_runtime_heartbeat(&heartbeat)?;
        Ok(())
    }

    fn record_tick_liveness(
        &self,
        runtime_id: &str,
        report: &WorkerTickReport,
        native_wait: Option<OwnerWaitReceiptV1>,
    ) -> Result<(), SupervisorRuntimeError> {
        let (Some(store), Some(prepared), Some(generation_id), Some(_incarnation)) = (
            self.lifecycle_store,
            self.prepared_activation,
            self.generation_id.as_deref(),
            self.incarnations.get(runtime_id),
        ) else {
            return Ok(());
        };
        let assignment_id = &prepared.assignment.assignment_id;
        if report.made_progress()
            || !report.retryable_errors.is_empty()
            || !report.fatal_errors.is_empty()
            || report.budget_exhausted
        {
            return store
                .clear_wait(assignment_id, generation_id, runtime_id)
                .map_err(Into::into);
        }
        let native_wait = native_wait.ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand(format!(
                "runtime '{runtime_id}' supplied no native wait for a clean no-work step"
            ))
        })?;
        store.record_wait(assignment_id, generation_id, runtime_id, native_wait)?;
        Ok(())
    }

    fn write_health_snapshot(
        &self,
        runtime_id: &RuntimeId,
        lease_id: Option<String>,
        status: RuntimeHealthStatus,
        now_ms: u64,
        restart_count: u64,
        last_restart_cause: Option<RestartCause>,
    ) -> Result<(), SupervisorRuntimeError> {
        let heartbeat = self.supervisor_store.get_runtime_heartbeat(runtime_id)?;
        self.supervisor_store
            .put_health_snapshot(&RuntimeHealthSnapshot {
                runtime_id: runtime_id.clone(),
                lease_id,
                observed_at_ms: now_ms,
                status,
                last_heartbeat_at_ms: heartbeat.as_ref().map(|record| record.observed_at_ms),
                retryable_error_count: heartbeat
                    .as_ref()
                    .map(|record| record.health.retryable_error_count)
                    .unwrap_or(0),
                fatal_error_count: heartbeat
                    .as_ref()
                    .map(|record| record.health.fatal_error_count)
                    .unwrap_or(0),
                budget_exhausted: heartbeat
                    .as_ref()
                    .map(|record| record.health.budget_exhausted)
                    .unwrap_or(false),
                restart_count,
                last_restart_cause,
            })?;
        Ok(())
    }

    fn write_lifecycle_event(
        &mut self,
        runtime_id: Option<RuntimeId>,
        lease_id: Option<String>,
        occurred_at_ms: u64,
        event_type: SupervisorLifecycleEventType,
        message: Option<String>,
    ) -> Result<(), SupervisorRuntimeError> {
        self.event_sequence += 1;
        let event_id = format!(
            "event:{}:{}:{:0width$}",
            self.instance_id,
            occurred_at_ms,
            self.event_sequence,
            width = LIFECYCLE_EVENT_SEQUENCE_WIDTH
        );
        self.supervisor_store
            .put_lifecycle_event(&SupervisorLifecycleEvent {
                event_id,
                instance_id: self.instance_id.clone(),
                runtime_id,
                lease_id,
                occurred_at_ms,
                event_type,
                message,
            })?;
        Ok(())
    }
}

fn desired_runtime_map(
    states: &[DesiredRuntimeState],
    restart_policy: RestartPolicy,
    registration_set: Option<&RegistrationSet>,
    handle_factories: &crate::runtime::assembly::RuntimeHandleFactoryRegistry,
) -> Result<BTreeMap<String, SupervisorRuntimeDesired>, SupervisorRuntimeError> {
    if let Some(set) = registration_set {
        for registration in &set.registrations {
            if !states
                .iter()
                .any(|state| state.runtime_id == registration.runtime_id)
            {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "registration '{}' references unknown runtime id '{}'",
                    registration.registration_id, registration.runtime_id
                )));
            }
        }
    }

    let mut desired = BTreeMap::new();
    for state in states {
        let runtime_id = RuntimeId::new(state.runtime_id.clone())?;
        let has_semantic_body = handle_factories
            .get(&state.runtime_id)
            .is_some_and(|factory| factory.has_semantic_body());
        let explicit_kind = registration_set.and_then(|set| set.kind_of(&state.runtime_id));
        desired.insert(
            state.runtime_id.clone(),
            SupervisorRuntimeDesired {
                runtime_id,
                enabled: state.enabled,
                factory_available: state.factory_available,
                restart_policy,
                classification: classify_runtime(explicit_kind, has_semantic_body),
            },
        );
    }
    Ok(desired)
}

fn reverse_participant_order(prepared: &PreparedActivationClosureV1) -> Vec<String> {
    fn visit(
        participant_id: &str,
        specifications: &BTreeMap<&str, &crate::theory::ActivationParticipantSpec>,
        visited: &mut std::collections::BTreeSet<String>,
        ordered: &mut Vec<String>,
    ) {
        if !visited.insert(participant_id.to_string()) {
            return;
        }
        if let Some(specification) = specifications.get(participant_id) {
            for dependency in &specification.depends_on {
                visit(dependency, specifications, visited, ordered);
            }
        }
        ordered.push(participant_id.to_string());
    }

    let specifications = prepared
        .participant_plan
        .participants
        .iter()
        .map(|participant| (participant.participant_id.as_str(), participant))
        .collect::<BTreeMap<_, _>>();
    let mut visited = std::collections::BTreeSet::new();
    let mut ordered = Vec::new();
    for participant in &prepared.participant_plan.participants {
        visit(
            &participant.participant_id,
            &specifications,
            &mut visited,
            &mut ordered,
        );
    }
    ordered.reverse();
    ordered
}

fn lifecycle_action_key(action: LifecycleAction) -> &'static str {
    match action {
        LifecycleAction::Activate => "activate",
        LifecycleAction::Replace => "replace",
        LifecycleAction::Recover => "recover",
    }
}

/// Classify one runtime id from its declared kind and bound body.
///
/// A declared passive service wins unconditionally. A declared or assumed
/// active actor is bound only when a semantic body exists; an enabled active
/// requirement without a body is an unresolved required binding, never a
/// healthy placeholder. The `None` arm is the conservative classification
/// seam the actor-binding workstream replaces.
fn classify_runtime(
    explicit_kind: Option<RegistrationKind>,
    has_semantic_body: bool,
) -> RuntimeClassification {
    match explicit_kind {
        Some(RegistrationKind::PassiveService) => RuntimeClassification::Passive,
        Some(RegistrationKind::ActiveActor) | None => {
            if has_semantic_body {
                RuntimeClassification::ActiveBound
            } else {
                RuntimeClassification::ActiveUnresolved
            }
        }
    }
}

fn is_retryable_restart_signal(heartbeat: &RuntimeHeartbeat) -> bool {
    heartbeat.health.fatal_error_count == 0
        && matches!(
            heartbeat.health.status,
            RuntimeHealthStatus::Degraded | RuntimeHealthStatus::Unhealthy
        )
        && (heartbeat.health.retryable_error_count > 0
            || heartbeat
                .diagnostic
                .as_ref()
                .is_some_and(|diagnostic| diagnostic.retryable_issue_count > 0))
}

/// Project one real bounded tick report onto coarse operational health.
///
/// This function deliberately has no missing-report arm: absence of a report
/// is never health. Body-less roles never lease or tick, and a failed step
/// is converted into a truthful fatal report before projection.
fn health_status_from_tick_report(report: &WorkerTickReport) -> RuntimeHealthStatus {
    if !report.fatal_errors.is_empty() {
        RuntimeHealthStatus::Unhealthy
    } else if !report.retryable_errors.is_empty() || report.budget_exhausted {
        RuntimeHealthStatus::Degraded
    } else {
        RuntimeHealthStatus::Healthy
    }
}

/// Project the truthful registration lifecycle for one status row.
///
/// Passive services carry no actor lifecycle. Disabled participants are
/// stopped. An enabled active requirement without a bound body is an
/// unresolved required binding. A bound actor's idle-versus-working detail
/// derives from its latest durable action record, so the lifecycle summary
/// derives from the preserved report rather than replacing it.
fn lifecycle_projection(
    classification: RuntimeClassification,
    enabled: bool,
    health_status: RuntimeHealthStatus,
    latest_action: Option<&RuntimeActionRecord>,
) -> Option<RegistrationLifecycle> {
    match classification {
        RuntimeClassification::Passive => None,
        _ if !enabled => Some(RegistrationLifecycle::Stopped),
        RuntimeClassification::ActiveUnresolved => {
            Some(RegistrationLifecycle::UnresolvedRequiredBinding)
        }
        RuntimeClassification::ActiveBound => Some(match health_status {
            RuntimeHealthStatus::Stopped => RegistrationLifecycle::Stopped,
            RuntimeHealthStatus::Unknown | RuntimeHealthStatus::Starting => {
                RegistrationLifecycle::Starting
            }
            RuntimeHealthStatus::Unhealthy => RegistrationLifecycle::Unhealthy,
            RuntimeHealthStatus::Healthy | RuntimeHealthStatus::Degraded => latest_action
                .map(lifecycle_from_action_record)
                .unwrap_or(RegistrationLifecycle::Starting),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentOwnerActivity {
    Working,
    Idle,
    Failed,
    Missing,
}

fn current_owner_activity(
    record: Option<&RuntimeActionRecord>,
    generation_id: &str,
    incarnation_id: &str,
) -> CurrentOwnerActivity {
    let Some(record) =
        record.filter(|record| action_matches_lifecycle(record, generation_id, incarnation_id))
    else {
        return CurrentOwnerActivity::Missing;
    };
    match record.outcome {
        RuntimeActionOutcome::Started | RuntimeActionOutcome::Succeeded => {
            CurrentOwnerActivity::Working
        }
        RuntimeActionOutcome::NoWork | RuntimeActionOutcome::Duplicate => {
            CurrentOwnerActivity::Idle
        }
        RuntimeActionOutcome::Rejected
        | RuntimeActionOutcome::Blocked
        | RuntimeActionOutcome::RetryableFailure
        | RuntimeActionOutcome::FatalFailure
        | RuntimeActionOutcome::Cancelled => CurrentOwnerActivity::Failed,
    }
}

fn action_matches_lifecycle(
    record: &RuntimeActionRecord,
    generation_id: &str,
    incarnation_id: &str,
) -> bool {
    record.generation_id.as_deref() == Some(generation_id)
        && record.incarnation_id.as_deref() == Some(incarnation_id)
}

/// Project one preserved action record onto the registration lifecycle.
fn lifecycle_from_action_record(record: &RuntimeActionRecord) -> RegistrationLifecycle {
    match record.outcome {
        RuntimeActionOutcome::FatalFailure => RegistrationLifecycle::Unhealthy,
        RuntimeActionOutcome::NoWork => RegistrationLifecycle::ActiveIdle,
        _ => RegistrationLifecycle::ActiveWorking,
    }
}

/// Return the owning domain segment of one runtime id for fatal reports.
fn runtime_domain_id(runtime_id: &str) -> &str {
    runtime_id.split('.').next().unwrap_or(runtime_id)
}

fn last_error_code(report: &WorkerTickReport) -> Option<String> {
    report
        .fatal_errors
        .first()
        .or_else(|| report.retryable_errors.first())
        .map(|issue| issue.code.clone())
}

#[cfg(test)]
mod tests {
    use crate::runtime::assembly::{ProductRuntimeAssembly, ProductRuntimeConfig};
    use meld_events::{AppendMode, DomainObjectRef, EventEnvelope};
    use serde_json::json;

    use super::*;

    #[test]
    fn activation_activity_rejects_failure_and_predecessor_reports() {
        let fatal = WorkerTickReport::fatal(
            "execution.task_admission",
            "execution",
            Some("planning"),
            "task_network_revision",
            "planning_failed",
            "planning failed",
        );
        let current = RuntimeActionRecord::from_worker_tick(
            "action-fatal",
            "execution.task_admission",
            10,
            fatal,
        )
        .bind_lifecycle("generation-1".into(), "incarnation-2".into());
        assert_eq!(
            current_owner_activity(Some(&current), "generation-1", "incarnation-2"),
            CurrentOwnerActivity::Failed
        );
        assert_eq!(
            current_owner_activity(Some(&current), "generation-1", "incarnation-3"),
            CurrentOwnerActivity::Missing
        );
        assert_eq!(
            current_owner_activity(Some(&current), "generation-2", "incarnation-2"),
            CurrentOwnerActivity::Missing
        );

        let mut succeeded = current.clone();
        succeeded.outcome = RuntimeActionOutcome::Succeeded;
        assert_eq!(
            current_owner_activity(Some(&succeeded), "generation-1", "incarnation-2"),
            CurrentOwnerActivity::Working
        );
        let mut idle = current;
        idle.outcome = RuntimeActionOutcome::NoWork;
        assert_eq!(
            current_owner_activity(Some(&idle), "generation-1", "incarnation-2"),
            CurrentOwnerActivity::Idle
        );
    }

    #[test]
    fn supervisor_start_registers_instance_starts_enabled_runtimes_and_reports_status() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        let status = supervisor.status_snapshot(150).unwrap();

        assert_eq!(status.instance_id, "instance-a");
        assert_eq!(status.product_root, temp.path());
        assert_eq!(status.instance_status, RuntimeInstanceStatus::Running);
        assert_eq!(status.runtimes.len(), 14);
        // Only the two roles with concrete semantic
        // bodies start; the remaining enabled roles stay unresolved instead
        // of leasing as healthy no-op placeholders.
        assert_eq!(
            status
                .runtimes
                .iter()
                .filter(|runtime| runtime.desired_enabled && runtime.handle_started)
                .count(),
            2
        );
        let event_append = runtime_status(&status, "event.append");
        assert!(event_append.desired_enabled);
        assert!(event_append.factory_available);
        assert!(event_append.handle_started);
        // A started actor that has not ticked yet is starting, not healthy.
        assert_eq!(event_append.health_status, RuntimeHealthStatus::Starting);
        assert_eq!(
            event_append.lifecycle,
            Some(RegistrationLifecycle::Starting)
        );
        assert_eq!(event_append.last_heartbeat_at_ms, Some(100));
        assert_eq!(event_append.heartbeat_age_ms, Some(50));
        assert_eq!(
            event_append.last_lifecycle_event,
            Some(SupervisorLifecycleEventType::HeartbeatAccepted)
        );
        let dispatch = runtime_status(&status, "execution.task_dispatch");
        assert!(!dispatch.desired_enabled);
        assert!(!dispatch.handle_started);
        assert_eq!(dispatch.health_status, RuntimeHealthStatus::Stopped);
        assert_eq!(dispatch.lifecycle, Some(RegistrationLifecycle::Stopped));
    }

    #[test]
    fn body_less_enabled_roles_are_never_healthy_and_hold_no_lease() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        supervisor.tick(120).unwrap();
        let status = supervisor.status_snapshot(130).unwrap();

        let body_less = status
            .runtimes
            .iter()
            .filter(|runtime| runtime.desired_enabled && !runtime.handle_started)
            .collect::<Vec<_>>();
        assert_eq!(body_less.len(), 11);
        for runtime in body_less {
            assert_ne!(
                runtime.health_status,
                RuntimeHealthStatus::Healthy,
                "{} must never be healthy without a semantic body",
                runtime.runtime_id
            );
            assert_eq!(
                runtime.lifecycle,
                Some(RegistrationLifecycle::UnresolvedRequiredBinding),
                "{}",
                runtime.runtime_id
            );
            assert!(
                runtime.active_lease_id.is_none(),
                "{} must hold no lease",
                runtime.runtime_id
            );
            let runtime_id = RuntimeId::new(runtime.runtime_id.clone()).unwrap();
            assert!(assembly
                .supervisor_store()
                .get_active_runtime_lease(&runtime_id)
                .unwrap()
                .is_none());
            assert!(assembly
                .supervisor_store()
                .get_runtime_heartbeat(&runtime_id)
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn passive_registrations_receive_no_lease_and_no_health_rows() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec![
            "event.append".to_string(),
            "execution.task_network_command".to_string(),
        ];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.registration_set = Some(crate::runtime::registration::RegistrationSet {
            registrations: vec![crate::runtime::registration::RuntimeRegistration {
                registration_id: "registration-command".to_string(),
                runtime_id: "execution.task_network_command".to_string(),
                kind: RegistrationKind::PassiveService,
                required_resources: Vec::new(),
            }],
        });
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();

        supervisor.tick(120).unwrap();
        let status = supervisor.status_snapshot(130).unwrap();
        let passive = runtime_status(&status, "execution.task_network_command");
        let runtime_id = RuntimeId::new("execution.task_network_command").unwrap();

        assert_eq!(passive.registration_kind, RegistrationKind::PassiveService);
        assert_eq!(passive.lifecycle, None);
        assert!(!passive.handle_started);
        assert!(passive.active_lease_id.is_none());
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert!(assembly
            .supervisor_store()
            .get_runtime_heartbeat(&runtime_id)
            .unwrap()
            .is_none());
        assert!(assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .is_none());
        assert!(supervisor
            .report_store()
            .latest_action_for_runtime("execution.task_network_command")
            .unwrap()
            .is_none());
    }

    #[test]
    fn registration_set_with_unknown_runtime_id_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.registration_set = Some(crate::runtime::registration::RegistrationSet {
            registrations: vec![crate::runtime::registration::RuntimeRegistration {
                registration_id: "registration-unknown".to_string(),
                runtime_id: "future.unknown".to_string(),
                kind: RegistrationKind::ActiveActor,
                required_resources: Vec::new(),
            }],
        });

        let error = match RuntimeSupervisor::start(assembly.supervisor_startup_package(), command) {
            Ok(_) => panic!("unknown registration runtime id should fail"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            SupervisorRuntimeError::InvalidCommand(message)
                if message.contains("unknown runtime id")
        ));
    }

    #[test]
    fn supervisor_start_keeps_missing_factory_as_unhealthy_status() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["future.runtime".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        let status = supervisor.status_snapshot(100).unwrap();
        let future = runtime_status(&status, "future.runtime");

        assert!(future.desired_enabled);
        assert!(!future.factory_available);
        assert!(!future.handle_started);
        assert_eq!(future.health_status, RuntimeHealthStatus::Unhealthy);
        assert!(future.active_lease_id.is_none());
    }

    #[test]
    fn supervisor_start_recovers_expired_lease_before_starting_new_owner() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();
        assembly
            .supervisor_store()
            .acquire_runtime_lease(runtime_id.clone(), "lease-old", "old-instance", 10, 10)
            .unwrap();

        let supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 30),
        )
        .unwrap();

        let old_lease = assembly
            .supervisor_store()
            .get_runtime_lease(&runtime_id, "lease-old")
            .unwrap()
            .unwrap();
        let active = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(
            old_lease.status,
            super::super::contracts::RuntimeLeaseStatus::Expired
        );
        assert_eq!(active.instance_id, "instance-a");
        assert_ne!(active.lease_id, "lease-old");
        assert!(
            runtime_status(&supervisor.status_snapshot(31).unwrap(), "event.append").handle_started
        );
    }

    #[test]
    fn shutdown_writes_final_heartbeat_releases_leases_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        let first = supervisor.request_shutdown(200).unwrap();
        let second = supervisor.request_shutdown(210).unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();
        let heartbeat = assembly
            .supervisor_store()
            .get_runtime_heartbeat(&runtime_id)
            .unwrap()
            .unwrap();
        let shutdown = assembly
            .supervisor_store()
            .get_shutdown_state(&first.shutdown_id)
            .unwrap()
            .unwrap();
        let instance = assembly
            .supervisor_store()
            .get_runtime_instance("instance-a")
            .unwrap()
            .unwrap();

        assert_eq!(first.stopped_runtime_ids, vec!["event.append"]);
        assert!(second.stopped_runtime_ids.is_empty());
        assert_eq!(heartbeat.health.status, RuntimeHealthStatus::Stopped);
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert_eq!(shutdown.status, RuntimeShutdownStatus::Completed);
        assert_eq!(instance.status, RuntimeInstanceStatus::Stopped);
        assert_eq!(instance.stopped_at_ms, Some(200));
    }

    #[test]
    fn heartbeat_expiry_policy_restarts_with_new_lease_and_counter() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();
        let old_lease = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        let old_owner = old_lease.owner();

        let evaluation = supervisor.evaluate_restart_policies(130).unwrap();
        let active = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        let health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(evaluation.expired_runtime_ids, vec!["event.append"]);
        assert_eq!(evaluation.restarted_runtime_ids, vec!["event.append"]);
        assert_ne!(active.lease_id, old_lease.lease_id);
        assert!(matches!(
            assembly
                .supervisor_store()
                .renew_runtime_lease(&old_owner, 131, 20)
                .unwrap_err(),
            SupervisorStoreError::StaleLeaseOwner { .. }
        ));
        assert_eq!(health.restart_count, 1);
        assert_eq!(
            health.last_restart_cause,
            Some(RestartCause::HeartbeatExpired)
        );
    }

    #[test]
    fn never_restart_policy_leaves_expired_runtime_unhealthy() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.default_restart_policy = RestartPolicy::Never;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();

        let evaluation = supervisor.evaluate_restart_policies(130).unwrap();
        let health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(evaluation.expired_runtime_ids, vec!["event.append"]);
        assert!(evaluation.restarted_runtime_ids.is_empty());
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert_eq!(health.status, RuntimeHealthStatus::Unhealthy);
    }

    #[test]
    fn retryable_failure_policy_restarts_without_reading_domain_state() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.default_restart_policy = RestartPolicy::OnRetryableFailure;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();
        let active = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        assembly
            .supervisor_store()
            .write_runtime_heartbeat(&RuntimeHeartbeat {
                runtime_id: runtime_id.clone(),
                lease_id: active.lease_id.clone(),
                instance_id: active.instance_id.clone(),
                observed_at_ms: 120,
                health: RuntimeHealth {
                    status: RuntimeHealthStatus::Degraded,
                    retryable_error_count: 1,
                    fatal_error_count: 0,
                    budget_exhausted: false,
                    last_successful_tick_at_ms: Some(100),
                    last_error_code: Some("retryable_io".to_string()),
                },
                diagnostic: Some(RuntimeDiagnosticSummary {
                    actor_id: "event.append".to_string(),
                    retryable_issue_count: 1,
                    fatal_issue_count: 0,
                    budget_exhausted: false,
                    last_error_code: Some("retryable_io".to_string()),
                }),
            })
            .unwrap();

        let evaluation = supervisor.evaluate_restart_policies(130).unwrap();
        let health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();

        assert!(evaluation.expired_runtime_ids.is_empty());
        assert_eq!(evaluation.restarted_runtime_ids, vec!["event.append"]);
        assert_eq!(health.restart_count, 1);
        assert_eq!(
            health.last_restart_cause,
            Some(RestartCause::RetryableFailure)
        );
        // Truthfulness fix: a restarted actor is starting until its next
        // real bounded tick report, never immediately healthy.
        assert_eq!(health.status, RuntimeHealthStatus::Starting);
    }

    #[test]
    fn every_active_actor_gets_the_first_opportunity_without_extra_invocations() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".into(), "world_model.graph_replay".into()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("fair-actors", 100),
        )
        .unwrap();
        let expected = std::collections::BTreeSet::from([
            "event.append".to_string(),
            "world_model.graph_replay".to_string(),
        ]);
        let mut first = std::collections::BTreeSet::new();
        for pass in 0..expected.len() {
            let report = supervisor.tick(120 + pass as u64 * 10).unwrap();
            assert_eq!(report.renewed_runtime_ids.len(), expected.len());
            assert_eq!(report.actions.len(), expected.len());
            assert_eq!(
                report
                    .renewed_runtime_ids
                    .iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>(),
                expected
            );
            first.insert(report.renewed_runtime_ids[0].clone());
        }
        assert_eq!(first, expected);
    }

    #[test]
    fn tick_renews_lease_and_writes_heartbeat() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();

        let report = supervisor.tick(120).unwrap();
        let active = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        let heartbeat = assembly
            .supervisor_store()
            .get_runtime_heartbeat(&runtime_id)
            .unwrap()
            .unwrap();
        let health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();
        let event = assembly
            .supervisor_store()
            .latest_lifecycle_event_for_runtime(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(report.renewed_runtime_ids, vec!["event.append"]);
        assert_eq!(report.heartbeat_runtime_ids, vec!["event.append"]);
        assert!(report.restart_evaluation.expired_runtime_ids.is_empty());
        assert_eq!(active.renewed_at_ms, Some(120));
        assert_eq!(heartbeat.observed_at_ms, 120);
        assert_eq!(heartbeat.health.status, RuntimeHealthStatus::Healthy);
        assert_eq!(health.last_heartbeat_at_ms, Some(120));
        assert_eq!(
            event.event_type,
            SupervisorLifecycleEventType::HeartbeatAccepted
        );
    }

    #[test]
    fn tick_runs_graph_replay_handle_and_records_worker_diagnostic() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let subject = DomainObjectRef::new("context", "head", "node-a::analysis").unwrap();
        assembly
            .event_authority()
            .append_capability()
            .append_durable(
                EventEnvelope::new_domain(
                    "2026-06-22T00:00:00Z".to_string(),
                    "session-a",
                    "context",
                    "workspace-a",
                    "context.head_tombstoned",
                    None,
                    json!({ "node": "node-a" }),
                )
                .with_graph(vec![subject], Vec::new())
                .with_record_id("context-head-a"),
                AppendMode::Idempotent,
            )
            .unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();

        let report = supervisor.tick(120).unwrap();
        let heartbeat = assembly
            .supervisor_store()
            .get_runtime_heartbeat(&runtime_id)
            .unwrap()
            .unwrap();
        let reduced_seq = assembly
            .ports()
            .graph_cursor()
            .current()
            .unwrap()
            .expect("graph cursor should be reported")
            .reported_seq;

        assert_eq!(report.renewed_runtime_ids, vec!["world_model.graph_replay"]);
        assert_eq!(reduced_seq, 1);
        assert_eq!(heartbeat.health.status, RuntimeHealthStatus::Healthy);
        assert_eq!(heartbeat.health.last_successful_tick_at_ms, Some(120));
        let diagnostic = heartbeat.diagnostic.expect("expected diagnostic");
        assert_eq!(diagnostic.actor_id, "world_state.graph.reducer");
        assert_eq!(diagnostic.fatal_issue_count, 0);
    }

    #[test]
    fn zero_work_ticks_project_active_idle_and_do_not_flood_action_records() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        for (index, now_ms) in [120_u64, 140, 160].into_iter().enumerate() {
            let report = supervisor.tick(now_ms).unwrap();
            // Exactly one bounded invocation per active actor per pass.
            assert_eq!(report.actions.len(), 1, "tick {index}");
            assert_eq!(report.actions[0].runtime_id, "event.append");
            assert_eq!(
                report.actions[0].outcome,
                crate::runtime::contracts::RuntimeActionOutcome::NoWork
            );
        }

        let status = supervisor.status_snapshot(170).unwrap();
        let event_append = runtime_status(&status, "event.append");
        assert_eq!(event_append.health_status, RuntimeHealthStatus::Healthy);
        assert_eq!(
            event_append.lifecycle,
            Some(RegistrationLifecycle::ActiveIdle)
        );

        use crate::runtime::contracts::RuntimeStatusReader;
        let actions = supervisor.report_store().read_recent_actions(16).unwrap();
        assert_eq!(actions.len(), 3);
        // Repeated idle ticks change no semantic checkpoints.
        for action in &actions {
            assert_eq!(
                action.checkpoints[0].input_value,
                action.checkpoints[0].output_value
            );
            assert_eq!(action.checkpoints[0].output_value, 0);
        }
    }

    #[test]
    fn per_tick_reports_are_recoverable_including_checkpoint_movement() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let frames_dir = tempfile::tempdir().unwrap();
        let frames = crate::context::frame::FrameStorage::new(frames_dir.path()).unwrap();
        let publication =
            crate::context::publication::head_publication(&crate::heads::HeadIndex::new(), &frames)
                .unwrap();
        assembly
            .event_authority()
            .append_capability()
            .append_durable(
                crate::world_state::graph::events::owner_publication_envelope(
                    "session-a",
                    &publication,
                )
                .unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        supervisor.tick(120).unwrap();
        let working = supervisor.status_snapshot(125).unwrap();
        supervisor.tick(140).unwrap();
        let idle = supervisor.status_snapshot(145).unwrap();

        assert_eq!(
            runtime_status(&working, "world_model.graph_replay").lifecycle,
            Some(RegistrationLifecycle::ActiveWorking)
        );
        assert_eq!(
            runtime_status(&idle, "world_model.graph_replay").lifecycle,
            Some(RegistrationLifecycle::ActiveIdle)
        );

        // The full per-tick reports are recoverable through the public
        // reader path over a freshly opened report store.
        use crate::runtime::contracts::{RuntimeActionOutcome, RuntimeStatusReader};
        let reader = SupervisorReportStore::open(assembly.supervisor_store()).unwrap();
        let actions = reader.read_recent_actions(16).unwrap();

        assert_eq!(actions.len(), 2);
        let first = &actions[0];
        assert_eq!(first.actor_id, "world_state.graph.reducer");
        assert_eq!(first.outcome, RuntimeActionOutcome::Succeeded);
        assert_eq!(first.checkpoints[0].input_name, "event_spine_seq");
        assert_eq!(first.checkpoints[0].input_value, 0);
        assert_eq!(first.checkpoints[0].output_value, 1);
        assert_eq!(first.metrics.committed, 1);
        let second = &actions[1];
        assert_eq!(second.outcome, RuntimeActionOutcome::NoWork);
        assert_eq!(second.checkpoints[0].input_value, 1);
        assert_eq!(second.checkpoints[0].output_value, 1);

        // The idle tick moved no semantic checkpoint: the durable domain
        // cursor still matches the first tick's output.
        let cursor = assembly
            .ports()
            .graph_cursor()
            .current()
            .unwrap()
            .expect("graph cursor should be reported");
        assert_eq!(cursor.reported_seq, 1);
    }

    #[test]
    fn fatal_reports_project_unhealthy_and_missing_reports_are_never_healthy() {
        let fatal = WorkerTickReport::fatal(
            "execution.task_admission",
            "execution",
            Some("planning"),
            "task_network_revision",
            "planning_failed",
            "planning failed",
        );

        assert_eq!(
            health_status_from_tick_report(&fatal),
            RuntimeHealthStatus::Unhealthy
        );
        let record = RuntimeActionRecord::from_worker_tick(
            "action-fatal",
            "execution.task_admission",
            10,
            fatal,
        );
        assert_eq!(
            lifecycle_projection(
                RuntimeClassification::ActiveBound,
                true,
                RuntimeHealthStatus::Unhealthy,
                Some(&record),
            ),
            Some(RegistrationLifecycle::Unhealthy)
        );
        // The missing-report arm no longer exists: an unresolved required
        // binding projects truthfully instead of defaulting healthy.
        assert_eq!(
            lifecycle_projection(
                RuntimeClassification::ActiveUnresolved,
                true,
                RuntimeHealthStatus::Unhealthy,
                None,
            ),
            Some(RegistrationLifecycle::UnresolvedRequiredBinding)
        );
        assert_eq!(
            lifecycle_projection(
                RuntimeClassification::Passive,
                true,
                RuntimeHealthStatus::Unknown,
                None
            ),
            None
        );
        assert_eq!(
            lifecycle_projection(
                RuntimeClassification::ActiveBound,
                true,
                RuntimeHealthStatus::Starting,
                None,
            ),
            Some(RegistrationLifecycle::Starting)
        );
    }

    #[test]
    fn tick_preserves_restart_count_in_health_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();

        supervisor.evaluate_restart_policies(130).unwrap();
        supervisor.tick(131).unwrap();
        let health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(health.restart_count, 1);
        assert_eq!(
            health.last_restart_cause,
            Some(RestartCause::HeartbeatExpired)
        );
        assert_eq!(health.status, RuntimeHealthStatus::Healthy);
    }

    #[test]
    fn tick_runs_restart_policy_evaluation() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        config.lifecycle_config.lease_duration_ms = 5;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("event.append").unwrap();
        let old_lease = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();

        let report = supervisor.tick(110).unwrap();
        let active = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(
            report.restart_evaluation.expired_runtime_ids,
            vec!["event.append"]
        );
        assert_eq!(
            report.restart_evaluation.restarted_runtime_ids,
            vec!["event.append"]
        );
        assert!(report.renewed_runtime_ids.is_empty());
        assert_ne!(active.lease_id, old_lease.lease_id);
        assert!(
            runtime_status(&supervisor.status_snapshot(111).unwrap(), "event.append")
                .handle_started
        );
    }

    fn runtime_status<'a>(
        status: &'a SupervisorStatusSnapshot,
        runtime_id: &str,
    ) -> &'a SupervisorRuntimeStatus {
        status
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == runtime_id)
            .unwrap()
    }
}
