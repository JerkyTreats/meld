//! Durable task network store record shapes.

use crate::task_network::{command, journal::JournalRecord, state::NetworkState};
use serde::{Deserialize, Serialize};

pub(super) const TREE_JOURNAL_BY_REVISION: &str = "task_network_journal_by_revision";
pub(super) const TREE_COMMAND_REQUESTS: &str = "task_network_command_requests";
pub(super) const TREE_COMMAND_RESPONSES: &str = "task_network_command_responses";
pub(super) const TREE_COMMAND_AUTHENTICATIONS: &str = "task_network_command_authentications";
pub(super) const TREE_COMMAND_SCHEMA: &str = "task_network_command_schema";
pub(super) const TREE_LATEST_STATE: &str = "task_network_latest_state";
pub(super) const TREE_AUTHORITY_LIFECYCLE: &str = "task_network_authority_lifecycle";
pub(super) const KEY_LATEST_STATE: &[u8] = b"latest";
pub(super) const KEY_AUTHORITY_EPOCH: &[u8] = b"epoch";
pub(super) const KEY_COMMAND_SCHEMA: &[u8] = b"schema";
pub(super) const COMMAND_SCHEMA_VERSION: u16 = 2;
pub(super) const PRE_AUTH_COMMAND_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CommandSchemaMarker {
    pub(super) schema_version: u16,
    pub(super) migrated_from: Option<u16>,
}

impl CommandSchemaMarker {
    pub(super) fn current(migrated_from: Option<u16>) -> Self {
        Self {
            schema_version: COMMAND_SCHEMA_VERSION,
            migrated_from,
        }
    }
}
pub(super) const KEY_COMMAND_AUTHENTICATION_SCHEMA: &[u8] = b"command_authentication_schema";
pub(super) const COMMAND_AUTHENTICATION_SCHEMA_V1: &[u8] = b"task_network.command_auth.v1";
pub(super) const KEY_COMMAND_AUTHENTICATION_DOWNGRADE_FENCE: &[u8] =
    b"\xffmeld.task_network.command_auth.v1";
pub(super) const COMMAND_AUTHENTICATION_DOWNGRADE_FENCE_V1: &[u8] =
    b"task_network.command_auth.v1.predecessor_rejected";

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
    #[serde(default)]
    pub(super) authentication: Option<command::ResponseAuthentication>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredCommandAuthentication {
    pub(super) command_id: String,
    pub(super) request_hash: String,
    pub(super) response_authentication: command::ResponseAuthentication,
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

impl StoredCommandResponse {
    pub(super) fn authenticated(
        command_id: String,
        request_hash: String,
        response: command::Response,
    ) -> Result<Self, String> {
        let authentication =
            command::ResponseAuthentication::bind(&command_id, &request_hash, &response)?;
        Ok(Self {
            command_id,
            request_hash,
            response,
            authentication: Some(authentication),
        })
    }
}

impl StoredCommandAuthentication {
    pub(super) fn for_response(response: &StoredCommandResponse) -> Result<Self, String> {
        let response_authentication = response
            .authentication
            .clone()
            .ok_or_else(|| "modern command response authentication is missing".to_string())?;
        Ok(Self {
            command_id: response.command_id.clone(),
            request_hash: response.request_hash.clone(),
            response_authentication,
        })
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
