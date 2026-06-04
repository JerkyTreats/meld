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
    pub(super) command_id: String,
    pub(super) request_hash: String,
    pub(super) request: command::Request,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredCommandResponse {
    pub(super) command_id: String,
    pub(super) request_hash: String,
    pub(super) response: command::Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredJournalRecord {
    pub(super) network_id: String,
    pub(super) revision: u64,
    pub(super) state_hash: String,
    pub(super) record: JournalRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredStateSnapshot {
    pub(super) network_id: String,
    pub(super) revision: u64,
    pub(super) state_hash: String,
    pub(super) state: NetworkState,
}

pub(super) fn revision_key(revision: u64) -> [u8; 8] {
    revision.to_be_bytes()
}
