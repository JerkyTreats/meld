//! Explicit root supervisor lifecycle entrypoint.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use thiserror::Error;

use crate::runtime::assembly::{
    DesiredRuntimeState, RuntimeHandle, RuntimeHandleFlushReport, RuntimeHandleStopReport,
    RuntimeLeaseContext, SupervisorStartupPackage,
};
use crate::runtime::contracts::{
    RuntimeActionRecord, RuntimeImplementationState, RuntimeRoleClass, WorkBudget, WorkerTickReport,
};
use crate::runtime::error::{RuntimeAssemblyError, RuntimePortError};
use crate::runtime::ports::ProductEventAppendPort;

use super::contracts::{
    RestartCause, RestartPolicy, RuntimeDesiredState, RuntimeDiagnosticSummary, RuntimeHealth,
    RuntimeHealthSnapshot, RuntimeHealthStatus, RuntimeHeartbeat, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLease, RuntimeLeaseOwner, RuntimeLeaseStatus,
    RuntimeReplacementCheckpoint, RuntimeReplacementStage, RuntimeRestartRecord,
    RuntimeRestartSchedule, RuntimeShutdownCompletion, RuntimeShutdownState, RuntimeShutdownStatus,
    SupervisorContractError, SupervisorLifecycleEvent, SupervisorLifecycleEventType,
};
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
    /// A root-owned runtime port failed during lifecycle coordination.
    #[error("runtime port error: {0}")]
    Port(#[from] RuntimePortError),
    /// The caller supplied an invalid lifecycle command.
    #[error("invalid supervisor command: {0}")]
    InvalidCommand(String),
    /// Another supervisor instance owns a required runtime lease.
    #[error("runtime contender rejected for '{runtime_id}'; active lease is '{active_lease_id}'")]
    ActiveOwnerConflict {
        /// Runtime whose lease rejected this contender.
        runtime_id: String,
        /// Lease id retained by the active owner.
        active_lease_id: String,
    },
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
    /// Canonical lifecycle shape for this role.
    pub role_class: RuntimeRoleClass,
    /// Hosted implementation posture for this role.
    pub implementation_state: RuntimeImplementationState,
    /// Whether this process currently has a local started handle.
    pub handle_started: bool,
    /// Active lease id when present.
    pub active_lease_id: Option<String>,
    /// Supervisor instance that owns the active lease.
    pub active_owner_instance_id: Option<String>,
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
    /// Final closed event ingress barrier and durable watermark.
    pub final_event_barrier: meld_events::EventFinalBarrier,
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
    /// Bounded semantic actions observed during this tick.
    pub actions: Vec<RuntimeActionRecord>,
    /// Restart policy evaluation result for this tick.
    pub restart_evaluation: SupervisorRestartEvaluation,
}

/// Explicit root runtime supervisor.
///
/// This entrypoint owns operational lifecycle records only. Domain runtimes
/// still resume and validate semantic progress through their own stores.
pub struct RuntimeSupervisor<'a> {
    product_root: PathBuf,
    stores: &'a crate::runtime::storage::OpenProductStores,
    supervisor_store: &'a SupervisorStore,
    event_append: ProductEventAppendPort,
    handle_factories: &'a crate::runtime::assembly::RuntimeHandleFactoryRegistry,
    lifecycle_config: crate::runtime::assembly::RuntimeLifecycleConfig,
    default_work_budget: WorkBudget,
    instance_id: String,
    started_at_ms: u64,
    desired: BTreeMap<String, SupervisorRuntimeDesired>,
    handles: BTreeMap<String, SupervisedRuntimeHandle>,
    event_sequence: u64,
    action_sequence: u64,
    restart_attempt_limit: u64,
    restart_backoff_ms: u64,
    shutdown_completion: Option<RuntimeShutdownCompletion>,
    shutdown_recovery: Option<RuntimeShutdownState>,
    #[cfg(test)]
    abort_after_replacement_stage: Option<RuntimeReplacementStage>,
    #[cfg(test)]
    abort_after_shutdown_barrier: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SupervisorRuntimeDesired {
    runtime_id: RuntimeId,
    enabled: bool,
    factory_available: bool,
    role_class: RuntimeRoleClass,
    implementation_state: RuntimeImplementationState,
    restart_policy: RestartPolicy,
}

impl SupervisorRuntimeDesired {
    fn host_eligible(&self) -> bool {
        self.enabled
            && self.factory_available
            && self.implementation_state == RuntimeImplementationState::Concrete
            && matches!(
                self.role_class,
                RuntimeRoleClass::Actor | RuntimeRoleClass::PassiveService
            )
    }

    fn tick_eligible(&self) -> bool {
        self.host_eligible() && self.role_class == RuntimeRoleClass::Actor
    }
}

struct SupervisedRuntimeHandle {
    handle: RuntimeHandle,
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

        let desired = desired_runtime_map(
            package.desired_runtime_state,
            command.default_restart_policy,
        )?;
        let shutdown_id = format!("shutdown:{}:{}", command.instance_id, command.started_at_ms);
        let shutdown_completion = package
            .supervisor_store
            .get_shutdown_completion(&shutdown_id)?;
        let mut shutdown_recovery = None;
        if shutdown_completion.is_none() {
            for state in package
                .supervisor_store
                .list_shutdown_states()?
                .into_iter()
                .rev()
            {
                if state.status != RuntimeShutdownStatus::Completed
                    && package
                        .supervisor_store
                        .get_shutdown_completion(&state.shutdown_id)?
                        .is_none()
                {
                    shutdown_recovery = Some(state);
                    break;
                }
            }
        }
        let recovery_instance = match shutdown_recovery.as_ref() {
            Some(state) => package
                .supervisor_store
                .get_runtime_instance(&state.instance_id)?,
            None => None,
        };
        let effective_instance_id = shutdown_recovery
            .as_ref()
            .map(|state| state.instance_id.clone())
            .unwrap_or_else(|| command.instance_id.clone());
        let effective_started_at_ms = recovery_instance
            .as_ref()
            .map(|instance| instance.started_at_ms)
            .unwrap_or(command.started_at_ms);
        let mut supervisor = Self {
            product_root: package.product_root.to_path_buf(),
            stores: package.stores,
            supervisor_store: package.supervisor_store,
            event_append: package.ports.event_append().clone(),
            handle_factories: package.handle_factories,
            lifecycle_config: package.lifecycle_config.clone(),
            default_work_budget: package.default_work_budget.clone(),
            instance_id: effective_instance_id,
            started_at_ms: effective_started_at_ms,
            desired,
            handles: BTreeMap::new(),
            event_sequence: INITIAL_EVENT_SEQUENCE,
            action_sequence: 0,
            restart_attempt_limit: command.restart_attempt_limit,
            restart_backoff_ms: command.restart_backoff_ms,
            shutdown_completion,
            shutdown_recovery,
            #[cfg(test)]
            abort_after_replacement_stage: None,
            #[cfg(test)]
            abort_after_shutdown_barrier: false,
        };

        if let Some(completion) = &supervisor.shutdown_completion {
            let recovered_barrier = supervisor.event_append.close_and_drain()?;
            if recovered_barrier != completion.final_event_barrier() {
                return Err(SupervisorRuntimeError::InvalidCommand(
                    "completed shutdown barrier diverged while ingress was re-fenced".to_string(),
                ));
            }
            return Ok(supervisor);
        }
        if supervisor.shutdown_recovery.is_some() {
            supervisor.event_append.close_and_drain()?;
            return Ok(supervisor);
        }

        supervisor.register_instance()?;
        let recovered = supervisor.recover_expired_leases(command.started_at_ms)?;
        let mut pending_replacements = supervisor
            .schedule_recovered_startup_replacements(&recovered, command.started_at_ms)?;
        pending_replacements.extend(supervisor.pending_replacement_runtime_ids()?);
        let startup_leases =
            match supervisor.acquire_startup_leases(command.started_at_ms, &pending_replacements) {
                Ok(leases) => leases,
                Err(error) => {
                    supervisor.mark_instance(
                        RuntimeInstanceStatus::Failed,
                        Some(command.started_at_ms),
                        command.started_at_ms,
                    )?;
                    supervisor.supervisor_store.flush()?;
                    return Err(error);
                }
            };
        if let Err(error) = supervisor.persist_desired_state() {
            supervisor.rollback_startup_ownership(&startup_leases, command.started_at_ms)?;
            supervisor.mark_instance(
                RuntimeInstanceStatus::Failed,
                Some(command.started_at_ms),
                command.started_at_ms,
            )?;
            supervisor.supervisor_store.flush()?;
            return Err(error);
        }
        if let Err(error) =
            supervisor.start_enabled_runtimes(command.started_at_ms, &startup_leases)
        {
            supervisor.rollback_startup_ownership(&startup_leases, command.started_at_ms)?;
            supervisor.mark_instance(
                RuntimeInstanceStatus::Failed,
                Some(command.started_at_ms),
                command.started_at_ms,
            )?;
            supervisor.supervisor_store.flush()?;
            return Err(error);
        }
        if let Err(error) = supervisor.resume_pending_replacements(command.started_at_ms) {
            supervisor.rollback_startup_ownership(&startup_leases, command.started_at_ms)?;
            supervisor.mark_instance(
                RuntimeInstanceStatus::Failed,
                Some(command.started_at_ms),
                command.started_at_ms,
            )?;
            supervisor.supervisor_store.flush()?;
            return Err(error);
        }
        supervisor.mark_instance(RuntimeInstanceStatus::Running, None, command.started_at_ms)?;
        supervisor.supervisor_store.flush()?;
        Ok(supervisor)
    }

    /// Return this supervisor instance id.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
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
            let active_lease = active_lease.filter(|lease| lease.is_active_at(now_ms));
            let heartbeat = self
                .supervisor_store
                .get_runtime_heartbeat(&desired.runtime_id)?
                .filter(|heartbeat| match active_lease.as_ref() {
                    Some(lease) => {
                        heartbeat.lease_id == lease.lease_id
                            && heartbeat.instance_id == lease.instance_id
                    }
                    None => {
                        desired.host_eligible()
                            && matches!(
                                heartbeat.health.status,
                                RuntimeHealthStatus::Stopped | RuntimeHealthStatus::Unhealthy
                            )
                    }
                });
            let health = self
                .supervisor_store
                .get_health_snapshot(&desired.runtime_id)?
                .filter(|health| match active_lease.as_ref() {
                    Some(lease) => health.lease_id.as_deref() == Some(lease.lease_id.as_str()),
                    None => {
                        desired.host_eligible()
                            && matches!(
                                health.status,
                                RuntimeHealthStatus::Stopped | RuntimeHealthStatus::Unhealthy
                            )
                    }
                });
            let event = self
                .supervisor_store
                .latest_lifecycle_event_for_runtime(&desired.runtime_id)?;
            let last_heartbeat_at_ms = heartbeat.as_ref().map(|record| record.observed_at_ms);
            let heartbeat_age_ms =
                last_heartbeat_at_ms.map(|observed| now_ms.saturating_sub(observed));
            let default_health_status = desired_default_health_status(desired);
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

            runtimes.push(SupervisorRuntimeStatus {
                runtime_id: desired.runtime_id.to_string(),
                desired_enabled: desired.enabled,
                factory_available: desired.factory_available,
                role_class: desired.role_class,
                implementation_state: desired.implementation_state,
                handle_started: self
                    .handles
                    .get(desired.runtime_id.as_str())
                    .is_some_and(|runtime| runtime.handle.is_started()),
                active_lease_id: active_lease.as_ref().map(|record| record.lease_id.clone()),
                active_owner_instance_id: active_lease
                    .as_ref()
                    .map(|record| record.instance_id.clone()),
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

        Ok(SupervisorStatusSnapshot {
            instance_id: instance.instance_id,
            product_root: instance.product_root,
            instance_status: instance.status,
            runtimes,
        })
    }

    /// Evaluate conservative restart policy for expired or retryable runtimes.
    pub fn evaluate_restart_policies(
        &mut self,
        now_ms: u64,
    ) -> Result<SupervisorRestartEvaluation, SupervisorRuntimeError> {
        let mut restarted_runtime_ids = self.resume_pending_replacements(now_ms)?;
        let expired = self
            .supervisor_store
            .list_expired_active_runtime_leases(now_ms)?;
        let mut expired_runtime_ids = Vec::new();

        for lease in expired {
            let runtime_id = lease.runtime_id.to_string();
            expired_runtime_ids.push(runtime_id.clone());
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
                self.stop_flush_and_release_runtime(&runtime_id, &lease.owner(), now_ms)?;
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

    /// Renew local leases, write healthy heartbeats, and evaluate restarts.
    pub fn tick(&mut self, now_ms: u64) -> Result<SupervisorTickReport, SupervisorRuntimeError> {
        let owners = self
            .handles
            .values()
            .map(|runtime| runtime.owner.clone())
            .collect::<Vec<_>>();
        let mut renewed_runtime_ids = Vec::new();
        let mut heartbeat_runtime_ids = Vec::new();
        let mut actions = Vec::new();
        let mut stale_runtime_ids = Vec::new();

        for owner in owners {
            let active_lease = self
                .supervisor_store
                .get_active_runtime_lease(&owner.runtime_id)?;
            match active_lease.as_ref() {
                Some(lease) if lease.is_owned_by(&owner) && lease.is_active_at(now_ms) => {}
                Some(lease) if lease.is_owned_by(&owner) => continue,
                _ => {
                    stale_runtime_ids.push(owner.runtime_id.to_string());
                    continue;
                }
            }
            self.supervisor_store.renew_runtime_lease(
                &owner,
                now_ms,
                self.lifecycle_config.lease_duration_ms,
            )?;
            renewed_runtime_ids.push(owner.runtime_id.to_string());

            let tick_eligible = self
                .desired
                .get(owner.runtime_id.as_str())
                .is_some_and(SupervisorRuntimeDesired::tick_eligible);
            let semantic_report = if tick_eligible {
                self.handles
                    .get_mut(owner.runtime_id.as_str())
                    .and_then(|runtime| runtime.handle.tick(self.default_work_budget.clone()))
            } else {
                None
            };
            let health_status =
                health_status_from_tick_report(tick_eligible, semantic_report.as_ref());
            let action = semantic_report.clone().map(|report| {
                self.action_sequence = self.action_sequence.saturating_add(1);
                RuntimeActionRecord::from_worker_tick(
                    format!(
                        "action:{}:{}:{}",
                        self.instance_id, owner.runtime_id, self.action_sequence
                    ),
                    owner.runtime_id.to_string(),
                    now_ms,
                    report,
                )
            });

            self.write_runtime_heartbeat(&owner, health_status, now_ms, semantic_report.as_ref())?;
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
                Some(owner.lease_id),
                health_status,
                now_ms,
                existing_health
                    .as_ref()
                    .map(|snapshot| snapshot.restart_count)
                    .unwrap_or(0),
                existing_health.and_then(|snapshot| snapshot.last_restart_cause),
            )?;
            if let Some(action) = action {
                actions.push(action);
            }
        }

        for runtime_id in stale_runtime_ids {
            self.stop_and_flush_stale_handle(&runtime_id)?;
        }

        let restart_evaluation = self.evaluate_restart_policies(now_ms)?;
        self.supervisor_store.flush()?;
        Ok(SupervisorTickReport {
            renewed_runtime_ids,
            heartbeat_runtime_ids,
            actions,
            restart_evaluation,
        })
    }

    /// Coordinate graceful shutdown and flush supervisor lifecycle records.
    pub fn request_shutdown(
        &mut self,
        now_ms: u64,
    ) -> Result<SupervisorShutdownReport, SupervisorRuntimeError> {
        let shutdown_id = self
            .shutdown_recovery
            .as_ref()
            .map(|state| state.shutdown_id.clone())
            .unwrap_or_else(|| format!("shutdown:{}:{}", self.instance_id, self.started_at_ms));
        if let Some(completion) = &self.shutdown_completion {
            return Ok(SupervisorShutdownReport {
                shutdown_id,
                stopped_runtime_ids: Vec::new(),
                stop_reports: Vec::new(),
                flush_reports: Vec::new(),
                final_event_barrier: completion.final_event_barrier(),
            });
        }

        let requested_at_ms = self
            .shutdown_recovery
            .as_ref()
            .map(|state| state.requested_at_ms)
            .or(self
                .supervisor_store
                .get_shutdown_state(&shutdown_id)?
                .map(|state| state.requested_at_ms))
            .unwrap_or(now_ms);

        self.mark_instance(RuntimeInstanceStatus::Stopping, None, now_ms)?;
        self.put_shutdown_phase(
            &shutdown_id,
            requested_at_ms,
            None,
            RuntimeShutdownStatus::Requested,
        )?;
        self.write_lifecycle_event(
            None,
            None,
            now_ms,
            SupervisorLifecycleEventType::ShutdownRequested,
            Some("shutdown requested".to_string()),
        )?;
        self.put_shutdown_phase(
            &shutdown_id,
            requested_at_ms,
            None,
            RuntimeShutdownStatus::Signaling,
        )?;
        self.supervisor_store.flush()?;
        let final_event_barrier = match self.event_append.close_and_drain() {
            Ok(barrier) => barrier,
            Err(error) => {
                self.record_shutdown_failure(&shutdown_id, requested_at_ms, now_ms)?;
                return Err(error.into());
            }
        };
        #[cfg(test)]
        if self.abort_after_shutdown_barrier {
            if let Ok(marker) = std::env::var("MELD_SHUTDOWN_CRASH_MARKER") {
                std::fs::write(marker, b"barrier-closed").expect("write shutdown crash marker");
            }
            std::process::abort();
        }

        let mut stop_reports = Vec::new();
        let mut flush_reports = Vec::new();
        let shutdown_runtime_ids = ordered_handle_ids(&self.handles, false);
        for runtime_id in &shutdown_runtime_ids {
            let runtime = self.handles.get_mut(runtime_id).ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{runtime_id}' disappeared during shutdown signaling"
                ))
            })?;
            stop_reports.push(runtime.handle.request_stop());
        }
        self.put_shutdown_phase(
            &shutdown_id,
            requested_at_ms,
            None,
            RuntimeShutdownStatus::WaitingForSafePoint,
        )?;
        for runtime_id in &shutdown_runtime_ids {
            let runtime = self.handles.get(runtime_id).ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{runtime_id}' disappeared before its safe point"
                ))
            })?;
            let safe_point = runtime.handle.wait_for_safe_point();
            if !safe_point.safe_for_flush {
                self.record_shutdown_failure(&shutdown_id, requested_at_ms, now_ms)?;
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{}' did not reach a safe point",
                    safe_point.runtime_id
                )));
            }
        }
        self.put_shutdown_phase(
            &shutdown_id,
            requested_at_ms,
            None,
            RuntimeShutdownStatus::FlushingStores,
        )?;
        for runtime_id in &shutdown_runtime_ids {
            let runtime = self.handles.get(runtime_id).ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{runtime_id}' disappeared before resource flush"
                ))
            })?;
            match runtime.handle.flush_resources() {
                Ok(report) => flush_reports.push(report),
                Err(error) => {
                    self.record_shutdown_failure(&shutdown_id, requested_at_ms, now_ms)?;
                    return Err(error.into());
                }
            }
        }

        if let Err(error) = self.stores.flush_boundary() {
            self.record_shutdown_failure(&shutdown_id, requested_at_ms, now_ms)?;
            return Err(RuntimeAssemblyError::from(error).into());
        }

        let owners = self.shutdown_lease_owners()?;
        let stopped = owners
            .iter()
            .map(|owner| owner.runtime_id.to_string())
            .collect::<Vec<_>>();

        for owner in owners {
            if self
                .supervisor_store
                .get_active_runtime_lease(&owner.runtime_id)?
                .as_ref()
                .is_some_and(|lease| lease.is_owned_by(&owner) && lease.is_active_at(now_ms))
            {
                self.write_runtime_heartbeat(&owner, RuntimeHealthStatus::Stopped, now_ms, None)?;
            }
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
                Some(owner.lease_id),
                RuntimeHealthStatus::Stopped,
                now_ms,
                NO_RESTART_ATTEMPTS,
                None,
            )?;
        }

        self.handles.clear();
        let completed_shutdown = RuntimeShutdownState {
            shutdown_id: shutdown_id.clone(),
            instance_id: self.instance_id.clone(),
            requested_at_ms,
            completed_at_ms: Some(now_ms),
            status: RuntimeShutdownStatus::Completed,
        };
        self.supervisor_store
            .put_shutdown_state(&completed_shutdown)?;
        self.mark_instance(RuntimeInstanceStatus::Stopped, Some(now_ms), now_ms)?;
        self.write_lifecycle_event(
            None,
            None,
            now_ms,
            SupervisorLifecycleEventType::InstanceStopped,
            Some("supervisor instance stopped".to_string()),
        )?;
        let shutdown_completion =
            RuntimeShutdownCompletion::try_new(completed_shutdown, final_event_barrier)?;
        self.supervisor_store
            .put_shutdown_completion(&shutdown_completion)?;
        self.shutdown_completion = Some(shutdown_completion);
        self.shutdown_recovery = None;

        Ok(SupervisorShutdownReport {
            shutdown_id,
            stopped_runtime_ids: stopped,
            stop_reports,
            flush_reports,
            final_event_barrier,
        })
    }

    fn put_shutdown_phase(
        &self,
        shutdown_id: &str,
        requested_at_ms: u64,
        completed_at_ms: Option<u64>,
        status: RuntimeShutdownStatus,
    ) -> Result<(), SupervisorRuntimeError> {
        self.supervisor_store
            .put_shutdown_state(&RuntimeShutdownState {
                shutdown_id: shutdown_id.to_string(),
                instance_id: self.instance_id.clone(),
                requested_at_ms,
                completed_at_ms,
                status,
            })?;
        Ok(())
    }

    fn shutdown_lease_owners(&self) -> Result<Vec<RuntimeLeaseOwner>, SupervisorRuntimeError> {
        let mut owners = self
            .handles
            .values()
            .map(|runtime| runtime.owner.clone())
            .collect::<Vec<_>>();
        for lease in self.supervisor_store.list_runtime_leases()? {
            if lease.instance_id != self.instance_id || !lease.has_active_status() {
                continue;
            }
            let owner = lease.owner();
            if self
                .supervisor_store
                .get_active_runtime_lease(&owner.runtime_id)?
                .as_ref()
                .is_some_and(|active| active.is_owned_by(&owner))
                && !owners.contains(&owner)
            {
                owners.push(owner);
            }
        }
        owners.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        Ok(owners)
    }

    fn record_shutdown_failure(
        &self,
        shutdown_id: &str,
        requested_at_ms: u64,
        now_ms: u64,
    ) -> Result<(), SupervisorRuntimeError> {
        self.put_shutdown_phase(
            shutdown_id,
            requested_at_ms,
            Some(now_ms),
            RuntimeShutdownStatus::Failed,
        )?;
        self.mark_instance(RuntimeInstanceStatus::Failed, Some(now_ms), now_ms)?;
        for owner in self.shutdown_lease_owners()? {
            self.write_health_snapshot(
                &owner.runtime_id,
                Some(owner.lease_id),
                RuntimeHealthStatus::Unhealthy,
                now_ms,
                NO_RESTART_ATTEMPTS,
                None,
            )?;
        }
        self.supervisor_store.flush()?;
        Ok(())
    }

    fn register_instance(&mut self) -> Result<(), SupervisorRuntimeError> {
        self.supervisor_store
            .register_runtime_instance(&RuntimeInstance {
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

    fn pending_replacement_runtime_ids(&self) -> Result<BTreeSet<String>, SupervisorRuntimeError> {
        let mut pending = BTreeSet::new();
        for checkpoint in self.supervisor_store.list_replacement_checkpoints()? {
            if !replacement_checkpoint_completed(&checkpoint) {
                pending.insert(checkpoint.schedule().restart().runtime_id.to_string());
            }
        }
        Ok(pending)
    }

    fn schedule_recovered_startup_replacements(
        &mut self,
        recovered: &[RuntimeLease],
        now_ms: u64,
    ) -> Result<BTreeSet<String>, SupervisorRuntimeError> {
        let mut blocked = BTreeSet::new();
        let existing_pending = self
            .supervisor_store
            .list_replacement_checkpoints()?
            .into_iter()
            .filter(|checkpoint| !replacement_checkpoint_completed(checkpoint))
            .map(|checkpoint| {
                let restart = checkpoint.schedule().restart();
                (
                    restart.runtime_id.to_string(),
                    restart.previous_lease_id.clone(),
                )
            })
            .collect::<BTreeSet<_>>();
        for lease in recovered {
            let runtime_id = lease.runtime_id.to_string();
            if existing_pending.contains(&(runtime_id.clone(), Some(lease.lease_id.clone()))) {
                blocked.insert(runtime_id);
                continue;
            }
            let Some(desired) = self.desired.get(&runtime_id) else {
                continue;
            };
            if !desired.host_eligible()
                || desired.restart_policy != RestartPolicy::OnHeartbeatExpiry
            {
                blocked.insert(runtime_id);
                continue;
            }
            let prior_restart_count = self
                .supervisor_store
                .get_health_snapshot(&lease.runtime_id)?
                .map(|snapshot| snapshot.restart_count)
                .unwrap_or(0);
            let attempt = prior_restart_count.saturating_add(1);
            if attempt > self.restart_attempt_limit {
                self.write_health_snapshot(
                    &lease.runtime_id,
                    None,
                    RuntimeHealthStatus::Unhealthy,
                    now_ms,
                    prior_restart_count,
                    Some(RestartCause::HeartbeatExpired),
                )?;
                blocked.insert(runtime_id);
                continue;
            }
            let schedule = RuntimeRestartSchedule::try_new(RuntimeRestartRecord {
                restart_id: format!(
                    "restart:{}:{}:{}:{}",
                    self.instance_id, runtime_id, attempt, now_ms
                ),
                runtime_id: lease.runtime_id.clone(),
                instance_id: self.instance_id.clone(),
                previous_lease_id: Some(lease.lease_id.clone()),
                cause: RestartCause::HeartbeatExpired,
                attempt,
                requested_at_ms: now_ms,
                backoff_ms: self.restart_backoff_ms,
            })?;
            self.supervisor_store
                .persist_restart_replacement(&schedule)?;
            self.write_lifecycle_event(
                Some(lease.runtime_id.clone()),
                Some(lease.lease_id.clone()),
                now_ms,
                SupervisorLifecycleEventType::RestartScheduled,
                Some("expired startup owner scheduled for checked replacement".to_string()),
            )?;
            blocked.insert(runtime_id);
        }
        Ok(blocked)
    }

    fn acquire_startup_leases(
        &self,
        now_ms: u64,
        pending_replacements: &BTreeSet<String>,
    ) -> Result<BTreeMap<String, RuntimeLease>, SupervisorRuntimeError> {
        let mut acquired = BTreeMap::new();
        for desired in self.desired.values().filter(|desired| {
            desired.host_eligible() && !pending_replacements.contains(desired.runtime_id.as_str())
        }) {
            let runtime_id = desired.runtime_id.to_string();
            let lease_id = format!("lease:{}:{}:{}:{}", self.instance_id, runtime_id, 0, now_ms);
            match self.supervisor_store.acquire_runtime_lease(
                desired.runtime_id.clone(),
                lease_id,
                self.instance_id.clone(),
                now_ms,
                self.lifecycle_config.lease_duration_ms,
            ) {
                Ok(lease) => {
                    acquired.insert(runtime_id, lease);
                }
                Err(SupervisorStoreError::DuplicateActiveLease {
                    runtime_id,
                    active_lease_id,
                }) => {
                    self.release_startup_leases(&acquired, now_ms)?;
                    return Err(SupervisorRuntimeError::ActiveOwnerConflict {
                        runtime_id,
                        active_lease_id,
                    });
                }
                Err(error) => {
                    self.release_startup_leases(&acquired, now_ms)?;
                    return Err(error.into());
                }
            }
        }
        Ok(acquired)
    }

    fn start_enabled_runtimes(
        &mut self,
        now_ms: u64,
        startup_leases: &BTreeMap<String, RuntimeLease>,
    ) -> Result<(), SupervisorRuntimeError> {
        let mut runtime_ids = startup_leases.keys().cloned().collect::<Vec<_>>();
        runtime_ids.sort_by_key(|runtime_id| {
            let role_class = self
                .desired
                .get(runtime_id)
                .map(|desired| desired.role_class)
                .unwrap_or(RuntimeRoleClass::Unknown);
            (start_role_order(role_class), runtime_id.clone())
        });
        for runtime_id in runtime_ids {
            let lease = startup_leases.get(&runtime_id).ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "startup lease for runtime '{runtime_id}' disappeared"
                ))
            })?;
            self.start_runtime_after_lease(&runtime_id, lease, now_ms, NO_RESTART_ATTEMPTS, None)?;
        }
        Ok(())
    }

    fn rollback_startup_ownership(
        &mut self,
        startup_leases: &BTreeMap<String, RuntimeLease>,
        now_ms: u64,
    ) -> Result<(), SupervisorRuntimeError> {
        for runtime_id in ordered_handle_ids(&self.handles, false) {
            if let Some(runtime) = self.handles.get_mut(&runtime_id) {
                if runtime.handle.is_started() {
                    runtime.handle.request_stop();
                    runtime.handle.wait_for_safe_point();
                }
            }
        }
        self.handles.clear();
        self.release_startup_leases(startup_leases, now_ms)?;
        Ok(())
    }

    fn release_startup_leases(
        &self,
        startup_leases: &BTreeMap<String, RuntimeLease>,
        now_ms: u64,
    ) -> Result<(), SupervisorRuntimeError> {
        for lease in startup_leases.values() {
            let owner = lease.owner();
            if self
                .supervisor_store
                .get_active_runtime_lease(&owner.runtime_id)?
                .as_ref()
                .is_some_and(|active| active.is_owned_by(&owner))
            {
                self.supervisor_store
                    .release_runtime_lease(&owner, now_ms)?;
            }
        }
        Ok(())
    }

    fn start_runtime_after_lease(
        &mut self,
        runtime_id: &str,
        lease: &RuntimeLease,
        now_ms: u64,
        restart_count: u64,
        last_restart_cause: Option<RestartCause>,
    ) -> Result<RuntimeLeaseOwner, SupervisorRuntimeError> {
        let runtime = lease.runtime_id.clone();
        let factory = self.handle_factories.get(runtime_id).ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand(format!(
                "eligible runtime '{}' has no handle factory",
                runtime_id
            ))
        })?;
        self.write_lifecycle_event(
            Some(runtime.clone()),
            None,
            now_ms,
            SupervisorLifecycleEventType::RuntimeStartRequested,
            Some("runtime start requested".to_string()),
        )?;
        self.write_lifecycle_event(
            Some(runtime.clone()),
            Some(lease.lease_id.clone()),
            now_ms,
            SupervisorLifecycleEventType::LeaseAcquired,
            Some("lease acquired".to_string()),
        )?;

        let mut handle = factory.build_handle();
        handle.start_after_lease(RuntimeLeaseContext {
            runtime_id: runtime_id.to_string(),
            lease_id: lease.lease_id.clone(),
        })?;
        let owner = lease.owner();
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
        self.handles.insert(
            runtime_id.to_string(),
            SupervisedRuntimeHandle {
                handle,
                owner: owner.clone(),
            },
        );
        Ok(owner)
    }

    fn restart_runtime(
        &mut self,
        runtime_id: &str,
        previous_lease_id: Option<String>,
        cause: RestartCause,
        now_ms: u64,
    ) -> Result<bool, SupervisorRuntimeError> {
        let runtime = RuntimeId::new(runtime_id.to_string())?;
        if !self.handles.get(runtime_id).is_some_and(|handle| {
            previous_lease_id
                .as_deref()
                .is_some_and(|lease_id| handle.owner.lease_id == lease_id)
        }) {
            return Ok(false);
        }
        let prior_restart_count = self
            .supervisor_store
            .get_health_snapshot(&runtime)?
            .map(|snapshot| snapshot.restart_count)
            .unwrap_or(0);
        let attempt = prior_restart_count + 1;
        if attempt > self.restart_attempt_limit {
            let owner = self
                .handles
                .get(runtime_id)
                .map(|handle| handle.owner.clone())
                .ok_or_else(|| {
                    SupervisorRuntimeError::InvalidCommand(format!(
                        "runtime '{}' lost its handle at the restart limit",
                        runtime_id
                    ))
                })?;
            self.stop_flush_and_release_runtime(runtime_id, &owner, now_ms)?;
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
        let schedule = RuntimeRestartSchedule::try_new(RuntimeRestartRecord {
            restart_id,
            runtime_id: runtime.clone(),
            instance_id: self.instance_id.clone(),
            previous_lease_id,
            cause: cause.clone(),
            attempt,
            requested_at_ms: now_ms,
            backoff_ms: self.restart_backoff_ms,
        })?;
        let checkpoint = self
            .supervisor_store
            .persist_restart_replacement(&schedule)?;
        self.write_lifecycle_event(
            Some(runtime),
            None,
            now_ms,
            SupervisorLifecycleEventType::RestartScheduled,
            Some("restart scheduled".to_string()),
        )?;
        self.drive_replacement(checkpoint, now_ms)
    }

    fn resume_pending_replacements(
        &mut self,
        now_ms: u64,
    ) -> Result<Vec<String>, SupervisorRuntimeError> {
        let mut restarted = Vec::new();
        for mut checkpoint in self.supervisor_store.list_replacement_checkpoints()? {
            if replacement_checkpoint_completed(&checkpoint) {
                continue;
            }
            if !self
                .handles
                .contains_key(checkpoint.schedule().restart().runtime_id.as_str())
            {
                checkpoint = self.reconcile_recovered_replacement(checkpoint, now_ms)?;
            }
            let restart = checkpoint.schedule().restart();
            let runtime_id = restart.runtime_id.to_string();
            if !self
                .desired
                .get(&runtime_id)
                .is_some_and(SupervisorRuntimeDesired::host_eligible)
            {
                continue;
            }
            if self.drive_replacement(checkpoint, now_ms)? {
                restarted.push(runtime_id);
            }
        }
        Ok(restarted)
    }

    fn reconcile_recovered_replacement(
        &self,
        mut checkpoint: RuntimeReplacementCheckpoint,
        now_ms: u64,
    ) -> Result<RuntimeReplacementCheckpoint, SupervisorRuntimeError> {
        if checkpoint.completed_stages().len() >= 4 {
            return Ok(checkpoint);
        }
        let restart = checkpoint.schedule().restart();
        let previous_lease_id = restart.previous_lease_id.as_deref().ok_or_else(|| {
            SupervisorRuntimeError::InvalidCommand(format!(
                "restart '{}' cannot recover without its previous lease identity",
                restart.restart_id
            ))
        })?;
        let previous = self
            .supervisor_store
            .get_runtime_lease(&restart.runtime_id, previous_lease_id)?
            .ok_or_else(|| {
                SupervisorRuntimeError::InvalidCommand(format!(
                    "restart '{}' previous lease is missing",
                    restart.restart_id
                ))
            })?;
        if !matches!(
            previous.status,
            RuntimeLeaseStatus::Expired | RuntimeLeaseStatus::Released
        ) {
            return Err(SupervisorRuntimeError::ActiveOwnerConflict {
                runtime_id: restart.runtime_id.to_string(),
                active_lease_id: previous.lease_id,
            });
        }

        if checkpoint.completed_stages().is_empty() {
            checkpoint = self.advance_replacement_checkpoint(
                &checkpoint,
                RuntimeReplacementStage::StopOldHandle,
                now_ms,
            )?;
        }
        if checkpoint.completed_stages().len() == 1 {
            checkpoint = self.advance_replacement_checkpoint(
                &checkpoint,
                RuntimeReplacementStage::AwaitOldSafePoint,
                now_ms,
            )?;
        }
        if checkpoint.completed_stages().len() == 2 {
            self.stores
                .flush_boundary()
                .map_err(RuntimeAssemblyError::from)?;
            checkpoint = self.advance_replacement_checkpoint(
                &checkpoint,
                RuntimeReplacementStage::FlushOldHandle,
                now_ms,
            )?;
        }
        if checkpoint.completed_stages().len() == 3 {
            checkpoint = self.advance_replacement_checkpoint(
                &checkpoint,
                RuntimeReplacementStage::ReleaseOldLease,
                now_ms,
            )?;
        }
        Ok(checkpoint)
    }

    fn drive_replacement(
        &mut self,
        mut checkpoint: RuntimeReplacementCheckpoint,
        now_ms: u64,
    ) -> Result<bool, SupervisorRuntimeError> {
        let restart = checkpoint.schedule().restart().clone();
        let runtime_id = restart.runtime_id.to_string();

        loop {
            match checkpoint.completed_stages().len() {
                0 => {
                    let runtime = self.handles.get_mut(&runtime_id).ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' has no old handle to stop",
                            runtime_id
                        ))
                    })?;
                    runtime.handle.request_stop();
                    checkpoint = self.advance_replacement_checkpoint(
                        &checkpoint,
                        RuntimeReplacementStage::StopOldHandle,
                        now_ms,
                    )?;
                }
                1 => {
                    let runtime = self.handles.get(&runtime_id).ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' lost its old handle before safe point",
                            runtime_id
                        ))
                    })?;
                    let safe_point = runtime.handle.wait_for_safe_point();
                    if !safe_point.safe_for_flush {
                        return Err(SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' did not reach a safe point before replacement",
                            runtime_id
                        )));
                    }
                    checkpoint = self.advance_replacement_checkpoint(
                        &checkpoint,
                        RuntimeReplacementStage::AwaitOldSafePoint,
                        now_ms,
                    )?;
                }
                2 => {
                    let runtime = self.handles.get(&runtime_id).ok_or_else(|| {
                        SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' lost its old handle before flush",
                            runtime_id
                        ))
                    })?;
                    runtime.handle.flush_resources()?;
                    self.stores
                        .flush_boundary()
                        .map_err(RuntimeAssemblyError::from)?;
                    checkpoint = self.advance_replacement_checkpoint(
                        &checkpoint,
                        RuntimeReplacementStage::FlushOldHandle,
                        now_ms,
                    )?;
                }
                3 => {
                    let owner = self
                        .handles
                        .get(&runtime_id)
                        .map(|runtime| runtime.owner.clone())
                        .ok_or_else(|| {
                            SupervisorRuntimeError::InvalidCommand(format!(
                                "runtime '{}' lost its old owner before lease release",
                                runtime_id
                            ))
                        })?;
                    let active = self
                        .supervisor_store
                        .get_active_runtime_lease(&owner.runtime_id)?;
                    if active
                        .as_ref()
                        .is_some_and(|lease| lease.is_owned_by(&owner))
                    {
                        self.supervisor_store
                            .release_runtime_lease(&owner, now_ms)?;
                        self.write_lifecycle_event(
                            Some(owner.runtime_id.clone()),
                            Some(owner.lease_id.clone()),
                            now_ms,
                            SupervisorLifecycleEventType::LeaseReleased,
                            Some("old lease released after safe replacement flush".to_string()),
                        )?;
                    } else if active.is_some() {
                        return Err(SupervisorRuntimeError::ActiveOwnerConflict {
                            runtime_id: runtime_id.clone(),
                            active_lease_id: active.unwrap().lease_id,
                        });
                    } else if !self
                        .supervisor_store
                        .get_runtime_lease(&owner.runtime_id, &owner.lease_id)?
                        .is_some_and(|lease| {
                            lease.is_owned_by(&owner)
                                && lease.status == RuntimeLeaseStatus::Released
                        })
                    {
                        return Err(SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' prior lease disappeared before checked release",
                            runtime_id
                        )));
                    }
                    checkpoint = self.advance_replacement_checkpoint(
                        &checkpoint,
                        RuntimeReplacementStage::ReleaseOldLease,
                        now_ms,
                    )?;
                    self.handles.remove(&runtime_id);
                }
                4 => {
                    if now_ms < checkpoint.schedule().next_eligible_at_ms() {
                        return Ok(false);
                    }
                    let lease = self.acquire_or_recover_replacement_lease(&restart, now_ms)?;
                    checkpoint = self.advance_replacement_checkpoint(
                        &checkpoint,
                        RuntimeReplacementStage::AcquireReplacementLease,
                        now_ms,
                    )?;
                    debug_assert_eq!(lease.runtime_id, restart.runtime_id);
                }
                5 => {
                    let lease = self.acquire_or_recover_replacement_lease(&restart, now_ms)?;
                    if let Some(handle) = self.handles.get(&runtime_id) {
                        if !lease.is_owned_by(&handle.owner) || !handle.handle.is_started() {
                            return Err(SupervisorRuntimeError::InvalidCommand(format!(
                                "runtime '{}' replacement handle does not match its lease",
                                runtime_id
                            )));
                        }
                    } else {
                        self.start_runtime_after_lease(
                            &runtime_id,
                            &lease,
                            now_ms,
                            restart.attempt,
                            Some(restart.cause.clone()),
                        )?;
                    }
                    checkpoint = self.advance_replacement_checkpoint(
                        &checkpoint,
                        RuntimeReplacementStage::StartReplacementHandle,
                        now_ms,
                    )?;
                }
                6 => {
                    let current = self
                        .supervisor_store
                        .get_active_runtime_lease(&restart.runtime_id)?;
                    if self.handles.get(&runtime_id).is_some_and(|handle| {
                        !current.as_ref().is_some_and(|lease| {
                            lease.is_owned_by(&handle.owner) && lease.is_active_at(now_ms)
                        })
                    }) {
                        self.stop_and_flush_stale_handle(&runtime_id)?;
                    }
                    let active = self.acquire_or_recover_replacement_lease(&restart, now_ms)?;
                    if !self.handles.contains_key(&runtime_id) {
                        self.start_runtime_after_lease(
                            &runtime_id,
                            &active,
                            now_ms,
                            restart.attempt,
                            Some(restart.cause.clone()),
                        )?;
                    }
                    let handle = self.handles.get(&runtime_id).expect("handle just started");
                    if !active.is_owned_by(&handle.owner)
                        || !active.is_active_at(now_ms)
                        || !handle.handle.is_started()
                    {
                        return Err(SupervisorRuntimeError::InvalidCommand(format!(
                            "runtime '{}' replacement is not active at completion",
                            runtime_id
                        )));
                    }
                    self.advance_replacement_checkpoint(
                        &checkpoint,
                        RuntimeReplacementStage::Completed,
                        now_ms,
                    )?;
                    return Ok(true);
                }
                _ => return Ok(true),
            }
        }
    }

    fn acquire_or_recover_replacement_lease(
        &mut self,
        restart: &RuntimeRestartRecord,
        now_ms: u64,
    ) -> Result<RuntimeLease, SupervisorRuntimeError> {
        if let Some(active) = self
            .supervisor_store
            .get_active_runtime_lease(&restart.runtime_id)?
        {
            if active.instance_id != self.instance_id {
                return Err(SupervisorRuntimeError::ActiveOwnerConflict {
                    runtime_id: restart.runtime_id.to_string(),
                    active_lease_id: active.lease_id,
                });
            }
            if active.is_active_at(now_ms) {
                return Ok(active);
            }
            let owner = active.owner();
            if self.handles.contains_key(restart.runtime_id.as_str()) {
                self.stop_flush_and_release_runtime(restart.runtime_id.as_str(), &owner, now_ms)?;
            } else {
                self.supervisor_store
                    .release_runtime_lease(&owner, now_ms)?;
                self.write_lifecycle_event(
                    Some(owner.runtime_id.clone()),
                    Some(owner.lease_id),
                    now_ms,
                    SupervisorLifecycleEventType::LeaseReleased,
                    Some("expired replacement lease released before recovery".to_string()),
                )?;
            }
        }

        let lease_id = format!(
            "lease:{}:{}:restart:{}:{}",
            self.instance_id, restart.runtime_id, restart.restart_id, now_ms
        );
        match self.supervisor_store.acquire_runtime_lease(
            restart.runtime_id.clone(),
            lease_id,
            self.instance_id.clone(),
            now_ms,
            self.lifecycle_config.lease_duration_ms,
        ) {
            Ok(lease) => Ok(lease),
            Err(SupervisorStoreError::DuplicateActiveLease {
                runtime_id,
                active_lease_id,
            }) => Err(SupervisorRuntimeError::ActiveOwnerConflict {
                runtime_id,
                active_lease_id,
            }),
            Err(error) => Err(error.into()),
        }
    }

    fn advance_replacement_checkpoint(
        &self,
        checkpoint: &RuntimeReplacementCheckpoint,
        stage: RuntimeReplacementStage,
        now_ms: u64,
    ) -> Result<RuntimeReplacementCheckpoint, SupervisorRuntimeError> {
        let successor = checkpoint.clone().advance(stage, now_ms)?;
        let committed = self
            .supervisor_store
            .compare_and_swap_replacement_checkpoint(checkpoint, &successor)?;
        #[cfg(test)]
        if self.abort_after_replacement_stage == Some(stage) {
            if let Ok(marker) = std::env::var("MELD_REPLACEMENT_CRASH_MARKER") {
                std::fs::write(marker, format!("{stage:?}")).expect("write crash-stage marker");
            }
            std::process::abort();
        }
        Ok(committed)
    }

    fn stop_flush_and_release_runtime(
        &mut self,
        runtime_id: &str,
        owner: &RuntimeLeaseOwner,
        now_ms: u64,
    ) -> Result<(), SupervisorRuntimeError> {
        let Some(runtime) = self.handles.get_mut(runtime_id) else {
            return Ok(());
        };
        if runtime.owner != *owner {
            return Ok(());
        }
        runtime.handle.request_stop();
        let safe_point = runtime.handle.wait_for_safe_point();
        if !safe_point.safe_for_flush {
            return Err(SupervisorRuntimeError::InvalidCommand(format!(
                "runtime '{}' did not reach a safe point",
                runtime_id
            )));
        }
        runtime.handle.flush_resources()?;
        self.stores
            .flush_boundary()
            .map_err(RuntimeAssemblyError::from)?;
        self.supervisor_store.release_runtime_lease(owner, now_ms)?;
        self.write_lifecycle_event(
            Some(owner.runtime_id.clone()),
            Some(owner.lease_id.clone()),
            now_ms,
            SupervisorLifecycleEventType::LeaseReleased,
            Some("expired lease released after safe stop and flush".to_string()),
        )?;
        self.handles.remove(runtime_id);
        Ok(())
    }

    fn stop_and_flush_stale_handle(
        &mut self,
        runtime_id: &str,
    ) -> Result<(), SupervisorRuntimeError> {
        let Some(runtime) = self.handles.get_mut(runtime_id) else {
            return Ok(());
        };
        runtime.handle.request_stop();
        let safe_point = runtime.handle.wait_for_safe_point();
        if !safe_point.safe_for_flush {
            return Err(SupervisorRuntimeError::InvalidCommand(format!(
                "runtime '{}' did not stop after losing its lease",
                runtime_id
            )));
        }
        runtime.handle.flush_resources()?;
        self.stores
            .flush_boundary()
            .map_err(RuntimeAssemblyError::from)?;
        self.handles.remove(runtime_id);
        Ok(())
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
            diagnostic: tick_report.map(|tick_report| RuntimeDiagnosticSummary {
                actor_id: tick_report.actor_id.clone(),
                retryable_issue_count: tick_report.retryable_errors.len() as u64,
                fatal_issue_count: tick_report.fatal_errors.len() as u64,
                budget_exhausted: tick_report.budget_exhausted,
                last_error_code: last_error_code(tick_report),
            }),
        };
        self.supervisor_store.write_runtime_heartbeat(&heartbeat)?;
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
) -> Result<BTreeMap<String, SupervisorRuntimeDesired>, SupervisorRuntimeError> {
    let mut desired = BTreeMap::new();
    for state in states {
        let runtime_id = RuntimeId::new(state.runtime_id.clone())?;
        desired.insert(
            state.runtime_id.clone(),
            SupervisorRuntimeDesired {
                runtime_id,
                enabled: state.enabled,
                factory_available: state.factory_available,
                role_class: state.role_class,
                implementation_state: state.implementation_state,
                restart_policy,
            },
        );
    }
    Ok(desired)
}

fn desired_default_health_status(desired: &SupervisorRuntimeDesired) -> RuntimeHealthStatus {
    if !desired.enabled {
        RuntimeHealthStatus::Stopped
    } else if !desired.factory_available
        || matches!(
            desired.implementation_state,
            RuntimeImplementationState::Inert | RuntimeImplementationState::Unavailable
        )
    {
        RuntimeHealthStatus::Unhealthy
    } else {
        RuntimeHealthStatus::Unknown
    }
}

fn start_role_order(role_class: RuntimeRoleClass) -> u8 {
    match role_class {
        RuntimeRoleClass::PassiveService => 0,
        RuntimeRoleClass::Actor => 1,
        RuntimeRoleClass::PortOnly => 2,
        RuntimeRoleClass::Unknown => 3,
    }
}

fn stop_role_order(role_class: RuntimeRoleClass) -> u8 {
    match role_class {
        RuntimeRoleClass::Actor => 0,
        RuntimeRoleClass::PassiveService => 1,
        RuntimeRoleClass::PortOnly => 2,
        RuntimeRoleClass::Unknown => 3,
    }
}

fn ordered_handle_ids(
    handles: &BTreeMap<String, SupervisedRuntimeHandle>,
    start_order: bool,
) -> Vec<String> {
    let mut runtime_ids = handles.keys().cloned().collect::<Vec<_>>();
    runtime_ids.sort_by_key(|runtime_id| {
        let role_class = handles
            .get(runtime_id)
            .map(|runtime| runtime.handle.role_class())
            .unwrap_or(RuntimeRoleClass::Unknown);
        let order = if start_order {
            start_role_order(role_class)
        } else {
            stop_role_order(role_class)
        };
        (order, runtime_id.clone())
    });
    runtime_ids
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

fn replacement_checkpoint_completed(checkpoint: &RuntimeReplacementCheckpoint) -> bool {
    checkpoint
        .completed_stages()
        .last()
        .is_some_and(|receipt| receipt.stage == RuntimeReplacementStage::Completed)
}

fn health_status_from_tick_report(
    tick_eligible: bool,
    report: Option<&WorkerTickReport>,
) -> RuntimeHealthStatus {
    if !tick_eligible {
        return RuntimeHealthStatus::Healthy;
    }
    let Some(report) = report else {
        return RuntimeHealthStatus::Unhealthy;
    };
    if !report.fatal_errors.is_empty() {
        RuntimeHealthStatus::Unhealthy
    } else if !report.retryable_errors.is_empty() || report.budget_exhausted {
        RuntimeHealthStatus::Degraded
    } else {
        RuntimeHealthStatus::Healthy
    }
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
    use std::process::Command;
    use std::time::{Duration, Instant};

    use crate::runtime::assembly::{ProductRuntimeAssembly, ProductRuntimeConfig};
    use meld_events::{
        AppendMode, DomainObjectRef, EventEnvelope, EventIngressFenceSnapshot,
        EventIngressFenceState,
    };
    use meld_execution::task_network::EventAppendSink;
    use serde_json::json;

    use super::*;

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
        assert_eq!(status.runtimes.len(), 15);
        assert_eq!(
            status
                .runtimes
                .iter()
                .filter(|runtime| runtime.desired_enabled && runtime.handle_started)
                .count(),
            1
        );
        let graph_replay = runtime_status(&status, "world_model.graph_replay");
        assert!(graph_replay.desired_enabled);
        assert!(graph_replay.factory_available);
        assert!(graph_replay.handle_started);
        assert_eq!(graph_replay.health_status, RuntimeHealthStatus::Starting);
        assert_eq!(graph_replay.last_heartbeat_at_ms, Some(100));
        assert_eq!(graph_replay.heartbeat_age_ms, Some(50));
        assert_eq!(
            graph_replay.last_lifecycle_event,
            Some(SupervisorLifecycleEventType::HeartbeatAccepted)
        );
        let event_append = runtime_status(&status, "event.append");
        assert!(!event_append.desired_enabled);
        assert!(event_append.factory_available);
        assert!(!event_append.handle_started);
        assert_eq!(event_append.health_status, RuntimeHealthStatus::Stopped);
        let dispatch = runtime_status(&status, "execution.task_dispatch");
        assert!(!dispatch.desired_enabled);
        assert!(!dispatch.handle_started);
        assert_eq!(dispatch.health_status, RuntimeHealthStatus::Stopped);
    }

    #[test]
    fn historical_restart_audit_does_not_trigger_replacement_on_startup() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
        assembly
            .supervisor_store()
            .put_restart_record(&RuntimeRestartRecord {
                restart_id: "historical-restart".to_string(),
                runtime_id: runtime_id.clone(),
                instance_id: "retired-instance".to_string(),
                previous_lease_id: None,
                cause: RestartCause::OperatorRequested,
                attempt: 1,
                requested_at_ms: 10,
                backoff_ms: 5,
            })
            .unwrap();

        let supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        assert!(
            runtime_status(
                &supervisor.status_snapshot(100).unwrap(),
                runtime_id.as_str()
            )
            .handle_started
        );
        assert_eq!(
            assembly
                .supervisor_store()
                .get_active_runtime_lease(&runtime_id)
                .unwrap()
                .unwrap()
                .instance_id,
            "instance-a"
        );
        assert!(assembly
            .supervisor_store()
            .list_replacement_checkpoints()
            .unwrap()
            .is_empty());
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
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
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
            runtime_status(
                &supervisor.status_snapshot(31).unwrap(),
                "world_model.graph_replay"
            )
            .handle_started
        );
    }

    #[test]
    fn former_owner_stops_local_handle_after_expired_lease_takeover() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut former = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let mut takeover = SupervisorStartCommand::new("instance-b", 130);
        takeover.restart_backoff_ms = 50;
        let mut current =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), takeover).unwrap();

        assert!(
            !runtime_status(
                &current.status_snapshot(130).unwrap(),
                "world_model.graph_replay"
            )
            .handle_started
        );
        former.tick(131).unwrap();
        assert_eq!(
            current
                .evaluate_restart_policies(180)
                .unwrap()
                .restarted_runtime_ids,
            vec!["world_model.graph_replay"]
        );

        assert!(
            !runtime_status(
                &former.status_snapshot(131).unwrap(),
                "world_model.graph_replay"
            )
            .handle_started
        );
        assert!(
            runtime_status(
                &current.status_snapshot(131).unwrap(),
                "world_model.graph_replay"
            )
            .handle_started
        );
    }

    #[test]
    fn losing_contender_cannot_overwrite_active_owner_truth() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut owner = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
        let owner_lease = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        let sentinel_runtime = RuntimeId::new("event.append").unwrap();
        let sentinel_desired = RuntimeDesiredState {
            runtime_id: sentinel_runtime.clone(),
            enabled: true,
            restart_policy: RestartPolicy::Never,
        };
        let sentinel_health = RuntimeHealthSnapshot {
            runtime_id: sentinel_runtime.clone(),
            lease_id: None,
            observed_at_ms: 100,
            status: RuntimeHealthStatus::Degraded,
            last_heartbeat_at_ms: None,
            retryable_error_count: 7,
            fatal_error_count: 0,
            budget_exhausted: false,
            restart_count: 9,
            last_restart_cause: Some(RestartCause::RetryableFailure),
        };
        assembly
            .supervisor_store()
            .put_desired_runtime_state(&sentinel_desired)
            .unwrap();
        assembly
            .supervisor_store()
            .put_health_snapshot(&sentinel_health)
            .unwrap();

        let error = match RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-b", 101),
        ) {
            Ok(_) => panic!("a second supervisor must not start over an active owner"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            SupervisorRuntimeError::ActiveOwnerConflict { .. }
        ));
        let contender = assembly
            .supervisor_store()
            .get_runtime_instance("instance-b")
            .unwrap()
            .unwrap();
        assert_eq!(contender.status, RuntimeInstanceStatus::Failed);
        let active = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        assert_eq!(active, owner_lease);
        assert_eq!(active.instance_id, "instance-a");
        let heartbeat = assembly
            .supervisor_store()
            .get_runtime_heartbeat(&runtime_id)
            .unwrap()
            .unwrap();
        assert_eq!(heartbeat.instance_id, "instance-a");
        assert_eq!(heartbeat.lease_id, active.lease_id);
        let health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();
        assert_eq!(health.lease_id.as_deref(), Some(active.lease_id.as_str()));
        assert_eq!(health.status, RuntimeHealthStatus::Starting);
        assert_eq!(
            assembly
                .supervisor_store()
                .get_desired_runtime_state(&sentinel_runtime)
                .unwrap(),
            Some(sentinel_desired)
        );
        assert_eq!(
            assembly
                .supervisor_store()
                .get_health_snapshot(&sentinel_runtime)
                .unwrap(),
            Some(sentinel_health)
        );
        let status = owner.status_snapshot(101).unwrap();
        let graph = runtime_status(&status, "world_model.graph_replay");
        assert_eq!(
            graph.active_owner_instance_id.as_deref(),
            Some("instance-a")
        );
        assert_eq!(graph.health_status, RuntimeHealthStatus::Starting);

        owner.tick(102).unwrap();
    }

    #[test]
    fn duplicate_instance_id_cannot_overwrite_active_owner_truth() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let owner = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
        let owner_instance = assembly
            .supervisor_store()
            .get_runtime_instance("instance-a")
            .unwrap()
            .unwrap();
        let owner_lease = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        let owner_heartbeat = assembly
            .supervisor_store()
            .get_runtime_heartbeat(&runtime_id)
            .unwrap()
            .unwrap();
        let owner_health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();
        let owner_event = assembly
            .supervisor_store()
            .latest_lifecycle_event_for_runtime(&runtime_id)
            .unwrap()
            .unwrap();

        let error = match RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 101),
        ) {
            Ok(_) => panic!("duplicate instance id must fail before registration"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            SupervisorRuntimeError::Store(SupervisorStoreError::DuplicateInstanceId { .. })
        ));
        assert_eq!(
            assembly
                .supervisor_store()
                .get_runtime_instance("instance-a")
                .unwrap(),
            Some(owner_instance)
        );
        assert_eq!(
            assembly
                .supervisor_store()
                .get_active_runtime_lease(&runtime_id)
                .unwrap(),
            Some(owner_lease)
        );
        assert_eq!(
            assembly
                .supervisor_store()
                .get_runtime_heartbeat(&runtime_id)
                .unwrap(),
            Some(owner_heartbeat)
        );
        assert_eq!(
            assembly
                .supervisor_store()
                .get_health_snapshot(&runtime_id)
                .unwrap(),
            Some(owner_health)
        );
        assert_eq!(
            assembly
                .supervisor_store()
                .latest_lifecycle_event_for_runtime(&runtime_id)
                .unwrap(),
            Some(owner_event)
        );
        assert_eq!(owner.instance_id(), "instance-a");
    }

    #[test]
    fn legacy_runtime_alias_hosts_canonical_passive_service_without_worker_ticks() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["events.ledger".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let canonical = RuntimeId::new("event.append").unwrap();
        let legacy = RuntimeId::new("events.ledger").unwrap();

        assert_eq!(legacy, canonical);

        assert_eq!(
            assembly
                .desired_runtime_state()
                .iter()
                .filter(|state| state.enabled)
                .map(|state| state.runtime_id.as_str())
                .collect::<Vec<_>>(),
            vec!["event.append"]
        );
        assert!(assembly
            .supervisor_store()
            .get_desired_runtime_state(&canonical)
            .unwrap()
            .is_some());
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&canonical)
            .unwrap()
            .is_some());
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&legacy)
            .unwrap()
            .is_some());
        assert!(assembly
            .supervisor_store()
            .get_runtime_heartbeat(&legacy)
            .unwrap()
            .is_some());
        assert!(assembly
            .supervisor_store()
            .get_health_snapshot(&legacy)
            .unwrap()
            .is_some());
        assert!(assembly
            .supervisor_store()
            .latest_lifecycle_event_for_runtime(&legacy)
            .unwrap()
            .is_some());
        let snapshot = supervisor.status_snapshot(100).unwrap();
        let status = runtime_status(&snapshot, "event.append");
        assert!(status.desired_enabled);
        assert!(status.handle_started);
        assert_eq!(
            status.active_owner_instance_id.as_deref(),
            Some("instance-a")
        );
        assert_eq!(status.health_status, RuntimeHealthStatus::Starting);

        let mut supervisor = supervisor;
        let tick = supervisor.tick(101).unwrap();
        assert!(tick.actions.is_empty());
        let heartbeat = assembly
            .supervisor_store()
            .get_runtime_heartbeat(&canonical)
            .unwrap()
            .unwrap();
        assert_eq!(heartbeat.health.status, RuntimeHealthStatus::Healthy);
        assert!(heartbeat.diagnostic.is_none());
    }

    #[test]
    fn passive_services_start_before_actors_and_stop_after_them() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec![
            "event.append".to_string(),
            "world_model.graph_replay".to_string(),
        ];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let event_append = RuntimeId::new("event.append").unwrap();
        let graph_replay = RuntimeId::new("world_model.graph_replay").unwrap();
        let service_started = assembly
            .supervisor_store()
            .latest_lifecycle_event_for_runtime(&event_append)
            .unwrap()
            .unwrap();
        let actor_started = assembly
            .supervisor_store()
            .latest_lifecycle_event_for_runtime(&graph_replay)
            .unwrap()
            .unwrap();
        assert!(service_started.event_id < actor_started.event_id);

        let shutdown = supervisor.request_shutdown(101).unwrap();
        assert_eq!(
            shutdown
                .stop_reports
                .iter()
                .map(|report| report.runtime_id.as_str())
                .collect::<Vec<_>>(),
            vec!["world_model.graph_replay", "event.append"]
        );
    }

    #[test]
    fn explicitly_enabled_port_only_role_never_acquires_worker_lease() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.goal.set".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("execution.goal_set").unwrap();

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
        let snapshot = supervisor.status_snapshot(100).unwrap();
        let status = runtime_status(&snapshot, "execution.goal_set");
        assert!(status.desired_enabled);
        assert!(status.factory_available);
        assert!(!status.handle_started);
        assert_eq!(status.health_status, RuntimeHealthStatus::Unknown);
    }

    #[test]
    fn shutdown_writes_final_heartbeat_releases_leases_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "execution",
                    "network-a",
                    "execution.before_shutdown",
                    None,
                    json!({"accepted": true}),
                )
                .with_record_id("before-shutdown"),
            )
            .unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        let first = supervisor.request_shutdown(200).unwrap();
        let second = supervisor.request_shutdown(210).unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
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

        assert_eq!(first.stopped_runtime_ids, vec!["world_model.graph_replay"]);
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
        assert_eq!(first.final_event_barrier, second.final_event_barrier);
        assert_eq!(
            first.final_event_barrier.fence().state,
            EventIngressFenceState::Closed
        );
        assert_eq!(
            first.final_event_barrier.fence().ledger_id,
            first.final_event_barrier.watermark().ledger_id
        );
        assert_eq!(
            first.final_event_barrier.watermark().committed_seq,
            first.final_event_barrier.watermark().tip_seq
        );
        assert_eq!(first.final_event_barrier.watermark().tip_seq, 1);
        assert!(assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "execution",
                    "network-a",
                    "execution.after_shutdown",
                    None,
                    json!({"accepted": false}),
                )
                .with_record_id("after-shutdown"),
            )
            .is_err());
        assert_eq!(
            runtime_status(
                &supervisor.status_snapshot(210).unwrap(),
                "world_model.graph_replay"
            )
            .health_status,
            RuntimeHealthStatus::Stopped
        );
    }

    #[test]
    fn shutdown_completion_and_final_barrier_survive_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let first = supervisor.request_shutdown(200).unwrap();
        let persisted = assembly
            .supervisor_store()
            .get_shutdown_completion(&first.shutdown_id)
            .unwrap()
            .unwrap();
        assert_eq!(persisted.final_event_barrier(), first.final_event_barrier);
        drop(supervisor);
        drop(assembly);

        let reopened = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut resumed = RuntimeSupervisor::start(
            reopened.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let replay = resumed.request_shutdown(300).unwrap();

        assert_eq!(replay.shutdown_id, first.shutdown_id);
        assert_eq!(replay.final_event_barrier, first.final_event_barrier);
        assert!(replay.stopped_runtime_ids.is_empty());
        assert!(reopened
            .supervisor_store()
            .get_active_runtime_lease(&RuntimeId::new("world_model.graph_replay").unwrap())
            .unwrap()
            .is_none());
        assert_eq!(
            reopened
                .supervisor_store()
                .get_runtime_instance("instance-a")
                .unwrap()
                .unwrap()
                .status,
            RuntimeInstanceStatus::Stopped
        );
        assert!(reopened
            .ports()
            .event_append()
            .append_envelope_idempotent(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "execution",
                    "network-a",
                    "execution.after_reopened_shutdown",
                    None,
                    json!({"accepted": false}),
                )
                .with_record_id("after-reopened-shutdown"),
            )
            .is_err());
    }

    #[test]
    fn completed_shutdown_rejects_equal_watermark_with_divergent_fence() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let watermark = assembly.ports().event_append().watermark().unwrap();
        let forged_barrier = meld_events::EventFinalBarrier::try_new(
            EventIngressFenceSnapshot {
                ledger_id: watermark.ledger_id,
                generation: 99,
                state: EventIngressFenceState::Closed,
            },
            watermark,
        )
        .unwrap();
        let shutdown = RuntimeShutdownState {
            shutdown_id: "shutdown:instance-a:100".to_string(),
            instance_id: "instance-a".to_string(),
            requested_at_ms: 200,
            completed_at_ms: Some(200),
            status: RuntimeShutdownStatus::Completed,
        };
        assembly
            .supervisor_store()
            .put_shutdown_completion(
                &RuntimeShutdownCompletion::try_new(shutdown, forged_barrier).unwrap(),
            )
            .unwrap();

        assert!(matches!(
            RuntimeSupervisor::start(
                assembly.supervisor_startup_package(),
                SupervisorStartCommand::new("instance-a", 100),
            ),
            Err(SupervisorRuntimeError::InvalidCommand(message))
                if message.contains("barrier diverged")
        ));
    }

    #[test]
    fn shutdown_failure_is_durable_unhealthy_and_exactly_retryable() {
        for (fail_safe_point, fail_flush) in [(true, false), (false, true)] {
            let temp = tempfile::tempdir().unwrap();
            let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
            let mut supervisor = RuntimeSupervisor::start(
                assembly.supervisor_startup_package(),
                SupervisorStartCommand::new("instance-a", 100),
            )
            .unwrap();
            supervisor
                .handles
                .get_mut("world_model.graph_replay")
                .unwrap()
                .handle
                .set_test_lifecycle_failures(fail_safe_point, fail_flush);
            let shutdown_id = "shutdown:instance-a:100";
            let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();

            assert!(supervisor.request_shutdown(200).is_err());
            assert_eq!(
                assembly
                    .supervisor_store()
                    .get_shutdown_state(shutdown_id)
                    .unwrap()
                    .unwrap()
                    .status,
                RuntimeShutdownStatus::Failed
            );
            assert_eq!(
                assembly
                    .supervisor_store()
                    .get_runtime_instance("instance-a")
                    .unwrap()
                    .unwrap()
                    .status,
                RuntimeInstanceStatus::Failed
            );
            assert_eq!(
                assembly
                    .supervisor_store()
                    .get_health_snapshot(&runtime_id)
                    .unwrap()
                    .unwrap()
                    .status,
                RuntimeHealthStatus::Unhealthy
            );
            assert!(assembly
                .supervisor_store()
                .get_active_runtime_lease(&runtime_id)
                .unwrap()
                .is_some());
            assert!(assembly
                .ports()
                .event_append()
                .append_envelope_idempotent(
                    EventEnvelope::with_now_domain(
                        "session-a",
                        "execution",
                        "network-a",
                        "execution.during_failed_shutdown",
                        None,
                        json!({"accepted": false}),
                    )
                    .with_record_id("during-failed-shutdown"),
                )
                .is_err());

            supervisor
                .handles
                .get_mut("world_model.graph_replay")
                .unwrap()
                .handle
                .set_test_lifecycle_failures(false, false);
            let completed = supervisor.request_shutdown(210).unwrap();
            let state = assembly
                .supervisor_store()
                .get_shutdown_completion(&completed.shutdown_id)
                .unwrap()
                .unwrap();
            assert_eq!(state.shutdown().requested_at_ms, 200);
            assert_eq!(state.shutdown().completed_at_ms, Some(210));
        }
    }

    #[test]
    fn interrupted_shutdown_reopens_fenced_and_resumes_without_runtime_start() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        supervisor
            .handles
            .get_mut("world_model.graph_replay")
            .unwrap()
            .handle
            .set_test_lifecycle_failures(true, false);
        assert!(supervisor.request_shutdown(200).is_err());
        drop(supervisor);
        drop(assembly);

        let reopened = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut recovered = RuntimeSupervisor::start(
            reopened.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-b", 300),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
        assert_eq!(recovered.instance_id(), "instance-a");
        assert!(reopened
            .ports()
            .event_append()
            .append_envelope_idempotent(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "execution",
                    "network-a",
                    "execution.after_interrupted_shutdown",
                    None,
                    json!({"accepted": false}),
                )
                .with_record_id("after-interrupted-shutdown"),
            )
            .is_err());
        assert!(recovered.handles.is_empty());
        assert!(reopened
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_some());

        let completion = recovered.request_shutdown(310).unwrap();
        assert_eq!(completion.shutdown_id, "shutdown:instance-a:100");
        assert!(reopened
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert_eq!(
            reopened
                .supervisor_store()
                .get_shutdown_state(&completion.shutdown_id)
                .unwrap()
                .unwrap()
                .status,
            RuntimeShutdownStatus::Completed
        );
    }

    #[test]
    fn shutdown_barrier_crash_child() {
        let Ok(root) = std::env::var("MELD_SHUTDOWN_CRASH_ROOT") else {
            return;
        };
        let assembly = ProductRuntimeAssembly::load_for_product_root(root).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        supervisor.abort_after_shutdown_barrier = true;
        let _ = supervisor.request_shutdown(200);
        panic!("shutdown crash hook did not abort after final barrier");
    }

    #[test]
    fn abrupt_loss_after_shutdown_barrier_reopens_fenced_and_completes() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("shutdown-crash-stage");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("runtime::supervisor::entrypoint::tests::shutdown_barrier_crash_child")
            .arg("--test-threads=1")
            .current_dir(temp.path())
            .env("MELD_SHUTDOWN_CRASH_ROOT", temp.path())
            .env("MELD_SHUTDOWN_CRASH_MARKER", &marker)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("shutdown barrier crash child timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(!status.success());
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "barrier-closed");

        let reopened = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut recovered = RuntimeSupervisor::start(
            reopened.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-b", 300),
        )
        .unwrap();
        assert!(recovered.handles.is_empty());
        assert!(reopened
            .ports()
            .event_append()
            .append_envelope_idempotent(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "execution",
                    "network-a",
                    "execution.after_shutdown_crash",
                    None,
                    json!({"accepted": false}),
                )
                .with_record_id("after-shutdown-crash"),
            )
            .is_err());
        let completion = recovered.request_shutdown(310).unwrap();
        assert_eq!(completion.shutdown_id, "shutdown:instance-a:100");
    }

    #[test]
    fn heartbeat_expiry_policy_restarts_with_new_lease_and_counter() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
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

        assert_eq!(
            evaluation.expired_runtime_ids,
            vec!["world_model.graph_replay"]
        );
        assert_eq!(
            evaluation.restarted_runtime_ids,
            vec!["world_model.graph_replay"]
        );
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
    fn restart_backoff_releases_only_after_flush_and_waits_until_eligible() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.restart_backoff_ms = 50;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
        let old_lease = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();

        let scheduled = supervisor.evaluate_restart_policies(130).unwrap();
        let checkpoint = assembly
            .supervisor_store()
            .list_replacement_checkpoints()
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(
            scheduled.expired_runtime_ids,
            vec!["world_model.graph_replay"]
        );
        assert!(scheduled.restarted_runtime_ids.is_empty());
        assert_eq!(checkpoint.schedule().next_eligible_at_ms(), 180);
        assert_eq!(
            checkpoint
                .completed_stages()
                .iter()
                .map(|receipt| receipt.stage)
                .collect::<Vec<_>>(),
            vec![
                RuntimeReplacementStage::StopOldHandle,
                RuntimeReplacementStage::AwaitOldSafePoint,
                RuntimeReplacementStage::FlushOldHandle,
                RuntimeReplacementStage::ReleaseOldLease,
            ]
        );
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert_eq!(
            assembly
                .supervisor_store()
                .get_runtime_lease(&runtime_id, &old_lease.lease_id)
                .unwrap()
                .unwrap()
                .status,
            crate::runtime::supervisor::contracts::RuntimeLeaseStatus::Released
        );

        assert!(supervisor
            .evaluate_restart_policies(179)
            .unwrap()
            .restarted_runtime_ids
            .is_empty());
        assert_eq!(
            supervisor
                .evaluate_restart_policies(180)
                .unwrap()
                .restarted_runtime_ids,
            vec!["world_model.graph_replay"]
        );
        let completed = assembly
            .supervisor_store()
            .list_replacement_checkpoints()
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(
            completed.completed_stages().last().unwrap().stage,
            RuntimeReplacementStage::Completed
        );
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_some());
    }

    #[test]
    fn restart_backoff_resumes_through_supervisor_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config.clone()).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.restart_backoff_ms = 50;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        assert!(supervisor
            .evaluate_restart_policies(130)
            .unwrap()
            .restarted_runtime_ids
            .is_empty());
        drop(supervisor);
        drop(assembly);

        let reopened = ProductRuntimeAssembly::load(config).unwrap();
        let mut resumed = RuntimeSupervisor::start(
            reopened.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-b", 150),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
        assert!(reopened
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert!(
            !runtime_status(&resumed.status_snapshot(150).unwrap(), runtime_id.as_str())
                .handle_started
        );

        assert_eq!(
            resumed
                .evaluate_restart_policies(180)
                .unwrap()
                .restarted_runtime_ids,
            vec!["world_model.graph_replay"]
        );
        let active = reopened
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();
        assert_eq!(active.instance_id, "instance-b");
        assert_eq!(
            reopened
                .supervisor_store()
                .list_replacement_checkpoints()
                .unwrap()
                .pop()
                .unwrap()
                .completed_stages()
                .last()
                .unwrap()
                .stage,
            RuntimeReplacementStage::Completed
        );
    }

    #[test]
    fn replacement_reopen_recovers_after_acquire_and_after_start_receipts() {
        for final_stage in [
            RuntimeReplacementStage::AcquireReplacementLease,
            RuntimeReplacementStage::StartReplacementHandle,
        ] {
            let temp = tempfile::tempdir().unwrap();
            let mut config = ProductRuntimeConfig::for_product_root(temp.path());
            config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
            config.lifecycle_config.lease_duration_ms = 20;
            let assembly = ProductRuntimeAssembly::load(config.clone()).unwrap();
            let mut command = SupervisorStartCommand::new("instance-a", 100);
            command.restart_backoff_ms = 50;
            let mut supervisor =
                RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
            supervisor.evaluate_restart_policies(130).unwrap();
            let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
            let checkpoint = assembly
                .supervisor_store()
                .list_replacement_checkpoints()
                .unwrap()
                .pop()
                .unwrap();
            let replacement = assembly
                .supervisor_store()
                .acquire_runtime_lease(
                    runtime_id.clone(),
                    "crashed-replacement",
                    "instance-a",
                    180,
                    20,
                )
                .unwrap();
            let acquired = checkpoint
                .clone()
                .advance(RuntimeReplacementStage::AcquireReplacementLease, 180)
                .unwrap();
            assembly
                .supervisor_store()
                .compare_and_swap_replacement_checkpoint(&checkpoint, &acquired)
                .unwrap();
            if final_stage == RuntimeReplacementStage::StartReplacementHandle {
                let started = acquired
                    .clone()
                    .advance(RuntimeReplacementStage::StartReplacementHandle, 181)
                    .unwrap();
                assembly
                    .supervisor_store()
                    .compare_and_swap_replacement_checkpoint(&acquired, &started)
                    .unwrap();
            }
            assert_eq!(replacement.expires_at_ms, 200);
            drop(supervisor);
            drop(assembly);

            let reopened = ProductRuntimeAssembly::load(config).unwrap();
            let resumed = RuntimeSupervisor::start(
                reopened.supervisor_startup_package(),
                SupervisorStartCommand::new("instance-b", 201),
            )
            .unwrap();
            let active = reopened
                .supervisor_store()
                .get_active_runtime_lease(&runtime_id)
                .unwrap()
                .unwrap();

            assert_eq!(active.instance_id, "instance-b");
            assert!(active.is_active_at(201));
            assert!(
                runtime_status(&resumed.status_snapshot(201).unwrap(), runtime_id.as_str())
                    .handle_started
            );
            assert_eq!(
                reopened
                    .supervisor_store()
                    .list_replacement_checkpoints()
                    .unwrap()
                    .pop()
                    .unwrap()
                    .completed_stages()
                    .last()
                    .unwrap()
                    .stage,
                RuntimeReplacementStage::Completed
            );
        }
    }

    #[test]
    fn replacement_reopen_recovers_every_pre_release_checkpoint() {
        let stages = [
            RuntimeReplacementStage::StopOldHandle,
            RuntimeReplacementStage::AwaitOldSafePoint,
            RuntimeReplacementStage::FlushOldHandle,
        ];
        for completed_stage_count in 0..=stages.len() {
            let temp = tempfile::tempdir().unwrap();
            let mut config = ProductRuntimeConfig::for_product_root(temp.path());
            config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
            config.lifecycle_config.lease_duration_ms = 20;
            let assembly = ProductRuntimeAssembly::load(config.clone()).unwrap();
            let supervisor = RuntimeSupervisor::start(
                assembly.supervisor_startup_package(),
                SupervisorStartCommand::new("instance-a", 100),
            )
            .unwrap();
            let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
            let old_lease = assembly
                .supervisor_store()
                .get_active_runtime_lease(&runtime_id)
                .unwrap()
                .unwrap();
            let schedule = RuntimeRestartSchedule::try_new(RuntimeRestartRecord {
                restart_id: format!("interrupted-{completed_stage_count}"),
                runtime_id: runtime_id.clone(),
                instance_id: "instance-a".to_string(),
                previous_lease_id: Some(old_lease.lease_id),
                cause: RestartCause::HeartbeatExpired,
                attempt: 1,
                requested_at_ms: 110,
                backoff_ms: 50,
            })
            .unwrap();
            let mut checkpoint = assembly
                .supervisor_store()
                .persist_restart_replacement(&schedule)
                .unwrap();
            for (offset, stage) in stages.iter().take(completed_stage_count).enumerate() {
                let successor = checkpoint
                    .clone()
                    .advance(*stage, 111 + offset as u64)
                    .unwrap();
                checkpoint = assembly
                    .supervisor_store()
                    .compare_and_swap_replacement_checkpoint(&checkpoint, &successor)
                    .unwrap();
            }
            drop(supervisor);
            drop(assembly);

            let reopened = ProductRuntimeAssembly::load(config).unwrap();
            let mut resumed = RuntimeSupervisor::start(
                reopened.supervisor_startup_package(),
                SupervisorStartCommand::new("instance-b", 130),
            )
            .unwrap();
            let recovered = reopened
                .supervisor_store()
                .list_replacement_checkpoints()
                .unwrap()
                .pop()
                .unwrap();
            assert_eq!(
                recovered.completed_stages().last().unwrap().stage,
                RuntimeReplacementStage::ReleaseOldLease
            );
            assert!(reopened
                .supervisor_store()
                .get_active_runtime_lease(&runtime_id)
                .unwrap()
                .is_none());

            assert_eq!(
                resumed
                    .evaluate_restart_policies(160)
                    .unwrap()
                    .restarted_runtime_ids,
                vec!["world_model.graph_replay"]
            );
        }
    }

    #[test]
    fn replacement_crash_child() {
        let Ok(root) = std::env::var("MELD_REPLACEMENT_CRASH_ROOT") else {
            return;
        };
        let stage_name = std::env::var("MELD_REPLACEMENT_CRASH_STAGE").unwrap();
        let stage = match stage_name.as_str() {
            "stop" => RuntimeReplacementStage::StopOldHandle,
            "safe" => RuntimeReplacementStage::AwaitOldSafePoint,
            "flush" => RuntimeReplacementStage::FlushOldHandle,
            "release" => RuntimeReplacementStage::ReleaseOldLease,
            "acquire" => RuntimeReplacementStage::AcquireReplacementLease,
            "start" => RuntimeReplacementStage::StartReplacementHandle,
            other => panic!("unknown replacement crash stage {other}"),
        };
        let mut config = ProductRuntimeConfig::for_product_root(root);
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.restart_backoff_ms = if matches!(
            stage,
            RuntimeReplacementStage::AcquireReplacementLease
                | RuntimeReplacementStage::StartReplacementHandle
        ) {
            0
        } else {
            50
        };
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        supervisor.abort_after_replacement_stage = Some(stage);
        let _ = supervisor.evaluate_restart_policies(130);
        panic!("replacement crash hook did not abort at {stage_name}");
    }

    #[test]
    fn abrupt_process_loss_recovers_every_replacement_stage() {
        for stage_name in ["stop", "safe", "flush", "release", "acquire", "start"] {
            let temp = tempfile::tempdir().unwrap();
            let marker = temp.path().join("replacement-crash-stage");
            let mut child = Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("runtime::supervisor::entrypoint::tests::replacement_crash_child")
                .arg("--test-threads=1")
                .current_dir(temp.path())
                .env("MELD_REPLACEMENT_CRASH_ROOT", temp.path())
                .env("MELD_REPLACEMENT_CRASH_STAGE", stage_name)
                .env("MELD_REPLACEMENT_CRASH_MARKER", &marker)
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() >= deadline {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("replacement crash child timed out at {stage_name}");
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            assert!(!status.success());
            assert!(std::fs::read_to_string(&marker)
                .unwrap()
                .contains(stage_name_to_debug_name(stage_name)));

            let mut config = ProductRuntimeConfig::for_product_root(temp.path());
            config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
            config.lifecycle_config.lease_duration_ms = 20;
            let reopened = ProductRuntimeAssembly::load(config).unwrap();
            let resumed = RuntimeSupervisor::start(
                reopened.supervisor_startup_package(),
                SupervisorStartCommand::new("instance-b", 180),
            )
            .unwrap();
            let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();

            assert!(
                runtime_status(&resumed.status_snapshot(180).unwrap(), runtime_id.as_str())
                    .handle_started
            );
            assert_eq!(
                reopened
                    .supervisor_store()
                    .list_replacement_checkpoints()
                    .unwrap()
                    .pop()
                    .unwrap()
                    .completed_stages()
                    .last()
                    .unwrap()
                    .stage,
                RuntimeReplacementStage::Completed
            );
        }
    }

    #[test]
    fn disabled_runtime_does_not_resume_pending_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config.clone()).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.restart_backoff_ms = 50;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        supervisor.evaluate_restart_policies(130).unwrap();
        drop(supervisor);
        drop(assembly);

        config.enabled_runtime_ids.clear();
        config.disabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        let reopened = ProductRuntimeAssembly::load(config).unwrap();
        let mut resumed = RuntimeSupervisor::start(
            reopened.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-b", 200),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();

        assert!(resumed
            .evaluate_restart_policies(250)
            .unwrap()
            .restarted_runtime_ids
            .is_empty());
        assert!(reopened
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert!(
            !runtime_status(&resumed.status_snapshot(250).unwrap(), runtime_id.as_str())
                .handle_started
        );
    }

    #[test]
    fn never_restart_policy_leaves_expired_runtime_unhealthy() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.default_restart_policy = RestartPolicy::Never;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();

        let evaluation = supervisor.evaluate_restart_policies(130).unwrap();
        let health = assembly
            .supervisor_store()
            .get_health_snapshot(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(
            evaluation.expired_runtime_ids,
            vec!["world_model.graph_replay"]
        );
        assert!(evaluation.restarted_runtime_ids.is_empty());
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert_eq!(health.status, RuntimeHealthStatus::Unhealthy);
        assert_eq!(
            runtime_status(
                &supervisor.status_snapshot(131).unwrap(),
                "world_model.graph_replay"
            )
            .health_status,
            RuntimeHealthStatus::Unhealthy
        );
    }

    #[test]
    fn exhausted_restart_limit_safely_releases_old_ownership() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.restart_attempt_limit = 0;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
        let old = assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .unwrap();

        let evaluation = supervisor.evaluate_restart_policies(130).unwrap();

        assert!(evaluation.restarted_runtime_ids.is_empty());
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert_eq!(
            assembly
                .supervisor_store()
                .get_runtime_lease(&runtime_id, &old.lease_id)
                .unwrap()
                .unwrap()
                .status,
            RuntimeLeaseStatus::Released
        );
        assert_eq!(
            assembly
                .supervisor_store()
                .get_health_snapshot(&runtime_id)
                .unwrap()
                .unwrap()
                .status,
            RuntimeHealthStatus::Unhealthy
        );
        assert!(
            !runtime_status(
                &supervisor.status_snapshot(131).unwrap(),
                runtime_id.as_str()
            )
            .handle_started
        );
    }

    #[test]
    fn retryable_failure_policy_restarts_without_reading_domain_state() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.default_restart_policy = RestartPolicy::OnRetryableFailure;
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
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
                    actor_id: "world_model.graph_replay".to_string(),
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
        assert_eq!(
            evaluation.restarted_runtime_ids,
            vec!["world_model.graph_replay"]
        );
        assert_eq!(health.restart_count, 1);
        assert_eq!(
            health.last_restart_cause,
            Some(RestartCause::RetryableFailure)
        );
        assert_eq!(health.status, RuntimeHealthStatus::Starting);
    }

    #[test]
    fn tick_renews_lease_and_writes_heartbeat() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();

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

        assert_eq!(report.renewed_runtime_ids, vec!["world_model.graph_replay"]);
        assert_eq!(
            report.heartbeat_runtime_ids,
            vec!["world_model.graph_replay"]
        );
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
    fn same_timestamp_ticks_receive_distinct_action_ids() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();

        let first = supervisor.tick(120).unwrap();
        let second = supervisor.tick(120).unwrap();

        assert_eq!(first.actions.len(), 1);
        assert_eq!(second.actions.len(), 1);
        assert_ne!(first.actions[0].action_id, second.actions[0].action_id);
        assert_eq!(first.actions[0].runtime_id, "world_model.graph_replay");
        assert_eq!(second.actions[0].runtime_id, "world_model.graph_replay");
    }

    #[test]
    fn tick_runs_graph_replay_handle_and_records_worker_diagnostic() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        assembly
            .event_authority()
            .append_capability()
            .append_durable(
                EventEnvelope::new_domain(
                    "2026-06-22T00:00:00Z".to_string(),
                    "session-a",
                    "workspace_fs",
                    "workspace-a",
                    "workspace.node.observed",
                    None,
                    json!({ "node": "node-a" }),
                )
                .with_graph(vec![subject], Vec::new())
                .with_record_id("workspace-node-a"),
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
    fn tick_preserves_restart_count_in_health_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 20;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();

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
        config.enabled_runtime_ids = vec!["world_model.graph_replay".to_string()];
        config.lifecycle_config.lease_duration_ms = 5;
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("instance-a", 100),
        )
        .unwrap();
        let runtime_id = RuntimeId::new("world_model.graph_replay").unwrap();
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
            vec!["world_model.graph_replay"]
        );
        assert_eq!(
            report.restart_evaluation.restarted_runtime_ids,
            vec!["world_model.graph_replay"]
        );
        assert!(report.renewed_runtime_ids.is_empty());
        assert_ne!(active.lease_id, old_lease.lease_id);
        assert!(
            runtime_status(
                &supervisor.status_snapshot(111).unwrap(),
                "world_model.graph_replay"
            )
            .handle_started
        );
    }

    fn stage_name_to_debug_name(stage_name: &str) -> &str {
        match stage_name {
            "stop" => "StopOldHandle",
            "safe" => "AwaitOldSafePoint",
            "flush" => "FlushOldHandle",
            "release" => "ReleaseOldLease",
            "acquire" => "AcquireReplacementLease",
            "start" => "StartReplacementHandle",
            other => panic!("unknown replacement stage {other}"),
        }
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
