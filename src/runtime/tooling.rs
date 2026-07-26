//! Runtime CLI tooling adapter.
//!
//! Owner: root runtime foreground availability. This adapter keeps
//! `meld runtime run` a truthful foreground process: the supervisor stays
//! alive through work, quiescence, satisfaction, and wake; committed-event
//! notification is the wake transport with the tick interval as heartbeat
//! fallback; and every maintenance pass prints one structured account line
//! derived from the durable per-tick reports.
//!
//! Follow hook: the documented event-follow primitives for external
//! consumers are the event subscription poll
//! (`EventAuthority::subscription_capability`) and the durable watermark
//! wait ([`ProductEventAppendPort::watermark`] and
//! [`ProductEventAppendPort::wait_past`]) — the same transport
//! [`sleep_until_next_tick`] uses for work-driven wakeups.
//!
//! Invariants: account emission derives from records and never gates,
//! reorders, or fails semantic work; snapshot publishing is bounded to one
//! startup and one shutdown envelope per run.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::cli::RuntimeCommands;
use crate::error::ApiError;
use crate::runtime::assembly::{DesiredRuntimeState, ProductRuntimeAssembly};
use crate::runtime::contracts::{
    RuntimeActionIssueSeverity, RuntimeActionOutcome, RuntimeActionRecord, RuntimeHandleKind,
    RuntimeLaunchStatus, RuntimeRunMode, RuntimeStatusCacheRecord, RuntimeStatusHealthCounts,
    RuntimeStatusHealthSummary, RuntimeStatusHeartbeatSummary, RuntimeStatusInstanceSummary,
    RuntimeStatusLeaseSummary, RuntimeStatusLedgerSummary, RuntimeStatusProcessSummary,
    RuntimeStatusPublisher, RuntimeStatusReader, RuntimeStatusRuntimeRow,
    RuntimeStatusShutdownSummary, RuntimeStatusSnapshot, RuntimeStatusWriterIdentity,
};
use crate::runtime::presentation::{format_runtime_run_result, format_runtime_status};
use crate::runtime::registration::{RegistrationLifecycle, RegistrationSet};
use crate::runtime::supervisor::{
    RestartCause, RestartPolicy, RuntimeHealthStatus, RuntimeId, RuntimeInstance,
    RuntimeInstanceStatus, RuntimeLeaseStatus, RuntimeSupervisor, SupervisorLifecycleEventType,
    SupervisorReportStore, SupervisorRuntimeError, SupervisorRuntimeStatus, SupervisorStartCommand,
    SupervisorStatusSnapshot, SupervisorStore,
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

/// One structured foreground account for one supervisor maintenance pass.
///
/// Emission derives from the durable per-tick action records the supervisor
/// already preserved; the account never becomes semantic truth. A quiescent
/// pass still emits (with `quiescent: true` and no working actors), so the
/// account alone distinguishes a dead process (no line), a quiescent
/// runtime, and active work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeTickAccount {
    /// Stable account schema discriminator.
    #[serde(rename = "type")]
    pub kind: String,
    /// Maintenance pass ordinal for this foreground run, starting at 1.
    pub tick: u64,
    /// Supervisor time of the pass in milliseconds.
    pub at_ms: u64,
    /// Supervisor instance id.
    pub instance_id: String,
    /// True when every bounded invocation truthfully found no work.
    pub quiescent: bool,
    /// One entry per bounded actor invocation this pass.
    pub actors: Vec<RuntimeTickActorAccount>,
}

/// Per-actor slice of one foreground tick account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeTickActorAccount {
    /// Supervised runtime id.
    pub runtime_id: String,
    /// Truthful lifecycle projection after this pass.
    pub lifecycle: String,
    /// Operator-facing outcome of the bounded invocation.
    pub outcome: String,
    /// Durable input items attempted.
    pub items_attempted: u64,
    /// Durable business outputs committed.
    pub items_committed: u64,
    /// Checkpoint movement observed by the invocation.
    pub checkpoint: Option<RuntimeTickCheckpointAccount>,
    /// Bounded issue summaries from the invocation.
    pub issues: Vec<RuntimeTickIssueAccount>,
    /// True when the invocation consumed its bounded budget.
    pub budget_exhausted: bool,
}

/// Checkpoint movement rendered into one actor account entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeTickCheckpointAccount {
    /// Checkpoint name (output side).
    pub name: String,
    /// Checkpoint value before the invocation.
    pub input: u64,
    /// Checkpoint value after the invocation.
    pub output: u64,
}

/// Issue summary rendered into one actor account entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeTickIssueAccount {
    /// Issue severity: `retryable` or `fatal`.
    pub severity: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable issue message.
    pub message: String,
}

/// Dispatch one runtime CLI command, printing tick accounts to stdout.
pub fn handle_cli_command(
    assembly: &ProductRuntimeAssembly,
    command: &RuntimeCommands,
) -> Result<String, ApiError> {
    handle_cli_command_with_account_writer(assembly, command, &mut std::io::stdout())
}

/// Dispatch one runtime CLI command with an explicit tick-account writer.
///
/// The writer receives one structured line per foreground maintenance pass
/// — text by default, one JSON object per line under `--format json`. Tests
/// and embedding tools capture the account through this entry; the standard
/// CLI path writes to stdout.
pub fn handle_cli_command_with_account_writer(
    assembly: &ProductRuntimeAssembly,
    command: &RuntimeCommands,
    account_writer: &mut dyn Write,
) -> Result<String, ApiError> {
    match command {
        RuntimeCommands::Status {
            format,
            runtime_ids,
        } => runtime_status(assembly, format, runtime_ids),
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
            account_writer,
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
    assembly: &ProductRuntimeAssembly,
    format: &str,
    runtime_ids: &[String],
) -> Result<String, ApiError> {
    validate_format(format)?;
    let selected = select_desired_runtime_state(assembly.desired_runtime_state(), runtime_ids)?;
    let store = assembly.supervisor_store();
    let now_ms = current_time_ms()?;

    let instance = store
        .latest_runtime_instance()
        .map_err(runtime_error)?
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
    account_writer: &mut dyn Write,
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
    // The composed registration set reaches the supervisor so classification
    // follows the derived declaration, intersected to the operator's
    // runtime-id selection so the supervisor never rejects a registration
    // naming an absent desired runtime.
    command.registration_set = registration_set_for_supervisor(
        assembly.registration_set(),
        assembly.desired_runtime_state(),
    );

    let mut supervisor = RuntimeSupervisor::start(assembly.supervisor_startup_package(), command)
        .map_err(runtime_error)?;
    let startup_status = supervisor.status_snapshot(started_at_ms);
    // Startup snapshot: one bounded lifecycle envelope through the report
    // store's frozen publisher, observational only — a publish failure
    // never gates the run.
    let mut snapshot_publisher = open_snapshot_publisher(assembly);
    if let (Ok(status), Some(publisher)) = (startup_status.as_ref(), snapshot_publisher.as_mut()) {
        publish_lifecycle_snapshot(
            publisher,
            assembly,
            status,
            &instance_id,
            RuntimeLaunchStatus::Ready,
            None,
            started_at_ms,
            SnapshotPhase::Startup,
        );
    }
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
    let mut watcher = crate::runtime::self_observation::SelfObservationWatcher::new(
        options.restart_attempt_limit,
    );
    let mut account = TickAccountEmitter {
        writer: account_writer,
        json: options.format == "json",
        instance_id: instance_id.clone(),
    };
    let tick_result = run_tick_loop(
        &mut supervisor,
        &cancelled,
        options.tick_ms,
        options.duration_ms,
        &mut tick_count,
        started,
        &mut last_supervisor_time_ms,
        assembly.ports().event_append(),
        &mut watcher,
        &mut account,
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
    // Shutdown snapshot: the clean-stop envelope RuntimeStatusReader
    // consumers observe after the process exits.
    if let (Ok(status), Some(publisher)) = (
        supervisor.status_snapshot(shutdown_at_ms).as_ref(),
        snapshot_publisher.as_mut(),
    ) {
        publish_lifecycle_snapshot(
            publisher,
            assembly,
            status,
            &instance_id,
            RuntimeLaunchStatus::Stopped,
            Some(RuntimeStatusShutdownSummary {
                shutdown_id: Some(shutdown.shutdown_id.clone()),
                status: "stopped".to_string(),
                requested_at_ms: Some(shutdown_at_ms),
                completed_at_ms: Some(shutdown_at_ms),
            }),
            shutdown_at_ms,
            SnapshotPhase::Shutdown,
        );
    }
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

/// Intersect the composed registration set with the desired runtime state
/// through the registration domain contract.
fn registration_set_for_supervisor(
    registration_set: Option<&RegistrationSet>,
    desired: &[DesiredRuntimeState],
) -> Option<RegistrationSet> {
    registration_set?
        .intersect_desired_runtime_ids(desired.iter().map(|state| state.runtime_id.as_str()))
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
    account: &mut TickAccountEmitter<'_>,
) -> Result<(), SupervisorRuntimeError> {
    loop {
        if cancelled.load(Ordering::SeqCst) || duration_elapsed(started, duration_ms) {
            break;
        }

        let now_ms = current_time_ms_for_supervisor()?;
        *last_supervisor_time_ms = (*last_supervisor_time_ms).max(now_ms);
        let tick_report = supervisor.tick(now_ms)?;
        *tick_count += 1;

        // One post-tick snapshot serves the restart watcher and the account
        // lifecycle projection; a failed read skips both observations
        // rather than failing the loop.
        let status_snapshot = match supervisor.status_snapshot(now_ms) {
            Ok(snapshot) => Some(snapshot),
            Err(error) => {
                tracing::debug!(error = %error, "status snapshot unavailable this tick");
                None
            }
        };

        // Promote threshold crossings after the tick; a failed health read
        // skips the observation rather than failing the loop.
        match event_port.health() {
            Ok(health) => {
                let restart_counts: Vec<(String, u64)> = status_snapshot
                    .as_ref()
                    .map(|snapshot| {
                        snapshot
                            .runtimes
                            .iter()
                            .map(|row| (row.runtime_id.clone(), row.restart_count))
                            .collect()
                    })
                    .unwrap_or_default();
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

        // The structured per-tick account: emission derives from the same
        // durable action records the supervisor preserved, and a write
        // failure never gates or reorders semantic work.
        account.emit(
            *tick_count,
            now_ms,
            status_snapshot.as_ref(),
            &tick_report.actions,
        );

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

/// Streams tick accounts to one writer in the run's selected format.
struct TickAccountEmitter<'w> {
    writer: &'w mut dyn Write,
    json: bool,
    instance_id: String,
}

impl TickAccountEmitter<'_> {
    /// Emit one account line for one completed maintenance pass.
    ///
    /// Emission is observational: any serialization or write failure is
    /// logged and dropped so it can never gate semantic work.
    fn emit(
        &mut self,
        tick: u64,
        at_ms: u64,
        status: Option<&SupervisorStatusSnapshot>,
        actions: &[RuntimeActionRecord],
    ) {
        let account = build_tick_account(tick, at_ms, &self.instance_id, status, actions);
        let line = if self.json {
            crate::cli::render_runtime_tick_account_json(&account)
        } else {
            crate::cli::render_runtime_tick_account_text(&account)
        };
        if let Err(error) = writeln!(self.writer, "{line}") {
            tracing::debug!(error = %error, "tick account write skipped");
        }
    }
}

/// Build one tick account from durable action records and the post-tick
/// lifecycle projection.
fn build_tick_account(
    tick: u64,
    at_ms: u64,
    instance_id: &str,
    status: Option<&SupervisorStatusSnapshot>,
    actions: &[RuntimeActionRecord],
) -> RuntimeTickAccount {
    let lifecycles: BTreeMap<&str, RegistrationLifecycle> = status
        .map(|snapshot| {
            snapshot
                .runtimes
                .iter()
                .filter_map(|row| {
                    row.lifecycle
                        .map(|lifecycle| (row.runtime_id.as_str(), lifecycle))
                })
                .collect()
        })
        .unwrap_or_default();
    let actors: Vec<RuntimeTickActorAccount> = actions
        .iter()
        .map(|action| RuntimeTickActorAccount {
            runtime_id: action.runtime_id.clone(),
            lifecycle: lifecycles
                .get(action.runtime_id.as_str())
                .map(|lifecycle| lifecycle_text(*lifecycle))
                .unwrap_or("unknown")
                .to_string(),
            outcome: outcome_text(action.outcome).to_string(),
            items_attempted: action.metrics.attempted,
            items_committed: action.metrics.committed,
            checkpoint: action
                .checkpoints
                .first()
                .map(|checkpoint| RuntimeTickCheckpointAccount {
                    name: checkpoint.output_name.clone(),
                    input: checkpoint.input_value,
                    output: checkpoint.output_value,
                }),
            issues: action
                .issues
                .iter()
                .map(|issue| RuntimeTickIssueAccount {
                    severity: severity_text(issue.severity).to_string(),
                    code: issue.code.clone(),
                    message: issue.message.clone(),
                })
                .collect(),
            budget_exhausted: action.metrics.budget_exhausted,
        })
        .collect();
    // A pass is quiescent when every bounded invocation truthfully found no
    // work; a pass with no bound actors at all is likewise quiescent. Both
    // still emit, which is what separates quiescence from a dead process.
    let quiescent = actors.iter().all(|actor| actor.outcome == "no_work");
    RuntimeTickAccount {
        kind: "runtime_tick_account".to_string(),
        tick,
        at_ms,
        instance_id: instance_id.to_string(),
        quiescent,
        actors,
    }
}

fn lifecycle_text(lifecycle: RegistrationLifecycle) -> &'static str {
    match lifecycle {
        RegistrationLifecycle::UnresolvedRequiredBinding => "unresolved_required_binding",
        RegistrationLifecycle::Starting => "starting",
        RegistrationLifecycle::ActiveWorking => "active_working",
        RegistrationLifecycle::ActiveIdle => "active_idle",
        RegistrationLifecycle::Unhealthy => "unhealthy",
        RegistrationLifecycle::Stopped => "stopped",
    }
}

fn outcome_text(outcome: RuntimeActionOutcome) -> &'static str {
    match outcome {
        RuntimeActionOutcome::Started => "started",
        RuntimeActionOutcome::Succeeded => "succeeded",
        RuntimeActionOutcome::NoWork => "no_work",
        RuntimeActionOutcome::Duplicate => "duplicate",
        RuntimeActionOutcome::Rejected => "rejected",
        RuntimeActionOutcome::Blocked => "blocked",
        RuntimeActionOutcome::RetryableFailure => "retryable_failure",
        RuntimeActionOutcome::FatalFailure => "fatal_failure",
        RuntimeActionOutcome::Cancelled => "cancelled",
    }
}

fn severity_text(severity: RuntimeActionIssueSeverity) -> &'static str {
    match severity {
        RuntimeActionIssueSeverity::Retryable => "retryable",
        RuntimeActionIssueSeverity::Fatal => "fatal",
    }
}

/// Which lifecycle publisher hook one snapshot goes through.
enum SnapshotPhase {
    Startup,
    Shutdown,
}

/// Bounded recent-action window copied into each lifecycle snapshot.
const SNAPSHOT_RECENT_ACTION_LIMIT: usize = 32;

/// Open a snapshot publisher over the supervisor's report trees.
///
/// The publisher writes only the latest-snapshot key — never action records
/// — so it composes with the supervisor instance's own action publishing
/// inside the same process without a second writer of the same records.
fn open_snapshot_publisher(assembly: &ProductRuntimeAssembly) -> Option<SupervisorReportStore> {
    match SupervisorReportStore::open(assembly.supervisor_store()) {
        Ok(store) => Some(store),
        Err(error) => {
            tracing::warn!(error = %error, "lifecycle snapshot publisher unavailable");
            None
        }
    }
}

/// Publish one bounded lifecycle snapshot through the frozen publisher.
///
/// Observational only: failures are logged and never gate the run.
#[allow(clippy::too_many_arguments)]
fn publish_lifecycle_snapshot(
    publisher: &mut SupervisorReportStore,
    assembly: &ProductRuntimeAssembly,
    status: &SupervisorStatusSnapshot,
    instance_id: &str,
    launch_status: RuntimeLaunchStatus,
    shutdown: Option<RuntimeStatusShutdownSummary>,
    now_ms: u64,
    phase: SnapshotPhase,
) {
    let record = build_status_cache_record(
        publisher,
        assembly,
        status,
        instance_id,
        launch_status,
        shutdown,
        now_ms,
    );
    let published = match phase {
        SnapshotPhase::Startup => publisher.publish_startup_snapshot(&record),
        SnapshotPhase::Shutdown => publisher.publish_shutdown_snapshot(&record),
    }
    .and_then(|()| publisher.flush_cache());
    if let Err(error) = published {
        tracing::warn!(error = %error, "lifecycle snapshot publish skipped");
    }
}

/// Copy one supervisor status snapshot into the frozen cache record shape.
fn build_status_cache_record(
    reader: &SupervisorReportStore,
    assembly: &ProductRuntimeAssembly,
    status: &SupervisorStatusSnapshot,
    instance_id: &str,
    launch_status: RuntimeLaunchStatus,
    shutdown: Option<RuntimeStatusShutdownSummary>,
    now_ms: u64,
) -> RuntimeStatusCacheRecord {
    let mut health_counts = RuntimeStatusHealthCounts {
        unknown: 0,
        starting: 0,
        healthy: 0,
        degraded: 0,
        unhealthy: 0,
        stopped: 0,
    };
    let runtimes = status
        .runtimes
        .iter()
        .map(|row| {
            count_health(&mut health_counts, row.health_status);
            status_cache_row(reader, row)
        })
        .collect();
    // The durable instance record carries the start and stop times the
    // operational snapshot omits.
    let instance_record = assembly
        .supervisor_store()
        .latest_runtime_instance()
        .ok()
        .flatten();
    let snapshot = RuntimeStatusSnapshot {
        instance: Some(RuntimeStatusInstanceSummary {
            instance_id: status.instance_id.clone(),
            status: instance_status_text(status.instance_status).to_string(),
            started_at_ms: instance_record
                .as_ref()
                .map(|record| record.started_at_ms)
                .unwrap_or(0),
            stopped_at_ms: instance_record
                .as_ref()
                .and_then(|record| record.stopped_at_ms),
        }),
        process: Some(RuntimeStatusProcessSummary {
            process_id: Some(std::process::id()),
            parent_process_id: None,
            run_mode: RuntimeRunMode::Foreground,
            launch_status,
            started_at_ms: instance_record.as_ref().map(|record| record.started_at_ms),
            ready_at_ms: None,
            log_path: None,
        }),
        shutdown,
        runtimes,
        health_counts,
        ledger: assembly
            .ports()
            .event_append()
            .health()
            .ok()
            .map(|health| RuntimeStatusLedgerSummary::from_health(&health)),
        warnings: Vec::new(),
    };
    let recent_actions = reader
        .read_recent_actions(SNAPSHOT_RECENT_ACTION_LIMIT)
        .unwrap_or_default();
    RuntimeStatusCacheRecord::new(
        assembly.product_root(),
        assembly.supervisor_store().path(),
        // The snapshot trees live inside the supervisor store database; the
        // store path is the cache location.
        assembly.supervisor_store().path(),
        RuntimeStatusWriterIdentity {
            instance_id: Some(instance_id.to_string()),
            process_id: Some(std::process::id()),
            parent_process_id: None,
            run_mode: RuntimeRunMode::Foreground,
            launch_status,
        },
        snapshot,
        recent_actions,
        now_ms,
    )
}

fn status_cache_row(
    reader: &SupervisorReportStore,
    row: &SupervisorRuntimeStatus,
) -> RuntimeStatusRuntimeRow {
    let last_action = reader
        .latest_action_for_runtime(&row.runtime_id)
        .ok()
        .flatten();
    RuntimeStatusRuntimeRow {
        runtime_id: row.runtime_id.clone(),
        desired_enabled: row.desired_enabled,
        factory_available: row.factory_available,
        handle_kind: match row.lifecycle {
            None => RuntimeHandleKind::Inert,
            Some(RegistrationLifecycle::UnresolvedRequiredBinding) => {
                RuntimeHandleKind::Unavailable
            }
            Some(_) => RuntimeHandleKind::Concrete,
        },
        lease: row
            .active_lease_id
            .as_ref()
            .zip(row.lease_status)
            .zip(row.lease_expires_at_ms)
            .map(
                |((lease_id, status), expires_at_ms)| RuntimeStatusLeaseSummary {
                    lease_id: lease_id.clone(),
                    status: format!("{status:?}").to_lowercase(),
                    expires_at_ms,
                },
            ),
        heartbeat: row.last_heartbeat_at_ms.zip(row.heartbeat_age_ms).map(
            |(observed_at_ms, age_ms)| RuntimeStatusHeartbeatSummary {
                observed_at_ms,
                age_ms,
            },
        ),
        health: RuntimeStatusHealthSummary {
            status: health_status_text(row.health_status).to_string(),
            retryable_error_count: row.retryable_error_count,
            fatal_error_count: row.fatal_error_count,
            budget_exhausted: row.budget_exhausted,
        },
        restart_count: row.restart_count,
        last_restart_cause: row
            .last_restart_cause
            .as_ref()
            .map(|cause| format!("{cause:?}")),
        last_lifecycle_event: row.last_lifecycle_event.map(|event| format!("{event:?}")),
        last_progress: last_action
            .as_ref()
            .and_then(|action| action.checkpoints.first().cloned()),
        last_action,
    }
}

fn count_health(counts: &mut RuntimeStatusHealthCounts, status: RuntimeHealthStatus) {
    match status {
        RuntimeHealthStatus::Unknown => counts.unknown += 1,
        RuntimeHealthStatus::Starting => counts.starting += 1,
        RuntimeHealthStatus::Healthy => counts.healthy += 1,
        RuntimeHealthStatus::Degraded => counts.degraded += 1,
        RuntimeHealthStatus::Unhealthy => counts.unhealthy += 1,
        RuntimeHealthStatus::Stopped => counts.stopped += 1,
    }
}

fn instance_status_text(status: RuntimeInstanceStatus) -> &'static str {
    match status {
        RuntimeInstanceStatus::Starting => "starting",
        RuntimeInstanceStatus::Running => "running",
        RuntimeInstanceStatus::Stopping => "stopping",
        RuntimeInstanceStatus::Stopped => "stopped",
        RuntimeInstanceStatus::Stale => "stale",
        RuntimeInstanceStatus::Failed => "failed",
    }
}

fn health_status_text(status: RuntimeHealthStatus) -> &'static str {
    match status {
        RuntimeHealthStatus::Unknown => "unknown",
        RuntimeHealthStatus::Starting => "starting",
        RuntimeHealthStatus::Healthy => "healthy",
        RuntimeHealthStatus::Degraded => "degraded",
        RuntimeHealthStatus::Unhealthy => "unhealthy",
        RuntimeHealthStatus::Stopped => "stopped",
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

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::runtime::registration::{RegistrationKind, RuntimeRegistration};

    fn registration(runtime_id: &str, kind: RegistrationKind) -> RuntimeRegistration {
        RuntimeRegistration {
            registration_id: format!("test::{runtime_id}"),
            runtime_id: runtime_id.to_string(),
            kind,
            required_resources: Vec::new(),
        }
    }

    fn desired(runtime_id: &str, enabled: bool) -> DesiredRuntimeState {
        DesiredRuntimeState {
            runtime_id: runtime_id.to_string(),
            enabled,
            factory_available: true,
        }
    }

    fn run_command(instance_id: &str, format: &str, duration_ms: u64) -> RuntimeCommands {
        RuntimeCommands::Run {
            instance_id: Some(instance_id.to_string()),
            tick_ms: 1,
            duration_ms: Some(duration_ms),
            format: format.to_string(),
            restart_policy: "on-heartbeat-expiry".to_string(),
            restart_attempt_limit: 3,
            restart_backoff_ms: 0,
        }
    }

    #[test]
    fn registration_set_passes_through_unchanged_for_the_complete_selection() {
        let set = RegistrationSet {
            registrations: vec![
                registration("event.append", RegistrationKind::PassiveService),
                registration("world_model.graph_replay", RegistrationKind::ActiveActor),
            ],
        };
        let desired_state = vec![
            desired("event.append", true),
            desired("world_model.graph_replay", true),
        ];

        let passed = registration_set_for_supervisor(Some(&set), &desired_state).unwrap();

        assert_eq!(passed, set);
    }

    #[test]
    fn registration_set_intersects_to_the_operator_subset() {
        let set = RegistrationSet {
            registrations: vec![
                registration("event.append", RegistrationKind::PassiveService),
                registration("world_model.graph_replay", RegistrationKind::ActiveActor),
            ],
        };
        let desired_state = vec![desired("event.append", true)];

        let passed = registration_set_for_supervisor(Some(&set), &desired_state).unwrap();

        assert_eq!(passed.registrations.len(), 1);
        assert_eq!(passed.registrations[0].runtime_id, "event.append");
    }

    #[test]
    fn empty_intersection_omits_the_registration_set() {
        let set = RegistrationSet {
            registrations: vec![registration(
                "world_model.graph_replay",
                RegistrationKind::ActiveActor,
            )],
        };
        let desired_state = vec![desired("event.append", true)];

        assert!(registration_set_for_supervisor(Some(&set), &desired_state).is_none());
        assert!(registration_set_for_supervisor(None, &desired_state).is_none());
    }

    #[test]
    fn declared_passive_registration_reaches_the_supervisor_on_a_default_run() {
        let temp = tempfile::tempdir().unwrap();
        // The plain (non-stewardship) composition binds semantic bodies for
        // the event observer and graph replay. Declaring the observer a
        // passive service through the composed registration set must stop
        // the supervisor from leasing and starting it: exactly one started
        // handle proves the derived set reached the supervisor.
        let mut config = crate::runtime::assembly::ProductRuntimeConfig::for_product_root(
            temp.path().join("product"),
        );
        config.registration_set = Some(RegistrationSet {
            registrations: vec![
                registration("event.append", RegistrationKind::PassiveService),
                registration("world_model.graph_replay", RegistrationKind::ActiveActor),
            ],
        });
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut sink = Vec::new();

        let output = handle_cli_command_with_account_writer(
            &assembly,
            &run_command("tooling-default-set", "json", 5),
            &mut sink,
        )
        .unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["started_runtime_count"], 1);
    }

    #[test]
    fn operator_subset_run_survives_a_wider_derived_registration_set() {
        let temp = tempfile::tempdir().unwrap();
        // The derived set names runtime ids outside the operator's
        // enabled-runtime-id subset; without the intersection the
        // supervisor start command would be rejected for referencing an
        // absent desired runtime.
        let mut config = crate::runtime::assembly::ProductRuntimeConfig::for_product_root(
            temp.path().join("product"),
        );
        config.enabled_runtime_ids = vec!["event.append".to_string()];
        config.disabled_runtime_ids = Vec::new();
        config.registration_set = Some(RegistrationSet {
            registrations: vec![
                registration("event.append", RegistrationKind::PassiveService),
                registration("world_model.graph_replay", RegistrationKind::ActiveActor),
                registration("execution.publication", RegistrationKind::ActiveActor),
            ],
        });
        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let mut sink = Vec::new();

        let output = handle_cli_command_with_account_writer(
            &assembly,
            &run_command("tooling-subset-set", "json", 5),
            &mut sink,
        )
        .unwrap();

        let parsed: Value = serde_json::from_str(&output).unwrap();
        // The one desired runtime is a declared passive service, so nothing
        // is leased or started, and the run still completes cleanly.
        assert_eq!(parsed["started_runtime_count"], 0);
    }

    #[test]
    fn foreground_run_emits_quiescent_accounts_and_durable_lifecycle_snapshots() {
        let temp = tempfile::tempdir().unwrap();
        let assembly =
            ProductRuntimeAssembly::load_for_product_root(temp.path().join("product")).unwrap();
        let mut sink = Vec::new();

        let output = handle_cli_command_with_account_writer(
            &assembly,
            &run_command("tooling-account", "json", 20),
            &mut sink,
        )
        .unwrap();

        let result: Value = serde_json::from_str(&output).unwrap();
        let lines = String::from_utf8(sink).unwrap();
        let accounts: Vec<Value> = lines
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(!accounts.is_empty(), "each maintenance pass must emit");
        for (index, account) in accounts.iter().enumerate() {
            assert_eq!(account["type"], "runtime_tick_account");
            assert_eq!(account["tick"], (index + 1) as u64);
            assert_eq!(account["instance_id"], "tooling-account");
            // An empty product has no committed work: quiescent passes.
            assert_eq!(account["quiescent"], true);
        }

        // Shutdown snapshot is durable and readable through the frozen
        // reader contract after the run.
        let reader = SupervisorReportStore::open(assembly.supervisor_store()).unwrap();
        let latest = reader.read_latest_snapshot().unwrap().unwrap();
        assert_eq!(latest.writer.launch_status, RuntimeLaunchStatus::Stopped);
        assert_eq!(latest.writer.run_mode, RuntimeRunMode::Foreground);
        let shutdown = latest.snapshot.shutdown.expect("shutdown summary");
        assert_eq!(
            shutdown.shutdown_id.as_deref(),
            result["shutdown_id"].as_str()
        );
    }

    #[test]
    fn text_accounts_render_one_line_per_pass() {
        let temp = tempfile::tempdir().unwrap();
        let assembly =
            ProductRuntimeAssembly::load_for_product_root(temp.path().join("product")).unwrap();
        let mut sink = Vec::new();

        handle_cli_command_with_account_writer(
            &assembly,
            &run_command("tooling-text", "text", 10),
            &mut sink,
        )
        .unwrap();

        let lines = String::from_utf8(sink).unwrap();
        assert!(lines.lines().count() >= 1);
        for line in lines.lines() {
            assert!(line.starts_with("tick="), "unexpected account line: {line}");
            assert!(line.contains("pass="), "unexpected account line: {line}");
        }
    }

    #[test]
    fn startup_snapshot_publishes_a_ready_envelope_before_shutdown() {
        let temp = tempfile::tempdir().unwrap();
        let assembly =
            ProductRuntimeAssembly::load_for_product_root(temp.path().join("product")).unwrap();
        let command = SupervisorStartCommand::new("snapshot-phases", 100);
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let mut publisher = open_snapshot_publisher(&assembly).unwrap();

        let status = supervisor.status_snapshot(100).unwrap();
        publish_lifecycle_snapshot(
            &mut publisher,
            &assembly,
            &status,
            "snapshot-phases",
            RuntimeLaunchStatus::Ready,
            None,
            100,
            SnapshotPhase::Startup,
        );
        let startup = publisher.read_latest_snapshot().unwrap().unwrap();
        assert_eq!(startup.writer.launch_status, RuntimeLaunchStatus::Ready);
        assert!(startup.snapshot.shutdown.is_none());
        assert!(startup.snapshot.instance.is_some());

        let shutdown_report = supervisor.request_shutdown(200).unwrap();
        let status = supervisor.status_snapshot(200).unwrap();
        publish_lifecycle_snapshot(
            &mut publisher,
            &assembly,
            &status,
            "snapshot-phases",
            RuntimeLaunchStatus::Stopped,
            Some(RuntimeStatusShutdownSummary {
                shutdown_id: Some(shutdown_report.shutdown_id.clone()),
                status: "stopped".to_string(),
                requested_at_ms: Some(200),
                completed_at_ms: Some(200),
            }),
            200,
            SnapshotPhase::Shutdown,
        );

        let reader = SupervisorReportStore::open(assembly.supervisor_store()).unwrap();
        let latest = reader.read_latest_snapshot().unwrap().unwrap();
        assert_eq!(latest.writer.launch_status, RuntimeLaunchStatus::Stopped);
        assert_eq!(
            latest.snapshot.shutdown.unwrap().shutdown_id.as_deref(),
            Some(shutdown_report.shutdown_id.as_str())
        );
    }

    #[test]
    fn tick_account_distinguishes_work_from_quiescence() {
        let working = RuntimeActionRecord::from_worker_tick(
            "action-1",
            "world_model.graph_replay",
            10,
            crate::runtime::contracts::WorkerTickReport {
                actor_id: "world_model.graph_replay".to_string(),
                scope: crate::runtime::contracts::WorkerScope {
                    domain_id: "world_model".to_string(),
                    stream_id: None,
                    work_key: None,
                    agent_id: None,
                    perspective_key: None,
                    branch_id: None,
                    subject_key: None,
                },
                input_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                    name: "event_commit_watermark".to_string(),
                    value: 0,
                },
                output_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                    name: "event_commit_watermark".to_string(),
                    value: 4,
                },
                items_attempted: 4,
                items_committed: 4,
                retryable_errors: Vec::new(),
                fatal_errors: Vec::new(),
                budget_exhausted: false,
            },
        );

        let idle = RuntimeActionRecord::from_worker_tick(
            "action-2",
            "world_model.graph_replay",
            20,
            crate::runtime::contracts::WorkerTickReport {
                actor_id: "world_model.graph_replay".to_string(),
                scope: crate::runtime::contracts::WorkerScope {
                    domain_id: "world_model".to_string(),
                    stream_id: None,
                    work_key: None,
                    agent_id: None,
                    perspective_key: None,
                    branch_id: None,
                    subject_key: None,
                },
                input_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                    name: "event_commit_watermark".to_string(),
                    value: 4,
                },
                output_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                    name: "event_commit_watermark".to_string(),
                    value: 4,
                },
                items_attempted: 0,
                items_committed: 0,
                retryable_errors: Vec::new(),
                fatal_errors: Vec::new(),
                budget_exhausted: false,
            },
        );

        let active = build_tick_account(1, 10, "instance-a", None, &[working]);
        assert!(!active.quiescent);
        assert_eq!(active.actors[0].outcome, "succeeded");
        assert_eq!(active.actors[0].items_committed, 4);
        assert_eq!(active.actors[0].checkpoint.as_ref().unwrap().output, 4);

        // A pass whose only invocation truthfully found no work is
        // quiescent, and a pass with no bound actors at all is quiescent —
        // both still emit, unlike a dead process.
        let idle_pass = build_tick_account(2, 20, "instance-a", None, &[idle]);
        assert!(idle_pass.quiescent);
        assert_eq!(idle_pass.actors[0].outcome, "no_work");

        let empty_pass = build_tick_account(3, 30, "instance-a", None, &[]);
        assert!(empty_pass.quiescent);
        assert!(empty_pass.actors.is_empty());
    }
}
