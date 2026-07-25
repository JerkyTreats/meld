//! Durable task executor progress storage.
//!
//! The task domain owns executor progress meaning. Root runtime code may
//! choose the concrete sled database, but progress records stay scoped by
//! task run id and carry the executor snapshot intact: this store only adds
//! schema version and run scoping metadata around the snapshot product.

use crate::task::executor::TaskExecutorSnapshot;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error returned by durable task progress operations.
#[derive(Debug, Error)]
pub enum TaskProgressStoreError {
    /// Sled returned an error.
    #[error("task progress store storage error: {0}")]
    Storage(String),
    /// Persisted data could not be decoded or validated.
    #[error("task progress store decode error: {0}")]
    Decode(String),
    /// Progress store invariants were violated during a write.
    #[error("task progress store invariant error: {0}")]
    Invariant(String),
}

const TREE_TASK_PROGRESS_RECORDS: &str = "task_progress_records_v1";
const TASK_PROGRESS_RECORD_SCHEMA_VERSION: u32 = 1;

/// Stored wrapper around one executor snapshot.
///
/// `task_run_id` is the storage key scope and is validated against the
/// contained snapshot on write and read; the snapshot remains the single
/// source of run truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTaskProgressRecord {
    record_schema_version: u32,
    task_run_id: String,
    snapshot: TaskExecutorSnapshot,
}

/// Sled-backed store holding one executor progress snapshot per task run.
///
/// Progress writes are last-write-wins because executor progress is monotone
/// within one run: every save happens at a quiesced step boundary that
/// strictly extends the previous snapshot.
#[derive(Debug, Clone)]
pub struct TaskProgressStore {
    db: sled::Db,
    tree: sled::Tree,
}

impl TaskProgressStore {
    /// Opens the progress store within a caller-owned database.
    pub fn open(db: sled::Db) -> Result<Self, TaskProgressStoreError> {
        let tree = db
            .open_tree(TREE_TASK_PROGRESS_RECORDS)
            .map_err(to_storage)?;
        Ok(Self { db, tree })
    }

    /// Loads the persisted snapshot for one task run, when present.
    pub fn load(
        &self,
        task_run_id: &str,
    ) -> Result<Option<TaskExecutorSnapshot>, TaskProgressStoreError> {
        let Some(raw) = self.tree.get(task_run_id.as_bytes()).map_err(to_storage)? else {
            return Ok(None);
        };
        let stored: StoredTaskProgressRecord = serde_json::from_slice(&raw).map_err(to_decode)?;
        if stored.record_schema_version != TASK_PROGRESS_RECORD_SCHEMA_VERSION {
            return Err(TaskProgressStoreError::Decode(format!(
                "unsupported task progress record schema version '{}'",
                stored.record_schema_version
            )));
        }
        if stored.task_run_id != task_run_id
            || stored.snapshot.init_payload.task_run_context.task_run_id != task_run_id
        {
            return Err(TaskProgressStoreError::Decode(
                "stored task progress run id mismatch".to_string(),
            ));
        }
        Ok(Some(stored.snapshot))
    }

    /// Persists the snapshot for one task run and flushes the database.
    pub fn save(
        &self,
        task_run_id: &str,
        snapshot: &TaskExecutorSnapshot,
    ) -> Result<(), TaskProgressStoreError> {
        if snapshot.init_payload.task_run_context.task_run_id != task_run_id {
            return Err(TaskProgressStoreError::Invariant(format!(
                "snapshot for run '{}' cannot be stored under run '{}'",
                snapshot.init_payload.task_run_context.task_run_id, task_run_id
            )));
        }
        let value = serde_json::to_vec(&StoredTaskProgressRecord {
            record_schema_version: TASK_PROGRESS_RECORD_SCHEMA_VERSION,
            task_run_id: task_run_id.to_string(),
            snapshot: snapshot.clone(),
        })
        .map_err(to_decode)?;
        self.tree
            .insert(task_run_id.as_bytes(), value)
            .map_err(to_storage)?;
        // Flush before returning so a reopen after this call always observes
        // the progress the caller believes is durable.
        self.db.flush().map_err(to_storage)?;
        Ok(())
    }
}

fn to_storage(error: sled::Error) -> TaskProgressStoreError {
    TaskProgressStoreError::Storage(error.to_string())
}

fn to_decode(error: serde_json::Error) -> TaskProgressStoreError {
    TaskProgressStoreError::Decode(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{CompiledTaskRecord, TaskInitializationPayload, TaskRunContext};

    fn snapshot(task_run_id: &str) -> TaskExecutorSnapshot {
        TaskExecutorSnapshot {
            compiled_task: CompiledTaskRecord {
                task_id: "task_docs_writer".to_string(),
                task_version: 1,
                init_slots: vec![],
                capability_instances: vec![],
                dependency_edges: vec![],
            },
            init_payload: TaskInitializationPayload {
                task_id: "task_docs_writer".to_string(),
                compiled_task_ref: "compiled_task_docs_writer".to_string(),
                init_artifacts: vec![],
                task_run_context: TaskRunContext {
                    task_run_id: task_run_id.to_string(),
                    session_id: None,
                    trigger: "test".to_string(),
                },
            },
            invocation_records: vec![],
            expansion_records: vec![],
            completed_instance_ids: vec![],
            started: false,
        }
    }

    fn open_store() -> TaskProgressStore {
        let db = sled::Config::new().temporary(true).open().unwrap();
        TaskProgressStore::open(db).unwrap()
    }

    #[test]
    fn save_and_load_round_trips_the_snapshot() {
        let store = open_store();
        let snapshot = snapshot("taskrun_1");

        store.save("taskrun_1", &snapshot).unwrap();

        assert_eq!(store.load("taskrun_1").unwrap(), Some(snapshot));
        assert_eq!(store.load("taskrun_other").unwrap(), None);
    }

    #[test]
    fn save_overwrites_prior_progress_for_the_same_run() {
        let store = open_store();
        let mut advanced = snapshot("taskrun_1");
        store.save("taskrun_1", &snapshot("taskrun_1")).unwrap();

        advanced.started = true;
        store.save("taskrun_1", &advanced).unwrap();

        assert_eq!(store.load("taskrun_1").unwrap(), Some(advanced));
    }

    #[test]
    fn save_rejects_run_id_drift() {
        let store = open_store();

        let error = store
            .save("taskrun_other", &snapshot("taskrun_1"))
            .unwrap_err();

        assert!(matches!(error, TaskProgressStoreError::Invariant(_)));
    }
}
