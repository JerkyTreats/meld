//! Durable task network store record shapes.

use crate::task_network::{command, journal::JournalRecord, state::NetworkState};
use serde::{Deserialize, Serialize};

pub(super) const TREE_JOURNAL_BY_REVISION: &str = "task_network_journal_by_revision";
pub(super) const TREE_COMMAND_REQUESTS: &str = "task_network_command_requests";
pub(super) const TREE_COMMAND_RESPONSES: &str = "task_network_command_responses";
pub(super) const TREE_LATEST_STATE: &str = "task_network_latest_state";
pub(super) const KEY_LATEST_STATE: &[u8] = b"latest";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredCommandRequest {
    pub(super) request_hash: String,
    pub(super) request: command::Request,
    #[serde(default, rename = "command_id", skip_serializing)]
    pub(super) legacy_command_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredCommandResponse {
    pub(super) command_id: String,
    pub(super) request_hash: String,
    pub(super) response: command::Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredJournalRecord {
    pub(super) record: JournalRecord,
    #[serde(default, rename = "network_id", skip_serializing)]
    pub(super) legacy_network_id: Option<String>,
    #[serde(default, rename = "revision", skip_serializing)]
    pub(super) legacy_revision: Option<u64>,
    #[serde(default, rename = "state_hash", skip_serializing)]
    pub(super) legacy_state_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredStateSnapshot {
    pub(super) state: NetworkState,
    #[serde(default, rename = "network_id", skip_serializing)]
    pub(super) legacy_network_id: Option<String>,
    #[serde(default, rename = "revision", skip_serializing)]
    pub(super) legacy_revision: Option<u64>,
    #[serde(default, rename = "state_hash", skip_serializing)]
    pub(super) legacy_state_hash: Option<String>,
}

impl StoredCommandRequest {
    pub(super) fn new(request_hash: String, request: command::Request) -> Self {
        Self {
            request_hash,
            request,
            legacy_command_id: None,
        }
    }
}

impl StoredJournalRecord {
    pub(super) fn new(record: JournalRecord) -> Self {
        Self {
            record,
            legacy_network_id: None,
            legacy_revision: None,
            legacy_state_hash: None,
        }
    }
}

impl StoredStateSnapshot {
    pub(super) fn new(state: NetworkState) -> Self {
        Self {
            state,
            legacy_network_id: None,
            legacy_revision: None,
            legacy_state_hash: None,
        }
    }
}

pub(super) fn revision_key(revision: u64) -> [u8; 8] {
    revision.to_be_bytes()
}
