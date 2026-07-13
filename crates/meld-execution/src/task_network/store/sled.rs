//! Sled-backed task network command store.

use crate::task_network::{
    command,
    journal::JournalRecord,
    mutation::Rejection,
    outcome::PublicationLedgerBinding,
    state::NetworkState,
    store::{
        codec::{decode_error, decode_optional, to_decode, to_storage},
        error::{AuthorityStoreError, TaskNetworkStoreError},
        memory::{command_request_hash, duplicate_or_replay, InMemoryTaskNetworkStore},
        records::{
            revision_key, StoredCommandRequest, StoredCommandResponse, StoredJournalRecord,
            StoredStateSnapshot, KEY_AUTHORITY_EPOCH, KEY_LATEST_STATE, TREE_AUTHORITY_LIFECYCLE,
            TREE_COMMAND_REQUESTS, TREE_COMMAND_RESPONSES, TREE_JOURNAL_BY_REVISION,
            TREE_LATEST_STATE,
        },
    },
};
use sled::{
    transaction::{ConflictableTransactionError, TransactionError, Transactional},
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
    authority_lifecycle: Tree,
    inner: InMemoryTaskNetworkStore,
    durability_indeterminate: bool,
    #[cfg(test)]
    fail_next_flush: bool,
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
        let authority_lifecycle = db.open_tree(TREE_AUTHORITY_LIFECYCLE).map_err(to_storage)?;
        let mut inner = InMemoryTaskNetworkStore::new(network_id.clone());

        for item in journal_by_revision.iter() {
            let (key, value) = item.map_err(to_storage)?;
            let revision = decode_revision_key(&key)?;
            let stored: StoredJournalRecord = serde_json::from_slice(&value).map_err(to_decode)?;
            inner.apply_journal_record_for_replay(revision, &stored)?;
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
            authority_lifecycle,
            inner,
            durability_indeterminate: false,
            #[cfg(test)]
            fail_next_flush: false,
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

    /// Returns the event ledger established by canonical publications.
    pub fn publication_ledger_binding(&self) -> Option<&PublicationLedgerBinding> {
        self.inner.publication_ledger_binding()
    }

    /// Submits one command and persists accepted records through the journal.
    // TODO compat-shim: remove unfenced direct submission after every runtime
    // caller uses task network authority ports and store parity tests continue
    // to prove replay compatibility for the persisted command products.
    pub fn submit(
        &mut self,
        request: command::Request,
    ) -> Result<command::Response, TaskNetworkStoreError> {
        self.submit_inner(request, None)
            .map_err(AuthorityStoreError::into_store_error)
    }

    pub(crate) fn acquire_authority_epoch(&mut self) -> Result<u64, TaskNetworkStoreError> {
        if self.durability_indeterminate {
            return Err(durability_indeterminate());
        }
        loop {
            let current = self
                .authority_lifecycle
                .get(KEY_AUTHORITY_EPOCH)
                .map_err(to_storage)?;
            let current_epoch = decode_authority_epoch(current.as_deref())?;
            let next_epoch = current_epoch.checked_add(1).ok_or_else(|| {
                TaskNetworkStoreError::Storage("task network authority epoch exhausted".to_string())
            })?;
            let next = next_epoch.to_be_bytes();
            match self
                .authority_lifecycle
                .compare_and_swap(
                    KEY_AUTHORITY_EPOCH,
                    current.as_deref(),
                    Some(next.as_slice()),
                )
                .map_err(to_storage)?
            {
                Ok(()) => {
                    self.flush_after_semantic_write()?;
                    return Ok(next_epoch);
                }
                Err(_) => continue,
            }
        }
    }

    pub(crate) fn submit_at_authority_epoch(
        &mut self,
        epoch: u64,
        request: command::Request,
    ) -> Result<command::Response, AuthorityStoreError> {
        self.submit_inner(request, Some(epoch))
    }

    pub(crate) fn validate_authority_epoch(
        &self,
        expected: u64,
    ) -> Result<(), AuthorityStoreError> {
        let current = self
            .authority_lifecycle
            .get(KEY_AUTHORITY_EPOCH)
            .map_err(to_storage)?;
        let actual = decode_authority_epoch(current.as_deref())?;
        if actual != expected {
            return Err(AuthorityStoreError::StaleEpoch { expected, actual });
        }
        Ok(())
    }

    fn submit_inner(
        &mut self,
        request: command::Request,
        authority_epoch: Option<u64>,
    ) -> Result<command::Response, AuthorityStoreError> {
        if self.durability_indeterminate {
            return Err(durability_indeterminate().into());
        }
        if let Some(expected_epoch) = authority_epoch {
            self.validate_authority_epoch(expected_epoch)?;
        }
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
                ))
                .into());
            };
            return Ok(duplicate_or_replay(&stored_response.response));
        }

        // Reduce into a candidate so queries never observe a state that has
        // not crossed the durable acknowledgement barrier.
        let mut candidate = self.inner.clone();
        let response = candidate.submit(request.clone());
        match &response {
            command::Response::Accepted {
                revision,
                state_hash: _,
            } => {
                let Some(record) = candidate.journal().last().cloned() else {
                    return Err(
                        decode_error("accepted command did not append journal record").into(),
                    );
                };
                let stored_request = StoredCommandRequest::new(request_hash.clone(), request);
                let stored_journal = StoredJournalRecord::new(record);
                let stored_response = StoredCommandResponse {
                    command_id,
                    request_hash,
                    response: response.clone(),
                };
                let snapshot = StoredStateSnapshot::new(candidate.state().clone());
                self.persist_accepted_command(
                    stored_request,
                    *revision,
                    stored_journal,
                    stored_response,
                    snapshot,
                    authority_epoch,
                )?;
            }
            command::Response::Duplicate { .. } => {
                return Err(decode_error("new durable command returned duplicate response").into());
            }
            command::Response::Rejected(_) => {
                let stored_request = StoredCommandRequest::new(request_hash.clone(), request);
                let stored_response = StoredCommandResponse {
                    command_id,
                    request_hash,
                    response: response.clone(),
                };
                self.persist_rejected_command(stored_request, stored_response, authority_epoch)?;
            }
        }
        self.flush_after_semantic_write()?;
        self.inner = candidate;
        Ok(response)
    }

    fn flush_after_semantic_write(&mut self) -> Result<(), TaskNetworkStoreError> {
        #[cfg(test)]
        if std::mem::take(&mut self.fail_next_flush) {
            self.durability_indeterminate = true;
            return Err(TaskNetworkStoreError::Storage(
                "task network durability is indeterminate: injected flush failure".to_string(),
            ));
        }
        if let Err(error) = self.db.flush() {
            self.durability_indeterminate = true;
            return Err(TaskNetworkStoreError::Storage(format!(
                "task network durability is indeterminate: {error}"
            )));
        }
        Ok(())
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
        revision: u64,
        journal: StoredJournalRecord,
        response: StoredCommandResponse,
        snapshot: StoredStateSnapshot,
        authority_epoch: Option<u64>,
    ) -> Result<(), AuthorityStoreError> {
        let command_key = request.request.command_id.as_bytes().to_vec();
        let request_value = serde_json::to_vec(&request).map_err(to_decode)?;
        let journal_key = revision_key(revision).to_vec();
        let journal_value = serde_json::to_vec(&journal).map_err(to_decode)?;
        let response_value = serde_json::to_vec(&response).map_err(to_decode)?;
        let snapshot_value = serde_json::to_vec(&snapshot).map_err(to_decode)?;
        let snapshot_key = KEY_LATEST_STATE.to_vec();

        if let Some(expected_epoch) = authority_epoch {
            // The durable epoch participates in the same serializable
            // transaction as the semantic commit, fencing stale writers at
            // the last possible boundary before persistence.
            (
                &self.authority_lifecycle,
                &self.command_requests,
                &self.journal_by_revision,
                &self.command_responses,
                &self.latest_state,
            )
                .transaction(
                    |(lifecycle, requests, journal_tree, responses, snapshots)| {
                        validate_transaction_epoch(lifecycle, expected_epoch)?;
                        requests.insert(command_key.clone(), request_value.clone())?;
                        journal_tree.insert(journal_key.clone(), journal_value.clone())?;
                        responses.insert(command_key.clone(), response_value.clone())?;
                        snapshots.insert(snapshot_key.clone(), snapshot_value.clone())?;
                        Ok(())
                    },
                )
                .map_err(to_authority_transaction)?;
        } else {
            // Compatibility callers retain the original atomic store API.
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
        }
        Ok(())
    }

    fn persist_rejected_command(
        &self,
        request: StoredCommandRequest,
        response: StoredCommandResponse,
        authority_epoch: Option<u64>,
    ) -> Result<(), AuthorityStoreError> {
        let command_key = request.request.command_id.as_bytes().to_vec();
        let request_value = serde_json::to_vec(&request).map_err(to_decode)?;
        let response_value = serde_json::to_vec(&response).map_err(to_decode)?;

        if let Some(expected_epoch) = authority_epoch {
            (
                &self.authority_lifecycle,
                &self.command_requests,
                &self.command_responses,
            )
                .transaction(|(lifecycle, requests, responses)| {
                    validate_transaction_epoch(lifecycle, expected_epoch)?;
                    requests.insert(command_key.clone(), request_value.clone())?;
                    responses.insert(command_key.clone(), response_value.clone())?;
                    Ok(())
                })
                .map_err(to_authority_transaction)?;
        } else {
            // Rejected commands still need atomic idempotency records so
            // replay is stable after restart.
            (&self.command_requests, &self.command_responses)
                .transaction(|(requests, responses)| {
                    requests.insert(command_key.clone(), request_value.clone())?;
                    responses.insert(command_key.clone(), response_value.clone())?;
                    Ok(())
                })
                .map_err(to_transaction)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum AuthorityTransactionAbort {
    InvalidEpoch(String),
    Stale { expected: u64, actual: u64 },
}

fn validate_transaction_epoch(
    lifecycle: &sled::transaction::TransactionalTree,
    expected: u64,
) -> Result<(), ConflictableTransactionError<AuthorityTransactionAbort>> {
    let bytes = lifecycle.get(KEY_AUTHORITY_EPOCH)?;
    let actual = decode_authority_epoch(bytes.as_deref()).map_err(|error| {
        ConflictableTransactionError::Abort(AuthorityTransactionAbort::InvalidEpoch(
            error.to_string(),
        ))
    })?;
    if actual != expected {
        return Err(ConflictableTransactionError::Abort(
            AuthorityTransactionAbort::Stale { expected, actual },
        ));
    }
    Ok(())
}

fn decode_authority_epoch(bytes: Option<&[u8]>) -> Result<u64, TaskNetworkStoreError> {
    let Some(bytes) = bytes else {
        return Ok(0);
    };
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| decode_error("task network authority epoch length mismatch"))?;
    Ok(u64::from_be_bytes(bytes))
}

fn durability_indeterminate() -> TaskNetworkStoreError {
    TaskNetworkStoreError::Storage(
        "task network durability is indeterminate because the store was retained after a failed durability barrier"
            .to_string(),
    )
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
        if stored.request.command_id != command_id {
            return Err(decode_error("stored command request key mismatch"));
        }
        if stored
            .legacy_command_id
            .as_ref()
            .is_some_and(|legacy_command_id| legacy_command_id != &command_id)
        {
            return Err(decode_error("stored command request legacy key mismatch"));
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
    if snapshot.state.network_id != network_id
        || snapshot
            .legacy_network_id
            .as_ref()
            .is_some_and(|legacy_network_id| legacy_network_id != network_id)
    {
        return Err(decode_error("task network snapshot network mismatch"));
    }
    if snapshot.state.revision != replayed.revision
        || snapshot
            .legacy_revision
            .is_some_and(|legacy_revision| legacy_revision != replayed.revision)
    {
        return Err(decode_error("task network snapshot revision mismatch"));
    }
    if snapshot.state.state_hash != replayed.state_hash
        || snapshot
            .legacy_state_hash
            .as_ref()
            .is_some_and(|legacy_state_hash| legacy_state_hash != &replayed.state_hash)
    {
        return Err(decode_error("task network snapshot state hash mismatch"));
    }
    let recomputed = snapshot.state.recompute_state_hash();
    if recomputed != snapshot.state.state_hash {
        return Err(decode_error(
            "task network snapshot recomputed hash mismatch",
        ));
    }
    Ok(())
}

fn decode_revision_key(key: &[u8]) -> Result<u64, TaskNetworkStoreError> {
    let bytes: [u8; 8] = key
        .try_into()
        .map_err(|_| decode_error("journal revision key length mismatch"))?;
    Ok(u64::from_be_bytes(bytes))
}

fn to_transaction(error: TransactionError) -> TaskNetworkStoreError {
    match error {
        TransactionError::Abort(error) => {
            TaskNetworkStoreError::Storage(format!("task network transaction aborted: {error:?}"))
        }
        TransactionError::Storage(error) => TaskNetworkStoreError::Storage(error.to_string()),
    }
}

fn to_authority_transaction(
    error: TransactionError<AuthorityTransactionAbort>,
) -> AuthorityStoreError {
    match error {
        TransactionError::Abort(AuthorityTransactionAbort::InvalidEpoch(message)) => {
            TaskNetworkStoreError::Decode(message).into()
        }
        TransactionError::Abort(AuthorityTransactionAbort::Stale { expected, actual }) => {
            AuthorityStoreError::StaleEpoch { expected, actual }
        }
        TransactionError::Storage(error) => {
            TaskNetworkStoreError::Storage(error.to_string()).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_network::mutation::Set;

    fn empty_command(state: &NetworkState, command_id: &str) -> command::Request {
        command::Request {
            command_id: command_id.to_string(),
            network_id: state.network_id.clone(),
            base_revision: state.revision,
            base_state_hash: state.state_hash.clone(),
            read_preconditions: Vec::new(),
            command: command::Command::ApplyMutationSet(Set::empty(
                state.network_id.clone(),
                "composition-docs",
                "once",
            )),
        }
    }

    #[test]
    fn failed_flush_never_advances_live_state_and_fences_reuse() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
        let request = empty_command(store.state(), "command-docs");
        store.fail_next_flush = true;

        let error = store.submit(request.clone()).unwrap_err();

        assert!(
            matches!(error, TaskNetworkStoreError::Storage(message) if message.contains("indeterminate"))
        );
        assert_eq!(store.state().revision, 0);
        assert!(store.journal().is_empty());
        assert!(matches!(
            store.submit(request),
            Err(TaskNetworkStoreError::Storage(message)) if message.contains("indeterminate")
        ));
        assert_eq!(store.state().revision, 0);

        drop(store);
        db.flush().unwrap();
        let reopened = SledTaskNetworkStore::open(db, "network-docs").unwrap();
        assert_eq!(reopened.state().revision, 1);
        assert_eq!(reopened.journal().len(), 1);
    }

    #[test]
    fn stale_authority_epoch_cannot_acknowledge_a_duplicate_replay() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let mut store = SledTaskNetworkStore::open(db, "network-docs").unwrap();
        let first_epoch = store.acquire_authority_epoch().unwrap();
        let request = empty_command(store.state(), "command-docs");
        store
            .submit_at_authority_epoch(first_epoch, request.clone())
            .unwrap();
        let second_epoch = store.acquire_authority_epoch().unwrap();
        assert!(second_epoch > first_epoch);

        assert!(matches!(
            store.submit_at_authority_epoch(first_epoch, request),
            Err(AuthorityStoreError::StaleEpoch {
                expected,
                actual
            }) if expected == first_epoch && actual == second_epoch
        ));
    }
}
