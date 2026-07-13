//! Runtime CLI tooling adapter.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::cli::RuntimeCommands;
use crate::error::ApiError;
use crate::runtime::assembly::{DesiredRuntimeState, ProductRuntimeAssembly};
use crate::runtime::contracts::{
    RuntimeActionRecord, RuntimeHandleKind, RuntimeImplementationState, RuntimeLaunchStatus,
    RuntimeRoleClass, RuntimeRunMode, RuntimeStatusCacheLayout, RuntimeStatusCacheRecord,
    RuntimeStatusHealthCounts, RuntimeStatusHealthSummary, RuntimeStatusHeartbeatSummary,
    RuntimeStatusInstanceSummary, RuntimeStatusLeaseSummary, RuntimeStatusLedgerSummary,
    RuntimeStatusProcessSummary, RuntimeStatusPublisher, RuntimeStatusRuntimeRow,
    RuntimeStatusShutdownSummary, RuntimeStatusSnapshot, RuntimeStatusWriterIdentity,
    RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT,
};
use crate::runtime::presentation::{
    format_runtime_activation_description, format_runtime_run_result, format_runtime_status,
};
use crate::runtime::status_cache::FilesystemRuntimeStatusPublisher;
use crate::runtime::supervisor::{
    RestartCause, RestartPolicy, RuntimeHealthStatus, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLeaseStatus, RuntimeSupervisor, SupervisorLifecycleEventType,
    SupervisorRuntimeError, SupervisorRuntimeStatus, SupervisorStartCommand,
    SupervisorStatusSnapshot, SupervisorStore, SupervisorTickReport,
};

static CTRL_C_TARGET: OnceLock<Mutex<Option<Weak<AtomicBool>>>> = OnceLock::new();
static CTRL_C_HANDLER_RESULT: OnceLock<Result<(), String>> = OnceLock::new();

/// Validate one explicit activation before product stores or `RunContext` open.
pub fn handle_cli_activation(
    workspace_root: &std::path::Path,
    activation_path: &std::path::Path,
    dry_run: bool,
    format: &str,
) -> Result<String, ApiError> {
    if !dry_run {
        return Err(ApiError::ConfigError(
            "runtime activation apply is unavailable until the bootstrap runtime is installed; use --dry-run"
                .to_string(),
        ));
    }
    let validated =
        crate::runtime::activation::load_and_validate_activation(workspace_root, activation_path)
            .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    format_runtime_activation_description(&validated.passive_description(), format)
}

/// CLI status DTO for runtime supervisor commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeCliStatus {
    /// Product runtime storage root.
    pub product_root: PathBuf,
    /// Supervisor store path.
    pub supervisor_store_path: PathBuf,
    /// Latest supervisor instance when present.
    pub instance: Option<RuntimeCliInstanceStatus>,
    /// Runtime status rows.
    pub runtimes: Vec<RuntimeCliRuntimeStatus>,
}

/// CLI status DTO for one supervisor instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeCliInstanceStatus {
    /// Supervisor instance id.
    pub instance_id: String,
    /// Product root recorded by the instance.
    pub product_root: PathBuf,
    /// Process start time in milliseconds.
    pub started_at_ms: u64,
    /// Process stop time in milliseconds when known.
    pub stopped_at_ms: Option<u64>,
    /// Current instance status.
    pub status: RuntimeInstanceStatus,
}

/// CLI status DTO for one runtime id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeCliRuntimeStatus {
    /// Stable runtime id.
    pub runtime_id: String,
    /// Whether desired state asks the supervisor to start the runtime.
    pub desired_enabled: bool,
    /// Whether assembly provided a factory for the runtime.
    pub factory_available: bool,
    /// Canonical lifecycle shape for this role.
    pub role_class: RuntimeRoleClass,
    /// Honest implementation posture for this desired row.
    pub implementation_state: RuntimeImplementationState,
    /// Whether this command has a process-local started handle.
    pub handle_started: bool,
    /// Active lease id when present.
    pub active_lease_id: Option<String>,
    /// Supervisor instance that owns the active lease.
    pub active_owner_instance_id: Option<String>,
    /// Active lease status when present.
    pub lease_status: Option<RuntimeLeaseStatus>,
    /// Active lease expiry time when present.
    pub lease_expires_at_ms: Option<u64>,
    /// Last heartbeat time when present.
    pub last_heartbeat_at_ms: Option<u64>,
    /// Age of the last heartbeat at status time.
    pub heartbeat_age_ms: Option<u64>,
    /// Derived health status.
    pub health_status: RuntimeHealthStatus,
    /// Restart attempts visible in the latest health snapshot.
    pub restart_count: u64,
    /// Last restart cause when present.
    pub last_restart_cause: Option<RestartCause>,
    /// Retryable issue count from latest health.
    pub retryable_error_count: u64,
    /// Fatal issue count from latest health.
    pub fatal_error_count: u64,
    /// Whether the latest diagnostic exhausted its budget.
    pub budget_exhausted: bool,
    /// Actor id from latest diagnostic summary.
    pub last_diagnostic_actor_id: Option<String>,
    /// Latest lifecycle event type for this runtime.
    pub last_lifecycle_event: Option<SupervisorLifecycleEventType>,
}

/// CLI result DTO for a completed foreground runtime run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeCliRunResult {
    /// Supervisor instance id.
    pub instance_id: String,
    /// Product runtime storage root.
    pub product_root: PathBuf,
    /// Supervisor store path.
    pub supervisor_store_path: PathBuf,
    /// Number of handles started by the supervisor.
    pub started_runtime_count: usize,
    /// Number of handles stopped by shutdown.
    pub stopped_runtime_count: usize,
    /// Number of maintenance ticks completed.
    pub tick_count: u64,
    /// Wall-clock run duration in milliseconds.
    pub duration_ms: u128,
    /// Shutdown id written by the supervisor.
    pub shutdown_id: String,
}

/// Dispatch one runtime CLI command.
pub fn handle_cli_command(
    assembly: &ProductRuntimeAssembly,
    command: &RuntimeCommands,
) -> Result<String, ApiError> {
    match command {
        RuntimeCommands::Status {
            format,
            runtime_ids,
        } => runtime_status(assembly, format, runtime_ids),
        RuntimeCommands::Activate { .. } => Err(ApiError::ConfigError(
            "runtime activate must be routed before product stores open".to_string(),
        )),
        RuntimeCommands::Run {
            instance_id,
            tick_ms,
            duration_ms,
            format,
            restart_policy,
            restart_attempt_limit,
            restart_backoff_ms,
        } => runtime_run(
            assembly,
            RuntimeRunOptions {
                instance_id: instance_id.clone(),
                tick_ms: *tick_ms,
                duration_ms: *duration_ms,
                format,
                restart_policy,
                restart_attempt_limit: *restart_attempt_limit,
                restart_backoff_ms: *restart_backoff_ms,
            },
        ),
    }
}

struct RuntimeRunOptions<'a> {
    instance_id: Option<String>,
    tick_ms: u64,
    duration_ms: Option<u64>,
    format: &'a str,
    restart_policy: &'a str,
    restart_attempt_limit: u64,
    restart_backoff_ms: u64,
}

struct RuntimeStatusCacheSession<'a> {
    assembly: &'a ProductRuntimeAssembly,
    publisher: FilesystemRuntimeStatusPublisher,
    recent_actions: Vec<RuntimeActionRecord>,
    started_at_ms: u64,
}

impl<'a> RuntimeStatusCacheSession<'a> {
    fn acquire(assembly: &'a ProductRuntimeAssembly, started_at_ms: u64) -> Result<Self, ApiError> {
        Ok(Self {
            publisher: FilesystemRuntimeStatusPublisher::acquire(assembly.product_root())
                .map_err(runtime_error)?,
            assembly,
            recent_actions: Vec::new(),
            started_at_ms,
        })
    }

    fn publish_startup(
        &mut self,
        status: &SupervisorStatusSnapshot,
        now_ms: u64,
    ) -> Result<(), ApiError> {
        let record = self.record(status, now_ms, RuntimeLaunchStatus::Ready, None)?;
        self.publisher
            .publish_startup_snapshot(&record)
            .map_err(runtime_error)
    }

    fn publish_tick(
        &mut self,
        status: &SupervisorStatusSnapshot,
        report: &SupervisorTickReport,
        now_ms: u64,
    ) -> Result<(), ApiError> {
        for action in &report.actions {
            self.publisher
                .publish_action(action)
                .map_err(runtime_error)?;
            self.recent_actions.push(action.clone());
        }
        if self.recent_actions.len() > RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT {
            let remove = self.recent_actions.len() - RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT;
            self.recent_actions.drain(..remove);
        }
        let record = self.record(status, now_ms, RuntimeLaunchStatus::Ready, None)?;
        if report.restart_evaluation.expired_runtime_ids.is_empty()
            && report.restart_evaluation.restarted_runtime_ids.is_empty()
        {
            self.publisher
                .publish_tick_snapshot(&record)
                .map_err(runtime_error)
        } else {
            self.publisher
                .publish_restart_snapshot(&record)
                .map_err(runtime_error)
        }
    }

    fn publish_shutdown(
        &mut self,
        status: &SupervisorStatusSnapshot,
        shutdown_id: &str,
        now_ms: u64,
    ) -> Result<(), ApiError> {
        let record = self.record(
            status,
            now_ms,
            RuntimeLaunchStatus::Stopped,
            Some(shutdown_id),
        )?;
        self.publisher
            .publish_shutdown_snapshot(&record)
            .map_err(runtime_error)?;
        self.publisher.flush_cache().map_err(runtime_error)
    }

    fn record(
        &self,
        status: &SupervisorStatusSnapshot,
        now_ms: u64,
        launch_status: RuntimeLaunchStatus,
        shutdown_id: Option<&str>,
    ) -> Result<RuntimeStatusCacheRecord, ApiError> {
        let snapshot = status_cache_snapshot(
            self.assembly,
            status,
            &self.recent_actions,
            self.started_at_ms,
            launch_status,
            shutdown_id,
            now_ms,
        )?;
        let layout = RuntimeStatusCacheLayout::from_product_root(self.assembly.product_root());
        Ok(RuntimeStatusCacheRecord::new(
            self.assembly.product_root(),
            self.assembly.supervisor_store().path(),
            layout.root,
            RuntimeStatusWriterIdentity {
                instance_id: Some(status.instance_id.clone()),
                process_id: Some(std::process::id()),
                parent_process_id: None,
                run_mode: RuntimeRunMode::Foreground,
                launch_status,
            },
            snapshot,
            self.recent_actions.clone(),
            now_ms,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn status_cache_snapshot(
    assembly: &ProductRuntimeAssembly,
    status: &SupervisorStatusSnapshot,
    recent_actions: &[RuntimeActionRecord],
    started_at_ms: u64,
    launch_status: RuntimeLaunchStatus,
    shutdown_id: Option<&str>,
    now_ms: u64,
) -> Result<RuntimeStatusSnapshot, ApiError> {
    let runtimes = status
        .runtimes
        .iter()
        .map(|runtime| status_cache_runtime_row(runtime, recent_actions))
        .collect::<Vec<_>>();
    let health_counts = status_cache_health_counts(&runtimes);
    let shutdown = shutdown_id
        .map(|shutdown_id| {
            assembly
                .supervisor_store()
                .get_shutdown_state(shutdown_id)
                .map_err(runtime_error)
                .map(|record| {
                    record.map(|record| RuntimeStatusShutdownSummary {
                        shutdown_id: Some(record.shutdown_id),
                        status: enum_name(record.status),
                        requested_at_ms: Some(record.requested_at_ms),
                        completed_at_ms: record.completed_at_ms,
                    })
                })
        })
        .transpose()?
        .flatten();
    let ledger = assembly
        .ports()
        .event_append()
        .health()
        .map(|report| RuntimeStatusLedgerSummary::from_health(&report))
        .map_err(runtime_error)?;

    Ok(RuntimeStatusSnapshot {
        instance: Some(RuntimeStatusInstanceSummary {
            instance_id: status.instance_id.clone(),
            status: enum_name(status.instance_status),
            started_at_ms,
            stopped_at_ms: if status.instance_status == RuntimeInstanceStatus::Stopped {
                Some(now_ms)
            } else {
                None
            },
        }),
        process: Some(RuntimeStatusProcessSummary {
            process_id: Some(std::process::id()),
            parent_process_id: None,
            run_mode: RuntimeRunMode::Foreground,
            launch_status,
            started_at_ms: Some(started_at_ms),
            ready_at_ms: Some(started_at_ms),
            log_path: None,
        }),
        shutdown,
        runtimes,
        health_counts,
        ledger: Some(ledger),
        warnings: Vec::new(),
    })
}

fn status_cache_runtime_row(
    runtime: &SupervisorRuntimeStatus,
    recent_actions: &[RuntimeActionRecord],
) -> RuntimeStatusRuntimeRow {
    let last_action = recent_actions
        .iter()
        .rev()
        .find(|action| action.runtime_id == runtime.runtime_id)
        .cloned();
    let last_progress = last_action
        .as_ref()
        .and_then(|action| action.checkpoints.last().cloned());
    RuntimeStatusRuntimeRow {
        runtime_id: runtime.runtime_id.clone(),
        desired_enabled: runtime.desired_enabled,
        factory_available: runtime.factory_available,
        role_class: runtime.role_class,
        implementation_state: runtime.implementation_state,
        handle_kind: match runtime.implementation_state {
            RuntimeImplementationState::Concrete if runtime.factory_available => {
                RuntimeHandleKind::Concrete
            }
            RuntimeImplementationState::Inert => RuntimeHandleKind::Inert,
            RuntimeImplementationState::Unknown => RuntimeHandleKind::Unknown,
            RuntimeImplementationState::Unavailable | RuntimeImplementationState::Concrete => {
                RuntimeHandleKind::Unavailable
            }
        },
        lease: runtime
            .active_lease_id
            .as_ref()
            .zip(runtime.lease_status)
            .zip(runtime.lease_expires_at_ms)
            .map(
                |((lease_id, status), expires_at_ms)| RuntimeStatusLeaseSummary {
                    lease_id: lease_id.clone(),
                    owner_instance_id: runtime.active_owner_instance_id.clone(),
                    status: enum_name(status),
                    expires_at_ms,
                },
            ),
        heartbeat: runtime
            .last_heartbeat_at_ms
            .zip(runtime.heartbeat_age_ms)
            .map(|(observed_at_ms, age_ms)| RuntimeStatusHeartbeatSummary {
                observed_at_ms,
                age_ms,
            }),
        health: RuntimeStatusHealthSummary {
            status: enum_name(runtime.health_status),
            retryable_error_count: runtime.retryable_error_count,
            fatal_error_count: runtime.fatal_error_count,
            budget_exhausted: runtime.budget_exhausted,
        },
        restart_count: runtime.restart_count,
        last_restart_cause: runtime.last_restart_cause.as_ref().map(enum_name),
        last_lifecycle_event: runtime.last_lifecycle_event.as_ref().map(enum_name),
        last_action,
        last_progress,
    }
}

fn status_cache_health_counts(rows: &[RuntimeStatusRuntimeRow]) -> RuntimeStatusHealthCounts {
    let mut counts = RuntimeStatusHealthCounts {
        unknown: 0,
        starting: 0,
        healthy: 0,
        degraded: 0,
        unhealthy: 0,
        stopped: 0,
    };
    for row in rows {
        match row.health.status.as_str() {
            "starting" => counts.starting += 1,
            "healthy" => counts.healthy += 1,
            "degraded" => counts.degraded += 1,
            "unhealthy" => counts.unhealthy += 1,
            "stopped" => counts.stopped += 1,
            _ => counts.unknown += 1,
        }
    }
    counts
}

fn enum_name(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn runtime_status(
    assembly: &ProductRuntimeAssembly,
    format: &str,
    runtime_ids: &[String],
) -> Result<String, ApiError> {
    validate_format(format)?;
    let canonical_runtime_ids = runtime_ids
        .iter()
        .map(|runtime_id| {
            assembly
                .registry()
                .canonical_id(runtime_id)
                .unwrap_or(runtime_id)
                .to_string()
        })
        .collect::<Vec<_>>();
    let selected =
        select_desired_runtime_state(assembly.desired_runtime_state(), &canonical_runtime_ids)?;
    let store = assembly.supervisor_store();
    let now_ms = current_time_ms()?;
    let active_owner_ids = selected
        .iter()
        .map(|desired| {
            let runtime_id = RuntimeId::new(desired.runtime_id.clone()).map_err(runtime_error)?;
            store
                .get_active_runtime_lease(&runtime_id)
                .map_err(runtime_error)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter(|lease| lease.is_active_at(now_ms))
        .map(|lease| lease.instance_id)
        .collect::<BTreeSet<_>>();
    let selected_runtime_ids = selected
        .iter()
        .map(|desired| desired.runtime_id.as_str())
        .collect::<BTreeSet<_>>();
    let historical_owner_id = if active_owner_ids.is_empty() {
        store
            .list_runtime_leases()
            .map_err(runtime_error)?
            .into_iter()
            .rev()
            .find(|lease| selected_runtime_ids.contains(lease.runtime_id.as_str()))
            .map(|lease| lease.instance_id)
    } else {
        None
    };
    let owner_instance_id = if active_owner_ids.len() == 1 {
        active_owner_ids.iter().next().cloned()
    } else {
        historical_owner_id
    };
    let instance = if let Some(owner_instance_id) = owner_instance_id {
        store
            .get_runtime_instance(&owner_instance_id)
            .map_err(runtime_error)?
    } else {
        store.latest_runtime_instance().map_err(runtime_error)?
    }
    .map(RuntimeCliInstanceStatus::from);
    store.list_desired_runtime_state().map_err(runtime_error)?;

    let runtimes = selected
        .iter()
        .map(|desired| runtime_status_row(desired, Some(store), now_ms))
        .collect::<Result<Vec<_>, _>>()?;
    let status = RuntimeCliStatus {
        product_root: assembly.product_root().to_path_buf(),
        supervisor_store_path: store.path().to_path_buf(),
        instance,
        runtimes,
    };
    format_runtime_status(&status, format)
}

fn runtime_run(
    assembly: &ProductRuntimeAssembly,
    options: RuntimeRunOptions<'_>,
) -> Result<String, ApiError> {
    validate_format(options.format)?;
    if options.tick_ms == 0 {
        return Err(runtime_message("tick-ms must be greater than 0"));
    }
    let restart_policy = parse_restart_policy(options.restart_policy)?;
    let started_at_ms = current_time_ms()?;
    let instance_id = options
        .instance_id
        .unwrap_or_else(|| format!("runtime-cli-{}-{}", std::process::id(), started_at_ms));

    let cancelled = Arc::new(AtomicBool::new(false));
    install_ctrl_c_handler(Arc::clone(&cancelled))?;
    let mut command = SupervisorStartCommand::new(instance_id.clone(), started_at_ms);
    command.default_restart_policy = restart_policy;
    command.restart_attempt_limit = options.restart_attempt_limit;
    command.restart_backoff_ms = options.restart_backoff_ms;

    let mut cache_session = RuntimeStatusCacheSession::acquire(assembly, started_at_ms)?;
    let mut supervisor = RuntimeSupervisor::start(assembly.supervisor_startup_package(), command)
        .map_err(runtime_error)?;
    let startup_result = supervisor
        .status_snapshot(started_at_ms)
        .map_err(runtime_error)
        .and_then(|status| {
            cache_session.publish_startup(&status, started_at_ms)?;
            Ok(status)
        });
    let started_runtime_count = startup_result
        .as_ref()
        .map(|status| {
            status
                .runtimes
                .iter()
                .filter(|runtime| runtime.handle_started)
                .count()
        })
        .unwrap_or(0);
    let started = Instant::now();
    let mut tick_count = 0;
    let mut last_supervisor_time_ms = started_at_ms;
    let mut watcher = crate::runtime::self_observation::SelfObservationWatcher::new(
        options.restart_attempt_limit,
    );
    let tick_result = if startup_result.is_ok() {
        run_tick_loop(
            &mut supervisor,
            &cancelled,
            options.tick_ms,
            options.duration_ms,
            &mut tick_count,
            started,
            &mut last_supervisor_time_ms,
            assembly.ports().event_append(),
            &mut watcher,
            &mut cache_session,
        )
    } else {
        Ok(())
    };
    let shutdown_at_ms = shutdown_time_ms(started_at_ms, started).max(last_supervisor_time_ms);
    let shutdown_result = supervisor.request_shutdown(shutdown_at_ms);
    let cache_shutdown_result = shutdown_result
        .as_ref()
        .map_err(|error| runtime_error(error.to_string()))
        .and_then(|shutdown| {
            let status = supervisor
                .status_snapshot(shutdown_at_ms)
                .map_err(runtime_error)?;
            cache_session.publish_shutdown(&status, &shutdown.shutdown_id, shutdown_at_ms)
        });
    clear_ctrl_c_target(&cancelled);

    if let Err(shutdown_error) = shutdown_result.as_ref() {
        let prior_error = startup_result
            .as_ref()
            .err()
            .map(ToString::to_string)
            .or_else(|| tick_result.as_ref().err().map(ToString::to_string));
        return Err(runtime_message(match prior_error {
            Some(prior_error) => format!(
                "shutdown failed after startup: {shutdown_error}; earlier runtime error: {prior_error}"
            ),
            None => format!("shutdown failed after startup: {shutdown_error}"),
        }));
    }
    startup_result?;
    tick_result?;
    let shutdown = shutdown_result.map_err(runtime_error)?;
    cache_shutdown_result?;
    let run_result = RuntimeCliRunResult {
        instance_id,
        product_root: assembly.product_root().to_path_buf(),
        supervisor_store_path: assembly.supervisor_store().path().to_path_buf(),
        started_runtime_count,
        stopped_runtime_count: shutdown.stopped_runtime_ids.len(),
        tick_count,
        duration_ms: started.elapsed().as_millis(),
        shutdown_id: shutdown.shutdown_id,
    };
    format_runtime_run_result(&run_result, options.format)
}

#[allow(clippy::too_many_arguments)]
fn run_tick_loop(
    supervisor: &mut RuntimeSupervisor<'_>,
    cancelled: &AtomicBool,
    tick_ms: u64,
    duration_ms: Option<u64>,
    tick_count: &mut u64,
    started: Instant,
    last_supervisor_time_ms: &mut u64,
    event_port: &crate::runtime::ports::ProductEventAppendPort,
    watcher: &mut crate::runtime::self_observation::SelfObservationWatcher,
    cache_session: &mut RuntimeStatusCacheSession<'_>,
) -> Result<(), ApiError> {
    loop {
        if cancelled.load(Ordering::SeqCst) || duration_elapsed(started, duration_ms) {
            break;
        }

        let now_ms = current_time_ms_for_supervisor().map_err(runtime_error)?;
        *last_supervisor_time_ms = (*last_supervisor_time_ms).max(now_ms);
        let report = supervisor.tick(now_ms).map_err(runtime_error)?;
        let status = supervisor.status_snapshot(now_ms).map_err(runtime_error)?;
        cache_session.publish_tick(&status, &report, now_ms)?;
        *tick_count += 1;

        // Promote threshold crossings after the tick; a failed health read
        // skips the observation rather than failing the loop.
        match event_port.health() {
            Ok(health) => {
                let restart_counts: Vec<(String, u64)> = match supervisor.status_snapshot(now_ms) {
                    Ok(snapshot) => snapshot
                        .runtimes
                        .iter()
                        .map(|row| (row.runtime_id.clone(), row.restart_count))
                        .collect(),
                    Err(error) => {
                        tracing::debug!(error = %error, "restart counts unavailable this tick");
                        Vec::new()
                    }
                };
                watcher.observe(
                    &health,
                    &restart_counts,
                    supervisor.instance_id(),
                    event_port,
                );
            }
            Err(error) => {
                tracing::debug!(error = %error, "health read skipped this tick");
            }
        }

        if cancelled.load(Ordering::SeqCst) || duration_elapsed(started, duration_ms) {
            break;
        }

        let sleep_ms = duration_ms
            .map(|limit| {
                let elapsed = started.elapsed().as_millis() as u64;
                limit.saturating_sub(elapsed).min(tick_ms)
            })
            .unwrap_or(tick_ms);
        if sleep_ms == 0 {
            break;
        }
        sleep_until_next_tick(cancelled, started, duration_ms, sleep_ms, event_port);
    }
    Ok(())
}

fn duration_elapsed(started: Instant, duration_ms: Option<u64>) -> bool {
    duration_ms.is_some_and(|limit| started.elapsed().as_millis() >= u128::from(limit))
}

/// Waits out the tick interval but wakes early when the ledger writer commits
/// new events, so work-driven ticks replace pure wall-clock polling while the
/// configured interval stays the fallback heartbeat.
fn sleep_until_next_tick(
    cancelled: &AtomicBool,
    started: Instant,
    duration_ms: Option<u64>,
    sleep_ms: u64,
    event_port: &crate::runtime::ports::ProductEventAppendPort,
) {
    // The baseline is captured after the tick on purpose: events committed
    // during the tick wait for the fallback interval instead of waking
    // immediately, because ticks emit their own telemetry through the writer
    // and a pre-tick baseline would self-wake into a spin.
    let Ok(baseline) = event_port.watermark() else {
        std::thread::sleep(Duration::from_millis(sleep_ms));
        return;
    };
    let cursor = meld_events::LedgerCursor {
        ledger_id: baseline.ledger_id,
        after_seq: baseline.committed_seq,
    };
    let mut remaining_ms = sleep_ms;
    while remaining_ms > 0
        && !cancelled.load(Ordering::SeqCst)
        && !duration_elapsed(started, duration_ms)
    {
        // Short chunks keep cancellation responsive while the condvar wait
        // keeps idle chunks free of scans and writes.
        let chunk_ms = remaining_ms.min(50);
        if let Ok(current) = event_port.wait_past(cursor, Duration::from_millis(chunk_ms)) {
            if current.committed_seq > cursor.after_seq {
                return;
            }
        }
        remaining_ms -= chunk_ms;
    }
}

fn runtime_status_row(
    desired: &DesiredRuntimeState,
    store: Option<&SupervisorStore>,
    now_ms: u64,
) -> Result<RuntimeCliRuntimeStatus, ApiError> {
    let runtime_id = RuntimeId::new(desired.runtime_id.clone()).map_err(runtime_error)?;
    let Some(store) = store else {
        return Ok(RuntimeCliRuntimeStatus {
            runtime_id: desired.runtime_id.clone(),
            desired_enabled: desired.enabled,
            factory_available: desired.factory_available,
            role_class: desired.role_class,
            implementation_state: desired.implementation_state,
            handle_started: false,
            active_lease_id: None,
            active_owner_instance_id: None,
            lease_status: None,
            lease_expires_at_ms: None,
            last_heartbeat_at_ms: None,
            heartbeat_age_ms: None,
            health_status: default_health_status(desired),
            restart_count: 0,
            last_restart_cause: None,
            retryable_error_count: 0,
            fatal_error_count: 0,
            budget_exhausted: false,
            last_diagnostic_actor_id: None,
            last_lifecycle_event: None,
        });
    };

    let active_lease = store
        .get_active_runtime_lease(&runtime_id)
        .map_err(runtime_error)?;
    let active_lease = active_lease.filter(|lease| lease.is_active_at(now_ms));
    let heartbeat = store
        .get_runtime_heartbeat(&runtime_id)
        .map_err(runtime_error)?
        .filter(|heartbeat| match active_lease.as_ref() {
            Some(lease) => {
                heartbeat.lease_id == lease.lease_id && heartbeat.instance_id == lease.instance_id
            }
            None => {
                desired_worker_eligible(desired)
                    && matches!(
                        heartbeat.health.status,
                        RuntimeHealthStatus::Stopped | RuntimeHealthStatus::Unhealthy
                    )
            }
        });
    let health = store
        .get_health_snapshot(&runtime_id)
        .map_err(runtime_error)?
        .filter(|health| match active_lease.as_ref() {
            Some(lease) => health.lease_id.as_deref() == Some(lease.lease_id.as_str()),
            None => {
                desired_worker_eligible(desired)
                    && matches!(
                        health.status,
                        RuntimeHealthStatus::Stopped | RuntimeHealthStatus::Unhealthy
                    )
            }
        });
    let event = store
        .latest_lifecycle_event_for_runtime(&runtime_id)
        .map_err(runtime_error)?;
    let last_heartbeat_at_ms = heartbeat.as_ref().map(|record| record.observed_at_ms);
    let heartbeat_age_ms = last_heartbeat_at_ms.map(|observed| now_ms.saturating_sub(observed));
    let health_status = health
        .as_ref()
        .map(|record| record.status)
        .or_else(|| heartbeat.as_ref().map(|record| record.health.status))
        .unwrap_or_else(|| default_health_status(desired));
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

    Ok(RuntimeCliRuntimeStatus {
        runtime_id: desired.runtime_id.clone(),
        desired_enabled: desired.enabled,
        factory_available: desired.factory_available,
        role_class: desired.role_class,
        implementation_state: desired.implementation_state,
        handle_started: false,
        active_lease_id: active_lease.as_ref().map(|record| record.lease_id.clone()),
        active_owner_instance_id: active_lease
            .as_ref()
            .map(|record| record.instance_id.clone()),
        lease_status: active_lease.as_ref().map(|record| record.status),
        lease_expires_at_ms: active_lease.as_ref().map(|record| record.expires_at_ms),
        last_heartbeat_at_ms,
        heartbeat_age_ms,
        health_status,
        restart_count: health
            .as_ref()
            .map(|record| record.restart_count)
            .unwrap_or(0),
        last_restart_cause: health
            .as_ref()
            .and_then(|record| record.last_restart_cause.clone()),
        retryable_error_count,
        fatal_error_count,
        budget_exhausted,
        last_diagnostic_actor_id: heartbeat
            .as_ref()
            .and_then(|record| record.diagnostic.as_ref())
            .map(|diagnostic| diagnostic.actor_id.clone()),
        last_lifecycle_event: event.map(|record| record.event_type),
    })
}

fn select_desired_runtime_state(
    desired: &[DesiredRuntimeState],
    runtime_ids: &[String],
) -> Result<Vec<DesiredRuntimeState>, ApiError> {
    if runtime_ids.is_empty() {
        return Ok(desired.to_vec());
    }

    let desired_by_id = desired
        .iter()
        .map(|state| (state.runtime_id.as_str(), state))
        .collect::<BTreeMap<_, _>>();
    let selected_ids = runtime_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut selected = Vec::with_capacity(selected_ids.len());
    for runtime_id in selected_ids {
        let state = desired_by_id
            .get(runtime_id.as_str())
            .ok_or_else(|| runtime_message(format!("unknown runtime id filter '{runtime_id}'")))?;
        selected.push((*state).clone());
    }
    Ok(selected)
}

fn default_health_status(desired: &DesiredRuntimeState) -> RuntimeHealthStatus {
    if !desired.enabled {
        RuntimeHealthStatus::Stopped
    } else if desired.implementation_state != RuntimeImplementationState::Concrete
        || !desired.factory_available
    {
        RuntimeHealthStatus::Unhealthy
    } else {
        RuntimeHealthStatus::Unknown
    }
}

fn desired_worker_eligible(desired: &DesiredRuntimeState) -> bool {
    desired.enabled
        && desired.factory_available
        && desired.role_class == RuntimeRoleClass::Actor
        && desired.implementation_state == RuntimeImplementationState::Concrete
}

fn parse_restart_policy(value: &str) -> Result<RestartPolicy, ApiError> {
    match value {
        "never" => Ok(RestartPolicy::Never),
        "on-retryable-failure" => Ok(RestartPolicy::OnRetryableFailure),
        "on-heartbeat-expiry" => Ok(RestartPolicy::OnHeartbeatExpiry),
        other => Err(runtime_message(format!(
            "unknown restart policy '{other}', expected 'never', 'on-retryable-failure', or 'on-heartbeat-expiry'"
        ))),
    }
}

fn validate_format(format: &str) -> Result<(), ApiError> {
    match format {
        "text" | "json" => Ok(()),
        other => Err(runtime_message(format!(
            "invalid format '{other}', expected 'text' or 'json'"
        ))),
    }
}

fn install_ctrl_c_handler(cancelled: Arc<AtomicBool>) -> Result<(), ApiError> {
    let target = CTRL_C_TARGET.get_or_init(|| Mutex::new(None));
    {
        let mut guard = target.lock().unwrap_or_else(|err| err.into_inner());
        *guard = Some(Arc::downgrade(&cancelled));
    }

    let result = CTRL_C_HANDLER_RESULT.get_or_init(|| {
        ctrlc::set_handler(|| {
            if let Some(target) = CTRL_C_TARGET.get() {
                let current = target.lock().unwrap_or_else(|err| err.into_inner()).clone();
                if let Some(cancelled) = current.and_then(|target| target.upgrade()) {
                    cancelled.store(true, Ordering::SeqCst);
                }
            }
        })
        .map_err(|err| err.to_string())
    });
    result.clone().map_err(runtime_error)
}

fn clear_ctrl_c_target(cancelled: &Arc<AtomicBool>) {
    let Some(target) = CTRL_C_TARGET.get() else {
        return;
    };
    let mut guard = target.lock().unwrap_or_else(|err| err.into_inner());
    let should_clear = guard
        .as_ref()
        .and_then(|target| target.upgrade())
        .is_some_and(|current| Arc::ptr_eq(&current, cancelled));
    if should_clear {
        *guard = None;
    }
}

fn current_time_ms() -> Result<u64, ApiError> {
    current_time_ms_for_supervisor().map_err(runtime_error)
}

fn current_time_ms_for_supervisor() -> Result<u64, SupervisorRuntimeError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| SupervisorRuntimeError::InvalidCommand(err.to_string()))?
        .as_millis();
    u64::try_from(millis)
        .map_err(|_| SupervisorRuntimeError::InvalidCommand("system time overflowed u64".into()))
}

fn shutdown_time_ms(started_at_ms: u64, started: Instant) -> u64 {
    let elapsed_ms = started.elapsed().as_millis();
    let elapsed_ms = u64::try_from(elapsed_ms).unwrap_or(u64::MAX);
    started_at_ms.saturating_add(elapsed_ms)
}

fn runtime_error(error: impl ToString) -> ApiError {
    runtime_message(error.to_string())
}

fn runtime_message(message: impl Into<String>) -> ApiError {
    ApiError::ConfigError(format!("Runtime command failed: {}", message.into()))
}

impl From<RuntimeInstance> for RuntimeCliInstanceStatus {
    fn from(instance: RuntimeInstance) -> Self {
        Self {
            instance_id: instance.instance_id,
            product_root: instance.product_root,
            started_at_ms: instance.started_at_ms,
            stopped_at_ms: instance.stopped_at_ms,
            status: instance.status,
        }
    }
}
