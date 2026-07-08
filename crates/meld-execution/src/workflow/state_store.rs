use crate::error::ApiError;
use crate::workflow::record_contracts::{
    validate_prompt_link_record_v1, validate_thread_turn_gate_record_v1, PromptLinkRecordV1,
    ThreadTurnGateRecordV1,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Workflow thread status contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowThreadStatus {
    /// Work is created but has not started.
    Pending,
    /// Work is actively being executed.
    Running,
    /// Work finished successfully.
    Completed,
    /// Work finished with an execution failure.
    Failed,
}

/// Workflow turn status contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTurnStatus {
    /// Work is created but has not started.
    Pending,
    /// Work is actively being executed.
    Running,
    /// Work finished successfully.
    Completed,
    /// Work finished with an execution failure.
    Failed,
    /// Work was intentionally skipped by workflow policy.
    Skipped,
}

/// Workflow thread record contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowThreadRecord {
    /// Workflow thread identifier within workflow runtime state.
    pub thread_id: String,
    /// Workflow profile identifier that owns this execution record.
    pub workflow_id: String,
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: String,
    /// Context frame type produced or consumed by this execution path.
    pub frame_type: String,
    /// Lifecycle status assigned by the owning runtime.
    pub status: WorkflowThreadStatus,
    /// Next workflow turn sequence to execute when resuming.
    pub next_turn_seq: u32,
    /// Last update time in milliseconds since the Unix epoch.
    pub updated_at_ms: u64,
    /// Final frame identifier produced by the workflow thread when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_frame_id: Option<String>,
}

/// Workflow turn record contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowTurnRecord {
    /// Workflow thread identifier within workflow runtime state.
    pub thread_id: String,
    /// Workflow turn identifier within the owning workflow profile or thread.
    pub turn_id: String,
    /// Workflow turn sequence number used for deterministic ordering.
    pub seq: u32,
    /// Declared output type produced by this workflow turn.
    pub output_type: String,
    /// Lifecycle status assigned by the owning runtime.
    pub status: WorkflowTurnStatus,
    /// Number of execution attempts recorded for this work item.
    pub attempt_count: usize,
    /// Context frame identifier associated with this execution record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<String>,
    /// Captured output text when the turn completed with persisted output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,
    /// Last update time in milliseconds since the Unix epoch.
    pub updated_at_ms: u64,
}

/// Workflow state store contract used by execution runtimes.
#[derive(Debug, Clone)]
pub struct WorkflowStateStore {
    root: PathBuf,
}

impl WorkflowStateStore {
    /// Opens a workflow state store rooted at the supplied directory.
    pub fn from_root(root: &Path) -> Result<Self, ApiError> {
        ensure_root_directories(root)?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Loads one workflow thread record when it exists.
    pub fn load_thread(&self, thread_id: &str) -> Result<Option<WorkflowThreadRecord>, ApiError> {
        let path = self.thread_path(thread_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path).map_err(|err| {
            ApiError::ConfigError(format!("Failed to read thread record: {}", err))
        })?;
        let record = serde_json::from_str::<WorkflowThreadRecord>(&content).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to parse thread record '{}': {}",
                thread_id, err
            ))
        })?;
        Ok(Some(record))
    }

    /// Writes or replaces one workflow thread record.
    pub fn upsert_thread(&self, record: &WorkflowThreadRecord) -> Result<(), ApiError> {
        let path = self.thread_path(&record.thread_id);
        write_json_file(&path, record)
    }

    /// Loads turn records for a thread in sequence order.
    pub fn load_turns(&self, thread_id: &str) -> Result<Vec<WorkflowTurnRecord>, ApiError> {
        let dir = self.turn_dir(thread_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut turns = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to read turn directory '{}': {}",
                dir.display(),
                err
            ))
        })? {
            let entry = entry.map_err(|err| {
                ApiError::ConfigError(format!("Failed to read turn entry: {}", err))
            })?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let content = fs::read_to_string(&path).map_err(|err| {
                ApiError::ConfigError(format!(
                    "Failed to read turn file '{}': {}",
                    path.display(),
                    err
                ))
            })?;
            let record = serde_json::from_str::<WorkflowTurnRecord>(&content).map_err(|err| {
                ApiError::ConfigError(format!(
                    "Failed to parse turn file '{}': {}",
                    path.display(),
                    err
                ))
            })?;
            turns.push(record);
        }
        turns.sort_by_key(|record| record.seq);
        Ok(turns)
    }

    /// Writes or replaces one workflow turn record.
    pub fn upsert_turn(&self, record: &WorkflowTurnRecord) -> Result<(), ApiError> {
        fs::create_dir_all(self.turn_dir(&record.thread_id)).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to create turn directory for thread '{}': {}",
                record.thread_id, err
            ))
        })?;
        let path = self.turn_path(&record.thread_id, &record.turn_id);
        write_json_file(&path, record)
    }

    /// Validates and writes the gate result for one thread turn.
    pub fn upsert_gate(
        &self,
        thread_id: &str,
        turn_id: &str,
        record: &ThreadTurnGateRecordV1,
    ) -> Result<(), ApiError> {
        validate_thread_turn_gate_record_v1(record)?;
        fs::create_dir_all(self.gate_dir(thread_id)).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to create gate directory for thread '{}': {}",
                thread_id, err
            ))
        })?;
        let path = self.gate_path(thread_id, turn_id);
        write_json_file(&path, record)
    }

    /// Validates and writes the prompt link record for one thread turn.
    pub fn upsert_prompt_link(
        &self,
        thread_id: &str,
        turn_id: &str,
        record: &PromptLinkRecordV1,
    ) -> Result<(), ApiError> {
        validate_prompt_link_record_v1(record)?;
        fs::create_dir_all(self.prompt_link_dir(thread_id)).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to create prompt link directory for thread '{}': {}",
                thread_id, err
            ))
        })?;
        let path = self.prompt_link_path(thread_id, turn_id);
        write_json_file(&path, record)
    }

    /// Returns completed turn outputs keyed by output type and turn id.
    pub fn completed_output_map(
        &self,
        thread_id: &str,
    ) -> Result<HashMap<String, String>, ApiError> {
        let turns = self.load_turns(thread_id)?;
        let mut output_map = HashMap::new();
        for turn in turns {
            if turn.status == WorkflowTurnStatus::Completed {
                if let Some(output) = turn.output_text {
                    output_map.insert(turn.output_type.clone(), output.clone());
                    output_map.insert(turn.turn_id.clone(), output);
                }
            }
        }
        Ok(output_map)
    }

    fn thread_path(&self, thread_id: &str) -> PathBuf {
        self.root
            .join("threads")
            .join(format!("{}.json", thread_id))
    }

    fn turn_dir(&self, thread_id: &str) -> PathBuf {
        self.root.join("turns").join(thread_id)
    }

    fn turn_path(&self, thread_id: &str, turn_id: &str) -> PathBuf {
        self.turn_dir(thread_id).join(format!("{}.json", turn_id))
    }

    fn gate_dir(&self, thread_id: &str) -> PathBuf {
        self.root.join("gates").join(thread_id)
    }

    fn gate_path(&self, thread_id: &str, turn_id: &str) -> PathBuf {
        self.gate_dir(thread_id).join(format!("{}.json", turn_id))
    }

    fn prompt_link_dir(&self, thread_id: &str) -> PathBuf {
        self.root.join("prompt_links").join(thread_id)
    }

    fn prompt_link_path(&self, thread_id: &str, turn_id: &str) -> PathBuf {
        self.prompt_link_dir(thread_id)
            .join(format!("{}.json", turn_id))
    }
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), ApiError> {
    let content = serde_json::to_string_pretty(value).map_err(|err| {
        ApiError::ConfigError(format!("Failed to encode workflow record: {}", err))
    })?;
    fs::write(path, content).map_err(|err| {
        ApiError::ConfigError(format!("Failed to write '{}': {}", path.display(), err))
    })
}

fn ensure_root_directories(root: &Path) -> Result<(), ApiError> {
    fs::create_dir_all(root.join("threads"))
        .map_err(|err| ApiError::ConfigError(format!("Failed to create thread store: {}", err)))?;
    fs::create_dir_all(root.join("turns"))
        .map_err(|err| ApiError::ConfigError(format!("Failed to create turn store: {}", err)))?;
    fs::create_dir_all(root.join("gates"))
        .map_err(|err| ApiError::ConfigError(format!("Failed to create gate store: {}", err)))?;
    fs::create_dir_all(root.join("prompt_links")).map_err(|err| {
        ApiError::ConfigError(format!("Failed to create prompt link store: {}", err))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::record_contracts::{
        GateOutcome, PromptLinkRecordV1, ThreadTurnGateRecordV1, WORKFLOW_RECORD_SCHEMA_VERSION_V1,
    };
    use tempfile::tempdir;

    fn hex64(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    #[test]
    fn state_store_persists_and_loads_thread_turn_and_completed_outputs() {
        let dir = tempdir().unwrap();
        let store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let thread = WorkflowThreadRecord {
            thread_id: "thread-1".to_string(),
            workflow_id: "workflow_docs".to_string(),
            node_id: "node-root".to_string(),
            frame_type: "summary".to_string(),
            status: WorkflowThreadStatus::Running,
            next_turn_seq: 2,
            updated_at_ms: 1,
            final_frame_id: None,
        };
        let turn = WorkflowTurnRecord {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            seq: 1,
            output_type: "summary".to_string(),
            status: WorkflowTurnStatus::Completed,
            attempt_count: 1,
            frame_id: Some("frame-1".to_string()),
            output_text: Some("complete".to_string()),
            updated_at_ms: 2,
        };

        store.upsert_thread(&thread).unwrap();
        store.upsert_turn(&turn).unwrap();

        assert_eq!(store.load_thread("thread-1").unwrap(), Some(thread));
        assert_eq!(store.load_turns("thread-1").unwrap(), vec![turn]);
        assert_eq!(
            store.completed_output_map("thread-1").unwrap()["summary"],
            "complete"
        );
        assert_eq!(
            store.completed_output_map("thread-1").unwrap()["turn-1"],
            "complete"
        );
    }

    #[test]
    fn state_store_validates_gate_and_prompt_link_records_before_write() {
        let dir = tempdir().unwrap();
        let store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let gate = ThreadTurnGateRecordV1::new(
            "thread-1".to_string(),
            "turn-1".to_string(),
            "schema_gate".to_string(),
            GateOutcome::Pass,
            vec![],
            1,
        );
        let prompt_link = PromptLinkRecordV1 {
            schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
            prompt_link_id: "prompt-link-1".to_string(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            node_id: hex64('a'),
            frame_id: hex64('b'),
            system_prompt_artifact_id: hex64('c'),
            user_prompt_template_artifact_id: hex64('d'),
            rendered_prompt_artifact_id: hex64('e'),
            context_artifact_id: hex64('f'),
            belief_bundle_digest: None,
            created_at_ms: 1,
        };

        store.upsert_gate("thread-1", "turn-1", &gate).unwrap();
        store
            .upsert_prompt_link("thread-1", "turn-1", &prompt_link)
            .unwrap();
        assert!(dir
            .path()
            .join("gates")
            .join("thread-1")
            .join("turn-1.json")
            .exists());
        assert!(dir
            .path()
            .join("prompt_links")
            .join("thread-1")
            .join("turn-1.json")
            .exists());

        let mut invalid_gate = gate.clone();
        invalid_gate.gate_name.clear();
        assert!(store
            .upsert_gate("thread-1", "turn-2", &invalid_gate)
            .unwrap_err()
            .to_string()
            .contains("gate_name"));
    }
}
