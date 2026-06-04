//! Sled-backed task network command store.

use crate::task_network::{
    command,
    journal::JournalRecord,
    mutation::Rejection,
    state::NetworkState,
    store::{
        codec::{decode_error, decode_optional, to_decode, to_storage},
        error::TaskNetworkStoreError,
        memory::{command_request_hash, duplicate_or_replay, InMemoryTaskNetworkStore},
        records::{
            revision_key, StoredCommandRequest, StoredCommandResponse, StoredJournalRecord,
            StoredStateSnapshot, KEY_LATEST_STATE, TREE_COMMAND_REQUESTS, TREE_COMMAND_RESPONSES,
            TREE_JOURNAL_BY_REVISION, TREE_LATEST_STATE,
        },
    },
};
use sled::{
    transaction::{TransactionError, Transactional},
    Tree,
};
use std::collections::BTreeMap;

/// Sled-backed task network store using the same reducer as the in-memory store.
#[derive(Debug)]
pub struct SledTaskNetworkStore {
    db: sled::Db,
    journal_by_revision: Tree,
    command_requests: Tree,
    command_responses: Tree,
    latest_state: Tree,
    inner: InMemoryTaskNetworkStore,
}

impl SledTaskNetworkStore {
    /// Opens or initializes a task network store in a caller supplied database.
    pub fn open(
        db: sled::Db,
        network_id: impl Into<String>,
    ) -> Result<Self, TaskNetworkStoreError> {
        let network_id = network_id.into();
        let journal_by_revision = db.open_tree(TREE_JOURNAL_BY_REVISION).map_err(to_storage)?;
        let command_requests = db.open_tree(TREE_COMMAND_REQUESTS).map_err(to_storage)?;
        let command_responses = db.open_tree(TREE_COMMAND_RESPONSES).map_err(to_storage)?;
        let latest_state = db.open_tree(TREE_LATEST_STATE).map_err(to_storage)?;
        let mut inner = InMemoryTaskNetworkStore::new(network_id.clone());

        for item in journal_by_revision.iter() {
            let (_, value) = item.map_err(to_storage)?;
            let stored: StoredJournalRecord = serde_json::from_slice(&value).map_err(to_decode)?;
            inner.apply_journal_record_for_replay(&stored)?;
        }

        load_command_identity(&mut inner, &command_requests, &command_responses)?;

        if let Some(bytes) = latest_state.get(KEY_LATEST_STATE).map_err(to_storage)? {
            let snapshot: StoredStateSnapshot =
                serde_json::from_slice(&bytes).map_err(to_decode)?;
            validate_snapshot(&snapshot, &network_id, inner.state())?;
        }

        Ok(Self {
            db,
            journal_by_revision,
            command_requests,
            command_responses,
            latest_state,
            inner,
        })
    }

    /// Flushes durable writes to the backing database.
    pub fn flush(&self) -> Result<(), TaskNetworkStoreError> {
        self.db.flush().map_err(to_storage)?;
        Ok(())
    }

    /// Returns the latest reduced state.
    pub fn state(&self) -> &NetworkState {
        self.inner.state()
    }

    /// Returns accepted journal records in revision order.
    pub fn journal(&self) -> &[JournalRecord] {
        self.inner.journal()
    }

    /// Submits one command and persists accepted records through the journal.
    pub fn submit(
        &mut self,
        request: command::Request,
    ) -> Result<command::Response, TaskNetworkStoreError> {
        let command_id = request.command_id.clone();
        let request_hash = command_request_hash(&request);

        if let Some(stored_request) = self.stored_command_request(&command_id)? {
            if stored_request.request_hash != request_hash {
                return Ok(command::Response::Rejected(Rejection::DuplicateCommand(
                    command_id,
                )));
            }
            let Some(stored_response) = self.stored_command_response(&command_id)? else {
                return Err(decode_error(format!(
                    "command '{}' has request but no response",
                    command_id
                )));
            };
            return Ok(duplicate_or_replay(&stored_response.response));
        }

        let response = self.inner.submit(request.clone());
        match &response {
            command::Response::Accepted {
                revision,
                state_hash,
            } => {
                let Some(record) = self.inner.journal().last().cloned() else {
                    return Err(decode_error(
                        "accepted command did not append journal record",
                    ));
                };
                let stored_request = StoredCommandRequest {
                    command_id: command_id.clone(),
                    request_hash: request_hash.clone(),
                    request,
                };
                let stored_journal = StoredJournalRecord {
                    network_id: self.inner.state().network_id.clone(),
                    revision: *revision,
                    state_hash: state_hash.clone(),
                    record,
                };
                let stored_response = StoredCommandResponse {
                    command_id,
                    request_hash,
                    response: response.clone(),
                };
                let snapshot = StoredStateSnapshot {
                    network_id: self.inner.state().network_id.clone(),
                    revision: self.inner.state().revision,
                    state_hash: self.inner.state().state_hash.clone(),
                    state: self.inner.state().clone(),
                };
                self.persist_accepted_command(
                    stored_request,
                    stored_journal,
                    stored_response,
                    snapshot,
                )?;
            }
            command::Response::Duplicate { .. } => {
                return Err(decode_error(
                    "new durable command returned duplicate response",
                ));
            }
            command::Response::Rejected(_) => {
                let stored_request = StoredCommandRequest {
                    command_id: command_id.clone(),
                    request_hash: request_hash.clone(),
                    request,
                };
                let stored_response = StoredCommandResponse {
                    command_id,
                    request_hash,
                    response: response.clone(),
                };
                self.persist_rejected_command(stored_request, stored_response)?;
            }
        }
        self.flush()?;
        Ok(response)
    }

    fn stored_command_request(
        &self,
        command_id: &str,
    ) -> Result<Option<StoredCommandRequest>, TaskNetworkStoreError> {
        decode_optional(self.command_requests.get(command_id).map_err(to_storage)?)
    }

    fn stored_command_response(
        &self,
        command_id: &str,
    ) -> Result<Option<StoredCommandResponse>, TaskNetworkStoreError> {
        decode_optional(self.command_responses.get(command_id).map_err(to_storage)?)
    }

    fn persist_accepted_command(
        &self,
        request: StoredCommandRequest,
        journal: StoredJournalRecord,
        response: StoredCommandResponse,
        snapshot: StoredStateSnapshot,
    ) -> Result<(), TaskNetworkStoreError> {
        let command_key = request.command_id.as_bytes().to_vec();
        let request_value = serde_json::to_vec(&request).map_err(to_decode)?;
        let journal_key = revision_key(journal.revision).to_vec();
        let journal_value = serde_json::to_vec(&journal).map_err(to_decode)?;
        let response_value = serde_json::to_vec(&response).map_err(to_decode)?;
        let snapshot_value = serde_json::to_vec(&snapshot).map_err(to_decode)?;
        let snapshot_key = KEY_LATEST_STATE.to_vec();

        // These trees must move together so command replay never observes a
        // request without its response or a response without its journal.
        (
            &self.command_requests,
            &self.journal_by_revision,
            &self.command_responses,
            &self.latest_state,
        )
            .transaction(|(requests, journal_tree, responses, snapshots)| {
                requests.insert(command_key.clone(), request_value.clone())?;
                journal_tree.insert(journal_key.clone(), journal_value.clone())?;
                responses.insert(command_key.clone(), response_value.clone())?;
                snapshots.insert(snapshot_key.clone(), snapshot_value.clone())?;
                Ok(())
            })
            .map_err(to_transaction)?;
        Ok(())
    }

    fn persist_rejected_command(
        &self,
        request: StoredCommandRequest,
        response: StoredCommandResponse,
    ) -> Result<(), TaskNetworkStoreError> {
        let command_key = request.command_id.as_bytes().to_vec();
        let request_value = serde_json::to_vec(&request).map_err(to_decode)?;
        let response_value = serde_json::to_vec(&response).map_err(to_decode)?;

        // Rejected commands still need atomic idempotency records so replay is
        // stable after restart.
        (&self.command_requests, &self.command_responses)
            .transaction(|(requests, responses)| {
                requests.insert(command_key.clone(), request_value.clone())?;
                responses.insert(command_key.clone(), response_value.clone())?;
                Ok(())
            })
            .map_err(to_transaction)?;
        Ok(())
    }
}

fn load_command_identity(
    inner: &mut InMemoryTaskNetworkStore,
    command_requests: &Tree,
    command_responses: &Tree,
) -> Result<(), TaskNetworkStoreError> {
    let requests = load_command_requests(command_requests)?;
    let responses = load_command_responses(command_responses)?;

    for command_id in requests.keys() {
        let Some(response) = responses.get(command_id) else {
            return Err(decode_error(format!(
                "command '{}' has request but no response",
                command_id
            )));
        };
        let request = requests
            .get(command_id)
            .expect("request key came from request map");
        if request.request_hash != response.request_hash {
            return Err(decode_error(format!(
                "command '{}' request and response hashes differ",
                command_id
            )));
        }
        inner.insert_command_identity(
            command_id.clone(),
            request.request_hash.clone(),
            response.response.clone(),
        );
    }

    for command_id in responses.keys() {
        if !requests.contains_key(command_id) {
            return Err(decode_error(format!(
                "command '{}' has response but no request",
                command_id
            )));
        }
    }

    Ok(())
}

fn load_command_requests(
    command_requests: &Tree,
) -> Result<BTreeMap<String, StoredCommandRequest>, TaskNetworkStoreError> {
    let mut requests = BTreeMap::new();
    for item in command_requests.iter() {
        let (key, value) = item.map_err(to_storage)?;
        let command_id = String::from_utf8(key.to_vec())
            .map_err(|error| decode_error(format!("command request key UTF-8 failed: {error}")))?;
        let stored: StoredCommandRequest = serde_json::from_slice(&value).map_err(to_decode)?;
        if stored.command_id != command_id {
            return Err(decode_error("stored command request key mismatch"));
        }
        requests.insert(command_id, stored);
    }
    Ok(requests)
}

fn load_command_responses(
    command_responses: &Tree,
) -> Result<BTreeMap<String, StoredCommandResponse>, TaskNetworkStoreError> {
    let mut responses = BTreeMap::new();
    for item in command_responses.iter() {
        let (key, value) = item.map_err(to_storage)?;
        let command_id = String::from_utf8(key.to_vec())
            .map_err(|error| decode_error(format!("command response key UTF-8 failed: {error}")))?;
        let stored: StoredCommandResponse = serde_json::from_slice(&value).map_err(to_decode)?;
        if stored.command_id != command_id {
            return Err(decode_error("stored command response key mismatch"));
        }
        responses.insert(command_id, stored);
    }
    Ok(responses)
}

fn validate_snapshot(
    snapshot: &StoredStateSnapshot,
    network_id: &str,
    replayed: &NetworkState,
) -> Result<(), TaskNetworkStoreError> {
    if snapshot.network_id != network_id || snapshot.state.network_id != network_id {
        return Err(decode_error("task network snapshot network mismatch"));
    }
    if snapshot.revision != replayed.revision || snapshot.state.revision != replayed.revision {
        return Err(decode_error("task network snapshot revision mismatch"));
    }
    if snapshot.state_hash != replayed.state_hash
        || snapshot.state.state_hash != replayed.state_hash
    {
        return Err(decode_error("task network snapshot state hash mismatch"));
    }
    let recomputed = snapshot.state.recompute_state_hash();
    if recomputed != snapshot.state_hash {
        return Err(decode_error(
            "task network snapshot recomputed hash mismatch",
        ));
    }
    Ok(())
}

fn to_transaction(error: TransactionError) -> TaskNetworkStoreError {
    match error {
        TransactionError::Abort(error) => {
            TaskNetworkStoreError::Storage(format!("task network transaction aborted: {error:?}"))
        }
        TransactionError::Storage(error) => TaskNetworkStoreError::Storage(error.to_string()),
    }
}
