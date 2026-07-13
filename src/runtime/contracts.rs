//! Runtime worker diagnostic and operator visibility contracts.

use std::path::{Path, PathBuf};

use meld_execution::planning::PlanningRuntimeActorReport;
use meld_execution::task_network::{PublicationBridgeReport, PublicationRuntimeReport};
use meld_world_model::world_state::graph::runtime::GraphCatchUpReport;
use meld_world_model::AgentRuntimeReport;
use serde::{Deserialize, Serialize};

use crate::runtime::ports::DocsTaskEvidenceReplayReport;

/// Current schema version for runtime status cache records.
pub const RUNTIME_STATUS_CACHE_SCHEMA_VERSION: u16 = 1;

/// Oldest cache schema supported through additive defaults.
pub const RUNTIME_STATUS_CACHE_MIN_SUPPORTED_SCHEMA_VERSION: u16 = 0;

/// Stable warning code for a cache timestamp ahead of the reader clock.
pub const RUNTIME_STATUS_WARNING_CLOCK_SKEW: &str = "runtime_status_clock_skew";

/// Stable warning code for an older compatible schema.
pub const RUNTIME_STATUS_WARNING_OLDER_SCHEMA: &str = "runtime_status_older_schema";

/// Stable warning code for truncated cache data.
pub const RUNTIME_STATUS_WARNING_TRUNCATED: &str = "runtime_status_truncated";

/// Stable warning code for an unreadable cache projection.
pub const RUNTIME_STATUS_WARNING_UNREADABLE: &str = "runtime_status_unreadable";

/// Maximum encoded byte size for `latest.json`.
pub const RUNTIME_STATUS_LATEST_MAX_BYTES: usize = 1024 * 1024;

/// Maximum encoded byte size for `actions.jsonl`, including line endings.
pub const RUNTIME_STATUS_ACTIONS_MAX_BYTES: usize = 4 * 1024 * 1024;

/// Maximum retained action count.
pub const RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT: usize = 256;

/// Maximum issue summaries retained for one action.
pub const RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT: usize = 16;

/// Maximum UTF-8 byte size for one issue message.
pub const RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES: usize = 1024;

/// Stable filesystem layout for passive runtime status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStatusCacheLayout {
    /// Directory containing every cache projection.
    pub root: PathBuf,
    /// Atomically replaced latest status snapshot.
    pub latest: PathBuf,
    /// Bounded recent action log.
    pub actions: PathBuf,
    /// Process locator projection reserved for process hosting.
    pub process: PathBuf,
    /// Exclusive advisory lock held for the publisher lifetime.
    pub writer_lock: PathBuf,
}

impl RuntimeStatusCacheLayout {
    /// Derive the cache paths from one product root.
    pub fn from_product_root(product_root: impl AsRef<Path>) -> Self {
        let root = product_root.as_ref().join("runtime").join("status");
        Self {
            latest: root.join("latest.json"),
            actions: root.join("actions.jsonl"),
            process: root.join("process.json"),
            writer_lock: root.join("writer.lock"),
            root,
        }
    }

    /// Derive a unique sibling used before sync and atomic rename.
    pub fn temporary_path(target: &Path, process_id: u32, nonce: u64) -> PathBuf {
        let mut name = target.as_os_str().to_os_string();
        name.push(format!(".{process_id}-{nonce}.tmp"));
        PathBuf::from(name)
    }
}

/// Frozen cache bounds shared by publishers and readers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStatusCacheLimits {
    /// Maximum encoded latest snapshot bytes.
    pub latest_max_bytes: usize,
    /// Maximum encoded recent action file bytes.
    pub actions_max_bytes: usize,
    /// Maximum recent action records retained.
    pub recent_action_max_count: usize,
    /// Maximum issues retained per action.
    pub issue_max_count: usize,
    /// Maximum UTF-8 bytes retained per issue message.
    pub issue_message_max_bytes: usize,
}

/// Ordered durability steps required for cache file replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatusAtomicWriteStep {
    /// Serialize and enforce bounds before touching the destination.
    SerializeAndValidate,
    /// Create one unique sibling temporary file without overwriting another.
    CreateNewTemporaryFile,
    /// Write all bytes and synchronize the temporary file.
    WriteAndSyncTemporaryFile,
    /// Atomically rename the temporary file over the destination.
    RenameOverDestination,
    /// Synchronize the parent directory after rename.
    SyncParentDirectory,
}

/// Frozen atomic replacement protocol for cache publishers.
pub const RUNTIME_STATUS_ATOMIC_WRITE_PROTOCOL: &[RuntimeStatusAtomicWriteStep] = &[
    RuntimeStatusAtomicWriteStep::SerializeAndValidate,
    RuntimeStatusAtomicWriteStep::CreateNewTemporaryFile,
    RuntimeStatusAtomicWriteStep::WriteAndSyncTemporaryFile,
    RuntimeStatusAtomicWriteStep::RenameOverDestination,
    RuntimeStatusAtomicWriteStep::SyncParentDirectory,
];

impl Default for RuntimeStatusCacheLimits {
    fn default() -> Self {
        Self {
            latest_max_bytes: RUNTIME_STATUS_LATEST_MAX_BYTES,
            actions_max_bytes: RUNTIME_STATUS_ACTIONS_MAX_BYTES,
            recent_action_max_count: RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT,
            issue_max_count: RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT,
            issue_message_max_bytes: RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES,
        }
    }
}

/// Reader compatibility decision derived before full cache decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatusCacheCompatibility {
    /// Current schema can be decoded normally.
    Current,
    /// Older schema is decoded with additive defaults and a warning.
    Older,
    /// Future schema is not decoded and is reported as unsupported.
    UnsupportedFuture,
}

impl RuntimeStatusCacheCompatibility {
    /// Classify one observed schema version.
    pub fn for_schema_version(schema_version: u16) -> Self {
        match schema_version.cmp(&RUNTIME_STATUS_CACHE_SCHEMA_VERSION) {
            std::cmp::Ordering::Less => Self::Older,
            std::cmp::Ordering::Equal => Self::Current,
            std::cmp::Ordering::Greater => Self::UnsupportedFuture,
        }
    }
}

/// Lifecycle shape owned by one registered runtime role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRoleClass {
    /// Older cache data did not classify the role.
    #[default]
    Unknown,
    /// A supervised worker that advances domain-owned durable state.
    Actor,
    /// A hosted service whose availability does not imply worker progress.
    PassiveService,
    /// A callable port that does not own a supervised lifecycle.
    PortOnly,
}

/// Current implementation posture for one runtime role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeImplementationState {
    /// Older cache data did not record implementation posture.
    #[default]
    Unknown,
    /// The registered role has a concrete hosted implementation.
    Concrete,
    /// The role is declared but has no semantic implementation.
    Inert,
    /// A required factory or resource is unavailable.
    Unavailable,
}

/// Bounded work request shared by runtime supervisor adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkBudget {
    /// Maximum durable input items to attempt during one tick.
    pub max_items: usize,
}

/// Diagnostic scope for one worker tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerScope {
    /// Owning domain identifier.
    pub domain_id: String,
    /// Optional event stream or aggregate stream identifier.
    pub stream_id: Option<String>,
    /// Optional stable owner-specific unit of work.
    pub work_key: Option<String>,
    /// Optional world-model agent identifier.
    pub agent_id: Option<String>,
    /// Optional perspective identifier.
    pub perspective_key: Option<String>,
    /// Optional branch identifier.
    pub branch_id: Option<String>,
    /// Optional subject index key.
    pub subject_key: Option<String>,
}

/// Durable input or output checkpoint observed by a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerCheckpoint {
    /// Stable checkpoint name.
    pub name: String,
    /// Monotonic checkpoint value copied from the owning domain.
    pub value: u64,
}

/// Diagnostic issue observed during a worker tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerTickIssue {
    /// Optional domain item identifier.
    pub item_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Supervisor-facing report from one bounded worker tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerTickReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Diagnostic scope copied from the owning domain.
    pub scope: WorkerScope,
    /// Durable input cursor or revision before work.
    pub input_checkpoint: WorkerCheckpoint,
    /// Durable output cursor or revision after work.
    pub output_checkpoint: WorkerCheckpoint,
    /// Durable input items attempted.
    pub items_attempted: usize,
    /// Durable business outputs committed.
    pub items_committed: usize,
    /// Retryable diagnostics observed during the tick.
    pub retryable_errors: Vec<WorkerTickIssue>,
    /// Fatal diagnostics observed during the tick.
    pub fatal_errors: Vec<WorkerTickIssue>,
    /// True when the worker stopped because its budget was consumed.
    pub budget_exhausted: bool,
}

/// Full cache record written by one runtime status cache publisher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusCacheRecord {
    /// Cache schema version.
    #[serde(default)]
    pub schema_version: u16,
    /// Product root this cache describes.
    pub product_root: PathBuf,
    /// Supervisor store path this cache observes.
    pub supervisor_store_path: PathBuf,
    /// Directory that contains status cache files.
    pub status_cache_path: PathBuf,
    /// Writer identity for the process that produced the cache.
    pub writer: RuntimeStatusWriterIdentity,
    /// Latest runtime status snapshot.
    pub snapshot: RuntimeStatusSnapshot,
    /// Bounded recent action window copied into the latest cache file.
    pub recent_actions: Vec<RuntimeActionRecord>,
    /// Cache write time in milliseconds.
    pub written_at_ms: u64,
}

impl RuntimeStatusCacheRecord {
    /// Build a cache record with the current schema version.
    pub fn new(
        product_root: impl Into<PathBuf>,
        supervisor_store_path: impl Into<PathBuf>,
        status_cache_path: impl Into<PathBuf>,
        writer: RuntimeStatusWriterIdentity,
        snapshot: RuntimeStatusSnapshot,
        recent_actions: Vec<RuntimeActionRecord>,
        written_at_ms: u64,
    ) -> Self {
        Self {
            schema_version: RUNTIME_STATUS_CACHE_SCHEMA_VERSION,
            product_root: product_root.into(),
            supervisor_store_path: supervisor_store_path.into(),
            status_cache_path: status_cache_path.into(),
            writer,
            snapshot,
            recent_actions,
            written_at_ms,
        }
    }

    /// Return cache age at the supplied wall clock time.
    pub fn age_ms_at(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.written_at_ms)
    }

    /// Return true when the cache is older than a caller supplied threshold.
    pub fn is_stale_at(&self, now_ms: u64, stale_after_ms: u64) -> bool {
        self.age_ms_at(now_ms) > stale_after_ms
    }
}

/// Request parameters for a cache reader operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStatusReadRequest {
    /// Wall clock used for age and stale derivation.
    pub now_ms: u64,
    /// Age after which a cache is reported stale.
    pub stale_after_ms: u64,
    /// Maximum recent action records to return.
    pub recent_action_limit: usize,
}

/// Tolerant low-level result for one latest snapshot read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStatusSnapshotRead {
    /// The snapshot file does not exist.
    Missing,
    /// One supported snapshot decoded successfully.
    Decoded {
        /// Decoded cache record.
        record: Box<RuntimeStatusCacheRecord>,
        /// Compatibility or recovery warnings from decoding.
        warnings: Vec<RuntimeStatusCacheWarning>,
    },
    /// A snapshot exists but is partial, corrupt, or otherwise unreadable.
    Unreadable {
        /// Stable warnings that explain the unreadable projection.
        warnings: Vec<RuntimeStatusCacheWarning>,
    },
    /// The snapshot declares a schema newer than this reader supports.
    UnsupportedVersion {
        /// Version observed before payload decoding.
        observed_version: u16,
        /// Stable warnings that explain the compatibility result.
        warnings: Vec<RuntimeStatusCacheWarning>,
    },
}

/// Tolerant low-level result for the bounded action file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeStatusActionsRead {
    /// Valid newest actions retained by the reader.
    pub actions: Vec<RuntimeActionRecord>,
    /// Warnings for skipped, partial, old, or unsupported lines.
    pub warnings: Vec<RuntimeStatusCacheWarning>,
}

/// Cache read result returned to operator commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStatusReadResult {
    /// Latest cache record when present.
    pub latest: Option<RuntimeStatusCacheRecord>,
    /// Recent actions returned by the reader.
    pub recent_actions: Vec<RuntimeActionRecord>,
    /// Wall clock used for this read.
    pub observed_at_ms: u64,
    /// Derived cache state.
    pub cache_state: RuntimeStatusCacheState,
    /// Reader warnings for tolerant cache behavior.
    pub warnings: Vec<RuntimeStatusCacheWarning>,
}

impl RuntimeStatusReadResult {
    /// Derive a read result from an optional latest cache record.
    pub fn from_latest(
        latest: Option<RuntimeStatusCacheRecord>,
        recent_actions: Vec<RuntimeActionRecord>,
        observed_at_ms: u64,
        stale_after_ms: u64,
    ) -> Self {
        let cache_state = match latest.as_ref() {
            Some(record) if record.is_stale_at(observed_at_ms, stale_after_ms) => {
                RuntimeStatusCacheState::Stale {
                    age_ms: record.age_ms_at(observed_at_ms),
                    stale_after_ms,
                }
            }
            Some(record) => RuntimeStatusCacheState::Fresh {
                age_ms: record.age_ms_at(observed_at_ms),
            },
            None => RuntimeStatusCacheState::Missing,
        };
        let warnings = latest
            .as_ref()
            .filter(|record| record.written_at_ms > observed_at_ms)
            .map(|record| RuntimeStatusCacheWarning {
                code: RUNTIME_STATUS_WARNING_CLOCK_SKEW.to_string(),
                message: format!(
                    "cache write time {} is ahead of reader time {observed_at_ms}",
                    record.written_at_ms
                ),
            })
            .into_iter()
            .collect();
        Self {
            latest,
            recent_actions,
            observed_at_ms,
            cache_state,
            warnings,
        }
    }
}

/// Derived status cache state for operator output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatusCacheState {
    /// No cache file has been observed.
    Missing,
    /// Cache is present and inside the stale threshold.
    Fresh {
        /// Cache age in milliseconds.
        age_ms: u64,
    },
    /// Cache is present but older than the stale threshold.
    Stale {
        /// Cache age in milliseconds.
        age_ms: u64,
        /// Stale threshold in milliseconds.
        stale_after_ms: u64,
    },
    /// Cache files were present but could not be safely decoded.
    Unreadable,
    /// Cache schema is newer than this reader supports.
    UnsupportedVersion {
        /// Version observed in the cache envelope.
        observed_version: u16,
        /// Newest schema supported by this reader.
        max_supported_version: u16,
    },
}

/// Identity of the cache writer process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusWriterIdentity {
    /// Supervisor instance id when startup has selected one.
    pub instance_id: Option<String>,
    /// Operating system process id when known.
    pub process_id: Option<u32>,
    /// Parent process id when known.
    pub parent_process_id: Option<u32>,
    /// How the supervisor was launched.
    pub run_mode: RuntimeRunMode,
    /// Current launch state.
    pub launch_status: RuntimeLaunchStatus,
}

/// Runtime supervisor launch mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRunMode {
    /// Launch mode has not been observed.
    Unknown,
    /// Supervisor owns the current foreground CLI process.
    Foreground,
    /// Supervisor owns a child or background process.
    Detached,
}

/// Operator-facing process launch status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLaunchStatus {
    /// Launch status has not been observed.
    Unknown,
    /// Runtime process has been started but is not ready.
    Launching,
    /// Runtime process has acquired initial ownership and written status.
    Ready,
    /// Runtime process is detached from the launching command.
    Detached,
    /// Runtime process is stopping.
    Stopping,
    /// Runtime process stopped cleanly.
    Stopped,
    /// Runtime process failed before clean shutdown.
    Failed,
    /// Runtime process appears stale to a cache reader.
    Stale,
}

/// Latest operator-facing runtime snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusSnapshot {
    /// Supervisor instance summary when known.
    pub instance: Option<RuntimeStatusInstanceSummary>,
    /// Process summary when known.
    pub process: Option<RuntimeStatusProcessSummary>,
    /// Shutdown summary when shutdown has started.
    pub shutdown: Option<RuntimeStatusShutdownSummary>,
    /// One row per desired or observed runtime.
    pub runtimes: Vec<RuntimeStatusRuntimeRow>,
    /// Aggregated health counts for quick text output.
    pub health_counts: RuntimeStatusHealthCounts,
    /// Event ledger health summary when the writer observed it.
    #[serde(default)]
    pub ledger: Option<RuntimeStatusLedgerSummary>,
    /// Cache warnings derived by writer or reader.
    pub warnings: Vec<RuntimeStatusCacheWarning>,
}

/// Event ledger health copied into the status cache.
///
/// An operational projection of the observability health report: the cache
/// stays non-authoritative, and append rates stay out because they are
/// windowed computations, not snapshot state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusLedgerSummary {
    /// Highest persisted ledger sequence.
    pub tip_seq: u64,
    /// Highest writer-committed sequence.
    pub committed_watermark: u64,
    /// First retained sequence; one means full history.
    pub retained_from: u64,
    /// Best-effort events dropped by backpressure since process start.
    pub dropped_events: u64,
    /// Per-consumer positions and lag against the watermark.
    pub consumers: Vec<RuntimeStatusConsumerLag>,
}

/// One consumer lag row copied into the status cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusConsumerLag {
    /// Stable consumer name from the cursor registry.
    pub name: String,
    /// Highest sequence the consumer reported as durably reduced.
    pub reported_seq: u64,
    /// Watermark minus reported sequence, zero when caught up.
    pub lag: u64,
}

impl RuntimeStatusLedgerSummary {
    /// Copies the observability health report into cache vocabulary.
    pub fn from_health(report: &meld_events::EventHealthReport) -> Self {
        Self {
            tip_seq: report.tip_seq,
            committed_watermark: report.committed_watermark,
            retained_from: report.retained_from,
            dropped_events: report.dropped_events,
            consumers: report
                .consumers
                .iter()
                .map(|consumer| RuntimeStatusConsumerLag {
                    name: consumer.name.clone(),
                    reported_seq: consumer.reported_seq,
                    lag: consumer.lag,
                })
                .collect(),
        }
    }
}

/// Supervisor instance data copied into the status cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusInstanceSummary {
    /// Supervisor instance id.
    pub instance_id: String,
    /// Instance lifecycle status as display text.
    pub status: String,
    /// Instance start time in milliseconds.
    pub started_at_ms: u64,
    /// Instance stop time in milliseconds when known.
    pub stopped_at_ms: Option<u64>,
}

/// Runtime process data copied into the status cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusProcessSummary {
    /// Operating system process id when known.
    pub process_id: Option<u32>,
    /// Parent process id when known.
    pub parent_process_id: Option<u32>,
    /// How the supervisor was launched.
    pub run_mode: RuntimeRunMode,
    /// Current launch status.
    pub launch_status: RuntimeLaunchStatus,
    /// Process start time in milliseconds when known.
    pub started_at_ms: Option<u64>,
    /// Process ready time in milliseconds when known.
    pub ready_at_ms: Option<u64>,
    /// Log path for process output when known.
    pub log_path: Option<PathBuf>,
}

/// Shutdown data copied into the status cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusShutdownSummary {
    /// Shutdown id when one has been created.
    pub shutdown_id: Option<String>,
    /// Shutdown lifecycle status as display text.
    pub status: String,
    /// Shutdown request time in milliseconds when known.
    pub requested_at_ms: Option<u64>,
    /// Shutdown completion time in milliseconds when known.
    pub completed_at_ms: Option<u64>,
}

/// One operator-facing status row for a runtime id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusRuntimeRow {
    /// Stable supervised runtime id.
    pub runtime_id: String,
    /// Whether this runtime was desired by configuration.
    pub desired_enabled: bool,
    /// Whether a matching factory was available.
    pub factory_available: bool,
    /// Lifecycle shape declared by the canonical runtime registry.
    #[serde(default)]
    pub role_class: RuntimeRoleClass,
    /// Honest implementation posture at cache write time.
    #[serde(default)]
    pub implementation_state: RuntimeImplementationState,
    /// Observed handle kind.
    pub handle_kind: RuntimeHandleKind,
    /// Active lease summary when present.
    pub lease: Option<RuntimeStatusLeaseSummary>,
    /// Latest heartbeat summary when present.
    pub heartbeat: Option<RuntimeStatusHeartbeatSummary>,
    /// Latest health summary.
    pub health: RuntimeStatusHealthSummary,
    /// Restart attempt count observed by the supervisor.
    pub restart_count: u64,
    /// Last restart cause as display text.
    pub last_restart_cause: Option<String>,
    /// Last supervisor lifecycle event as display text.
    pub last_lifecycle_event: Option<String>,
    /// Last meaningful runtime action when present.
    pub last_action: Option<RuntimeActionRecord>,
    /// Last progress checkpoint when present.
    pub last_progress: Option<RuntimeCheckpointObservation>,
}

/// Observed handle kind for one runtime row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHandleKind {
    /// The cache writer could not classify the handle.
    Unknown,
    /// No concrete domain work is currently wired behind this handle.
    Inert,
    /// A concrete domain worker is wired behind this handle.
    Concrete,
    /// Factory or required resource was unavailable.
    Unavailable,
}

/// Lease data copied into one status row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusLeaseSummary {
    /// Active lease id.
    pub lease_id: String,
    /// Supervisor instance that owns the active lease.
    #[serde(default)]
    pub owner_instance_id: Option<String>,
    /// Lease status as display text.
    pub status: String,
    /// Lease expiry time in milliseconds.
    pub expires_at_ms: u64,
}

/// Heartbeat data copied into one status row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusHeartbeatSummary {
    /// Last heartbeat observation time in milliseconds.
    pub observed_at_ms: u64,
    /// Heartbeat age at cache write time in milliseconds.
    pub age_ms: u64,
}

/// Health data copied into one status row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusHealthSummary {
    /// Health status as display text.
    pub status: String,
    /// Retryable error count.
    pub retryable_error_count: u64,
    /// Fatal error count.
    pub fatal_error_count: u64,
    /// True when the last bounded worker report exhausted its budget.
    pub budget_exhausted: bool,
}

/// Aggregated runtime health counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusHealthCounts {
    /// Count of runtimes with unknown health.
    pub unknown: u64,
    /// Count of runtimes that are starting.
    pub starting: u64,
    /// Count of healthy runtimes.
    pub healthy: u64,
    /// Count of degraded runtimes.
    pub degraded: u64,
    /// Count of unhealthy runtimes.
    pub unhealthy: u64,
    /// Count of stopped runtimes.
    pub stopped: u64,
}

/// Warning recorded in a status cache snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusCacheWarning {
    /// Stable warning code.
    pub code: String,
    /// Human-readable warning message.
    pub message: String,
}

/// Observation record for one runtime action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeActionRecord {
    /// Stable action id for dedupe and display.
    pub action_id: String,
    /// Observation time in milliseconds.
    pub observed_at_ms: u64,
    /// Supervised runtime id.
    pub runtime_id: String,
    /// Owning domain id.
    pub domain_id: String,
    /// Actor that produced the action.
    pub actor_id: String,
    /// Domain object touched by the action.
    pub object_ref: RuntimeObjectRef,
    /// Kind of action observed.
    pub action_kind: RuntimeActionKind,
    /// Cause that led to the action.
    pub cause: RuntimeActionCause,
    /// Outcome observed for the action.
    pub outcome: RuntimeActionOutcome,
    /// Bounded numeric summary.
    pub metrics: RuntimeActionMetrics,
    /// Checkpoint observations copied from the owning domain.
    pub checkpoints: Vec<RuntimeCheckpointObservation>,
    /// Bounded issue summaries.
    pub issues: Vec<RuntimeActionIssueSummary>,
    /// Explicit detail removed to satisfy cache bounds.
    #[serde(default)]
    pub truncation: RuntimeActionTruncation,
    /// Whether sensitive values were absent or redacted.
    pub redaction: RuntimeRedactionState,
}

impl RuntimeActionRecord {
    /// Build a runtime action record from one bounded worker report.
    pub fn from_worker_tick(
        action_id: impl Into<String>,
        runtime_id: impl Into<String>,
        observed_at_ms: u64,
        report: WorkerTickReport,
    ) -> Self {
        let runtime_id = runtime_id.into();
        let domain_id = report.scope.domain_id.clone();
        let object_ref = RuntimeObjectRef {
            domain_id: domain_id.clone(),
            object_type: report
                .scope
                .work_key
                .clone()
                .unwrap_or_else(|| "worker_tick".to_string()),
            object_id: report
                .scope
                .stream_id
                .clone()
                .or_else(|| report.scope.subject_key.clone())
                .or_else(|| report.scope.agent_id.clone())
                .or_else(|| report.scope.work_key.clone())
                .unwrap_or_else(|| runtime_id.clone()),
            branch_id: report.scope.branch_id.clone(),
            perspective_key: report.scope.perspective_key.clone(),
            parent_refs: Vec::new(),
            source_refs: report
                .scope
                .subject_key
                .iter()
                .cloned()
                .chain(report.scope.agent_id.iter().cloned())
                .collect(),
        };
        let issue_count = report.retryable_errors.len() + report.fatal_errors.len();
        let mut issue_message_truncated = false;
        let issues = report
            .fatal_errors
            .iter()
            .map(|issue| (RuntimeActionIssueSeverity::Fatal, issue))
            .chain(
                report
                    .retryable_errors
                    .iter()
                    .map(|issue| (RuntimeActionIssueSeverity::Retryable, issue)),
            )
            .take(RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT)
            .map(|(severity, issue)| {
                let (message, truncated) =
                    bounded_utf8(&issue.message, RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES);
                issue_message_truncated |= truncated;
                RuntimeActionIssueSummary {
                    severity,
                    item_id: issue.item_id.clone(),
                    code: issue.code.clone(),
                    message,
                    truncated,
                }
            })
            .collect();
        let checkpoints = vec![RuntimeCheckpointObservation {
            input_name: report.input_checkpoint.name.clone(),
            output_name: report.output_checkpoint.name.clone(),
            input_value: report.input_checkpoint.value,
            output_value: report.output_checkpoint.value,
        }];
        Self {
            action_id: action_id.into(),
            observed_at_ms,
            runtime_id,
            domain_id,
            actor_id: report.actor_id.clone(),
            object_ref,
            action_kind: RuntimeActionKind::Tick,
            cause: RuntimeActionCause::SupervisorTick,
            outcome: RuntimeActionOutcome::from_worker_tick(&report),
            metrics: RuntimeActionMetrics::from_worker_tick(&report),
            checkpoints,
            issues,
            truncation: RuntimeActionTruncation {
                omitted_issue_count: issue_count
                    .saturating_sub(RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT)
                    as u64,
                truncated_message_count: if issue_message_truncated {
                    report
                        .fatal_errors
                        .iter()
                        .chain(report.retryable_errors.iter())
                        .take(RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT)
                        .filter(|issue| {
                            issue.message.len() > RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES
                        })
                        .count() as u64
                } else {
                    0
                },
            },
            redaction: RuntimeRedactionState::NotNeeded,
        }
    }
}

/// Domain object reference carried by a runtime action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeObjectRef {
    /// Owning domain id.
    pub domain_id: String,
    /// Domain object type.
    pub object_type: String,
    /// Stable object id when known.
    pub object_id: String,
    /// Branch id when the action is branch scoped.
    pub branch_id: Option<String>,
    /// Perspective key when the action is perspective scoped.
    pub perspective_key: Option<String>,
    /// Parent object refs as compact ids.
    pub parent_refs: Vec<String>,
    /// Source refs as compact ids.
    pub source_refs: Vec<String>,
}

/// Runtime action kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeActionKind {
    /// Runtime startup action.
    Startup,
    /// Runtime shutdown action.
    Shutdown,
    /// Supervisor tick action.
    Tick,
    /// Lease action.
    Lease,
    /// Heartbeat action.
    Heartbeat,
    /// Health snapshot action.
    Health,
    /// Restart action.
    Restart,
    /// Event append action.
    Append,
    /// Event replay action.
    Replay,
    /// Domain reduction action.
    Reduction,
    /// Planner projection action.
    Projection,
    /// Domain command action.
    Command,
    /// Domain mutation action.
    Mutation,
    /// Task dispatch action.
    Dispatch,
    /// Provider action.
    Provider,
    /// Artifact action.
    Artifact,
    /// Publication action.
    Publication,
    /// Workspace scan action.
    Scan,
    /// Agent curation action.
    Curation,
}

/// Cause that triggered a runtime action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeActionCause {
    /// User or caller command.
    OperatorCommand,
    /// Periodic supervisor tick.
    SupervisorTick,
    /// Supervisor lease recovery.
    LeaseRecovery,
    /// Event replay batch.
    ReplayBatch,
    /// Domain command.
    DomainCommand,
    /// Task readiness.
    TaskReadiness,
    /// Retry after a prior failure.
    Retry,
    /// Shutdown request.
    Shutdown,
}

/// Outcome observed for a runtime action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeActionOutcome {
    /// Action has started.
    Started,
    /// Action completed successfully.
    Succeeded,
    /// Action found no durable work.
    NoWork,
    /// Action replayed a duplicate request.
    Duplicate,
    /// Action was rejected by a domain contract.
    Rejected,
    /// Action is blocked on a dependency.
    Blocked,
    /// Action failed in a retryable way.
    RetryableFailure,
    /// Action failed in a fatal way.
    FatalFailure,
    /// Action was cancelled.
    Cancelled,
}

impl RuntimeActionOutcome {
    /// Classify a worker tick into an operator-facing outcome.
    pub fn from_worker_tick(report: &WorkerTickReport) -> Self {
        if !report.fatal_errors.is_empty() {
            Self::FatalFailure
        } else if !report.retryable_errors.is_empty() {
            Self::RetryableFailure
        } else if report.made_progress() {
            Self::Succeeded
        } else {
            Self::NoWork
        }
    }
}

/// Bounded numeric summary for a runtime action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeActionMetrics {
    /// Durable items attempted.
    pub attempted: u64,
    /// Durable items committed.
    pub committed: u64,
    /// Retryable issue count.
    pub retryable_issue_count: u64,
    /// Fatal issue count.
    pub fatal_issue_count: u64,
    /// True when the action consumed its bounded budget.
    pub budget_exhausted: bool,
}

impl RuntimeActionMetrics {
    /// Build action metrics from one worker tick report.
    pub fn from_worker_tick(report: &WorkerTickReport) -> Self {
        Self {
            attempted: report.items_attempted as u64,
            committed: report.items_committed as u64,
            retryable_issue_count: report.retryable_errors.len() as u64,
            fatal_issue_count: report.fatal_errors.len() as u64,
            budget_exhausted: report.budget_exhausted,
        }
    }
}

/// Input and output checkpoint values observed during an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCheckpointObservation {
    /// Input checkpoint name.
    pub input_name: String,
    /// Output checkpoint name.
    pub output_name: String,
    /// Input value before the action.
    pub input_value: u64,
    /// Output value after the action.
    pub output_value: u64,
}

/// Bounded issue summary copied into an action record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeActionIssueSummary {
    /// Issue severity.
    pub severity: RuntimeActionIssueSeverity,
    /// Optional domain item id.
    pub item_id: Option<String>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable issue message.
    pub message: String,
    /// True when the message exceeded the cache byte bound.
    #[serde(default)]
    pub truncated: bool,
}

/// Counts of action detail removed by cache bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuntimeActionTruncation {
    /// Issue summaries omitted after severity-prioritized retention.
    pub omitted_issue_count: u64,
    /// Retained issue messages shortened on a UTF-8 boundary.
    pub truncated_message_count: u64,
}

impl RuntimeActionTruncation {
    /// Return whether any action detail was removed.
    pub fn is_truncated(self) -> bool {
        self.omitted_issue_count != 0 || self.truncated_message_count != 0
    }
}

/// Versioned line stored in `actions.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusActionEnvelope {
    /// Cache schema version used for this line.
    #[serde(default)]
    pub schema_version: u16,
    /// Bounded runtime action payload.
    pub action: RuntimeActionRecord,
}

impl RuntimeStatusActionEnvelope {
    /// Wrap one action with the current cache schema version.
    pub fn current(action: RuntimeActionRecord) -> Self {
        Self {
            schema_version: RUNTIME_STATUS_CACHE_SCHEMA_VERSION,
            action,
        }
    }
}

fn bounded_utf8(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_string(), false);
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_string(), true)
}

/// Runtime action issue severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeActionIssueSeverity {
    /// Retryable issue.
    Retryable,
    /// Fatal issue.
    Fatal,
}

/// Redaction state for an action record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRedactionState {
    /// No sensitive values were present.
    NotNeeded,
    /// Sensitive values were removed.
    Redacted,
    /// Sensitive values may still be present.
    ContainsSensitiveData,
}

/// Cache publisher interface owned by the runtime supervisor domain.
///
/// A concrete publisher must hold `writer_lock` exclusively for its full
/// lifetime so an inactive contender cannot replace active-owner truth.
pub trait RuntimeStatusPublisher {
    /// Error returned by the concrete publisher.
    type Error;

    /// Publish the startup snapshot.
    fn publish_startup_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error>;

    /// Publish a tick snapshot.
    fn publish_tick_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error>;

    /// Publish a snapshot immediately after restart policy changes ownership.
    fn publish_restart_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error>;

    /// Publish one runtime action.
    fn publish_action(&mut self, action: &RuntimeActionRecord) -> Result<(), Self::Error>;

    /// Publish the shutdown snapshot.
    fn publish_shutdown_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error>;

    /// Flush cache data before the caller reports completion.
    fn flush_cache(&mut self) -> Result<(), Self::Error>;
}

/// Cache reader interface used by operator commands.
pub trait RuntimeStatusReader {
    /// Error returned by the concrete reader.
    type Error;

    /// Read the latest cache snapshot when present.
    fn read_latest_snapshot(&self) -> Result<RuntimeStatusSnapshotRead, Self::Error>;

    /// Read recent actions up to the requested limit.
    fn read_recent_actions(&self, limit: usize) -> Result<RuntimeStatusActionsRead, Self::Error>;

    /// Read latest status and derive cache state in one operation.
    fn read_status(
        &self,
        request: RuntimeStatusReadRequest,
    ) -> Result<RuntimeStatusReadResult, Self::Error> {
        let snapshot_read = self.read_latest_snapshot()?;
        let mut actions_read = self.read_recent_actions(
            request
                .recent_action_limit
                .min(RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT),
        )?;
        match snapshot_read {
            RuntimeStatusSnapshotRead::Missing => Ok(RuntimeStatusReadResult {
                latest: None,
                recent_actions: actions_read.actions,
                observed_at_ms: request.now_ms,
                cache_state: RuntimeStatusCacheState::Missing,
                warnings: actions_read.warnings,
            }),
            RuntimeStatusSnapshotRead::Decoded {
                record,
                mut warnings,
            } => {
                let mut result = RuntimeStatusReadResult::from_latest(
                    Some(*record),
                    actions_read.actions,
                    request.now_ms,
                    request.stale_after_ms,
                );
                warnings.append(&mut result.warnings);
                warnings.append(&mut actions_read.warnings);
                result.warnings = warnings;
                Ok(result)
            }
            RuntimeStatusSnapshotRead::Unreadable { mut warnings } => {
                warnings.append(&mut actions_read.warnings);
                Ok(RuntimeStatusReadResult {
                    latest: None,
                    recent_actions: actions_read.actions,
                    observed_at_ms: request.now_ms,
                    cache_state: RuntimeStatusCacheState::Unreadable,
                    warnings,
                })
            }
            RuntimeStatusSnapshotRead::UnsupportedVersion {
                observed_version,
                mut warnings,
            } => {
                warnings.append(&mut actions_read.warnings);
                Ok(RuntimeStatusReadResult {
                    latest: None,
                    recent_actions: actions_read.actions,
                    observed_at_ms: request.now_ms,
                    cache_state: RuntimeStatusCacheState::UnsupportedVersion {
                        observed_version,
                        max_supported_version: RUNTIME_STATUS_CACHE_SCHEMA_VERSION,
                    },
                    warnings,
                })
            }
        }
    }
}

impl WorkerTickReport {
    /// Returns true when the report indicates durable progress.
    pub fn made_progress(&self) -> bool {
        self.output_checkpoint.value > self.input_checkpoint.value || self.items_committed > 0
    }

    /// Build a fatal report for failures before a domain tick can complete.
    pub fn fatal(
        actor_id: impl Into<String>,
        domain_id: impl Into<String>,
        work_key: Option<&str>,
        checkpoint_name: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let checkpoint_name = checkpoint_name.into();
        Self {
            actor_id: actor_id.into(),
            scope: WorkerScope {
                domain_id: domain_id.into(),
                stream_id: None,
                work_key: work_key.map(str::to_string),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: checkpoint_name.clone(),
                value: 0,
            },
            output_checkpoint: WorkerCheckpoint {
                name: checkpoint_name,
                value: 0,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: vec![WorkerTickIssue {
                item_id: None,
                code: code.into(),
                message: message.into(),
            }],
            budget_exhausted: false,
        }
    }
}

impl From<GraphCatchUpReport> for WorkerTickReport {
    fn from(report: GraphCatchUpReport) -> Self {
        Self {
            actor_id: report.actor_id,
            scope: WorkerScope {
                domain_id: "world_state".to_string(),
                stream_id: None,
                work_key: Some("graph".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: report.input_event_seq,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: report.output_event_seq,
            },
            items_attempted: report.events_attempted,
            items_committed: report.traversal_events_applied,
            retryable_errors: report
                .retryable_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.item_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            fatal_errors: report
                .fatal_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.item_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            budget_exhausted: report.budget_exhausted,
        }
    }
}

impl From<PublicationBridgeReport> for WorkerTickReport {
    fn from(report: PublicationBridgeReport) -> Self {
        Self {
            actor_id: report.actor_id,
            scope: WorkerScope {
                domain_id: "execution".to_string(),
                stream_id: Some(report.scope.network_id),
                work_key: Some("publication_outbox".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.input_revision,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.output_revision,
            },
            items_attempted: report.items_attempted,
            items_committed: report.items_committed,
            retryable_errors: report
                .retryable_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.publication_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            fatal_errors: report
                .fatal_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.publication_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            budget_exhausted: report.budget_exhausted,
        }
    }
}

impl From<PublicationRuntimeReport> for WorkerTickReport {
    fn from(report: PublicationRuntimeReport) -> Self {
        Self {
            actor_id: report.actor_id,
            scope: WorkerScope {
                domain_id: "execution".to_string(),
                stream_id: Some(report.scope.network_id),
                work_key: Some("publication_outbox".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.input_revision,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.output_revision,
            },
            items_attempted: report.attempted,
            items_committed: report.committed,
            retryable_errors: report
                .retryable_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.publication_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            fatal_errors: report
                .fatal_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.publication_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            budget_exhausted: report.budget_exhausted,
        }
    }
}

impl From<PlanningRuntimeActorReport> for WorkerTickReport {
    fn from(report: PlanningRuntimeActorReport) -> Self {
        Self {
            actor_id: report.actor_id,
            scope: WorkerScope {
                domain_id: "execution".to_string(),
                stream_id: None,
                work_key: Some("planning".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.input_revision,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: report.output_revision,
            },
            items_attempted: report.attempted,
            items_committed: report.committed,
            retryable_errors: report
                .retryable_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.goal_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            fatal_errors: report
                .fatal_errors
                .into_iter()
                .map(|issue| WorkerTickIssue {
                    item_id: issue.goal_id,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            budget_exhausted: report.budget_exhausted,
        }
    }
}

impl From<AgentRuntimeReport> for WorkerTickReport {
    fn from(report: AgentRuntimeReport) -> Self {
        Self {
            actor_id: report.actor_id.clone(),
            scope: WorkerScope {
                domain_id: "world_model".to_string(),
                stream_id: None,
                work_key: Some("agent_curation".to_string()),
                agent_id: Some(report.actor_id),
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "agent_input_sequence".to_string(),
                value: report.input_sequence,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "agent_output_sequence".to_string(),
                value: report.output_sequence,
            },
            items_attempted: report.delivered_count,
            items_committed: report.decision_count + report.sink_submission_count,
            retryable_errors: report
                .retryable_errors
                .into_iter()
                .map(string_issue)
                .collect(),
            fatal_errors: report.fatal_errors.into_iter().map(string_issue).collect(),
            budget_exhausted: report.budget_exhausted,
        }
    }
}

impl From<DocsTaskEvidenceReplayReport> for WorkerTickReport {
    fn from(report: DocsTaskEvidenceReplayReport) -> Self {
        Self {
            actor_id: "world_model.evidence_ingestion".to_string(),
            scope: WorkerScope {
                domain_id: "world_model".to_string(),
                stream_id: None,
                work_key: Some("docs_task_evidence".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: report.input_event_seq,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: report.output_event_seq,
            },
            items_attempted: report.events_attempted,
            items_committed: report.new_assignment_count,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }
}

fn string_issue(message: String) -> WorkerTickIssue {
    WorkerTickIssue {
        item_id: None,
        code: "runtime_issue".to_string(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_execution::planning::PlanningRuntimeActorIssue;
    use meld_execution::task_network::publication::{
        PublicationBridgeIssue, PublicationBridgeScope,
    };

    #[test]
    fn graph_report_maps_to_worker_report() {
        let report = GraphCatchUpReport {
            actor_id: "world_state.graph.reducer".to_string(),
            input_event_seq: 1,
            output_event_seq: 3,
            events_attempted: 2,
            traversal_events_applied: 1,
            derived_events_appended: 1,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        };

        let worker: WorkerTickReport = report.into();

        assert_eq!(worker.scope.domain_id, "world_state");
        assert_eq!(worker.scope.work_key.as_deref(), Some("graph"));
        assert_eq!(worker.input_checkpoint.name, "event_spine_seq");
        assert_eq!(worker.output_checkpoint.value, 3);
        assert_eq!(worker.items_attempted, 2);
        assert_eq!(worker.items_committed, 1);
        assert!(worker.made_progress());
    }

    #[test]
    fn publication_report_maps_to_worker_report() {
        let report = PublicationBridgeReport {
            actor_id: "execution.task_network.publication".to_string(),
            scope: PublicationBridgeScope {
                network_id: "network-a".to_string(),
                session_id: "session-a".to_string(),
                worker_id: "worker-a".to_string(),
            },
            input_revision: 4,
            output_revision: 5,
            items_attempted: 1,
            items_committed: 0,
            retryable_errors: vec![PublicationBridgeIssue {
                publication_id: Some("publication-a".to_string()),
                code: "publication_append_failed".to_string(),
                message: "append failed".to_string(),
            }],
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            results: Vec::new(),
        };

        let worker: WorkerTickReport = report.into();

        assert_eq!(worker.scope.domain_id, "execution");
        assert_eq!(worker.scope.stream_id.as_deref(), Some("network-a"));
        assert_eq!(worker.input_checkpoint.name, "task_network_revision");
        assert_eq!(worker.output_checkpoint.value, 5);
        assert_eq!(
            worker.retryable_errors[0].item_id.as_deref(),
            Some("publication-a")
        );
        assert!(worker.made_progress());
    }

    #[test]
    fn publication_runtime_report_maps_to_worker_report() {
        let report = PublicationRuntimeReport {
            actor_id: "execution.publication".to_string(),
            scope: PublicationBridgeScope {
                network_id: "network-a".to_string(),
                session_id: "session-a".to_string(),
                worker_id: "worker-a".to_string(),
            },
            input_revision: 7,
            output_revision: 8,
            attempted: 2,
            committed: 1,
            retryable_errors: Vec::new(),
            fatal_errors: vec![PublicationBridgeIssue {
                publication_id: Some("publication-b".to_string()),
                code: "mark_failed".to_string(),
                message: "mark failed".to_string(),
            }],
            budget_exhausted: true,
            results: Vec::new(),
        };

        let worker: WorkerTickReport = report.into();

        assert_eq!(worker.actor_id, "execution.publication");
        assert_eq!(worker.scope.stream_id.as_deref(), Some("network-a"));
        assert_eq!(worker.input_checkpoint.value, 7);
        assert_eq!(worker.output_checkpoint.value, 8);
        assert_eq!(worker.items_attempted, 2);
        assert_eq!(worker.items_committed, 1);
        assert_eq!(
            worker.fatal_errors[0].item_id.as_deref(),
            Some("publication-b")
        );
        assert!(worker.budget_exhausted);
    }

    #[test]
    fn planning_runtime_report_maps_to_worker_report() {
        let report = PlanningRuntimeActorReport {
            actor_id: "execution.planning".to_string(),
            active_goal_count: 3,
            input_revision: 10,
            output_revision: 11,
            attempted: 1,
            committed: 1,
            retryable_errors: vec![PlanningRuntimeActorIssue {
                goal_id: Some("goal-a".to_string()),
                code: "projection_unavailable".to_string(),
                message: "projection unavailable".to_string(),
            }],
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            results: Vec::new(),
        };

        let worker: WorkerTickReport = report.into();

        assert_eq!(worker.actor_id, "execution.planning");
        assert_eq!(worker.scope.work_key.as_deref(), Some("planning"));
        assert_eq!(worker.input_checkpoint.name, "task_network_revision");
        assert_eq!(worker.output_checkpoint.value, 11);
        assert_eq!(worker.items_attempted, 1);
        assert_eq!(worker.items_committed, 1);
        assert_eq!(
            worker.retryable_errors[0].item_id.as_deref(),
            Some("goal-a")
        );
    }

    #[test]
    fn agent_runtime_report_maps_to_worker_report() {
        let report = AgentRuntimeReport {
            actor_id: "agent-a".to_string(),
            input_sequence: 20,
            output_sequence: 21,
            delivered_count: 2,
            decision_count: 1,
            sink_submission_count: 1,
            sink_receipts: Vec::new(),
            retryable_errors: vec!["retry".to_string()],
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        };

        let worker: WorkerTickReport = report.into();
        let action = RuntimeActionRecord::from_worker_tick(
            "action-agent",
            "world_model.agent_goal_curation",
            25,
            worker.clone(),
        );

        assert_eq!(worker.actor_id, "agent-a");
        assert_eq!(worker.scope.agent_id.as_deref(), Some("agent-a"));
        assert_eq!(worker.input_checkpoint.name, "agent_input_sequence");
        assert_eq!(worker.output_checkpoint.name, "agent_output_sequence");
        assert_eq!(worker.items_attempted, 2);
        assert_eq!(worker.items_committed, 2);
        assert_eq!(worker.retryable_errors[0].code, "runtime_issue");
        assert_eq!(action.object_ref.object_type, "agent_curation");
        assert_eq!(action.object_ref.object_id, "agent-a");
        assert_eq!(action.checkpoints[0].input_name, "agent_input_sequence");
        assert_eq!(action.checkpoints[0].output_name, "agent_output_sequence");
    }

    #[test]
    fn docs_evidence_replay_report_maps_to_worker_report() {
        let report = DocsTaskEvidenceReplayReport {
            input_event_seq: 30,
            output_event_seq: 32,
            events_attempted: 2,
            promoted_evidence_count: 1,
            rejected_evidence_count: 0,
            normalized_evidence_count: 1,
            new_assignment_count: 1,
            ingestions: Vec::new(),
        };

        let worker: WorkerTickReport = report.into();

        assert_eq!(worker.actor_id, "world_model.evidence_ingestion");
        assert_eq!(worker.scope.work_key.as_deref(), Some("docs_task_evidence"));
        assert_eq!(worker.input_checkpoint.value, 30);
        assert_eq!(worker.output_checkpoint.value, 32);
        assert_eq!(worker.items_attempted, 2);
        assert_eq!(worker.items_committed, 1);
        assert!(worker.made_progress());
    }

    #[test]
    fn worker_report_maps_to_runtime_action_record() {
        let report = WorkerTickReport {
            actor_id: "world_state.graph.reducer".to_string(),
            scope: WorkerScope {
                domain_id: "world_state".to_string(),
                stream_id: Some("event-ledger".to_string()),
                work_key: Some("graph".to_string()),
                agent_id: None,
                perspective_key: Some("default".to_string()),
                branch_id: Some("main".to_string()),
                subject_key: Some("subject-a".to_string()),
            },
            input_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: 2,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "event_spine_seq".to_string(),
                value: 4,
            },
            items_attempted: 2,
            items_committed: 1,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        };

        let action = RuntimeActionRecord::from_worker_tick(
            "action-a",
            "world_model.graph_replay",
            9,
            report,
        );

        assert_eq!(action.action_id, "action-a");
        assert_eq!(action.domain_id, "world_state");
        assert_eq!(action.object_ref.object_type, "graph");
        assert_eq!(action.object_ref.object_id, "event-ledger");
        assert_eq!(action.object_ref.branch_id.as_deref(), Some("main"));
        assert_eq!(
            action.object_ref.perspective_key.as_deref(),
            Some("default")
        );
        assert_eq!(action.object_ref.source_refs, vec!["subject-a"]);
        assert_eq!(action.action_kind, RuntimeActionKind::Tick);
        assert_eq!(action.cause, RuntimeActionCause::SupervisorTick);
        assert_eq!(action.outcome, RuntimeActionOutcome::Succeeded);
        assert_eq!(action.metrics.attempted, 2);
        assert_eq!(action.metrics.committed, 1);
        assert_eq!(action.checkpoints[0].input_name, "event_spine_seq");
        assert_eq!(action.checkpoints[0].output_name, "event_spine_seq");
        assert_eq!(action.checkpoints[0].input_value, 2);
        assert_eq!(action.checkpoints[0].output_value, 4);
    }

    #[test]
    fn worker_report_action_classification_covers_issue_paths() {
        let mut report = WorkerTickReport {
            actor_id: "execution.planning".to_string(),
            scope: WorkerScope {
                domain_id: "execution".to_string(),
                stream_id: None,
                work_key: Some("planning".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: Some("goal-a".to_string()),
            },
            input_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: 1,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: 1,
            },
            items_attempted: 1,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: true,
        };

        let no_work = RuntimeActionRecord::from_worker_tick(
            "action-no-work",
            "execution.planning",
            3,
            report.clone(),
        );
        assert_eq!(no_work.outcome, RuntimeActionOutcome::NoWork);
        assert_eq!(no_work.object_ref.object_id, "goal-a");
        assert!(no_work.metrics.budget_exhausted);

        report.retryable_errors.push(WorkerTickIssue {
            item_id: Some("goal-a".to_string()),
            code: "projection_unavailable".to_string(),
            message: "projection unavailable".to_string(),
        });
        let retryable = RuntimeActionRecord::from_worker_tick(
            "action-retryable",
            "execution.planning",
            4,
            report.clone(),
        );
        assert_eq!(retryable.outcome, RuntimeActionOutcome::RetryableFailure);
        assert_eq!(retryable.metrics.retryable_issue_count, 1);
        assert_eq!(
            retryable.issues[0].severity,
            RuntimeActionIssueSeverity::Retryable
        );

        report.fatal_errors.push(WorkerTickIssue {
            item_id: Some("goal-a".to_string()),
            code: "planning_failed".to_string(),
            message: "planning failed".to_string(),
        });
        let fatal =
            RuntimeActionRecord::from_worker_tick("action-fatal", "execution.planning", 5, report);
        assert_eq!(fatal.outcome, RuntimeActionOutcome::FatalFailure);
        assert_eq!(fatal.metrics.fatal_issue_count, 1);
        assert_eq!(fatal.issues[0].severity, RuntimeActionIssueSeverity::Fatal);
        assert_eq!(
            fatal.issues[1].severity,
            RuntimeActionIssueSeverity::Retryable
        );
    }

    #[test]
    fn status_cache_record_round_trips_json() {
        let action = RuntimeActionRecord {
            action_id: "action-a".to_string(),
            observed_at_ms: 7,
            runtime_id: "event.replay".to_string(),
            domain_id: "event".to_string(),
            actor_id: "event.replay".to_string(),
            object_ref: RuntimeObjectRef {
                domain_id: "event".to_string(),
                object_type: "replay_batch".to_string(),
                object_id: "batch-a".to_string(),
                branch_id: None,
                perspective_key: None,
                parent_refs: Vec::new(),
                source_refs: Vec::new(),
            },
            action_kind: RuntimeActionKind::Replay,
            cause: RuntimeActionCause::ReplayBatch,
            outcome: RuntimeActionOutcome::NoWork,
            metrics: RuntimeActionMetrics {
                attempted: 0,
                committed: 0,
                retryable_issue_count: 0,
                fatal_issue_count: 0,
                budget_exhausted: false,
            },
            checkpoints: vec![RuntimeCheckpointObservation {
                input_name: "event_spine_seq".to_string(),
                output_name: "event_spine_seq".to_string(),
                input_value: 10,
                output_value: 10,
            }],
            issues: Vec::new(),
            truncation: RuntimeActionTruncation::default(),
            redaction: RuntimeRedactionState::NotNeeded,
        };
        let snapshot = RuntimeStatusSnapshot {
            instance: Some(RuntimeStatusInstanceSummary {
                instance_id: "runtime-cli-1".to_string(),
                status: "running".to_string(),
                started_at_ms: 1,
                stopped_at_ms: None,
            }),
            process: Some(RuntimeStatusProcessSummary {
                process_id: Some(42),
                parent_process_id: Some(41),
                run_mode: RuntimeRunMode::Foreground,
                launch_status: RuntimeLaunchStatus::Ready,
                started_at_ms: Some(1),
                ready_at_ms: Some(2),
                log_path: None,
            }),
            shutdown: None,
            runtimes: vec![RuntimeStatusRuntimeRow {
                runtime_id: "event.replay".to_string(),
                desired_enabled: true,
                factory_available: true,
                role_class: RuntimeRoleClass::Actor,
                implementation_state: RuntimeImplementationState::Concrete,
                handle_kind: RuntimeHandleKind::Concrete,
                lease: Some(RuntimeStatusLeaseSummary {
                    lease_id: "lease-a".to_string(),
                    owner_instance_id: Some("runtime-cli-1".to_string()),
                    status: "active".to_string(),
                    expires_at_ms: 30_000,
                }),
                heartbeat: Some(RuntimeStatusHeartbeatSummary {
                    observed_at_ms: 5,
                    age_ms: 2,
                }),
                health: RuntimeStatusHealthSummary {
                    status: "healthy".to_string(),
                    retryable_error_count: 0,
                    fatal_error_count: 0,
                    budget_exhausted: false,
                },
                restart_count: 0,
                last_restart_cause: None,
                last_lifecycle_event: Some("heartbeat_accepted".to_string()),
                last_action: Some(action.clone()),
                last_progress: Some(RuntimeCheckpointObservation {
                    input_name: "event_spine_seq".to_string(),
                    output_name: "event_spine_seq".to_string(),
                    input_value: 9,
                    output_value: 10,
                }),
            }],
            health_counts: RuntimeStatusHealthCounts {
                unknown: 0,
                starting: 0,
                healthy: 1,
                degraded: 0,
                unhealthy: 0,
                stopped: 0,
            },
            ledger: None,
            warnings: Vec::new(),
        };
        let record = RuntimeStatusCacheRecord::new(
            "/tmp/product",
            "/tmp/product/runtime/supervisor.sled",
            "/tmp/product/runtime/status",
            RuntimeStatusWriterIdentity {
                instance_id: Some("runtime-cli-1".to_string()),
                process_id: Some(42),
                parent_process_id: Some(41),
                run_mode: RuntimeRunMode::Foreground,
                launch_status: RuntimeLaunchStatus::Ready,
            },
            snapshot,
            vec![action],
            7,
        );

        let encoded = serde_json::to_string(&record).unwrap();
        let decoded: RuntimeStatusCacheRecord = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.schema_version, RUNTIME_STATUS_CACHE_SCHEMA_VERSION);
        assert_eq!(
            decoded.supervisor_store_path,
            std::path::PathBuf::from("/tmp/product/runtime/supervisor.sled")
        );
        assert_eq!(decoded.age_ms_at(12), 5);
        assert!(decoded.is_stale_at(20, 10));
        assert_eq!(decoded.snapshot.runtimes[0].runtime_id, "event.replay");
        assert_eq!(
            decoded.recent_actions[0].action_kind,
            RuntimeActionKind::Replay
        );

        let mut older_value = serde_json::to_value(&record).unwrap();
        older_value
            .as_object_mut()
            .unwrap()
            .remove("schema_version");
        let older: RuntimeStatusCacheRecord = serde_json::from_value(older_value).unwrap();
        assert_eq!(older.schema_version, 0);
        assert_eq!(
            RuntimeStatusCacheCompatibility::for_schema_version(older.schema_version),
            RuntimeStatusCacheCompatibility::Older
        );
    }

    #[test]
    fn status_read_result_derives_missing_fresh_and_stale_states() {
        let missing = RuntimeStatusReadResult::from_latest(None, Vec::new(), 10, 5);
        assert_eq!(missing.cache_state, RuntimeStatusCacheState::Missing);

        let snapshot = RuntimeStatusSnapshot {
            instance: None,
            process: None,
            shutdown: None,
            runtimes: Vec::new(),
            health_counts: RuntimeStatusHealthCounts {
                unknown: 0,
                starting: 0,
                healthy: 0,
                degraded: 0,
                unhealthy: 0,
                stopped: 0,
            },
            ledger: None,
            warnings: Vec::new(),
        };
        let record = RuntimeStatusCacheRecord::new(
            "/tmp/product",
            "/tmp/product/runtime/supervisor.sled",
            "/tmp/product/runtime/status",
            RuntimeStatusWriterIdentity {
                instance_id: None,
                process_id: None,
                parent_process_id: None,
                run_mode: RuntimeRunMode::Unknown,
                launch_status: RuntimeLaunchStatus::Unknown,
            },
            snapshot,
            Vec::new(),
            10,
        );

        let fresh = RuntimeStatusReadResult::from_latest(Some(record.clone()), Vec::new(), 12, 5);
        assert_eq!(
            fresh.cache_state,
            RuntimeStatusCacheState::Fresh { age_ms: 2 }
        );

        let boundary =
            RuntimeStatusReadResult::from_latest(Some(record.clone()), Vec::new(), 15, 5);
        assert_eq!(
            boundary.cache_state,
            RuntimeStatusCacheState::Fresh { age_ms: 5 }
        );

        let stale = RuntimeStatusReadResult::from_latest(Some(record.clone()), Vec::new(), 20, 5);
        assert_eq!(
            stale.cache_state,
            RuntimeStatusCacheState::Stale {
                age_ms: 10,
                stale_after_ms: 5,
            }
        );

        let future = RuntimeStatusReadResult::from_latest(Some(record), Vec::new(), 5, 5);
        assert_eq!(
            future.cache_state,
            RuntimeStatusCacheState::Fresh { age_ms: 0 }
        );
        assert_eq!(future.warnings[0].code, RUNTIME_STATUS_WARNING_CLOCK_SKEW);
    }

    #[test]
    fn status_reader_clamps_recent_action_requests() {
        struct RecordingReader(std::cell::Cell<usize>);

        impl RuntimeStatusReader for RecordingReader {
            type Error = std::convert::Infallible;

            fn read_latest_snapshot(&self) -> Result<RuntimeStatusSnapshotRead, Self::Error> {
                Ok(RuntimeStatusSnapshotRead::Missing)
            }

            fn read_recent_actions(
                &self,
                limit: usize,
            ) -> Result<RuntimeStatusActionsRead, Self::Error> {
                self.0.set(limit);
                Ok(RuntimeStatusActionsRead::default())
            }
        }

        let reader = RecordingReader(std::cell::Cell::new(0));
        reader
            .read_status(RuntimeStatusReadRequest {
                now_ms: 10,
                stale_after_ms: 5,
                recent_action_limit: usize::MAX,
            })
            .unwrap();

        assert_eq!(reader.0.get(), RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT);
    }

    #[test]
    fn status_reader_preserves_unreadable_and_unsupported_states() {
        struct FixedReader(RuntimeStatusSnapshotRead);

        impl RuntimeStatusReader for FixedReader {
            type Error = std::convert::Infallible;

            fn read_latest_snapshot(&self) -> Result<RuntimeStatusSnapshotRead, Self::Error> {
                Ok(self.0.clone())
            }

            fn read_recent_actions(
                &self,
                _limit: usize,
            ) -> Result<RuntimeStatusActionsRead, Self::Error> {
                Ok(RuntimeStatusActionsRead {
                    actions: Vec::new(),
                    warnings: vec![RuntimeStatusCacheWarning {
                        code: "action_warning".to_string(),
                        message: "one action line was skipped".to_string(),
                    }],
                })
            }
        }

        let unreadable = FixedReader(RuntimeStatusSnapshotRead::Unreadable {
            warnings: vec![RuntimeStatusCacheWarning {
                code: RUNTIME_STATUS_WARNING_UNREADABLE.to_string(),
                message: "latest snapshot is partial".to_string(),
            }],
        })
        .read_status(RuntimeStatusReadRequest {
            now_ms: 10,
            stale_after_ms: 5,
            recent_action_limit: 10,
        })
        .unwrap();
        assert_eq!(unreadable.cache_state, RuntimeStatusCacheState::Unreadable);
        assert_eq!(unreadable.warnings.len(), 2);

        let unsupported = FixedReader(RuntimeStatusSnapshotRead::UnsupportedVersion {
            observed_version: 9,
            warnings: Vec::new(),
        })
        .read_status(RuntimeStatusReadRequest {
            now_ms: 10,
            stale_after_ms: 5,
            recent_action_limit: 10,
        })
        .unwrap();
        assert_eq!(
            unsupported.cache_state,
            RuntimeStatusCacheState::UnsupportedVersion {
                observed_version: 9,
                max_supported_version: RUNTIME_STATUS_CACHE_SCHEMA_VERSION,
            }
        );
        assert_eq!(unsupported.warnings.len(), 1);
    }

    #[test]
    fn status_cache_layout_limits_and_compatibility_are_frozen() {
        let layout = RuntimeStatusCacheLayout::from_product_root("/tmp/product");
        let limits = RuntimeStatusCacheLimits::default();

        assert_eq!(layout.root, PathBuf::from("/tmp/product/runtime/status"));
        assert_eq!(layout.latest, layout.root.join("latest.json"));
        assert_eq!(layout.actions, layout.root.join("actions.jsonl"));
        assert_eq!(layout.process, layout.root.join("process.json"));
        assert_eq!(layout.writer_lock, layout.root.join("writer.lock"));
        assert_eq!(
            RuntimeStatusCacheLayout::temporary_path(&layout.latest, 7, 42),
            PathBuf::from("/tmp/product/runtime/status/latest.json.7-42.tmp")
        );
        assert_eq!(limits.latest_max_bytes, 1024 * 1024);
        assert_eq!(limits.actions_max_bytes, 4 * 1024 * 1024);
        assert_eq!(limits.recent_action_max_count, 256);
        assert_eq!(limits.issue_max_count, 16);
        assert_eq!(limits.issue_message_max_bytes, 1024);
        assert_eq!(RUNTIME_STATUS_ATOMIC_WRITE_PROTOCOL.len(), 5);
        assert_eq!(
            RUNTIME_STATUS_ATOMIC_WRITE_PROTOCOL.last(),
            Some(&RuntimeStatusAtomicWriteStep::SyncParentDirectory)
        );
        assert_eq!(
            RuntimeStatusCacheCompatibility::for_schema_version(0),
            RuntimeStatusCacheCompatibility::Older
        );
        assert_eq!(
            RuntimeStatusCacheCompatibility::for_schema_version(1),
            RuntimeStatusCacheCompatibility::Current
        );
        assert_eq!(
            RuntimeStatusCacheCompatibility::for_schema_version(2),
            RuntimeStatusCacheCompatibility::UnsupportedFuture
        );
    }

    #[test]
    fn action_issue_count_and_utf8_messages_are_bounded() {
        let report = WorkerTickReport {
            actor_id: "execution.planning".to_string(),
            scope: WorkerScope {
                domain_id: "execution".to_string(),
                stream_id: None,
                work_key: Some("planning".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "revision".to_string(),
                value: 1,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "revision".to_string(),
                value: 1,
            },
            items_attempted: 20,
            items_committed: 0,
            retryable_errors: (0..20)
                .map(|index| WorkerTickIssue {
                    item_id: Some(format!("item-{index}")),
                    code: "retry".to_string(),
                    message: "é".repeat(600),
                })
                .collect(),
            fatal_errors: vec![WorkerTickIssue {
                item_id: Some("fatal-item".to_string()),
                code: "fatal".to_string(),
                message: "é".repeat(600),
            }],
            budget_exhausted: true,
        };

        let action =
            RuntimeActionRecord::from_worker_tick("action-a", "execution.planning", 10, report);

        assert_eq!(action.issues.len(), RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT);
        assert!(action.truncation.is_truncated());
        assert_eq!(action.truncation.omitted_issue_count, 5);
        assert_eq!(action.truncation.truncated_message_count, 16);
        assert_eq!(action.issues[0].severity, RuntimeActionIssueSeverity::Fatal);
        assert!(action.issues.iter().all(|issue| issue.truncated));
        assert!(action
            .issues
            .iter()
            .all(|issue| issue.message.len() <= RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES));

        let envelope = RuntimeStatusActionEnvelope::current(action);
        let encoded = serde_json::to_string(&envelope).unwrap();
        let decoded: RuntimeStatusActionEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.schema_version, RUNTIME_STATUS_CACHE_SCHEMA_VERSION);
    }

    #[test]
    fn older_runtime_rows_default_new_classification_fields() {
        let row: RuntimeStatusRuntimeRow = serde_json::from_value(serde_json::json!({
            "runtime_id": "execution.planning",
            "desired_enabled": false,
            "factory_available": false,
            "handle_kind": "inert",
            "lease": null,
            "heartbeat": null,
            "health": {
                "status": "stopped",
                "retryable_error_count": 0,
                "fatal_error_count": 0,
                "budget_exhausted": false
            },
            "restart_count": 0,
            "last_restart_cause": null,
            "last_lifecycle_event": null,
            "last_action": null,
            "last_progress": null
        }))
        .unwrap();

        assert_eq!(row.role_class, RuntimeRoleClass::Unknown);
        assert_eq!(
            row.implementation_state,
            RuntimeImplementationState::Unknown
        );
    }

    #[test]
    fn ledger_summary_shape_and_mapping_are_pinned() {
        let report = meld_events::EventHealthReport {
            ledger_id: meld_events::LedgerIdentity::new(),
            tip_seq: 12,
            committed_watermark: 10,
            retained_from: 1,
            dropped_events: 2,
            consumers: vec![meld_events::ConsumerLagReport {
                name: "world_state.graph.reducer".to_string(),
                reported_seq: 9,
                lag: 1,
            }],
            append_rates: Vec::new(),
            append_rate_coverage: meld_events::EventReadCoverage {
                retained_from: 1,
                tip_seq: 12,
                scanned_from_seq: Some(1),
                scanned_through_seq: Some(12),
                truncation: meld_events::CoverageTruncation::None,
            },
        };
        let summary = RuntimeStatusLedgerSummary::from_health(&report);
        assert_eq!(
            serde_json::to_value(&summary).unwrap(),
            serde_json::json!({
                "tip_seq": 12,
                "committed_watermark": 10,
                "retained_from": 1,
                "dropped_events": 2,
                "consumers": [
                    { "name": "world_state.graph.reducer", "reported_seq": 9, "lag": 1 }
                ]
            })
        );
    }

    #[test]
    fn snapshot_without_ledger_summary_still_deserializes() {
        // Wave 1 cache files written before the ledger field must stay
        // readable: the field is optional with a serde default.
        let json = serde_json::json!({
            "instance": null,
            "process": null,
            "shutdown": null,
            "runtimes": [],
            "health_counts": {
                "unknown": 0, "starting": 0, "healthy": 0,
                "degraded": 0, "unhealthy": 0, "stopped": 0
            },
            "warnings": []
        });
        let snapshot: RuntimeStatusSnapshot = serde_json::from_value(json).unwrap();
        assert!(snapshot.ledger.is_none());
    }
}
