use crate::error::ApiError;
use std::path::{Path, PathBuf};

pub use meld_execution::workflow::state_store::{
    WorkflowThreadRecord, WorkflowThreadStatus, WorkflowTurnRecord, WorkflowTurnStatus,
};

#[derive(Debug, Clone)]
pub struct WorkflowStateStore {
    inner: meld_execution::workflow::WorkflowStateStore,
}

impl WorkflowStateStore {
    pub fn new(workspace_root: &Path) -> Result<Self, ApiError> {
        let fallback_root = fallback_workspace_data_dir(workspace_root).join("workflow");
        let root = match crate::config::xdg::workspace_data_dir(workspace_root) {
            Ok(data_dir) => {
                let primary = data_dir.join("workflow");
                if std::fs::create_dir_all(&primary).is_ok() {
                    primary
                } else {
                    std::fs::create_dir_all(&fallback_root).map_err(|err| {
                        ApiError::ConfigError(format!(
                            "Failed to create fallback workflow store '{}': {}",
                            fallback_root.display(),
                            err
                        ))
                    })?;
                    fallback_root.clone()
                }
            }
            Err(_) => {
                std::fs::create_dir_all(&fallback_root).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Failed to create fallback workflow store '{}': {}",
                        fallback_root.display(),
                        err
                    ))
                })?;
                fallback_root.clone()
            }
        };

        Ok(Self {
            inner: meld_execution::workflow::WorkflowStateStore::from_root(&root)?,
        })
    }

    pub fn load_thread(&self, thread_id: &str) -> Result<Option<WorkflowThreadRecord>, ApiError> {
        self.inner.load_thread(thread_id).map_err(Into::into)
    }

    pub fn upsert_thread(&self, record: &WorkflowThreadRecord) -> Result<(), ApiError> {
        self.inner.upsert_thread(record).map_err(Into::into)
    }

    pub fn load_turns(&self, thread_id: &str) -> Result<Vec<WorkflowTurnRecord>, ApiError> {
        self.inner.load_turns(thread_id).map_err(Into::into)
    }

    pub fn upsert_turn(&self, record: &WorkflowTurnRecord) -> Result<(), ApiError> {
        self.inner.upsert_turn(record).map_err(Into::into)
    }

    pub fn upsert_gate(
        &self,
        thread_id: &str,
        turn_id: &str,
        record: &crate::workflow::record_contracts::ThreadTurnGateRecordV1,
    ) -> Result<(), ApiError> {
        self.inner
            .upsert_gate(thread_id, turn_id, record)
            .map_err(Into::into)
    }

    pub fn upsert_prompt_link(
        &self,
        thread_id: &str,
        turn_id: &str,
        record: &crate::workflow::record_contracts::PromptLinkRecordV1,
    ) -> Result<(), ApiError> {
        self.inner
            .upsert_prompt_link(thread_id, turn_id, record)
            .map_err(Into::into)
    }

    pub fn completed_output_map(
        &self,
        thread_id: &str,
    ) -> Result<std::collections::HashMap<String, String>, ApiError> {
        self.inner
            .completed_output_map(thread_id)
            .map_err(Into::into)
    }
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
