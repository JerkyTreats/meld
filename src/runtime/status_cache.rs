//! Bounded filesystem persistence for passive runtime status.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use fs2::FileExt;
use serde::Deserialize;
use thiserror::Error;

use crate::runtime::contracts::{
    RuntimeActionIssueSeverity, RuntimeActionRecord, RuntimeActionTruncation,
    RuntimeStatusActionEnvelope, RuntimeStatusActionsRead,
    RuntimeStatusCacheCompatibility, RuntimeStatusCacheLayout, RuntimeStatusCacheLimits,
    RuntimeStatusCacheRecord, RuntimeStatusCacheWarning, RuntimeStatusPublisher,
    RuntimeStatusReader, RuntimeStatusSnapshotRead, RUNTIME_STATUS_CACHE_SCHEMA_VERSION,
    RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT, RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES,
    RUNTIME_STATUS_WARNING_OLDER_SCHEMA, RUNTIME_STATUS_WARNING_TRUNCATED,
    RUNTIME_STATUS_WARNING_UNREADABLE,
};

static TEMPORARY_FILE_NONCE: AtomicU64 = AtomicU64::new(1);

/// Filesystem status cache failure surfaced to the owning runtime adapter.
#[derive(Debug, Error)]
pub enum RuntimeStatusCacheError {
    /// A filesystem operation failed.
    #[error("runtime status cache I/O failed at {path}: {source}")]
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// Another publisher already owns the exclusive writer lock.
    #[error("runtime status cache writer lock is already held at {path}")]
    WriterLockUnavailable {
        /// Stable writer lock path.
        path: PathBuf,
    },
    /// A supported cache value could not be encoded.
    #[error("runtime status cache serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    /// Encoded cache data exceeds its frozen projection bound.
    #[error("runtime status cache projection {projection} is {actual_bytes} bytes and exceeds the {max_bytes} byte limit")]
    ProjectionTooLarge {
        /// Projection filename.
        projection: &'static str,
        /// Encoded byte count.
        actual_bytes: usize,
        /// Frozen maximum byte count.
        max_bytes: usize,
    },
}

/// Exclusive bounded publisher for one product status cache.
#[derive(Debug)]
pub struct FilesystemRuntimeStatusPublisher {
    layout: RuntimeStatusCacheLayout,
    limits: RuntimeStatusCacheLimits,
    actions: Vec<RuntimeActionRecord>,
    pending_action_omission_count: u64,
    writer_lock: File,
}

impl FilesystemRuntimeStatusPublisher {
    /// Acquire the product writer lock with the frozen production limits.
    pub fn acquire(product_root: impl AsRef<Path>) -> Result<Self, RuntimeStatusCacheError> {
        Self::acquire_inner(product_root, RuntimeStatusCacheLimits::default())
    }

    #[cfg(test)]
    fn acquire_with_limits(
        product_root: impl AsRef<Path>,
        limits: RuntimeStatusCacheLimits,
    ) -> Result<Self, RuntimeStatusCacheError> {
        Self::acquire_inner(product_root, limits)
    }

    fn acquire_inner(
        product_root: impl AsRef<Path>,
        limits: RuntimeStatusCacheLimits,
    ) -> Result<Self, RuntimeStatusCacheError> {
        let layout = RuntimeStatusCacheLayout::from_product_root(product_root);
        fs::create_dir_all(&layout.root).map_err(|source| io_error(&layout.root, source))?;
        let writer_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&layout.writer_lock)
            .map_err(|source| io_error(&layout.writer_lock, source))?;
        match writer_lock.try_lock_exclusive() {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::WouldBlock => {
                return Err(RuntimeStatusCacheError::WriterLockUnavailable {
                    path: layout.writer_lock,
                });
            }
            Err(source) => return Err(io_error(&layout.writer_lock, source)),
        }

        cleanup_temporary_files(&layout)?;
        let reader = FilesystemRuntimeStatusReader::from_layout(layout.clone(), limits);
        let actions = reader
            .read_recent_actions(limits.recent_action_max_count)?
            .actions
            .into_iter()
            .map(normalize_action)
            .collect();

        Ok(Self {
            layout,
            limits,
            actions,
            pending_action_omission_count: 0,
            writer_lock,
        })
    }

    /// Return the stable filesystem paths owned by this publisher.
    pub fn layout(&self) -> &RuntimeStatusCacheLayout {
        &self.layout
    }

    fn publish_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), RuntimeStatusCacheError> {
        let mut bounded = record.clone();
        for runtime in &mut bounded.snapshot.runtimes {
            runtime.last_action = runtime.last_action.take().map(normalize_action);
        }
        bounded.recent_actions = bounded
            .recent_actions
            .into_iter()
            .map(normalize_action)
            .collect();
        let original_action_count = bounded.recent_actions.len();
        retain_newest(&mut bounded.recent_actions, self.limits.recent_action_max_count);
        let mut omitted_action_count = original_action_count - bounded.recent_actions.len();
        let encoded = loop {
            let mut candidate = bounded.clone();
            if omitted_action_count != 0 {
                candidate.snapshot.warnings.push(truncated_warning(format!(
                    "latest.json omitted {omitted_action_count} oldest recent actions to satisfy cache bounds"
                )));
            }
            if self.pending_action_omission_count != 0 {
                let omitted = self.pending_action_omission_count;
                candidate.snapshot.warnings.push(truncated_warning(format!(
                    "actions.jsonl omitted {omitted} oldest actions to satisfy count or byte bounds since the prior snapshot"
                )));
            }
            let encoded = serde_json::to_vec(&candidate)?;
            if encoded.len() <= self.limits.latest_max_bytes {
                break encoded;
            }
            if bounded.recent_actions.is_empty() {
                return Err(RuntimeStatusCacheError::ProjectionTooLarge {
                    projection: "latest.json",
                    actual_bytes: encoded.len(),
                    max_bytes: self.limits.latest_max_bytes,
                });
            }
            bounded.recent_actions.remove(0);
            omitted_action_count += 1;
        };
        atomic_replace(&self.layout.latest, &encoded)?;
        self.pending_action_omission_count = 0;
        Ok(())
    }

    fn encode_actions(
        &self,
        actions: &[RuntimeActionRecord],
    ) -> Result<Vec<u8>, RuntimeStatusCacheError> {
        let mut encoded = Vec::new();
        for action in actions {
            serde_json::to_writer(
                &mut encoded,
                &RuntimeStatusActionEnvelope::current(action.clone()),
            )?;
            encoded.push(b'\n');
        }
        Ok(encoded)
    }
}

impl RuntimeStatusPublisher for FilesystemRuntimeStatusPublisher {
    type Error = RuntimeStatusCacheError;

    fn publish_startup_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error> {
        self.publish_snapshot(record)
    }

    fn publish_tick_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error> {
        self.publish_snapshot(record)
    }

    fn publish_action(&mut self, action: &RuntimeActionRecord) -> Result<(), Self::Error> {
        let mut next = self.actions.clone();
        next.push(normalize_action(action.clone()));
        let unbounded_count = next.len();
        retain_newest(&mut next, self.limits.recent_action_max_count);
        let mut omitted_action_count = unbounded_count - next.len();

        let mut encoded = self.encode_actions(&next)?;
        while encoded.len() > self.limits.actions_max_bytes && next.len() > 1 {
            next.remove(0);
            omitted_action_count += 1;
            encoded = self.encode_actions(&next)?;
        }
        enforce_bound(
            "actions.jsonl",
            encoded.len(),
            self.limits.actions_max_bytes,
        )?;
        atomic_replace(&self.layout.actions, &encoded)?;
        self.actions = next;
        self.pending_action_omission_count = self
            .pending_action_omission_count
            .saturating_add(omitted_action_count as u64);
        Ok(())
    }

    fn publish_shutdown_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error> {
        self.publish_snapshot(record)
    }

    fn flush_cache(&mut self) -> Result<(), Self::Error> {
        sync_if_present(&self.layout.latest)?;
        sync_if_present(&self.layout.actions)?;
        self.writer_lock
            .sync_all()
            .map_err(|source| io_error(&self.layout.writer_lock, source))?;
        File::open(&self.layout.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| io_error(&self.layout.root, source))
    }
}

/// Tolerant passive reader for one product status cache.
#[derive(Debug, Clone)]
pub struct FilesystemRuntimeStatusReader {
    layout: RuntimeStatusCacheLayout,
    limits: RuntimeStatusCacheLimits,
}

impl FilesystemRuntimeStatusReader {
    /// Build a reader with the frozen production limits.
    pub fn new(product_root: impl AsRef<Path>) -> Self {
        Self::from_layout(
            RuntimeStatusCacheLayout::from_product_root(product_root),
            RuntimeStatusCacheLimits::default(),
        )
    }

    #[cfg(test)]
    fn with_limits(
        product_root: impl AsRef<Path>,
        limits: RuntimeStatusCacheLimits,
    ) -> Self {
        Self::from_layout(
            RuntimeStatusCacheLayout::from_product_root(product_root),
            limits,
        )
    }

    fn from_layout(layout: RuntimeStatusCacheLayout, limits: RuntimeStatusCacheLimits) -> Self {
        Self { layout, limits }
    }

    /// Return the stable filesystem paths read by this adapter.
    pub fn layout(&self) -> &RuntimeStatusCacheLayout {
        &self.layout
    }
}

impl RuntimeStatusReader for FilesystemRuntimeStatusReader {
    type Error = RuntimeStatusCacheError;

    fn read_latest_snapshot(&self) -> Result<RuntimeStatusSnapshotRead, Self::Error> {
        let bytes = match read_bounded_file(&self.layout.latest, self.limits.latest_max_bytes)? {
            BoundedFileRead::Missing => return Ok(RuntimeStatusSnapshotRead::Missing),
            BoundedFileRead::Oversized { observed_bytes } => {
                return Ok(RuntimeStatusSnapshotRead::Unreadable {
                    warnings: vec![truncated_warning(format!(
                        "latest.json is {observed_bytes} bytes and exceeds the {} byte limit",
                        self.limits.latest_max_bytes
                    ))],
                });
            }
            BoundedFileRead::Bytes(bytes) => bytes,
        };

        let probe = match serde_json::from_slice::<SchemaVersionProbe>(&bytes) {
            Ok(probe) => probe,
            Err(error) => {
                return Ok(RuntimeStatusSnapshotRead::Unreadable {
                    warnings: vec![unreadable_warning(format!(
                        "latest.json could not be decoded: {error}"
                    ))],
                });
            }
        };
        match RuntimeStatusCacheCompatibility::for_schema_version(probe.schema_version) {
            RuntimeStatusCacheCompatibility::UnsupportedFuture => {
                Ok(RuntimeStatusSnapshotRead::UnsupportedVersion {
                    observed_version: probe.schema_version,
                    warnings: vec![unreadable_warning(format!(
                        "latest.json schema {} is newer than supported schema {}",
                        probe.schema_version, RUNTIME_STATUS_CACHE_SCHEMA_VERSION
                    ))],
                })
            }
            compatibility => match serde_json::from_slice::<RuntimeStatusCacheRecord>(&bytes) {
                Ok(record) => {
                    let warnings = compatibility_warning(compatibility, "latest.json");
                    Ok(RuntimeStatusSnapshotRead::Decoded {
                        record: Box::new(record),
                        warnings,
                    })
                }
                Err(error) => Ok(RuntimeStatusSnapshotRead::Unreadable {
                    warnings: vec![unreadable_warning(format!(
                        "latest.json could not be decoded: {error}"
                    ))],
                }),
            },
        }
    }

    fn read_recent_actions(&self, limit: usize) -> Result<RuntimeStatusActionsRead, Self::Error> {
        if limit == 0 {
            return Ok(RuntimeStatusActionsRead::default());
        }
        let read = read_bounded_tail(&self.layout.actions, self.limits.actions_max_bytes)?;
        let mut warnings = Vec::new();
        let bytes = match read {
            BoundedTailRead::Missing => return Ok(RuntimeStatusActionsRead::default()),
            BoundedTailRead::Bytes { bytes, truncated } => {
                if truncated {
                    warnings.push(truncated_warning(format!(
                        "actions.jsonl exceeded the {} byte read limit and its oldest bytes were skipped",
                        self.limits.actions_max_bytes
                    )));
                }
                bytes
            }
        };

        let mut actions = Vec::new();
        let mut malformed_line_count = 0usize;
        let mut older_line_count = 0usize;
        let mut future_line_count = 0usize;
        let retained_limit = limit.min(self.limits.recent_action_max_count);
        for line in bytes.split(|byte| *byte == b'\n') {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let probe = match serde_json::from_slice::<SchemaVersionProbe>(line) {
                Ok(probe) => probe,
                Err(_) => {
                    malformed_line_count += 1;
                    continue;
                }
            };
            let compatibility =
                RuntimeStatusCacheCompatibility::for_schema_version(probe.schema_version);
            match compatibility {
                RuntimeStatusCacheCompatibility::UnsupportedFuture => {
                    future_line_count += 1;
                    continue;
                }
                RuntimeStatusCacheCompatibility::Older
                | RuntimeStatusCacheCompatibility::Current => {}
            }
            match serde_json::from_slice::<RuntimeStatusActionEnvelope>(line) {
                Ok(envelope) => {
                    if compatibility == RuntimeStatusCacheCompatibility::Older {
                        older_line_count += 1;
                    }
                    actions.push(normalize_action(envelope.action));
                    retain_newest(&mut actions, retained_limit);
                }
                Err(_) => malformed_line_count += 1,
            }
        }
        if older_line_count != 0 {
            warnings.push(older_schema_warning(format!(
                "{older_line_count} actions.jsonl lines use compatible older schemas"
            )));
        }
        if future_line_count != 0 {
            warnings.push(unreadable_warning(format!(
                "{future_line_count} actions.jsonl lines use unsupported future schemas"
            )));
        }
        if malformed_line_count != 0 {
            warnings.push(unreadable_warning(format!(
                "{malformed_line_count} malformed or partial actions.jsonl lines were skipped"
            )));
        }
        Ok(RuntimeStatusActionsRead { actions, warnings })
    }
}

#[derive(Debug, Deserialize)]
struct SchemaVersionProbe {
    #[serde(default)]
    schema_version: u16,
}

enum BoundedFileRead {
    Missing,
    Bytes(Vec<u8>),
    Oversized { observed_bytes: u64 },
}

enum BoundedTailRead {
    Missing,
    Bytes { bytes: Vec<u8>, truncated: bool },
}

fn read_bounded_file(
    path: &Path,
    max_bytes: usize,
) -> Result<BoundedFileRead, RuntimeStatusCacheError> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(BoundedFileRead::Missing);
        }
        Err(source) => return Err(io_error(path, source)),
    };
    let observed_bytes = file
        .metadata()
        .map_err(|source| io_error(path, source))?
        .len();
    if observed_bytes > max_bytes as u64 {
        return Ok(BoundedFileRead::Oversized { observed_bytes });
    }
    let mut bytes = Vec::with_capacity(observed_bytes as usize);
    file.read_to_end(&mut bytes)
        .map_err(|source| io_error(path, source))?;
    if bytes.len() > max_bytes {
        return Ok(BoundedFileRead::Oversized {
            observed_bytes: bytes.len() as u64,
        });
    }
    Ok(BoundedFileRead::Bytes(bytes))
}

fn read_bounded_tail(
    path: &Path,
    max_bytes: usize,
) -> Result<BoundedTailRead, RuntimeStatusCacheError> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(BoundedTailRead::Missing);
        }
        Err(source) => return Err(io_error(path, source)),
    };
    let observed_bytes = file
        .metadata()
        .map_err(|source| io_error(path, source))?
        .len();
    let truncated = observed_bytes > max_bytes as u64;
    if truncated {
        file.seek(SeekFrom::Start(observed_bytes - max_bytes as u64))
            .map_err(|source| io_error(path, source))?;
    }
    let mut bytes = Vec::with_capacity(observed_bytes.min(max_bytes as u64) as usize);
    file.read_to_end(&mut bytes)
        .map_err(|source| io_error(path, source))?;
    if bytes.len() > max_bytes {
        bytes.drain(..bytes.len() - max_bytes);
    }
    if truncated {
        if let Some(first_line_end) = bytes.iter().position(|byte| *byte == b'\n') {
            bytes.drain(..=first_line_end);
        } else {
            bytes.clear();
        }
    }
    Ok(BoundedTailRead::Bytes { bytes, truncated })
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), RuntimeStatusCacheError> {
    let parent = path.parent().ok_or_else(|| {
        io_error(
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "cache path has no parent"),
        )
    })?;
    fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    let temporary = RuntimeStatusCacheLayout::temporary_path(
        path,
        std::process::id(),
        TEMPORARY_FILE_NONCE.fetch_add(1, Ordering::Relaxed),
    );
    let mut replacement = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|source| io_error(&temporary, source))?;
    let result = replacement
        .write_all(bytes)
        .and_then(|_| replacement.sync_all())
        .map_err(|source| io_error(&temporary, source))
        .and_then(|_| fs::rename(&temporary, path).map_err(|source| io_error(path, source)))
        .and_then(|_| {
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|source| io_error(parent, source))
        });
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn cleanup_temporary_files(
    layout: &RuntimeStatusCacheLayout,
) -> Result<(), RuntimeStatusCacheError> {
    for entry in fs::read_dir(&layout.root).map_err(|source| io_error(&layout.root, source))? {
        let entry = entry.map_err(|source| io_error(&layout.root, source))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let owned_temporary = name.starts_with("latest.json.")
            || name.starts_with("actions.jsonl.");
        if owned_temporary && name.ends_with(".tmp") {
            fs::remove_file(entry.path()).map_err(|source| io_error(&entry.path(), source))?;
        }
    }
    Ok(())
}

fn sync_if_present(path: &Path) -> Result<(), RuntimeStatusCacheError> {
    match File::open(path) {
        Ok(file) => file.sync_all().map_err(|source| io_error(path, source)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(path, source)),
    }
}

fn retain_newest<T>(values: &mut Vec<T>, limit: usize) {
    if values.len() > limit {
        values.drain(..values.len() - limit);
    }
}

fn normalize_action(mut action: RuntimeActionRecord) -> RuntimeActionRecord {
    let original_issue_count = action.issues.len();
    let prior_omitted_issue_count = action.truncation.omitted_issue_count;
    let mut fatal = Vec::new();
    let mut retryable = Vec::new();
    for issue in std::mem::take(&mut action.issues) {
        match issue.severity {
            RuntimeActionIssueSeverity::Fatal => fatal.push(issue),
            RuntimeActionIssueSeverity::Retryable => retryable.push(issue),
        }
    }
    fatal.extend(retryable);
    fatal.truncate(RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT);

    let mut truncated_message_count = 0u64;
    for issue in &mut fatal {
        let was_truncated = issue.truncated;
        let bounded = bounded_utf8(&issue.message, RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES);
        issue.message = bounded.0;
        issue.truncated = was_truncated || bounded.1;
        truncated_message_count += u64::from(issue.truncated);
    }
    action.truncation = RuntimeActionTruncation {
        omitted_issue_count: prior_omitted_issue_count.saturating_add(
            original_issue_count
                .saturating_sub(fatal.len())
                .try_into()
                .unwrap_or(u64::MAX),
        ),
        truncated_message_count,
    };
    action.issues = fatal;
    action
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

fn enforce_bound(
    projection: &'static str,
    actual_bytes: usize,
    max_bytes: usize,
) -> Result<(), RuntimeStatusCacheError> {
    if actual_bytes > max_bytes {
        Err(RuntimeStatusCacheError::ProjectionTooLarge {
            projection,
            actual_bytes,
            max_bytes,
        })
    } else {
        Ok(())
    }
}

fn compatibility_warning(
    compatibility: RuntimeStatusCacheCompatibility,
    projection: &str,
) -> Vec<RuntimeStatusCacheWarning> {
    match compatibility {
        RuntimeStatusCacheCompatibility::Older => vec![older_schema_warning(format!(
            "{projection} uses an older compatible schema"
        ))],
        RuntimeStatusCacheCompatibility::Current
        | RuntimeStatusCacheCompatibility::UnsupportedFuture => Vec::new(),
    }
}

fn older_schema_warning(message: String) -> RuntimeStatusCacheWarning {
    RuntimeStatusCacheWarning {
        code: RUNTIME_STATUS_WARNING_OLDER_SCHEMA.to_string(),
        message,
    }
}

fn truncated_warning(message: String) -> RuntimeStatusCacheWarning {
    RuntimeStatusCacheWarning {
        code: RUNTIME_STATUS_WARNING_TRUNCATED.to_string(),
        message,
    }
}

fn unreadable_warning(message: String) -> RuntimeStatusCacheWarning {
    RuntimeStatusCacheWarning {
        code: RUNTIME_STATUS_WARNING_UNREADABLE.to_string(),
        message,
    }
}

fn io_error(path: &Path, source: io::Error) -> RuntimeStatusCacheError {
    RuntimeStatusCacheError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;
    use crate::runtime::contracts::{
        RuntimeActionCause, RuntimeActionIssueSeverity, RuntimeActionIssueSummary,
        RuntimeActionKind, RuntimeActionMetrics, RuntimeActionOutcome, RuntimeActionTruncation,
        RuntimeCheckpointObservation, RuntimeLaunchStatus, RuntimeObjectRef, RuntimeRedactionState,
        RuntimeRunMode, RuntimeStatusCacheState, RuntimeStatusHealthCounts, RuntimeStatusReadRequest,
        RuntimeStatusSnapshot, RuntimeStatusWriterIdentity,
    };

    fn action(index: usize) -> RuntimeActionRecord {
        RuntimeActionRecord {
            action_id: format!("action-{index:03}"),
            observed_at_ms: index as u64,
            runtime_id: "world_model.graph_replay".to_string(),
            domain_id: "world_model".to_string(),
            actor_id: "graph_replay".to_string(),
            object_ref: RuntimeObjectRef {
                domain_id: "world_model".to_string(),
                object_type: "event".to_string(),
                object_id: format!("event-{index:03}"),
                branch_id: None,
                perspective_key: None,
                parent_refs: Vec::new(),
                source_refs: Vec::new(),
            },
            action_kind: RuntimeActionKind::Replay,
            cause: RuntimeActionCause::ReplayBatch,
            outcome: RuntimeActionOutcome::Succeeded,
            metrics: RuntimeActionMetrics {
                attempted: 1,
                committed: 1,
                retryable_issue_count: 0,
                fatal_issue_count: 0,
                budget_exhausted: false,
            },
            checkpoints: vec![RuntimeCheckpointObservation {
                input_name: "sequence".to_string(),
                output_name: "sequence".to_string(),
                input_value: index.saturating_sub(1) as u64,
                output_value: index as u64,
            }],
            issues: Vec::new(),
            truncation: RuntimeActionTruncation::default(),
            redaction: RuntimeRedactionState::NotNeeded,
        }
    }

    fn unbounded_action(index: usize) -> RuntimeActionRecord {
        let mut action = action(index);
        action.issues = (0..20)
            .map(|issue_index| RuntimeActionIssueSummary {
                severity: RuntimeActionIssueSeverity::Retryable,
                item_id: Some(format!("retryable-{issue_index}")),
                code: "retryable".to_string(),
                message: "é".repeat(600),
                truncated: false,
            })
            .chain((0..2).map(|issue_index| RuntimeActionIssueSummary {
                severity: RuntimeActionIssueSeverity::Fatal,
                item_id: Some(format!("fatal-{issue_index}")),
                code: "fatal".to_string(),
                message: "é".repeat(600),
                truncated: false,
            }))
            .collect();
        action.truncation.omitted_issue_count = 3;
        action
    }

    fn record(root: &Path, written_at_ms: u64) -> RuntimeStatusCacheRecord {
        let layout = RuntimeStatusCacheLayout::from_product_root(root);
        RuntimeStatusCacheRecord::new(
            root,
            root.join("runtime/supervisor.sled"),
            layout.root,
            RuntimeStatusWriterIdentity {
                instance_id: Some("instance-a".to_string()),
                process_id: Some(std::process::id()),
                parent_process_id: None,
                run_mode: RuntimeRunMode::Foreground,
                launch_status: RuntimeLaunchStatus::Ready,
            },
            RuntimeStatusSnapshot {
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
            },
            Vec::new(),
            written_at_ms,
        )
    }

    #[test]
    fn publisher_writes_compact_snapshot_and_reader_derives_staleness() {
        let temp = TempDir::new().unwrap();
        let mut publisher = FilesystemRuntimeStatusPublisher::acquire(temp.path()).unwrap();
        publisher
            .publish_startup_snapshot(&record(temp.path(), 10))
            .unwrap();
        publisher.flush_cache().unwrap();

        let bytes = fs::read(&publisher.layout().latest).unwrap();
        assert!(!bytes.contains(&b'\n'));
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["schema_version"], 1);

        let read = FilesystemRuntimeStatusReader::new(temp.path())
            .read_status(RuntimeStatusReadRequest {
                now_ms: 30,
                stale_after_ms: 5,
                recent_action_limit: 10,
            })
            .unwrap();
        assert_eq!(
            read.cache_state,
            RuntimeStatusCacheState::Stale {
                age_ms: 20,
                stale_after_ms: 5,
            }
        );
    }

    #[test]
    fn publisher_lock_is_exclusive_and_released_on_drop() {
        let temp = TempDir::new().unwrap();
        let publisher = FilesystemRuntimeStatusPublisher::acquire(temp.path()).unwrap();
        let conflict = FilesystemRuntimeStatusPublisher::acquire(temp.path()).unwrap_err();
        assert!(matches!(
            conflict,
            RuntimeStatusCacheError::WriterLockUnavailable { .. }
        ));
        drop(publisher);
        FilesystemRuntimeStatusPublisher::acquire(temp.path()).unwrap();
    }

    #[test]
    fn action_file_retains_the_newest_bounded_window() {
        let temp = TempDir::new().unwrap();
        let limits = RuntimeStatusCacheLimits {
            recent_action_max_count: 3,
            ..RuntimeStatusCacheLimits::default()
        };
        let mut publisher =
            FilesystemRuntimeStatusPublisher::acquire_with_limits(temp.path(), limits).unwrap();
        for index in 0..5 {
            publisher.publish_action(&action(index)).unwrap();
        }

        let read = FilesystemRuntimeStatusReader::with_limits(temp.path(), limits)
            .read_recent_actions(10)
            .unwrap();
        assert_eq!(
            read.actions
                .iter()
                .map(|action| action.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["action-002", "action-003", "action-004"]
        );
        assert!(fs::metadata(&publisher.layout().actions).unwrap().len()
            <= limits.actions_max_bytes as u64);
    }

    #[test]
    fn publisher_normalizes_arbitrary_action_issues_with_fatal_priority() {
        let temp = TempDir::new().unwrap();
        let mut publisher = FilesystemRuntimeStatusPublisher::acquire(temp.path()).unwrap();
        publisher.publish_action(&unbounded_action(1)).unwrap();

        let read = FilesystemRuntimeStatusReader::new(temp.path())
            .read_recent_actions(10)
            .unwrap();
        let action = &read.actions[0];
        assert_eq!(action.issues.len(), RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT);
        assert_eq!(action.issues[0].severity, RuntimeActionIssueSeverity::Fatal);
        assert_eq!(action.issues[1].severity, RuntimeActionIssueSeverity::Fatal);
        assert!(action
            .issues
            .iter()
            .all(|issue| issue.message.len() <= RUNTIME_STATUS_ISSUE_MESSAGE_MAX_BYTES));
        assert!(action.issues.iter().all(|issue| issue.truncated));
        assert_eq!(action.truncation.omitted_issue_count, 9);
        assert_eq!(action.truncation.truncated_message_count, 16);
    }

    #[test]
    fn snapshot_normalizes_embedded_recent_actions() {
        let temp = TempDir::new().unwrap();
        let mut publisher = FilesystemRuntimeStatusPublisher::acquire(temp.path()).unwrap();
        let mut snapshot = record(temp.path(), 10);
        snapshot.recent_actions = vec![unbounded_action(1)];
        publisher.publish_tick_snapshot(&snapshot).unwrap();

        let RuntimeStatusSnapshotRead::Decoded { record, .. } =
            FilesystemRuntimeStatusReader::new(temp.path())
                .read_latest_snapshot()
                .unwrap()
        else {
            panic!("snapshot should decode");
        };
        let action = &record.recent_actions[0];
        assert_eq!(action.issues.len(), RUNTIME_STATUS_ACTION_ISSUE_MAX_COUNT);
        assert_eq!(action.issues[0].severity, RuntimeActionIssueSeverity::Fatal);
        assert_eq!(action.truncation.omitted_issue_count, 9);
        assert_eq!(action.truncation.truncated_message_count, 16);
    }

    #[test]
    fn action_count_retention_is_reported_by_the_next_snapshot() {
        let temp = TempDir::new().unwrap();
        let limits = RuntimeStatusCacheLimits {
            recent_action_max_count: 2,
            ..RuntimeStatusCacheLimits::default()
        };
        let mut publisher =
            FilesystemRuntimeStatusPublisher::acquire_with_limits(temp.path(), limits).unwrap();
        for index in 0..3 {
            publisher.publish_action(&action(index)).unwrap();
        }
        publisher
            .publish_tick_snapshot(&record(temp.path(), 10))
            .unwrap();

        let RuntimeStatusSnapshotRead::Decoded {
            record: decoded, ..
        } =
            FilesystemRuntimeStatusReader::with_limits(temp.path(), limits)
                .read_latest_snapshot()
                .unwrap()
        else {
            panic!("snapshot should decode");
        };
        assert!(decoded.snapshot.warnings.iter().any(|warning| {
            warning.code == RUNTIME_STATUS_WARNING_TRUNCATED
                && warning.message.contains("actions.jsonl omitted 1 oldest actions")
        }));

        publisher
            .publish_tick_snapshot(&record(temp.path(), 20))
            .unwrap();
        let RuntimeStatusSnapshotRead::Decoded {
            record: decoded, ..
        } =
            FilesystemRuntimeStatusReader::with_limits(temp.path(), limits)
                .read_latest_snapshot()
                .unwrap()
        else {
            panic!("snapshot should decode");
        };
        assert!(!decoded.snapshot.warnings.iter().any(|warning| {
            warning.message.contains("since the prior snapshot")
        }));
    }

    #[test]
    fn action_reader_skips_malformed_partial_and_future_lines() {
        let temp = TempDir::new().unwrap();
        let layout = RuntimeStatusCacheLayout::from_product_root(temp.path());
        fs::create_dir_all(&layout.root).unwrap();
        let current = serde_json::to_string(&RuntimeStatusActionEnvelope::current(action(1)))
            .unwrap();
        let mut older = serde_json::to_value(RuntimeStatusActionEnvelope::current(action(2)))
            .unwrap();
        older.as_object_mut().unwrap().remove("schema_version");
        let mut future = serde_json::to_value(RuntimeStatusActionEnvelope::current(action(3)))
            .unwrap();
        future["schema_version"] = Value::from(99);
        fs::write(
            &layout.actions,
            format!(
                "{current}\nnot-json\n{}\n{}\n{{\"schema_version\":1",
                serde_json::to_string(&older).unwrap(),
                serde_json::to_string(&future).unwrap()
            ),
        )
        .unwrap();

        let read = FilesystemRuntimeStatusReader::new(temp.path())
            .read_recent_actions(10)
            .unwrap();
        assert_eq!(
            read.actions
                .iter()
                .map(|action| action.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["action-001", "action-002"]
        );
        assert_eq!(read.warnings.len(), 3);
        assert!(read
            .warnings
            .iter()
            .any(|warning| warning.code == RUNTIME_STATUS_WARNING_OLDER_SCHEMA));
    }

    #[test]
    fn snapshot_reader_supports_version_zero_and_rejects_future_versions() {
        let temp = TempDir::new().unwrap();
        let layout = RuntimeStatusCacheLayout::from_product_root(temp.path());
        fs::create_dir_all(&layout.root).unwrap();
        let mut older = serde_json::to_value(record(temp.path(), 10)).unwrap();
        older.as_object_mut().unwrap().remove("schema_version");
        fs::write(&layout.latest, serde_json::to_vec(&older).unwrap()).unwrap();

        let reader = FilesystemRuntimeStatusReader::new(temp.path());
        let older_read = reader.read_latest_snapshot().unwrap();
        assert!(matches!(
            older_read,
            RuntimeStatusSnapshotRead::Decoded { warnings, .. }
                if warnings.len() == 1
                    && warnings[0].code == RUNTIME_STATUS_WARNING_OLDER_SCHEMA
        ));

        older["schema_version"] = Value::from(99);
        fs::write(&layout.latest, serde_json::to_vec(&older).unwrap()).unwrap();
        assert!(matches!(
            reader.read_latest_snapshot().unwrap(),
            RuntimeStatusSnapshotRead::UnsupportedVersion {
                observed_version: 99,
                ..
            }
        ));
    }

    #[test]
    fn future_snapshot_is_rejected_before_typed_payload_decode() {
        let temp = TempDir::new().unwrap();
        let layout = RuntimeStatusCacheLayout::from_product_root(temp.path());
        fs::create_dir_all(&layout.root).unwrap();
        fs::write(
            &layout.latest,
            br#"{"schema_version":99,"not_a_cache_record":true}"#,
        )
        .unwrap();

        assert!(matches!(
            FilesystemRuntimeStatusReader::new(temp.path())
                .read_latest_snapshot()
                .unwrap(),
            RuntimeStatusSnapshotRead::UnsupportedVersion {
                observed_version: 99,
                ..
            }
        ));
    }

    #[test]
    fn snapshot_drops_oldest_embedded_actions_and_records_truncation() {
        let temp = TempDir::new().unwrap();
        let base = record(temp.path(), 10);
        let base_size = serde_json::to_vec(&base).unwrap().len();
        let limits = RuntimeStatusCacheLimits {
            latest_max_bytes: base_size + 1_000,
            ..RuntimeStatusCacheLimits::default()
        };
        let mut publisher =
            FilesystemRuntimeStatusPublisher::acquire_with_limits(temp.path(), limits).unwrap();
        let mut crowded = base;
        crowded.recent_actions = (0..8).map(action).collect();
        publisher.publish_tick_snapshot(&crowded).unwrap();

        let read = FilesystemRuntimeStatusReader::with_limits(temp.path(), limits)
            .read_latest_snapshot()
            .unwrap();
        let RuntimeStatusSnapshotRead::Decoded { record, .. } = read else {
            panic!("bounded snapshot should decode");
        };
        assert!(!record.recent_actions.is_empty());
        assert!(record.recent_actions.len() < 8);
        assert_eq!(record.recent_actions.last().unwrap().action_id, "action-007");
        assert!(record
            .snapshot
            .warnings
            .iter()
            .any(|warning| warning.code == RUNTIME_STATUS_WARNING_TRUNCATED));
    }

    #[test]
    fn oversized_snapshot_preserves_last_good_file() {
        let temp = TempDir::new().unwrap();
        let limits = RuntimeStatusCacheLimits {
            latest_max_bytes: 600,
            ..RuntimeStatusCacheLimits::default()
        };
        let mut publisher =
            FilesystemRuntimeStatusPublisher::acquire_with_limits(temp.path(), limits).unwrap();
        let good = record(temp.path(), 10);
        publisher.publish_tick_snapshot(&good).unwrap();
        let before = fs::read(&publisher.layout().latest).unwrap();

        let mut oversized = record(temp.path(), 20);
        oversized.snapshot.warnings.push(RuntimeStatusCacheWarning {
            code: "large".to_string(),
            message: "x".repeat(1_000),
        });
        assert!(matches!(
            publisher.publish_tick_snapshot(&oversized),
            Err(RuntimeStatusCacheError::ProjectionTooLarge { .. })
        ));
        assert_eq!(fs::read(&publisher.layout().latest).unwrap(), before);
    }

    #[test]
    fn oversized_action_drops_oldest_records_before_replacement() {
        let temp = TempDir::new().unwrap();
        let limits = RuntimeStatusCacheLimits {
            actions_max_bytes: 1_050,
            ..RuntimeStatusCacheLimits::default()
        };
        let mut publisher =
            FilesystemRuntimeStatusPublisher::acquire_with_limits(temp.path(), limits).unwrap();
        for index in 0..4 {
            publisher.publish_action(&action(index)).unwrap();
        }
        let read = FilesystemRuntimeStatusReader::with_limits(temp.path(), limits)
            .read_recent_actions(256)
            .unwrap();
        assert!(!read.actions.is_empty());
        assert_eq!(read.actions.last().unwrap().action_id, "action-003");
        assert!(fs::metadata(&publisher.layout().actions).unwrap().len() <= 1_050);

        publisher
            .publish_tick_snapshot(&record(temp.path(), 10))
            .unwrap();
        let RuntimeStatusSnapshotRead::Decoded {
            record: decoded, ..
        } = FilesystemRuntimeStatusReader::with_limits(temp.path(), limits)
            .read_latest_snapshot()
            .unwrap()
        else {
            panic!("snapshot should decode");
        };
        assert!(decoded.snapshot.warnings.iter().any(|warning| {
            warning.code == RUNTIME_STATUS_WARNING_TRUNCATED
                && warning.message.contains("actions.jsonl omitted")
        }));
    }

    #[test]
    fn oversized_action_file_reads_only_a_bounded_legacy_tail() {
        let temp = TempDir::new().unwrap();
        let layout = RuntimeStatusCacheLayout::from_product_root(temp.path());
        fs::create_dir_all(&layout.root).unwrap();
        let mut older = serde_json::to_value(RuntimeStatusActionEnvelope::current(action(7)))
            .unwrap();
        older.as_object_mut().unwrap().remove("schema_version");
        let older = serde_json::to_vec(&older).unwrap();
        let mut bytes = vec![b'x'; 2_000];
        bytes.push(b'\n');
        bytes.extend_from_slice(&older);
        bytes.push(b'\n');
        fs::write(&layout.actions, bytes).unwrap();
        let limits = RuntimeStatusCacheLimits {
            actions_max_bytes: older.len() + 32,
            ..RuntimeStatusCacheLimits::default()
        };

        let read = FilesystemRuntimeStatusReader::with_limits(temp.path(), limits)
            .read_recent_actions(10)
            .unwrap();
        assert_eq!(read.actions.len(), 1);
        assert_eq!(read.actions[0].action_id, "action-007");
        assert!(read
            .warnings
            .iter()
            .any(|warning| warning.code == RUNTIME_STATUS_WARNING_TRUNCATED));
        assert!(read
            .warnings
            .iter()
            .any(|warning| warning.code == RUNTIME_STATUS_WARNING_OLDER_SCHEMA));
    }

    #[test]
    fn failed_oversized_action_preserves_last_good_file() {
        let temp = TempDir::new().unwrap();
        let limits = RuntimeStatusCacheLimits {
            actions_max_bytes: 800,
            ..RuntimeStatusCacheLimits::default()
        };
        let mut publisher =
            FilesystemRuntimeStatusPublisher::acquire_with_limits(temp.path(), limits).unwrap();
        publisher.publish_action(&action(1)).unwrap();
        let before = fs::read(&publisher.layout().actions).unwrap();
        let mut huge = action(2);
        huge.issues.push(RuntimeActionIssueSummary {
            severity: RuntimeActionIssueSeverity::Fatal,
            item_id: None,
            code: "huge".to_string(),
            message: "x".repeat(2_000),
            truncated: false,
        });
        assert!(matches!(
            publisher.publish_action(&huge),
            Err(RuntimeStatusCacheError::ProjectionTooLarge { .. })
        ));
        assert_eq!(fs::read(&publisher.layout().actions).unwrap(), before);
    }

    #[test]
    fn acquisition_cleans_only_owned_temporary_files() {
        let temp = TempDir::new().unwrap();
        let layout = RuntimeStatusCacheLayout::from_product_root(temp.path());
        fs::create_dir_all(&layout.root).unwrap();
        let latest_temp = layout.root.join("latest.json.1-1.tmp");
        let actions_temp = layout.root.join("actions.jsonl.1-2.tmp");
        let process_temp = layout.root.join("process.json.1-3.tmp");
        fs::write(&latest_temp, b"partial").unwrap();
        fs::write(&actions_temp, b"partial").unwrap();
        fs::write(&process_temp, b"other owner").unwrap();

        let _publisher = FilesystemRuntimeStatusPublisher::acquire(temp.path()).unwrap();
        assert!(!latest_temp.exists());
        assert!(!actions_temp.exists());
        assert!(process_temp.exists());
    }
}
