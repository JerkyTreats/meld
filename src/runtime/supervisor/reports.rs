//! Durable per-tick report preservation for the runtime supervisor.
//!
//! Owner: root runtime supervisor. This module is the first production
//! implementation of the frozen `RuntimeStatusPublisher` and
//! `RuntimeStatusReader` contracts. Every real bounded tick report becomes
//! one `RuntimeActionRecord` here, so full reports — including checkpoint
//! movement and per-item issues — stay recoverable after the run. The
//! supervisor lifecycle summary derives from these durable records rather
//! than replacing them.
//!
//! Invariants:
//!
//! - Records are observational copies of domain-owned reports. Checkpoint
//!   values here are never read back to drive semantic work; domain stores
//!   remain the only authoritative cursor home. The supervisor lifecycle
//!   trees stay lifecycle-only; these sibling trees are the one sanctioned
//!   observational surface.
//! - One record per actor per bounded invocation, appended under a
//!   monotonic sequence, with deterministic oldest-first pruning beyond the
//!   retention bound so repeated idle ticks cannot flood storage.
//! - Single writer: the owning supervisor instance is the only publisher,
//!   matching the lease discipline of the lifecycle store.

use serde::{de::DeserializeOwned, Serialize};

use crate::runtime::contracts::{
    RuntimeActionRecord, RuntimeActionRecordCompatV1, RuntimeStatusCacheRecord,
    RuntimeStatusCacheRecordCompatV1, RuntimeStatusPublisher, RuntimeStatusReader,
};

use super::store::{SupervisorStore, SupervisorStoreError};

const TREE_TICK_ACTIONS: &str = "runtime_tick_action_records";
const TREE_TICK_ACTIONS_LATEST: &str = "runtime_tick_action_latest_by_runtime";
const TREE_STATUS_SNAPSHOTS: &str = "runtime_status_snapshots";
const LATEST_SNAPSHOT_KEY: &[u8] = b"latest";

/// Default retention bound for durable per-tick action records.
pub const DEFAULT_MAX_TICK_ACTION_RECORDS: usize = 4096;

/// Sled-backed publisher and reader for preserved per-tick reports.
///
/// Opened over the supervisor store database as sibling trees so one flush
/// boundary covers lifecycle records and preserved reports together.
pub struct SupervisorReportStore {
    actions: sled::Tree,
    latest_by_runtime: sled::Tree,
    snapshots: sled::Tree,
    next_sequence: u64,
    record_count: usize,
    max_records: usize,
}

impl SupervisorReportStore {
    /// Open the report trees beside one supervisor lifecycle store.
    pub fn open(store: &SupervisorStore) -> Result<Self, SupervisorStoreError> {
        let db = store.database();
        let actions = db.open_tree(TREE_TICK_ACTIONS).map_err(to_sled)?;
        let latest_by_runtime = db.open_tree(TREE_TICK_ACTIONS_LATEST).map_err(to_sled)?;
        let snapshots = db.open_tree(TREE_STATUS_SNAPSHOTS).map_err(to_sled)?;
        // Resume the append sequence after the last retained record so a
        // reopened store never overwrites preserved history.
        let next_sequence = match actions.last().map_err(to_sled)? {
            Some((key, _)) => decode_sequence(key.as_ref())? + 1,
            None => 0,
        };
        let record_count = actions.len();
        Ok(Self {
            actions,
            latest_by_runtime,
            snapshots,
            next_sequence,
            record_count,
            max_records: DEFAULT_MAX_TICK_ACTION_RECORDS,
        })
    }

    /// Override the retention bound; used by focused tests.
    pub fn with_max_records(mut self, max_records: usize) -> Self {
        self.max_records = max_records.max(1);
        self
    }

    /// Return the number of retained action records.
    pub fn action_record_count(&self) -> usize {
        self.record_count
    }

    /// Return the latest preserved action record for one runtime id.
    ///
    /// This is the durable report the supervisor lifecycle projection reads;
    /// it survives the retention prune because it is indexed per runtime.
    pub fn latest_action_for_runtime(
        &self,
        runtime_id: &str,
    ) -> Result<Option<RuntimeActionRecord>, SupervisorStoreError> {
        self.latest_by_runtime
            .get(runtime_id.as_bytes())
            .map_err(to_sled)?
            .map(|raw| decode_action(&raw))
            .transpose()
    }

    fn append_action(&mut self, action: &RuntimeActionRecord) -> Result<(), SupervisorStoreError> {
        let encoded = encode(action)?;
        self.actions
            .insert(self.next_sequence.to_be_bytes(), encoded.clone())
            .map_err(to_sled)?;
        self.latest_by_runtime
            .insert(action.runtime_id.as_bytes(), encoded)
            .map_err(to_sled)?;
        self.next_sequence += 1;
        self.record_count += 1;
        // Deterministic oldest-first prune keeps the record window bounded
        // without touching the per-runtime latest index.
        while self.record_count > self.max_records {
            let Some((oldest_key, _)) = self.actions.first().map_err(to_sled)? else {
                break;
            };
            self.actions.remove(oldest_key).map_err(to_sled)?;
            self.record_count -= 1;
        }
        Ok(())
    }

    fn put_latest_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), SupervisorStoreError> {
        self.snapshots
            .insert(LATEST_SNAPSHOT_KEY, encode(record)?)
            .map_err(to_sled)?;
        Ok(())
    }
}

impl RuntimeStatusPublisher for SupervisorReportStore {
    type Error = SupervisorStoreError;

    fn publish_startup_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error> {
        self.put_latest_snapshot(record)
    }

    fn publish_tick_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error> {
        self.put_latest_snapshot(record)
    }

    fn publish_action(&mut self, action: &RuntimeActionRecord) -> Result<(), Self::Error> {
        self.append_action(action)
    }

    fn publish_shutdown_snapshot(
        &mut self,
        record: &RuntimeStatusCacheRecord,
    ) -> Result<(), Self::Error> {
        self.put_latest_snapshot(record)
    }

    fn flush_cache(&mut self) -> Result<(), Self::Error> {
        self.actions.flush().map_err(to_sled)?;
        self.latest_by_runtime.flush().map_err(to_sled)?;
        self.snapshots.flush().map_err(to_sled)?;
        Ok(())
    }
}

impl RuntimeStatusReader for SupervisorReportStore {
    type Error = SupervisorStoreError;

    fn read_latest_snapshot(&self) -> Result<Option<RuntimeStatusCacheRecord>, Self::Error> {
        self.snapshots
            .get(LATEST_SNAPSHOT_KEY)
            .map_err(to_sled)?
            .map(|raw| decode_snapshot(&raw))
            .transpose()
    }

    /// Read the most recent action records in ascending sequence order.
    fn read_recent_actions(&self, limit: usize) -> Result<Vec<RuntimeActionRecord>, Self::Error> {
        let mut records = self
            .actions
            .iter()
            .rev()
            .take(limit)
            .map(|entry| {
                let (_key, raw) = entry.map_err(to_sled)?;
                decode_action(&raw)
            })
            .collect::<Result<Vec<RuntimeActionRecord>, _>>()?;
        records.reverse();
        Ok(records)
    }
}

fn to_sled(error: impl ToString) -> SupervisorStoreError {
    SupervisorStoreError::Sled(error.to_string())
}

fn encode<T: Serialize>(record: &T) -> Result<Vec<u8>, SupervisorStoreError> {
    bincode::serialize(record).map_err(|error| SupervisorStoreError::Codec(error.to_string()))
}

fn decode<T: DeserializeOwned>(raw: &[u8]) -> Result<T, SupervisorStoreError> {
    bincode::deserialize(raw).map_err(|error| SupervisorStoreError::Codec(error.to_string()))
}

/// Decode one action record, falling back to the pre-waiting-on shape.
///
/// Bincode consumes exact bytes, so a record persisted before DBG-016
/// fails the current-shape decode with trailing-field exhaustion and is
/// unambiguously re-read through the frozen compat mirror; a current
/// record can never mis-decode as the shorter legacy shape because the
/// current shape is tried first.
fn decode_action(raw: &[u8]) -> Result<RuntimeActionRecord, SupervisorStoreError> {
    match bincode::deserialize::<RuntimeActionRecord>(raw) {
        Ok(record) => Ok(record),
        Err(_) => decode::<RuntimeActionRecordCompatV1>(raw).map(RuntimeActionRecord::from),
    }
}

/// Decode one status snapshot, falling back to the shape whose embedded
/// action window predates the waiting-on field.
fn decode_snapshot(raw: &[u8]) -> Result<RuntimeStatusCacheRecord, SupervisorStoreError> {
    match bincode::deserialize::<RuntimeStatusCacheRecord>(raw) {
        Ok(record) => Ok(record),
        Err(_) => {
            decode::<RuntimeStatusCacheRecordCompatV1>(raw).map(RuntimeStatusCacheRecord::from)
        }
    }
}

fn decode_sequence(key: &[u8]) -> Result<u64, SupervisorStoreError> {
    let bytes: [u8; 8] = key.try_into().map_err(|_| {
        SupervisorStoreError::InvalidRecord("action record key is not a u64 sequence".to_string())
    })?;
    Ok(u64::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::runtime::contracts::{
        RuntimeActionOutcome, WorkerCheckpoint, WorkerScope, WorkerTickIssue, WorkerTickReport,
    };

    use super::*;

    fn open_report_store() -> (TempDir, SupervisorReportStore) {
        let temp = tempfile::tempdir().unwrap();
        let store = SupervisorStore::open(temp.path().join("supervisor.sled")).unwrap();
        let reports = SupervisorReportStore::open(&store).unwrap();
        (temp, reports)
    }

    fn worker_report(runtime_id: &str, input: u64, output: u64) -> WorkerTickReport {
        WorkerTickReport {
            actor_id: runtime_id.to_string(),
            scope: WorkerScope {
                domain_id: "events".to_string(),
                stream_id: None,
                work_key: None,
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: input,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: output,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: Vec::new(),
        }
    }

    fn action(
        action_id: &str,
        runtime_id: &str,
        at_ms: u64,
        input: u64,
        output: u64,
    ) -> RuntimeActionRecord {
        RuntimeActionRecord::from_worker_tick(
            action_id,
            runtime_id,
            at_ms,
            worker_report(runtime_id, input, output),
        )
    }

    #[test]
    fn published_actions_round_trip_in_ascending_order() {
        let (_temp, mut reports) = open_report_store();
        reports
            .publish_action(&action("action-1", "event.append", 10, 0, 1))
            .unwrap();
        reports
            .publish_action(&action("action-2", "event.append", 20, 1, 1))
            .unwrap();
        reports
            .publish_action(&action("action-3", "world_model.graph_replay", 20, 0, 2))
            .unwrap();
        reports.flush_cache().unwrap();

        let recent = reports.read_recent_actions(10).unwrap();
        let limited = reports.read_recent_actions(2).unwrap();

        assert_eq!(
            recent
                .iter()
                .map(|record| record.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["action-1", "action-2", "action-3"]
        );
        assert_eq!(recent[0].outcome, RuntimeActionOutcome::Succeeded);
        assert_eq!(recent[1].outcome, RuntimeActionOutcome::NoWork);
        assert_eq!(recent[0].checkpoints[0].input_value, 0);
        assert_eq!(recent[0].checkpoints[0].output_value, 1);
        assert_eq!(
            limited
                .iter()
                .map(|record| record.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["action-2", "action-3"]
        );
    }

    #[test]
    fn latest_action_is_indexed_per_runtime() {
        let (_temp, mut reports) = open_report_store();
        reports
            .publish_action(&action("action-1", "event.append", 10, 0, 1))
            .unwrap();
        reports
            .publish_action(&action("action-2", "event.append", 20, 1, 1))
            .unwrap();

        let latest = reports
            .latest_action_for_runtime("event.append")
            .unwrap()
            .unwrap();

        assert_eq!(latest.action_id, "action-2");
        assert!(reports
            .latest_action_for_runtime("execution.planning")
            .unwrap()
            .is_none());
    }

    #[test]
    fn retention_bound_prunes_oldest_records_deterministically() {
        let (_temp, reports) = open_report_store();
        let mut reports = reports.with_max_records(2);
        for index in 0..4_u64 {
            reports
                .publish_action(&action(
                    &format!("action-{index}"),
                    "event.append",
                    index,
                    index,
                    index,
                ))
                .unwrap();
        }

        let recent = reports.read_recent_actions(10).unwrap();

        assert_eq!(reports.action_record_count(), 2);
        assert_eq!(
            recent
                .iter()
                .map(|record| record.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["action-2", "action-3"]
        );
        // The per-runtime latest index survives the prune.
        assert_eq!(
            reports
                .latest_action_for_runtime("event.append")
                .unwrap()
                .unwrap()
                .action_id,
            "action-3"
        );
    }

    #[test]
    fn reopened_store_resumes_sequence_and_preserves_reports() {
        let temp = tempfile::tempdir().unwrap();
        let store = SupervisorStore::open(temp.path().join("supervisor.sled")).unwrap();
        let mut reports = SupervisorReportStore::open(&store).unwrap();
        reports
            .publish_action(&action("action-1", "event.append", 10, 0, 1))
            .unwrap();
        reports.flush_cache().unwrap();
        drop(reports);
        drop(store);

        let store = SupervisorStore::open(temp.path().join("supervisor.sled")).unwrap();
        let mut reports = SupervisorReportStore::open(&store).unwrap();
        reports
            .publish_action(&action("action-2", "event.append", 20, 1, 1))
            .unwrap();

        let recent = reports.read_recent_actions(10).unwrap();
        assert_eq!(
            recent
                .iter()
                .map(|record| record.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["action-1", "action-2"]
        );
    }

    #[test]
    fn fatal_report_is_preserved_with_full_issue_detail() {
        let (_temp, mut reports) = open_report_store();
        let mut report = worker_report("execution.planning", 3, 3);
        report.fatal_errors.push(WorkerTickIssue {
            item_id: Some("goal-a".to_string()),
            code: "planning_failed".to_string(),
            message: "planning failed".to_string(),
        });
        reports
            .publish_action(&RuntimeActionRecord::from_worker_tick(
                "action-fatal",
                "execution.planning",
                30,
                report,
            ))
            .unwrap();

        let latest = reports
            .latest_action_for_runtime("execution.planning")
            .unwrap()
            .unwrap();

        assert_eq!(latest.outcome, RuntimeActionOutcome::FatalFailure);
        assert_eq!(latest.issues.len(), 1);
        assert_eq!(latest.issues[0].code, "planning_failed");
        assert_eq!(latest.issues[0].item_id.as_deref(), Some("goal-a"));
    }

    /// Freeze one action record into the exact pre-DBG-016 byte shape.
    fn legacy_bytes(record: &RuntimeActionRecord) -> Vec<u8> {
        let legacy = RuntimeActionRecordCompatV1 {
            action_id: record.action_id.clone(),
            observed_at_ms: record.observed_at_ms,
            runtime_id: record.runtime_id.clone(),
            domain_id: record.domain_id.clone(),
            actor_id: record.actor_id.clone(),
            object_ref: record.object_ref.clone(),
            action_kind: record.action_kind.clone(),
            cause: record.cause.clone(),
            outcome: record.outcome.clone(),
            metrics: record.metrics.clone(),
            checkpoints: record.checkpoints.clone(),
            issues: record.issues.clone(),
            redaction: record.redaction.clone(),
        };
        bincode::serialize(&legacy).unwrap()
    }

    #[test]
    fn legacy_action_records_without_waiting_on_decode_unchanged() {
        let (_temp, reports) = open_report_store();
        let expected = action("action-legacy", "event.append", 10, 0, 1);
        assert!(expected.waiting_on.is_empty());
        // Persist the pre-field bytes directly, simulating a store written
        // before the waiting-on field existed.
        let bytes = legacy_bytes(&expected);
        reports
            .actions
            .insert(0u64.to_be_bytes(), bytes.clone())
            .unwrap();
        reports
            .latest_by_runtime
            .insert("event.append".as_bytes(), bytes)
            .unwrap();

        let recent = reports.read_recent_actions(10).unwrap();
        assert_eq!(recent, vec![expected.clone()]);
        assert_eq!(
            reports.latest_action_for_runtime("event.append").unwrap(),
            Some(expected)
        );
    }

    #[test]
    fn a_current_record_with_declarations_round_trips_ahead_of_the_fallback() {
        let (_temp, mut reports) = open_report_store();
        let mut report = worker_report("event.append", 0, 0);
        report
            .waiting_on
            .push(crate::runtime::contracts::WaitingOnDeclaration {
                condition: "graph_anchor_absent".to_string(),
                subject_key: Some("workspace_fs::node::docs".to_string()),
                detail: "no current anchor under frame_type::analysis".to_string(),
            });
        let action = RuntimeActionRecord::from_worker_tick("action-w", "event.append", 10, report);
        reports.publish_action(&action).unwrap();

        let recent = reports.read_recent_actions(10).unwrap();
        assert_eq!(recent[0].waiting_on.len(), 1);
        assert_eq!(recent[0].waiting_on[0].condition, "graph_anchor_absent");
        assert_eq!(
            recent[0].waiting_on[0].subject_key.as_deref(),
            Some("workspace_fs::node::docs")
        );
    }

    #[test]
    fn legacy_snapshots_with_embedded_actions_decode_unchanged() {
        use crate::runtime::contracts::{
            RuntimeStatusCacheRecordCompatV1, RuntimeStatusSnapshot, RuntimeStatusWriterIdentity,
        };
        let (_temp, reports) = open_report_store();
        let embedded = action("action-snap", "event.append", 10, 0, 1);
        let legacy_action: RuntimeActionRecordCompatV1 =
            bincode::deserialize(&legacy_bytes(&embedded)).unwrap();
        let legacy = RuntimeStatusCacheRecordCompatV1 {
            schema_version: 1,
            product_root: "/tmp/product".into(),
            supervisor_store_path: "/tmp/product/supervisor.sled".into(),
            status_cache_path: "/tmp/product/status".into(),
            writer: RuntimeStatusWriterIdentity {
                instance_id: Some("runtime-cli-1".to_string()),
                process_id: Some(42),
                parent_process_id: Some(41),
                run_mode: crate::runtime::contracts::RuntimeRunMode::Foreground,
                launch_status: crate::runtime::contracts::RuntimeLaunchStatus::Ready,
            },
            snapshot: RuntimeStatusSnapshot {
                instance: None,
                process: None,
                shutdown: None,
                runtimes: Vec::new(),
                health_counts: crate::runtime::contracts::RuntimeStatusHealthCounts {
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
            recent_actions: vec![legacy_action],
            written_at_ms: 11,
        };
        reports
            .snapshots
            .insert(LATEST_SNAPSHOT_KEY, bincode::serialize(&legacy).unwrap())
            .unwrap();

        let decoded = reports.read_latest_snapshot().unwrap().unwrap();
        // The stored writer version stays verbatim; only the byte shape is
        // translated forward.
        assert_eq!(decoded.schema_version, 1);
        assert_eq!(decoded.recent_actions, vec![embedded]);
    }
}
