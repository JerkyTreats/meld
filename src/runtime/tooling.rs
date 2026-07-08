//! Runtime CLI tooling adapter.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::cli::RuntimeCommands;
use crate::config::{ConfigLoader, MerkleConfig};
use crate::error::ApiError;
use crate::runtime::assembly::{DesiredRuntimeState, ProductRuntimeAssembly};
use crate::runtime::presentation::{format_runtime_run_result, format_runtime_status};
use crate::runtime::supervisor::{
    RestartCause, RestartPolicy, RuntimeHealthStatus, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLeaseStatus, RuntimeSupervisor, SupervisorLifecycleEventType,
    SupervisorRuntimeError, SupervisorStartCommand, SupervisorStore,
};

static CTRL_C_TARGET: OnceLock<Mutex<Option<Weak<AtomicBool>>>> = OnceLock::new();
static CTRL_C_HANDLER_RESULT: OnceLock<Result<(), String>> = OnceLock::new();

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
    /// Whether this command has a process-local started handle.
    pub handle_started: bool,
    /// Active lease id when present.
    pub active_lease_id: Option<String>,
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
    workspace_root: &Path,
    config_path: Option<&Path>,
    command: &RuntimeCommands,
) -> Result<String, ApiError> {
    match command {
        RuntimeCommands::Status {
            format,
            runtime_ids,
        } => runtime_status(workspace_root, config_path, format, runtime_ids),
        RuntimeCommands::Run {
            instance_id,
            tick_ms,
            duration_ms,
            format,
            restart_policy,
            restart_attempt_limit,
            restart_backoff_ms,
        } => runtime_run(
            workspace_root,
            config_path,
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

fn runtime_status(
    workspace_root: &Path,
    config_path: Option<&Path>,
    format: &str,
    runtime_ids: &[String],
) -> Result<String, ApiError> {
    validate_format(format)?;
    let config = load_config(workspace_root, config_path)?;
    let description = ProductRuntimeAssembly::describe_for_workspace(workspace_root, &config)
        .map_err(runtime_error)?;
    let selected = select_desired_runtime_state(&description.desired_runtime_state, runtime_ids)?;
    let store = SupervisorStore::open_existing(description.supervisor_store_path.clone())
        .map_err(runtime_error)?;
    let now_ms = current_time_ms()?;

    let instance = match store.as_ref() {
        Some(store) => store
            .latest_runtime_instance()
            .map_err(runtime_error)?
            .map(RuntimeCliInstanceStatus::from),
        None => None,
    };
    if let Some(store) = store.as_ref() {
        store.list_desired_runtime_state().map_err(runtime_error)?;
    }

    let runtimes = selected
        .iter()
        .map(|desired| runtime_status_row(desired, store.as_ref(), now_ms))
        .collect::<Result<Vec<_>, _>>()?;
    let status = RuntimeCliStatus {
        product_root: description.product_root,
        supervisor_store_path: description.supervisor_store_path,
        instance,
        runtimes,
    };
    format_runtime_status(&status, format)
}

fn runtime_run(
    workspace_root: &Path,
    config_path: Option<&Path>,
    options: RuntimeRunOptions<'_>,
) -> Result<String, ApiError> {
    validate_format(options.format)?;
    if options.tick_ms == 0 {
        return Err(runtime_message("tick-ms must be greater than 0"));
    }
    let restart_policy = parse_restart_policy(options.restart_policy)?;
    let config = load_config(workspace_root, config_path)?;
    let started_at_ms = current_time_ms()?;
    let instance_id = options
        .instance_id
        .unwrap_or_else(|| format!("runtime-cli-{}-{}", std::process::id(), started_at_ms));

    let assembly = ProductRuntimeAssembly::load_for_workspace(workspace_root, &config)
        .map_err(runtime_error)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    install_ctrl_c_handler(Arc::clone(&cancelled))?;
    let mut command = SupervisorStartCommand::new(instance_id.clone(), started_at_ms);
    command.default_restart_policy = restart_policy;
    command.restart_attempt_limit = options.restart_attempt_limit;
    command.restart_backoff_ms = options.restart_backoff_ms;

    let mut supervisor = RuntimeSupervisor::start(assembly.supervisor_startup_package(), command)
        .map_err(runtime_error)?;
    let startup_status = supervisor.status_snapshot(started_at_ms);
    let started_runtime_count = startup_status
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
    let watermark = assembly.ports().event_append().watermark();
    let tick_result = run_tick_loop(
        &mut supervisor,
        &cancelled,
        options.tick_ms,
        options.duration_ms,
        &mut tick_count,
        started,
        &mut last_supervisor_time_ms,
        watermark.as_ref(),
    );
    let shutdown_at_ms = shutdown_time_ms(started_at_ms, started).max(last_supervisor_time_ms);
    let shutdown_result = supervisor.request_shutdown(shutdown_at_ms);
    clear_ctrl_c_target(&cancelled);

    if let Err(shutdown_error) = shutdown_result.as_ref() {
        let prior_error = startup_status
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
    startup_status.map_err(runtime_error)?;
    tick_result.map_err(runtime_error)?;
    let shutdown = shutdown_result.map_err(runtime_error)?;
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
    watermark: &meld_events::CommitWatermark,
) -> Result<(), SupervisorRuntimeError> {
    loop {
        if cancelled.load(Ordering::SeqCst) || duration_elapsed(started, duration_ms) {
            break;
        }

        let now_ms = current_time_ms_for_supervisor()?;
        *last_supervisor_time_ms = (*last_supervisor_time_ms).max(now_ms);
        supervisor.tick(now_ms)?;
        *tick_count += 1;

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
        sleep_until_next_tick(cancelled, started, duration_ms, sleep_ms, watermark);
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
    watermark: &meld_events::CommitWatermark,
) {
    // The baseline is captured after the tick on purpose: events committed
    // during the tick wait for the fallback interval instead of waking
    // immediately, because ticks emit their own telemetry through the writer
    // and a pre-tick baseline would self-wake into a spin.
    let baseline_seq = watermark.committed_seq();
    let mut remaining_ms = sleep_ms;
    while remaining_ms > 0
        && !cancelled.load(Ordering::SeqCst)
        && !duration_elapsed(started, duration_ms)
    {
        // Short chunks keep cancellation responsive while the condvar wait
        // keeps idle chunks free of scans and writes.
        let chunk_ms = remaining_ms.min(50);
        if watermark.wait_past(baseline_seq, Duration::from_millis(chunk_ms)) > baseline_seq {
            return;
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
            handle_started: false,
            active_lease_id: None,
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
    let heartbeat = store
        .get_runtime_heartbeat(&runtime_id)
        .map_err(runtime_error)?;
    let health = store
        .get_health_snapshot(&runtime_id)
        .map_err(runtime_error)?;
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
        handle_started: false,
        active_lease_id: active_lease.as_ref().map(|record| record.lease_id.clone()),
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
    } else if !desired.factory_available {
        RuntimeHealthStatus::Unhealthy
    } else {
        RuntimeHealthStatus::Unknown
    }
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

fn load_config(
    workspace_root: &Path,
    config_path: Option<&Path>,
) -> Result<MerkleConfig, ApiError> {
    if let Some(config_path) = config_path {
        Ok(ConfigLoader::load_from_file(config_path)?)
    } else {
        Ok(ConfigLoader::load(workspace_root)?)
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
