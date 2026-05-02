use crate::error::ApiError;
use crate::workflow::record_contracts::{
    validate_prompt_link_record_v1, validate_thread_turn_gate_record_v1, PromptLinkRecordV1,
    ThreadTurnGateRecordV1,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowThreadStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTurnStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowThreadRecord {
    pub thread_id: String,
    pub workflow_id: String,
    pub node_id: String,
    pub frame_type: String,
    pub status: WorkflowThreadStatus,
    pub next_turn_seq: u32,
    pub updated_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_frame_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowTurnRecord {
    pub thread_id: String,
    pub turn_id: String,
    pub seq: u32,
    pub output_type: String,
    pub status: WorkflowTurnStatus,
    pub attempt_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct WorkflowStateStore {
    root: PathBuf,
}

impl WorkflowStateStore {
    pub fn from_root(root: &Path) -> Result<Self, ApiError> {
        ensure_root_directories(root)?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

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

    pub fn upsert_thread(&self, record: &WorkflowThreadRecord) -> Result<(), ApiError> {
        let path = self.thread_path(&record.thread_id);
        write_json_file(&path, record)
    }

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
