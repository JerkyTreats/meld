//! Durable workflow state storage for thread turn gate and prompt link records.

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
    pub fn new(workspace_root: &Path) -> Result<Self, ApiError> {
        let fallback_root = fallback_workspace_data_dir(workspace_root).join("workflow");
        let root = match crate::config::xdg::workspace_data_dir(workspace_root) {
            Ok(data_dir) => {
                let primary = data_dir.join("workflow");
                if ensure_root_directories(&primary).is_ok() {
                    primary
                } else {
                    ensure_root_directories(&fallback_root)?;
                    fallback_root.clone()
                }
            }
            Err(_) => {
                ensure_root_directories(&fallback_root)?;
                fallback_root.clone()
            }
        };
        Ok(Self { root })
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

fn fallback_workspace_data_dir(workspace_root: &Path) -> PathBuf {
    let canonical = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let mut data_dir = std::env::temp_dir().join("meld");

    for component in canonical.components() {
        match component {
            std::path::Component::RootDir => {}
            std::path::Component::Prefix(_) => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {}
            std::path::Component::Normal(name) => {
                data_dir = data_dir.join(name);
            }
        }
    }

    data_dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::record_contracts::{
        PromptLinkRecordV1, ThreadTurnGateRecordV1, WORKFLOW_RECORD_SCHEMA_VERSION_V1,
    };
    use tempfile::TempDir;

    fn valid_prompt_link_record() -> PromptLinkRecordV1 {
        let digest = "a".repeat(64);
        PromptLinkRecordV1 {
            schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
            prompt_link_id: "prompt-link-aaaaaaaaaaaaaaaa".to_string(),
            thread_id: "thread-aaaaaaaaaaaaaaaa".to_string(),
            turn_id: "turn-1".to_string(),
            node_id: digest.clone(),
            frame_id: digest.clone(),
            system_prompt_artifact_id: digest.clone(),
            user_prompt_template_artifact_id: digest.clone(),
            rendered_prompt_artifact_id: digest.clone(),
            context_artifact_id: digest,
            created_at_ms: 1,
        }
    }

    #[test]
    fn thread_and_turn_records_round_trip() {
        let temp = TempDir::new().unwrap();
        let store = WorkflowStateStore::new(temp.path()).unwrap();

        let thread = WorkflowThreadRecord {
            thread_id: "thread-aaaaaaaaaaaaaaaa".to_string(),
            workflow_id: "docs_writer_thread_v1".to_string(),
            node_id: "b".repeat(64),
            frame_type: "context-docs-writer".to_string(),
            status: WorkflowThreadStatus::Running,
            next_turn_seq: 2,
            updated_at_ms: 1,
        };
        store.upsert_thread(&thread).unwrap();

        let turn = WorkflowTurnRecord {
            thread_id: thread.thread_id.clone(),
            turn_id: "evidence_gather".to_string(),
            seq: 1,
            output_type: "evidence_map".to_string(),
            status: WorkflowTurnStatus::Completed,
            attempt_count: 1,
            frame_id: Some("c".repeat(64)),
            output_text: Some("output".to_string()),
            updated_at_ms: 2,
        };
        store.upsert_turn(&turn).unwrap();

        let loaded_thread = store.load_thread(&thread.thread_id).unwrap().unwrap();
        assert_eq!(loaded_thread, thread);

        let turns = store.load_turns(&thread.thread_id).unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0], turn);
    }

    #[test]
    fn gate_and_prompt_link_records_validate_before_write() {
        let temp = TempDir::new().unwrap();
        let store = WorkflowStateStore::new(temp.path()).unwrap();

        let gate = ThreadTurnGateRecordV1::new(
            "thread-aaaaaaaaaaaaaaaa".to_string(),
            "turn-1".to_string(),
            "schema_required_fields".to_string(),
            crate::workflow::record_contracts::GateOutcome::Pass,
            vec![],
            2,
        );

        store
            .upsert_gate("thread-aaaaaaaaaaaaaaaa", "turn-1", &gate)
            .unwrap();

        let prompt_link = valid_prompt_link_record();
        store
            .upsert_prompt_link("thread-aaaaaaaaaaaaaaaa", "turn-1", &prompt_link)
            .unwrap();
    }
}
