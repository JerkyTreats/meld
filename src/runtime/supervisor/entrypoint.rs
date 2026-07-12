//! Explicit root supervisor lifecycle entrypoint.

use std::collections::BTreeMap;
use std::path::PathBuf;

use thiserror::Error;

use crate::runtime::assembly::{
    DesiredRuntimeState, InertRuntimeHandle, RuntimeHandleFlushReport, RuntimeHandleStopReport,
    RuntimeLeaseContext, SupervisorStartupPackage,
};
use crate::runtime::contracts::{WorkBudget, WorkerTickReport};
use crate::runtime::error::RuntimeAssemblyError;

use super::contracts::{
    RestartCause, RestartPolicy, RuntimeDesiredState, RuntimeDiagnosticSummary, RuntimeHealth,
    RuntimeHealthSnapshot, RuntimeHealthStatus, RuntimeHeartbeat, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLease, RuntimeLeaseOwner, RuntimeRestartRecord,
    RuntimeShutdownState, RuntimeShutdownStatus, SupervisorContractError, SupervisorLifecycleEvent,
    SupervisorLifecycleEventType,
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
    /// Whether this process currently has a local started handle.
    pub handle_started: bool,
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
    event_sequence: u64,
    restart_attempt_limit: u64,
    restart_backoff_ms: u64,
    shutdown_completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SupervisorRuntimeDesired {
    runtime_id: RuntimeId,
    enabled: bool,
    factory_available: bool,
    restart_policy: RestartPolicy,
}

struct SupervisedRuntimeHandle {
    handle: InertRuntimeHandle,
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
            event_sequence: INITIAL_EVENT_SEQUENCE,
            restart_attempt_limit: command.restart_attempt_limit,
            restart_backoff_ms: command.restart_backoff_ms,
            shutdown_completed: false,
        };

        supervisor.register_instance()?;
        supervisor.persist_desired_state()?;
        supervisor.recover_expired_leases(command.started_at_ms)?;
        supervisor.start_enabled_runtimes(command.started_at_ms)?;
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
            } else if !desired.factory_available {
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

            runtimes.push(SupervisorRuntimeStatus {
                runtime_id: desired.runtime_id.to_string(),
                desired_enabled: desired.enabled,
                factory_available: desired.factory_available,
                handle_started: self
                    .handles
                    .get(desired.runtime_id.as_str())
                    .is_some_and(|runtime| runtime.handle.is_started()),
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

    /// Renew local leases, write healthy heartbeats, and evaluate restarts.
    pub fn tick(&mut self, now_ms: u64) -> Result<SupervisorTickReport, SupervisorRuntimeError> {
        let owners = self
            .handles
            .values()
            .map(|runtime| runtime.owner.clone())
            .collect::<Vec<_>>();
        let mut renewed_runtime_ids = Vec::new();
        let mut heartbeat_runtime_ids = Vec::new();

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

            let semantic_report = self
                .handles
                .get_mut(owner.runtime_id.as_str())
                .and_then(|runtime| runtime.handle.tick(self.default_work_budget.clone()));
            let health_status = health_status_from_tick_report(semantic_report.as_ref());

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
        }

        let restart_evaluation = self.evaluate_restart_policies(now_ms)?;
        self.supervisor_store.flush()?;
        Ok(SupervisorTickReport {
            renewed_runtime_ids,
            heartbeat_runtime_ids,
            restart_evaluation,
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

        let mut stop_reports = Vec::new();
        let mut flush_reports = Vec::new();
        for runtime in self.handles.values_mut() {
            stop_reports.push(runtime.handle.request_stop());
            let safe_point = runtime.handle.wait_for_safe_point();
            if !safe_point.safe_for_flush {
                return Err(SupervisorRuntimeError::InvalidCommand(format!(
                    "runtime '{}' did not reach a safe point",
                    safe_point.runtime_id
                )));
            }
            flush_reports.push(runtime.handle.flush_resources()?);
        }

        self.stores
            .flush_boundary()
            .map_err(RuntimeAssemblyError::from)?;

        let stopped = self.handles.keys().cloned().collect::<Vec<_>>();
        let owners = self
            .handles
            .values()
            .map(|runtime| runtime.owner.clone())
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
                Some(owner.lease_id),
                RuntimeHealthStatus::Stopped,
                now_ms,
                NO_RESTART_ATTEMPTS,
                None,
            )?;
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
        self.write_lifecycle_event(
            Some(runtime.clone()),
            None,
            now_ms,
            SupervisorLifecycleEventType::RuntimeStartRequested,
            Some("runtime start requested".to_string()),
        )?;

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

        if !desired.factory_available {
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
            Err(SupervisorStoreError::DuplicateActiveLease { .. }) => {
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
        self.write_runtime_heartbeat(&owner, RuntimeHealthStatus::Healthy, now_ms, None)?;
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
            RuntimeHealthStatus::Healthy,
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
        self.handles.remove(runtime_id);
        Ok(self
            .start_runtime(runtime_id, now_ms, attempt, Some(cause))?
            .is_some())
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
                restart_policy,
            },
        );
    }
    Ok(desired)
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

fn health_status_from_tick_report(report: Option<&WorkerTickReport>) -> RuntimeHealthStatus {
    let Some(report) = report else {
        return RuntimeHealthStatus::Healthy;
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
    use crate::runtime::assembly::{ProductRuntimeAssembly, ProductRuntimeConfig};
    use meld_events::{DomainObjectRef, EventEnvelope};
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
        assert_eq!(status.runtimes.len(), 12);
        assert_eq!(
            status
                .runtimes
                .iter()
                .filter(|runtime| runtime.desired_enabled && runtime.handle_started)
                .count(),
            11
        );
        let event_append = runtime_status(&status, "event.append");
        assert!(event_append.desired_enabled);
        assert!(event_append.factory_available);
        assert!(event_append.handle_started);
        assert_eq!(event_append.health_status, RuntimeHealthStatus::Healthy);
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
        assert_eq!(health.status, RuntimeHealthStatus::Healthy);
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
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        assembly
            .stores()
            .event_store
            .append_envelope(
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
