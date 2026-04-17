use serde::{Deserialize, Serialize};

use crate::session::policy::SessionStatus;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    #[default]
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    #[serde(default)]
    pub session_kind: SessionKind,
    pub command: String,
    pub started_at_ms: u64,
    pub ended_at_ms: Option<u64>,
    pub status: SessionStatus,
    pub status_text: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    #[serde(default = "default_next_seq")]
    pub next_seq: u64,
    pub latest_status: SessionStatus,
    pub updated_at_ms: u64,
}

impl SessionRecord {
    pub fn command(session_id: String, command: String, started_at_ms: u64) -> Self {
        Self {
            session_id,
            session_kind: SessionKind::Command,
            command,
            started_at_ms,
            ended_at_ms: None,
            status: SessionStatus::Active,
            status_text: SessionStatus::Active.as_str().to_string(),
            error: None,
        }
    }
}

impl SessionMeta {
    pub fn active(updated_at_ms: u64) -> Self {
        Self {
            next_seq: default_next_seq(),
            latest_status: SessionStatus::Active,
            updated_at_ms,
        }
    }
}

fn default_next_seq() -> u64 {
    1
}
