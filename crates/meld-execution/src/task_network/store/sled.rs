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
            revision_key, CommandSchemaMarker, StoredCommandAuthentication, StoredCommandRequest,
            StoredCommandResponse, StoredJournalRecord, StoredStateSnapshot,
            COMMAND_SCHEMA_VERSION, KEY_AUTHORITY_EPOCH, KEY_COMMAND_SCHEMA, KEY_LATEST_STATE,
            PRE_AUTH_COMMAND_SCHEMA_VERSION, TREE_AUTHORITY_LIFECYCLE,
            TREE_COMMAND_AUTHENTICATIONS, TREE_COMMAND_REQUESTS, TREE_COMMAND_RESPONSES,
            TREE_COMMAND_SCHEMA, TREE_JOURNAL_BY_REVISION, TREE_LATEST_STATE,
        },
    },
};
use sled::{
    transaction::{ConflictableTransactionError, TransactionError, Transactional},
    Tree,
};
use std::collections::BTreeMap;

const MAX_DURABLE_COMMAND_REQUEST_BYTES: usize = 16 * 1_048_576;
const MAX_DURABLE_COMMAND_RESPONSE_BYTES: usize = 1_048_576;
const MAX_DURABLE_COMMAND_AUTHENTICATION_BYTES: usize = 4_096;
const MAX_DURABLE_COMMAND_ID_BYTES: usize = 1_024;

/// Sled-backed task network store using the same reducer as the in-memory store.
#[derive(Debug)]
pub struct SledTaskNetworkStore {
    db: sled::Db,
    journal_by_revision: Tree,
    command_requests: Tree,
    command_responses: Tree,
    command_authentications: Tree,
    command_schema: Tree,
    latest_state: Tree,
    authority_lifecycle: Tree,
    inner: InMemoryTaskNetworkStore,
    // This derived index is reconstructed during open and advanced only from
    // journal records that crossed the durable acknowledgement barrier.
    incoming_edges_by_task: BTreeMap<String, Vec<crate::task_network::state::DependencyEdge>>,
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
        let command_authentications = db
            .open_tree(TREE_COMMAND_AUTHENTICATIONS)
            .map_err(to_storage)?;
        let command_schema = db.open_tree(TREE_COMMAND_SCHEMA).map_err(to_storage)?;
        let latest_state = db.open_tree(TREE_LATEST_STATE).map_err(to_storage)?;
        let authority_lifecycle = db.open_tree(TREE_AUTHORITY_LIFECYCLE).map_err(to_storage)?;
        let mut inner = InMemoryTaskNetworkStore::new(network_id.clone());

        for item in journal_by_revision.iter() {
            let (key, value) = item.map_err(to_storage)?;
            let revision = decode_revision_key(&key)?;
            let stored: StoredJournalRecord = serde_json::from_slice(&value).map_err(to_decode)?;
            inner.apply_journal_record_for_replay(revision, &stored)?;
        }

        initialize_command_schema(
            &inner,
            &command_requests,
            &command_responses,
            &command_authentications,
            &command_schema,
        )?;

        load_command_identity(
            &mut inner,
            &command_requests,
            &command_responses,
            &command_authentications,
        )?;

        if let Some(bytes) = latest_state.get(KEY_LATEST_STATE).map_err(to_storage)? {
            let snapshot: StoredStateSnapshot =
                serde_json::from_slice(&bytes).map_err(to_decode)?;
            validate_snapshot(&snapshot, &network_id, inner.state())?;
        }
        let incoming_edges_by_task = index_incoming_edges(inner.state());

        Ok(Self {
            db,
            journal_by_revision,
            command_requests,
            command_responses,
            command_authentications,
            command_schema,
            latest_state,
            authority_lifecycle,
            inner,
            incoming_edges_by_task,
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

    pub(crate) fn command_outcome_receipt(
        &self,
        command_id: &str,
    ) -> Result<Option<command::OutcomeReceipt>, TaskNetworkStoreError> {
        let request = self.stored_command_request(command_id)?;
        let response = self.stored_command_response(command_id)?;
        let authentication = self.stored_command_authentication(command_id)?;
        match (request, response, authentication) {
            (None, None, None) => Ok(None),
            (Some(_), None, _) | (None, Some(_), _) | (None, None, Some(_)) => Err(decode_error(
                format!("command '{command_id}' has an incomplete durable outcome"),
            )),
            (Some(request), Some(response), authentication) => {
                if request.request.command_id != command_id || response.command_id != command_id {
                    return Err(decode_error(format!(
                        "command '{command_id}' durable outcome identity mismatch"
                    )));
                }
                self.validate_live_command_outcome(&request, &response, authentication.as_ref())?;
                Ok(Some(command::OutcomeReceipt::issued(
                    command_id.to_string(),
                    request.request_hash,
                    request.request,
                    response.response,
                )))
            }
        }
    }

    pub(crate) fn task_materialization(
        &self,
        task_instance_id: &str,
    ) -> Option<(
        &crate::task_network::state::TaskNode,
        &[crate::task_network::state::DependencyEdge],
    )> {
        self.inner.state().tasks.get(task_instance_id).map(|task| {
            let incoming_edges = self
                .incoming_edges_by_task
                .get(task_instance_id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            (task, incoming_edges)
        })
    }

    fn validate_live_command_outcome(
        &self,
        request: &StoredCommandRequest,
        response: &StoredCommandResponse,
        authentication: Option<&StoredCommandAuthentication>,
    ) -> Result<(), TaskNetworkStoreError> {
        validate_command_outcome_binding(request, response, authentication, false)?;
        let command_id = &request.request.command_id;
        // Authentication detects accidental corruption. Comparing with the
        // already replay-verified owner state also rejects an external writer
        // that substituted a response and recomputed its public digest.
        if self.inner.command_requests.get(command_id) != Some(&request.request_hash)
            || self.inner.command_responses.get(command_id) != Some(&response.response)
        {
            return Err(decode_error(format!(
                "command '{command_id}' durable outcome diverges from the authority replay"
            )));
        }
        Ok(())
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
        validate_command_schema_marker(&self.command_schema)?;
        if request.command_id.trim().is_empty()
            || request.command_id.len() > MAX_DURABLE_COMMAND_ID_BYTES
        {
            return Err(TaskNetworkStoreError::InvalidConfiguration(format!(
                "task network command id must be non-empty and at most {MAX_DURABLE_COMMAND_ID_BYTES} bytes"
            ))
            .into());
        }
        let request_bytes = serde_json::to_vec(&request).map_err(to_decode)?;
        if request_bytes.len() > MAX_DURABLE_COMMAND_REQUEST_BYTES {
            return Err(TaskNetworkStoreError::InvalidConfiguration(format!(
                "task network command request encodes to {} bytes, exceeding limit {MAX_DURABLE_COMMAND_REQUEST_BYTES}",
                request_bytes.len()
            ))
            .into());
        }
        if let Some(expected_epoch) = authority_epoch {
            self.validate_authority_epoch(expected_epoch)?;
        }
        let command_id = request.command_id.clone();
        let request_hash = command_request_hash(&request);

        if let Some(stored_request) = self.stored_command_request(&command_id)? {
            let Some(stored_response) = self.stored_command_response(&command_id)? else {
                return Err(decode_error(format!(
                    "command '{}' has request but no response",
                    command_id
                ))
                .into());
            };
            let stored_authentication = self.stored_command_authentication(&command_id)?;
            self.validate_live_command_outcome(
                &stored_request,
                &stored_response,
                stored_authentication.as_ref(),
            )?;
            if stored_request.request_hash != request_hash {
                return Ok(command::Response::Rejected(Rejection::DuplicateCommand(
                    command_id,
                )));
            }
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
                let stored_response = StoredCommandResponse::authenticated(
                    command_id,
                    request_hash,
                    response.clone(),
                )
                .map_err(decode_error)?;
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
                let stored_response = StoredCommandResponse::authenticated(
                    command_id,
                    request_hash,
                    response.clone(),
                )
                .map_err(decode_error)?;
                self.persist_rejected_command(stored_request, stored_response, authority_epoch)?;
            }
        }
        self.flush_after_semantic_write()?;
        if matches!(response, command::Response::Accepted { .. }) {
            // The index is never advanced ahead of durable semantic state.
            // Its update is infallible and can therefore follow the flush
            // without introducing a second persistence boundary.
            let record = candidate
                .journal()
                .last()
                .expect("accepted command appended a journal record");
            apply_journal_to_incoming_edge_index(&mut self.incoming_edges_by_task, record);
        }
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
            return Err(classify_durability_failure(error));
        }
        Ok(())
    }

    fn stored_command_request(
        &self,
        command_id: &str,
    ) -> Result<Option<StoredCommandRequest>, TaskNetworkStoreError> {
        let raw = self.command_requests.get(command_id).map_err(to_storage)?;
        if raw
            .as_ref()
            .is_some_and(|raw| raw.len() > MAX_DURABLE_COMMAND_REQUEST_BYTES)
        {
            return Err(decode_error(format!(
                "command '{command_id}' durable request exceeds {MAX_DURABLE_COMMAND_REQUEST_BYTES} bytes"
            )));
        }
        decode_optional(raw)
    }

    fn stored_command_response(
        &self,
        command_id: &str,
    ) -> Result<Option<StoredCommandResponse>, TaskNetworkStoreError> {
        let raw = self.command_responses.get(command_id).map_err(to_storage)?;
        if raw
            .as_ref()
            .is_some_and(|raw| raw.len() > MAX_DURABLE_COMMAND_RESPONSE_BYTES)
        {
            return Err(decode_error(format!(
                "command '{command_id}' durable response exceeds {MAX_DURABLE_COMMAND_RESPONSE_BYTES} bytes"
            )));
        }
        decode_optional(raw)
    }

    fn stored_command_authentication(
        &self,
        command_id: &str,
    ) -> Result<Option<StoredCommandAuthentication>, TaskNetworkStoreError> {
        let raw = self
            .command_authentications
            .get(command_id)
            .map_err(to_storage)?;
        if raw
            .as_ref()
            .is_some_and(|raw| raw.len() > MAX_DURABLE_COMMAND_AUTHENTICATION_BYTES)
        {
            return Err(decode_error(format!(
                "command '{command_id}' durable authentication exceeds {MAX_DURABLE_COMMAND_AUTHENTICATION_BYTES} bytes"
            )));
        }
        decode_optional(raw)
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
        let authentication =
            StoredCommandAuthentication::for_response(&response).map_err(decode_error)?;
        let authentication_value = serde_json::to_vec(&authentication).map_err(to_decode)?;
        validate_durable_command_record_sizes(
            &request_value,
            &response_value,
            &authentication_value,
        )?;
        let snapshot_value = serde_json::to_vec(&snapshot).map_err(to_decode)?;
        let snapshot_key = KEY_LATEST_STATE.to_vec();

        if let Some(expected_epoch) = authority_epoch {
            // The durable epoch participates in the same serializable
            // transaction as the semantic commit, fencing stale writers at
            // the last possible boundary before persistence.
            (
                &self.authority_lifecycle,
                &self.command_schema,
                &self.command_requests,
                &self.journal_by_revision,
                &self.command_responses,
                &self.command_authentications,
                &self.latest_state,
            )
                .transaction(
                    |(
                        lifecycle,
                        schema,
                        requests,
                        journal_tree,
                        responses,
                        authentications,
                        snapshots,
                    )| {
                        validate_transaction_epoch(lifecycle, expected_epoch)?;
                        validate_transaction_command_schema(schema)?;
                        requests.insert(command_key.clone(), request_value.clone())?;
                        journal_tree.insert(journal_key.clone(), journal_value.clone())?;
                        responses.insert(command_key.clone(), response_value.clone())?;
                        authentications
                            .insert(command_key.clone(), authentication_value.clone())?;
                        snapshots.insert(snapshot_key.clone(), snapshot_value.clone())?;
                        Ok(())
                    },
                )
                .map_err(to_authority_transaction)?;
        } else {
            // Compatibility callers retain the original atomic store API.
            (
                &self.command_schema,
                &self.command_requests,
                &self.journal_by_revision,
                &self.command_responses,
                &self.command_authentications,
                &self.latest_state,
            )
                .transaction(
                    |(schema, requests, journal_tree, responses, authentications, snapshots)| {
                        validate_transaction_command_schema(schema)?;
                        requests.insert(command_key.clone(), request_value.clone())?;
                        journal_tree.insert(journal_key.clone(), journal_value.clone())?;
                        responses.insert(command_key.clone(), response_value.clone())?;
                        authentications
                            .insert(command_key.clone(), authentication_value.clone())?;
                        snapshots.insert(snapshot_key.clone(), snapshot_value.clone())?;
                        Ok(())
                    },
                )
                .map_err(to_authority_transaction)?;
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
        let authentication =
            StoredCommandAuthentication::for_response(&response).map_err(decode_error)?;
        let authentication_value = serde_json::to_vec(&authentication).map_err(to_decode)?;
        validate_durable_command_record_sizes(
            &request_value,
            &response_value,
            &authentication_value,
        )?;

        if let Some(expected_epoch) = authority_epoch {
            (
                &self.authority_lifecycle,
                &self.command_schema,
                &self.command_requests,
                &self.command_responses,
                &self.command_authentications,
            )
                .transaction(
                    |(lifecycle, schema, requests, responses, authentications)| {
                        validate_transaction_epoch(lifecycle, expected_epoch)?;
                        validate_transaction_command_schema(schema)?;
                        requests.insert(command_key.clone(), request_value.clone())?;
                        responses.insert(command_key.clone(), response_value.clone())?;
                        authentications
                            .insert(command_key.clone(), authentication_value.clone())?;
                        Ok(())
                    },
                )
                .map_err(to_authority_transaction)?;
        } else {
            // Rejected commands still need atomic idempotency records so
            // replay is stable after restart.
            (
                &self.command_schema,
                &self.command_requests,
                &self.command_responses,
                &self.command_authentications,
            )
                .transaction(|(schema, requests, responses, authentications)| {
                    validate_transaction_command_schema(schema)?;
                    requests.insert(command_key.clone(), request_value.clone())?;
                    responses.insert(command_key.clone(), response_value.clone())?;
                    authentications.insert(command_key.clone(), authentication_value.clone())?;
                    Ok(())
                })
                .map_err(to_authority_transaction)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum AuthorityTransactionAbort {
    InvalidEpoch(String),
    InvalidSchema(String),
    Stale { expected: u64, actual: u64 },
}

fn validate_transaction_command_schema(
    schema: &sled::transaction::TransactionalTree,
) -> Result<(), ConflictableTransactionError<AuthorityTransactionAbort>> {
    let raw = schema.get(KEY_COMMAND_SCHEMA)?.ok_or_else(|| {
        ConflictableTransactionError::Abort(AuthorityTransactionAbort::InvalidSchema(
            "task network command schema marker is missing".to_string(),
        ))
    })?;
    let marker: CommandSchemaMarker = serde_json::from_slice(&raw).map_err(|error| {
        ConflictableTransactionError::Abort(AuthorityTransactionAbort::InvalidSchema(
            error.to_string(),
        ))
    })?;
    if marker.schema_version != COMMAND_SCHEMA_VERSION
        || marker
            .migrated_from
            .is_some_and(|version| version != PRE_AUTH_COMMAND_SCHEMA_VERSION)
    {
        return Err(ConflictableTransactionError::Abort(
            AuthorityTransactionAbort::InvalidSchema(format!(
                "unsupported task network command schema marker {}",
                marker.schema_version
            )),
        ));
    }
    Ok(())
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

fn classify_durability_failure(error: sled::Error) -> TaskNetworkStoreError {
    match to_storage(error) {
        TaskNetworkStoreError::Storage(message) => TaskNetworkStoreError::Storage(format!(
            "task network durability is indeterminate: {message}"
        )),
        TaskNetworkStoreError::CorruptStorage(message) => TaskNetworkStoreError::CorruptStorage(
            format!("task network durability failed on corrupt storage: {message}"),
        ),
        TaskNetworkStoreError::InvalidConfiguration(_) => {
            unreachable!("sled errors cannot produce task network configuration errors")
        }
        TaskNetworkStoreError::Decode(_) => {
            unreachable!("sled errors cannot produce task network decode errors")
        }
    }
}

fn validate_durable_command_record_sizes(
    request: &[u8],
    response: &[u8],
    authentication: &[u8],
) -> Result<(), TaskNetworkStoreError> {
    if request.len() > MAX_DURABLE_COMMAND_REQUEST_BYTES {
        return Err(TaskNetworkStoreError::InvalidConfiguration(format!(
            "durable command request encodes to {} bytes, exceeding limit {MAX_DURABLE_COMMAND_REQUEST_BYTES}",
            request.len()
        )));
    }
    if response.len() > MAX_DURABLE_COMMAND_RESPONSE_BYTES {
        return Err(TaskNetworkStoreError::InvalidConfiguration(format!(
            "durable command response encodes to {} bytes, exceeding limit {MAX_DURABLE_COMMAND_RESPONSE_BYTES}",
            response.len()
        )));
    }
    if authentication.len() > MAX_DURABLE_COMMAND_AUTHENTICATION_BYTES {
        return Err(TaskNetworkStoreError::InvalidConfiguration(format!(
            "durable command authentication encodes to {} bytes, exceeding limit {MAX_DURABLE_COMMAND_AUTHENTICATION_BYTES}",
            authentication.len()
        )));
    }
    Ok(())
}

fn index_incoming_edges(
    state: &NetworkState,
) -> BTreeMap<String, Vec<crate::task_network::state::DependencyEdge>> {
    let mut index = state
        .tasks
        .keys()
        .map(|task_id| (task_id.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for edge in &state.edges {
        index.entry(edge.to.clone()).or_default().push(edge.clone());
    }
    index
}

fn apply_journal_to_incoming_edge_index(
    index: &mut BTreeMap<String, Vec<crate::task_network::state::DependencyEdge>>,
    record: &JournalRecord,
) {
    let JournalRecord::Commit(commit) = record else {
        return;
    };
    for mutation in &commit.mutation_set.mutations {
        let crate::task_network::mutation::Mutation::Inject(inject) = mutation;
        let mut incoming_edges = inject.incoming_edges.clone();
        incoming_edges.sort();
        incoming_edges.dedup();
        index.insert(inject.task_node.task_instance_id.clone(), incoming_edges);
    }
}

// TODO compat-shim: remove after the version one pre-auth command response
// format is outside the supported workspace window and no deployed store can
// omit response authentication. The accepted and rejected predecessor
// characterization, migration parity, exact replay, and concurrent writer
// fence tests must remain green before deletion.
fn initialize_command_schema(
    replayed: &InMemoryTaskNetworkStore,
    command_requests: &Tree,
    command_responses: &Tree,
    command_authentications: &Tree,
    command_schema: &Tree,
) -> Result<(), TaskNetworkStoreError> {
    initialize_command_schema_with_hook(
        replayed,
        command_requests,
        command_responses,
        command_authentications,
        command_schema,
        || {},
    )
}

fn initialize_command_schema_with_hook(
    replayed: &InMemoryTaskNetworkStore,
    command_requests: &Tree,
    command_responses: &Tree,
    command_authentications: &Tree,
    command_schema: &Tree,
    after_snapshot: impl FnOnce(),
) -> Result<(), TaskNetworkStoreError> {
    if let Some(raw_marker) = command_schema.get(KEY_COMMAND_SCHEMA).map_err(to_storage)? {
        let marker: CommandSchemaMarker = serde_json::from_slice(&raw_marker).map_err(to_decode)?;
        if marker.schema_version != COMMAND_SCHEMA_VERSION
            || marker
                .migrated_from
                .is_some_and(|version| version != PRE_AUTH_COMMAND_SCHEMA_VERSION)
        {
            return Err(decode_error(format!(
                "unsupported task network command schema marker {}",
                marker.schema_version
            )));
        }
        return Ok(());
    }

    let raw_requests = raw_tree_entries(command_requests)?;
    let raw_responses = raw_tree_entries(command_responses)?;
    let raw_authentications = raw_tree_entries(command_authentications)?;
    let requests = load_command_requests(command_requests)?;
    let responses = load_command_responses(command_responses)?;
    let authentications = load_command_authentications(command_authentications)?;
    // Sled excludes a predecessor process with its database lock. Within one
    // process, an already-open public store must still cross the schema marker
    // fence before it can add any key after this verified snapshot.
    after_snapshot();
    let immediate_predecessor = !requests.is_empty()
        && requests.values().all(|request| {
            request.legacy_command_id.is_none()
                && request.request_hash == command::request_hash(&request.request)
        })
        && responses
            .values()
            .all(|response| response.authentication.is_none())
        && authentications.is_empty();

    let mut verification = replayed.clone();
    load_command_identity_with_policy(
        &mut verification,
        command_requests,
        command_responses,
        command_authentications,
        immediate_predecessor,
    )?;

    let marker = CommandSchemaMarker::current(
        immediate_predecessor.then_some(PRE_AUTH_COMMAND_SCHEMA_VERSION),
    );
    let marker_value = serde_json::to_vec(&marker).map_err(to_decode)?;
    let mut migrated_requests = Vec::new();
    let mut migrated_responses = Vec::new();
    let mut migrated_authentications = Vec::new();
    if immediate_predecessor {
        for (command_id, request) in &requests {
            let response = responses
                .get(command_id)
                .expect("predecessor outcome key sets were verified");
            let authenticated = StoredCommandResponse::authenticated(
                response.command_id.clone(),
                response.request_hash.clone(),
                response.response.clone(),
            )
            .map_err(decode_error)?;
            let witness =
                StoredCommandAuthentication::for_response(&authenticated).map_err(decode_error)?;
            migrated_requests.push((
                command_id.as_bytes().to_vec(),
                serde_json::to_vec(request).map_err(to_decode)?,
            ));
            migrated_responses.push((
                command_id.as_bytes().to_vec(),
                serde_json::to_vec(&authenticated).map_err(to_decode)?,
            ));
            migrated_authentications.push((
                command_id.as_bytes().to_vec(),
                serde_json::to_vec(&witness).map_err(to_decode)?,
            ));
        }
    }

    (
        command_requests,
        command_responses,
        command_authentications,
        command_schema,
    )
        .transaction(
            |(requests, responses, authentications, schema)|
             -> Result<(), ConflictableTransactionError<String>> {
                if schema.get(KEY_COMMAND_SCHEMA)?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        "command schema initialization lost ownership".to_string(),
                    ));
                }
                validate_transaction_tree_snapshot(requests, &raw_requests)?;
                validate_transaction_tree_snapshot(responses, &raw_responses)?;
                validate_transaction_tree_snapshot(authentications, &raw_authentications)?;
                for (key, value) in &migrated_requests {
                    requests.insert(key.clone(), value.clone())?;
                }
                for (key, value) in &migrated_responses {
                    responses.insert(key.clone(), value.clone())?;
                }
                for (key, value) in &migrated_authentications {
                    authentications.insert(key.clone(), value.clone())?;
                }
                schema.insert(KEY_COMMAND_SCHEMA, marker_value.clone())?;
                Ok(())
            },
        )
        .map_err(|error| match error {
            TransactionError::Abort(message) => decode_error(message),
            TransactionError::Storage(error) => to_storage(error),
        })?;
    command_schema.flush().map_err(to_storage)?;
    Ok(())
}

fn validate_command_schema_marker(command_schema: &Tree) -> Result<(), TaskNetworkStoreError> {
    let raw = command_schema
        .get(KEY_COMMAND_SCHEMA)
        .map_err(to_storage)?
        .ok_or_else(|| decode_error("task network command schema marker is missing"))?;
    let marker: CommandSchemaMarker = serde_json::from_slice(&raw).map_err(to_decode)?;
    if marker.schema_version != COMMAND_SCHEMA_VERSION
        || marker
            .migrated_from
            .is_some_and(|version| version != PRE_AUTH_COMMAND_SCHEMA_VERSION)
    {
        return Err(decode_error(format!(
            "unsupported task network command schema marker {}",
            marker.schema_version
        )));
    }
    Ok(())
}

fn raw_tree_entries(tree: &Tree) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, TaskNetworkStoreError> {
    tree.iter()
        .map(|item| {
            let (key, value) = item.map_err(to_storage)?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect()
}

fn validate_transaction_tree_snapshot(
    tree: &sled::transaction::TransactionalTree,
    expected: &BTreeMap<Vec<u8>, Vec<u8>>,
) -> Result<(), ConflictableTransactionError<String>> {
    for (key, value) in expected {
        if tree.get(key.clone())?.as_deref() != Some(value.as_slice()) {
            return Err(ConflictableTransactionError::Abort(
                "command schema source changed during verified migration".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_command_outcome_binding(
    request: &StoredCommandRequest,
    response: &StoredCommandResponse,
    stored_authentication: Option<&StoredCommandAuthentication>,
    allow_verified_pre_auth: bool,
) -> Result<(), TaskNetworkStoreError> {
    if request.request.command_id != response.command_id
        || request.request_hash != response.request_hash
    {
        return Err(decode_error(
            "stored command request and response identities differ",
        ));
    }
    let is_legacy = request.legacy_command_id.is_some();
    if !is_legacy && request.request_hash != command::request_hash(&request.request) {
        return Err(decode_error("stored command request hash mismatch"));
    }
    match (&response.authentication, stored_authentication) {
        (Some(authentication), Some(stored_authentication)) => {
            authentication
                .validate(
                    &response.command_id,
                    &response.request_hash,
                    &response.response,
                )
                .map_err(decode_error)?;
            if stored_authentication.command_id != response.command_id
                || stored_authentication.request_hash != response.request_hash
                || stored_authentication.response_authentication != *authentication
            {
                return Err(decode_error(
                    "durable command authentication witness mismatch",
                ));
            }
        }
        (Some(_), None) => {
            return Err(decode_error(
                "modern command authentication witness is missing",
            ));
        }
        (None, Some(_)) => {
            return Err(decode_error(
                "legacy command response has a foreign authentication witness",
            ));
        }
        (None, None) if !is_legacy && !allow_verified_pre_auth => {
            return Err(decode_error(
                "modern command response authentication is missing",
            ));
        }
        (None, None) => {}
    }
    Ok(())
}

fn command_outcome_replay_revision(
    request: &StoredCommandRequest,
    response: &StoredCommandResponse,
) -> Result<u64, TaskNetworkStoreError> {
    let replay_revision = match &response.response {
        command::Response::Accepted { revision, .. } => revision
            .checked_sub(1)
            .ok_or_else(|| decode_error("accepted command response revision must be nonzero"))?,
        command::Response::Rejected(Rejection::StaleBase { expected, actual }) => {
            if *expected != request.request.base_revision {
                return Err(decode_error(
                    "stale command response expected revision mismatch",
                ));
            }
            *actual
        }
        command::Response::Rejected(Rejection::DuplicateCommand(_))
        | command::Response::Duplicate { .. } => {
            return Err(decode_error(
                "duplicate command responses are not durable outcome records",
            ));
        }
        command::Response::Rejected(_) => request.request.base_revision,
    };
    Ok(replay_revision)
}

fn load_command_identity(
    inner: &mut InMemoryTaskNetworkStore,
    command_requests: &Tree,
    command_responses: &Tree,
    command_authentications: &Tree,
) -> Result<(), TaskNetworkStoreError> {
    load_command_identity_with_policy(
        inner,
        command_requests,
        command_responses,
        command_authentications,
        false,
    )
}

fn load_command_identity_with_policy(
    inner: &mut InMemoryTaskNetworkStore,
    command_requests: &Tree,
    command_responses: &Tree,
    command_authentications: &Tree,
    allow_verified_pre_auth: bool,
) -> Result<(), TaskNetworkStoreError> {
    let requests = load_command_requests(command_requests)?;
    let responses = load_command_responses(command_responses)?;
    let authentications = load_command_authentications(command_authentications)?;
    let latest_revision = u64::try_from(inner.journal().len())
        .map_err(|_| decode_error("task network journal length exceeds revision range"))?;
    let mut replay_order = Vec::with_capacity(requests.len());

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
        validate_command_outcome_binding(
            request,
            response,
            authentications.get(command_id),
            allow_verified_pre_auth,
        )?;
        let mut replay_revision = command_outcome_replay_revision(request, response)?;
        if matches!(
            response.response,
            command::Response::Rejected(Rejection::InvalidGraph(_))
        ) && replay_revision > latest_revision
        {
            replay_revision = latest_revision;
        }
        if replay_revision > latest_revision {
            return Err(decode_error(format!(
                "command '{command_id}' response references missing historical revision {replay_revision}"
            )));
        }
        replay_order.push((
            replay_revision,
            matches!(response.response, command::Response::Accepted { .. }),
            command_id.clone(),
        ));
    }

    for command_id in responses.keys() {
        if !requests.contains_key(command_id) {
            return Err(decode_error(format!(
                "command '{}' has response but no request",
                command_id
            )));
        }
    }

    for command_id in authentications.keys() {
        if !requests.contains_key(command_id) || !responses.contains_key(command_id) {
            return Err(decode_error(format!(
                "command '{command_id}' has authentication but no complete outcome"
            )));
        }
    }

    // Validate every outcome in one chronological pass. Rejections leave the
    // replay state unchanged, while each accepted outcome must produce the
    // exact next durable journal record. This avoids rebuilding the full
    // network separately for every stored response.
    replay_order.sort();
    let mut replay = InMemoryTaskNetworkStore::new(inner.state().network_id.clone());
    for (replay_revision, advances_revision, command_id) in replay_order {
        advance_replay_through_unattributed_journal(&mut replay, inner.journal(), replay_revision)?;
        if replay.state().revision != replay_revision {
            return Err(decode_error(format!(
                "command '{command_id}' response cannot be placed at historical revision {replay_revision}"
            )));
        }
        let request = requests
            .get(&command_id)
            .expect("replay order contains a stored request");
        let response = responses
            .get(&command_id)
            .expect("replay order contains a stored response");
        let previous_journal_len = replay.journal().len();
        let replayed_response = replay.submit(request.request.clone());
        if replayed_response != response.response {
            return Err(decode_error(format!(
                "command '{command_id}' response differs from deterministic historical replay"
            )));
        }
        if advances_revision {
            let journal_index = usize::try_from(replay_revision)
                .map_err(|_| decode_error("command response revision exceeds index range"))?;
            if replay.journal().last() != inner.journal().get(journal_index) {
                return Err(decode_error(format!(
                    "command '{command_id}' accepted response does not match its journal record"
                )));
            }
        } else if replay.journal().len() != previous_journal_len
            || replay.state().revision != replay_revision
        {
            return Err(decode_error(format!(
                "command '{command_id}' rejected response advanced replay state"
            )));
        }
    }
    advance_replay_through_unattributed_journal(&mut replay, inner.journal(), latest_revision)?;
    if replay.journal() != inner.journal() || replay.state() != inner.state() {
        return Err(decode_error(
            "durable command outcomes and compatibility journal do not reconstruct exact state",
        ));
    }

    for (command_id, request) in requests {
        let response = responses
            .get(&command_id)
            .expect("request and response key sets were validated");
        inner.insert_command_identity(command_id, request.request_hash, response.response.clone());
    }

    Ok(())
}

fn advance_replay_through_unattributed_journal(
    replay: &mut InMemoryTaskNetworkStore,
    journal: &[JournalRecord],
    target_revision: u64,
) -> Result<(), TaskNetworkStoreError> {
    while replay.state().revision < target_revision {
        let journal_index = usize::try_from(replay.state().revision)
            .map_err(|_| decode_error("task network replay revision exceeds index range"))?;
        let record = journal.get(journal_index).ok_or_else(|| {
            decode_error(format!(
                "task network replay references missing revision {}",
                replay.state().revision.saturating_add(1)
            ))
        })?;
        let revision = replay
            .state()
            .revision
            .checked_add(1)
            .ok_or_else(|| decode_error("task network replay revision overflowed"))?;
        replay
            .apply_journal_record_for_replay(revision, &StoredJournalRecord::new(record.clone()))?;
    }
    Ok(())
}

fn load_command_requests(
    command_requests: &Tree,
) -> Result<BTreeMap<String, StoredCommandRequest>, TaskNetworkStoreError> {
    let mut requests = BTreeMap::new();
    for item in command_requests.iter() {
        let (key, value) = item.map_err(to_storage)?;
        if value.len() > MAX_DURABLE_COMMAND_REQUEST_BYTES {
            return Err(decode_error(format!(
                "durable command request exceeds {MAX_DURABLE_COMMAND_REQUEST_BYTES} bytes"
            )));
        }
        let command_id = String::from_utf8(key.to_vec())
            .map_err(|error| decode_error(format!("command request key UTF-8 failed: {error}")))?;
        let stored: StoredCommandRequest = serde_json::from_slice(&value).map_err(to_decode)?;
        if stored.request.command_id != command_id {
            return Err(decode_error("stored command request key mismatch"));
        }
        if stored.legacy_command_id.is_none()
            && stored.request_hash != command::request_hash(&stored.request)
        {
            return Err(decode_error("stored command request hash mismatch"));
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
        if value.len() > MAX_DURABLE_COMMAND_RESPONSE_BYTES {
            return Err(decode_error(format!(
                "durable command response exceeds {MAX_DURABLE_COMMAND_RESPONSE_BYTES} bytes"
            )));
        }
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

fn load_command_authentications(
    command_authentications: &Tree,
) -> Result<BTreeMap<String, StoredCommandAuthentication>, TaskNetworkStoreError> {
    let mut authentications = BTreeMap::new();
    for item in command_authentications.iter() {
        let (key, value) = item.map_err(to_storage)?;
        if value.len() > MAX_DURABLE_COMMAND_AUTHENTICATION_BYTES {
            return Err(decode_error(format!(
                "durable command authentication exceeds {MAX_DURABLE_COMMAND_AUTHENTICATION_BYTES} bytes"
            )));
        }
        let command_id = String::from_utf8(key.to_vec()).map_err(|error| {
            decode_error(format!("command authentication key UTF-8 failed: {error}"))
        })?;
        let stored: StoredCommandAuthentication =
            serde_json::from_slice(&value).map_err(to_decode)?;
        if stored.command_id != command_id {
            return Err(decode_error("stored command authentication key mismatch"));
        }
        authentications.insert(command_id, stored);
    }
    Ok(authentications)
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

fn to_authority_transaction(
    error: TransactionError<AuthorityTransactionAbort>,
) -> AuthorityStoreError {
    match error {
        TransactionError::Abort(
            AuthorityTransactionAbort::InvalidEpoch(message)
            | AuthorityTransactionAbort::InvalidSchema(message),
        ) => TaskNetworkStoreError::Decode(message).into(),
        TransactionError::Abort(AuthorityTransactionAbort::Stale { expected, actual }) => {
            AuthorityStoreError::StaleEpoch { expected, actual }
        }
        TransactionError::Storage(error) => to_storage(error).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_network::mutation::Set;
    use std::sync::{Arc, Barrier};

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

    fn replayed_journal(store: &SledTaskNetworkStore) -> InMemoryTaskNetworkStore {
        let mut replayed = InMemoryTaskNetworkStore::new(store.state().network_id.clone());
        for (index, record) in store.journal().iter().enumerate() {
            replayed
                .apply_journal_record_for_replay(
                    u64::try_from(index).unwrap() + 1,
                    &StoredJournalRecord::new(record.clone()),
                )
                .unwrap();
        }
        replayed
    }

    fn downgrade_to_pre_auth_schema(db: &sled::Db) {
        let responses = db.open_tree(TREE_COMMAND_RESPONSES).unwrap();
        let entries = raw_tree_entries(&responses).unwrap();
        for (key, value) in entries {
            let mut response: StoredCommandResponse = serde_json::from_slice(&value).unwrap();
            response.authentication = None;
            responses
                .insert(key, serde_json::to_vec(&response).unwrap())
                .unwrap();
        }
        db.open_tree(TREE_COMMAND_AUTHENTICATIONS)
            .unwrap()
            .clear()
            .unwrap();
        db.open_tree(TREE_COMMAND_SCHEMA)
            .unwrap()
            .remove(KEY_COMMAND_SCHEMA)
            .unwrap();
        db.flush().unwrap();
    }

    fn migrate_while_public_writer_attempts_insert(rejected: bool) {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let mut writer = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
        let predecessor = empty_command(writer.state(), "command-predecessor");
        assert!(matches!(
            writer.submit(predecessor).unwrap(),
            command::Response::Accepted { revision: 1, .. }
        ));
        downgrade_to_pre_auth_schema(&db);

        let mut candidate = empty_command(writer.state(), "command-concurrent");
        if rejected {
            candidate.base_revision = candidate.base_revision.saturating_add(7);
        }
        let replayed = replayed_journal(&writer);
        let requests = db.open_tree(TREE_COMMAND_REQUESTS).unwrap();
        let responses = db.open_tree(TREE_COMMAND_RESPONSES).unwrap();
        let authentications = db.open_tree(TREE_COMMAND_AUTHENTICATIONS).unwrap();
        let schema = db.open_tree(TREE_COMMAND_SCHEMA).unwrap();
        let snapshot_ready = Arc::new(Barrier::new(2));
        let continue_migration = Arc::new(Barrier::new(2));
        let migration = {
            let snapshot_ready = Arc::clone(&snapshot_ready);
            let continue_migration = Arc::clone(&continue_migration);
            std::thread::spawn(move || {
                initialize_command_schema_with_hook(
                    &replayed,
                    &requests,
                    &responses,
                    &authentications,
                    &schema,
                    || {
                        snapshot_ready.wait();
                        continue_migration.wait();
                    },
                )
            })
        };

        snapshot_ready.wait();
        let fenced = writer.submit(candidate.clone()).unwrap_err();
        assert!(fenced.to_string().contains("schema marker is missing"));
        continue_migration.wait();
        migration.join().unwrap().unwrap();

        let response = writer.submit(candidate.clone()).unwrap();
        if rejected {
            assert!(matches!(
                &response,
                command::Response::Rejected(Rejection::StaleBase { actual: 1, .. })
            ));
        } else {
            assert!(matches!(
                &response,
                command::Response::Accepted { revision: 2, .. }
            ));
        }
        drop(writer);

        let mut reopened = SledTaskNetworkStore::open(db, "network-docs").unwrap();
        let replay = reopened.submit(candidate).unwrap();
        if rejected {
            assert_eq!(replay, response);
        } else {
            assert!(matches!(
                replay,
                command::Response::Duplicate { revision: 2, .. }
            ));
        }
    }

    #[test]
    fn pre_auth_migration_fences_concurrent_public_accepted_insert_and_reopens() {
        migrate_while_public_writer_attempts_insert(false);
    }

    #[test]
    fn pre_auth_migration_fences_concurrent_public_rejected_insert_and_reopens() {
        migrate_while_public_writer_attempts_insert(true);
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
    fn durability_failure_preserves_transient_and_corrupt_storage_classes() {
        let transient = classify_durability_failure(sled::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "temporary flush failure",
        )));
        let corrupt = classify_durability_failure(sled::Error::Unsupported(
            "corrupt flush fixture".to_string(),
        ));

        assert!(matches!(transient, TaskNetworkStoreError::Storage(_)));
        assert!(matches!(corrupt, TaskNetworkStoreError::CorruptStorage(_)));
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
