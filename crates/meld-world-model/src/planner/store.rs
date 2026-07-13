//! Durable request and frame authority for planner projection actors.

use std::collections::BTreeMap;
use std::io;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use parking_lot::Mutex;
use sled::{transaction::Transactional, Db, Tree};

use crate::agent::AgentHydrationFenceCapability;
use crate::belief::{hash_readiness_view, BeliefReadinessSnapshot, BeliefView};
use crate::error::StorageError;
use crate::planner::{
    PlannerProjectionFrame, PlannerProjectionRequest, PlannerProjectionRequestRecord,
    PlannerProjectionRequestStatus, PreparedPlannerProjectionRequest,
};

const TREE_PROJECTION_REQUESTS: &str = "planner_projection_requests";
const TREE_PROJECTION_FRAMES: &str = "planner_projection_frames";
const TREE_PROJECTION_REQUEST_STATUS: &str = "planner_projection_request_status";
const TREE_PROJECTION_REQUEST_OWNER_FENCES: &str = "planner_projection_request_owner_fences";
const TREE_PREPARED_PROJECTION_REQUESTS: &str = "planner_prepared_projection_requests";
const TREE_PROJECTION_REQUEST_VIEWS: &str = "planner_projection_request_views";
const KEY_STATUS_SCHEMA: &[u8] = b"__schema";
const STATUS_SCHEMA_V1: &[u8] = b"planner_projection_request_status.v1";
const PENDING_PREFIX: &[u8] = b"pending::";

/// Deterministic bounded window over durable pending projection requests.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannerPendingSelection {
    /// Pending records ordered by creation sequence and request identity.
    pub records: Vec<PlannerProjectionRequestRecord>,
    /// True when at least one additional pending request remains.
    pub budget_exhausted: bool,
}

/// Opaque authority issued only to the planner actor for one attested request.
pub(super) struct PlannerProjectionTerminalCapability {
    pending: PlannerProjectionRequestRecord,
    owner_fence_bytes: Vec<u8>,
    hydration_fence: AgentHydrationFenceCapability,
}

impl PlannerProjectionTerminalCapability {
    fn validate_call(
        &self,
        current: &PlannerProjectionRequestRecord,
        request_id: &str,
        expected_updated_at_seq: u64,
    ) -> Result<(), StorageError> {
        if self.pending.status != PlannerProjectionRequestStatus::Pending
            || self.pending.request.attested_belief.is_none()
            || self.pending.request.request_id != request_id
            || self.pending.updated_at_seq != expected_updated_at_seq
            || current.request != self.pending.request
            || current.created_at_seq != self.pending.created_at_seq
        {
            return Err(StorageError::Backpressure(
                "planner terminal capability does not authorize this exact request".to_string(),
            ));
        }
        self.hydration_fence.validate_planner_terminal_scope(
            request_id,
            &current.request.agent_id,
            expected_updated_at_seq,
        )
    }
}

#[cfg(test)]
#[derive(Default)]
struct FlushProbe {
    flushes_before_failure: Option<usize>,
}

/// World-model-owned durable planner request and frame store.
#[derive(Clone)]
pub struct PlannerProjectionStore {
    db: Db,
    requests: Tree,
    frames: Tree,
    request_status: Tree,
    request_owner_fences: Tree,
    prepared_requests: Tree,
    request_views: Tree,
    #[cfg(test)]
    flush_probe: Arc<Mutex<FlushProbe>>,
}

impl PlannerProjectionStore {
    /// Open planner projection trees in the shared world-model database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        let store = Self {
            requests: db
                .open_tree(TREE_PROJECTION_REQUESTS)
                .map_err(to_storage_io)?,
            frames: db
                .open_tree(TREE_PROJECTION_FRAMES)
                .map_err(to_storage_io)?,
            request_status: db
                .open_tree(TREE_PROJECTION_REQUEST_STATUS)
                .map_err(to_storage_io)?,
            request_owner_fences: db
                .open_tree(TREE_PROJECTION_REQUEST_OWNER_FENCES)
                .map_err(to_storage_io)?,
            prepared_requests: db
                .open_tree(TREE_PREPARED_PROJECTION_REQUESTS)
                .map_err(to_storage_io)?,
            request_views: db
                .open_tree(TREE_PROJECTION_REQUEST_VIEWS)
                .map_err(to_storage_io)?,
            #[cfg(test)]
            flush_probe: Arc::new(Mutex::new(FlushProbe::default())),
            db,
        };
        store.ensure_status_index()?;
        Ok(store)
    }

    /// Atomically persist one pending request or replay its exact identity.
    pub fn put_pending(
        &self,
        request: PlannerProjectionRequest,
        created_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        request.validate().map_err(to_contract_error)?;
        if request.attested_belief.is_some() {
            return Err(StorageError::InvalidPath(
                "attested planner requests require the hydration submission command".to_string(),
            ));
        }
        self.put_initial_request(request, created_at_seq)
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn put_fenced_pending_for_fuzz(
        &self,
        request: PlannerProjectionRequest,
        created_at_seq: u64,
        capability: &AgentHydrationFenceCapability,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        request.validate().map_err(to_contract_error)?;
        if request.attested_belief.is_none() {
            return Err(StorageError::InvalidPath(
                "fuzz planner request requires an attested snapshot".to_string(),
            ));
        }
        let record = PlannerProjectionRequestRecord {
            request,
            status: PlannerProjectionRequestStatus::Pending,
            frame_id: None,
            last_error: None,
            created_at_seq,
            updated_at_seq: created_at_seq.checked_add(2).ok_or_else(|| {
                StorageError::InvalidPath("fuzz planner request sequence overflow".to_string())
            })?,
        };
        record.validate().map_err(to_contract_error)?;
        validate_store_record_sequence(&record)?;
        let request_bytes = serde_json::to_vec(&record).map_err(to_storage_data)?;
        let fence_bytes = capability.snapshot_bytes()?;
        let status_key = request_status_key(&record);
        (
            &self.requests,
            &self.request_status,
            &self.request_owner_fences,
        )
            .transaction(|(requests, request_status, owner_fences)| {
                insert_exact_transaction_value(
                    requests,
                    record.request.request_id.as_bytes(),
                    &request_bytes,
                    "fuzz fenced planner request",
                )?;
                insert_exact_transaction_value(
                    request_status,
                    status_key.as_bytes(),
                    record.request.request_id.as_bytes(),
                    "fuzz fenced planner status",
                )?;
                insert_exact_transaction_value(
                    owner_fences,
                    record.request.request_id.as_bytes(),
                    &fence_bytes,
                    "fuzz planner owner fence",
                )
            })
            .map_err(|error| match error {
                sled::transaction::TransactionError::Abort(message) => {
                    StorageError::Backpressure(message)
                }
                sled::transaction::TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush_durable("fuzz fenced planner request")?;
        Ok(record)
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn has_owner_fence_for_fuzz(&self, request_id: &str) -> Result<bool, StorageError> {
        self.request_owner_fences
            .contains_key(request_id.as_bytes())
            .map_err(to_storage_io)
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn complete_attested_for_fuzz(
        &self,
        request_id: &str,
        expected_updated_at_seq: u64,
        frame: PlannerProjectionFrame,
        completed_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        let pending = self.require_request(request_id)?;
        let capability = self.terminal_capability_for_actor(&pending)?;
        self.complete_attested_for_actor(
            &capability,
            request_id,
            expected_updated_at_seq,
            frame,
            completed_at_seq,
        )
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn fail_attested_for_fuzz(
        &self,
        request_id: &str,
        expected_updated_at_seq: u64,
        error: impl Into<String>,
        failed_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        let pending = self.require_request(request_id)?;
        let capability = self.terminal_capability_for_actor(&pending)?;
        self.fail_attested_for_actor(
            &capability,
            request_id,
            expected_updated_at_seq,
            error,
            failed_at_seq,
        )
    }

    /// Prepare a non-actionable request bound to one exact agent owner snapshot.
    pub(crate) fn put_prepared_fenced(
        &self,
        request: PlannerProjectionRequest,
        snapshot: &BeliefReadinessSnapshot,
        created_at_seq: u64,
        owner_fence_hash: &str,
    ) -> Result<PreparedPlannerProjectionRequest, StorageError> {
        validate_attested_request_snapshot(&request, snapshot, created_at_seq)?;
        validate_planner_owner_fence_hash(owner_fence_hash)?;
        let prepared = PreparedPlannerProjectionRequest {
            request,
            created_at_seq,
            owner_fence_hash: owner_fence_hash.to_string(),
        };
        let key = prepared.request.request_id.as_bytes();
        let encoded = serde_json::to_vec(&prepared).map_err(to_storage_data)?;
        let view_bytes = serde_json::to_vec(&snapshot.view).map_err(to_storage_data)?;
        use sled::transaction::TransactionError;
        (&self.prepared_requests, &self.request_views)
            .transaction(|(prepared_requests, request_views)| {
                insert_exact_transaction_value(
                    prepared_requests,
                    key,
                    &encoded,
                    "prepared planner request",
                )?;
                insert_exact_transaction_value(
                    request_views,
                    key,
                    &view_bytes,
                    "planner request belief snapshot",
                )
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush_durable("prepared planner request")?;
        Ok(prepared)
    }

    fn put_initial_request(
        &self,
        request: PlannerProjectionRequest,
        created_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        request.validate().map_err(to_contract_error)?;
        if created_at_seq == 0 {
            return Err(StorageError::InvalidPath(
                "planner request creation sequence must be greater than zero".to_string(),
            ));
        }
        let pending = PlannerProjectionRequestRecord {
            request,
            status: PlannerProjectionRequestStatus::Pending,
            frame_id: None,
            last_error: None,
            created_at_seq,
            updated_at_seq: created_at_seq,
        };
        pending.validate().map_err(to_contract_error)?;
        let key = pending.request.request_id.as_bytes();
        let encoded = serde_json::to_vec(&pending).map_err(to_storage_data)?;
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let status_key = request_status_key(&pending);
        // Request visibility and pending selection enter together. An exact
        // retry must also prove the index product instead of repairing drift.
        (&self.requests, &self.request_status)
            .transaction(|(requests, request_status)| {
                if let Some(existing) = requests.get(key)? {
                    if existing.as_ref() != encoded.as_slice() {
                        return Err(ConflictableTransactionError::Abort(
                            "planner request identity conflicts with durable state".to_string(),
                        ));
                    }
                    if request_status.get(status_key.as_bytes())?.as_deref() != Some(key) {
                        return Err(ConflictableTransactionError::Abort(
                            "planner request status index conflicts with durable state".to_string(),
                        ));
                    }
                    return Ok(());
                }
                requests.insert(key, encoded.as_slice())?;
                request_status.insert(status_key.as_bytes(), key)?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush_durable("pending planner request")?;
        Ok(pending)
    }

    pub(crate) fn get_prepared_request(
        &self,
        request_id: &str,
    ) -> Result<Option<PreparedPlannerProjectionRequest>, StorageError> {
        let prepared: Option<PreparedPlannerProjectionRequest> = decode_optional(
            self.prepared_requests
                .get(request_id.as_bytes())
                .map_err(to_storage_io)?,
        )?;
        if let Some(prepared) = prepared.as_ref() {
            validate_prepared_request(prepared)?;
            if prepared.request.request_id != request_id {
                return Err(StorageError::InvalidPath(
                    "prepared planner request tree key conflicts with embedded request identity"
                        .to_string(),
                ));
            }
        }
        Ok(prepared)
    }

    /// Issue an opaque terminal authority for one exact attested pending request.
    pub(super) fn terminal_capability_for_actor(
        &self,
        expected: &PlannerProjectionRequestRecord,
    ) -> Result<PlannerProjectionTerminalCapability, StorageError> {
        expected.validate().map_err(to_contract_error)?;
        validate_store_record_sequence(expected)?;
        if expected.status != PlannerProjectionRequestStatus::Pending
            || expected.request.attested_belief.is_none()
        {
            return Err(StorageError::InvalidPath(
                "planner terminal capability requires an attested pending request".to_string(),
            ));
        }
        if self.get_request(&expected.request.request_id)?.as_ref() != Some(expected) {
            return Err(StorageError::Backpressure(
                "planner request changed before terminal capability issuance".to_string(),
            ));
        }
        let owner_fence_bytes = self
            .request_owner_fences
            .get(expected.request.request_id.as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "attested planner request is missing its hydration owner fence".to_string(),
                )
            })?
            .to_vec();
        let hydration_fence = AgentHydrationFenceCapability::reopen(&self.db, &owner_fence_bytes)?;
        hydration_fence.validate_planner_terminal_scope(
            &expected.request.request_id,
            &expected.request.agent_id,
            expected.updated_at_seq,
        )?;
        Ok(PlannerProjectionTerminalCapability {
            pending: expected.clone(),
            owner_fence_bytes,
            hydration_fence,
        })
    }

    /// Activate one prepared request under an exact agent owner fence.
    pub(crate) fn activate_prepared(
        &self,
        expected: &PreparedPlannerProjectionRequest,
        activated_at_seq: u64,
        fence: &AgentHydrationFenceCapability,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        expected.request.validate().map_err(to_contract_error)?;
        if activated_at_seq != expected.created_at_seq.checked_add(2).unwrap_or(0) {
            return Err(StorageError::InvalidPath(
                "planner request activation requires a prepared record and advancing sequence"
                    .to_string(),
            ));
        }
        let activated = PlannerProjectionRequestRecord {
            request: expected.request.clone(),
            status: PlannerProjectionRequestStatus::Pending,
            frame_id: None,
            last_error: None,
            created_at_seq: expected.created_at_seq,
            updated_at_seq: activated_at_seq,
        };
        activated.validate().map_err(to_contract_error)?;
        let expected_bytes = serde_json::to_vec(expected).map_err(to_storage_data)?;
        let activated_bytes = serde_json::to_vec(&activated).map_err(to_storage_data)?;
        let fence_snapshot_bytes = fence.snapshot_bytes()?;
        let pending_status_key = request_status_key(&activated);
        fence.transaction_four(
            &self.prepared_requests,
            &self.requests,
            &self.request_status,
            &self.request_owner_fences,
            |prepared_requests, requests, request_status, owner_fences| {
                if expected.owner_fence_hash != fence.owner_hash() {
                    return Err(sled::transaction::ConflictableTransactionError::Abort(
                        "prepared planner request owner fence changed".to_string(),
                    ));
                }
                let current_request = requests.get(expected.request.request_id.as_bytes())?;
                if current_request.as_deref() == Some(activated_bytes.as_slice()) {
                    require_transaction_value(
                        request_status,
                        pending_status_key.as_bytes(),
                        expected.request.request_id.as_bytes(),
                        "planner request status index",
                    )?;
                    require_transaction_value(
                        owner_fences,
                        expected.request.request_id.as_bytes(),
                        &fence_snapshot_bytes,
                        "planner request owner fence",
                    )?;
                    if prepared_requests
                        .get(expected.request.request_id.as_bytes())?
                        .is_some()
                    {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(
                            "activated planner request retained its prepared intent".to_string(),
                        ));
                    }
                    return Ok(());
                }
                if current_request.is_some() {
                    return Err(sled::transaction::ConflictableTransactionError::Abort(
                        "planner request became visible before activation".to_string(),
                    ));
                }
                require_transaction_value(
                    prepared_requests,
                    expected.request.request_id.as_bytes(),
                    &expected_bytes,
                    "prepared planner request",
                )?;
                requests.insert(
                    expected.request.request_id.as_bytes(),
                    activated_bytes.as_slice(),
                )?;
                request_status.insert(
                    pending_status_key.as_bytes(),
                    expected.request.request_id.as_bytes(),
                )?;
                insert_exact_transaction_value(
                    owner_fences,
                    expected.request.request_id.as_bytes(),
                    &fence_snapshot_bytes,
                    "planner request owner fence",
                )?;
                prepared_requests.remove(expected.request.request_id.as_bytes())?;
                Ok(())
            },
        )?;
        self.flush_durable("activated planner request")?;
        Ok(activated)
    }

    /// Atomically complete one exact pending request with one durable frame.
    pub fn complete(
        &self,
        request_id: &str,
        expected_updated_at_seq: u64,
        frame: PlannerProjectionFrame,
        completed_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        self.complete_inner(
            request_id,
            expected_updated_at_seq,
            frame,
            completed_at_seq,
            None,
        )
    }

    /// Complete one attested request through actor-only terminal authority.
    pub(super) fn complete_attested_for_actor(
        &self,
        capability: &PlannerProjectionTerminalCapability,
        request_id: &str,
        expected_updated_at_seq: u64,
        frame: PlannerProjectionFrame,
        completed_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        self.complete_inner(
            request_id,
            expected_updated_at_seq,
            frame,
            completed_at_seq,
            Some(capability),
        )
    }

    fn complete_inner(
        &self,
        request_id: &str,
        expected_updated_at_seq: u64,
        frame: PlannerProjectionFrame,
        completed_at_seq: u64,
        capability: Option<&PlannerProjectionTerminalCapability>,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        frame.validate().map_err(to_contract_error)?;
        let expected_terminal_seq = expected_updated_at_seq.checked_add(1).ok_or_else(|| {
            StorageError::InvalidPath(
                "planner request completion sequence cannot advance".to_string(),
            )
        })?;
        if completed_at_seq != expected_terminal_seq || frame.completed_at_seq != completed_at_seq {
            return Err(StorageError::InvalidPath(
                "planner frame completion sequence must be the exact request successor".to_string(),
            ));
        }
        let current = self.require_request(request_id)?;
        match (current.request.attested_belief.is_some(), capability) {
            (true, Some(capability)) => {
                capability.validate_call(&current, request_id, expected_updated_at_seq)?;
            }
            (true, None) => {
                return Err(StorageError::InvalidPath(
                    "attested planner completion requires actor terminal authority".to_string(),
                ));
            }
            (false, Some(_)) => {
                return Err(StorageError::InvalidPath(
                    "attested planner terminal authority cannot complete a public request"
                        .to_string(),
                ));
            }
            (false, None) => {}
        }
        if current.status == PlannerProjectionRequestStatus::Completed {
            let durable_frame = self.get_frame(&frame.identity.frame_id)?;
            if current.updated_at_seq == completed_at_seq
                && current.updated_at_seq > current.created_at_seq
                && frame.completed_at_seq == current.updated_at_seq
                && current.frame_id.as_deref() == Some(frame.identity.frame_id.as_str())
                && durable_frame.as_ref() == Some(&frame)
            {
                self.flush_durable("completed planner request replay")?;
                return Ok(current);
            }
            return Err(StorageError::Backpressure(
                "completed planner request replay conflicts with durable state".to_string(),
            ));
        }
        if current.status != PlannerProjectionRequestStatus::Pending
            || current.updated_at_seq != expected_updated_at_seq
            || frame.identity.request_id != current.request.request_id
            || frame.identity.source_request_hash != current.request.source_request_hash
        {
            return Err(StorageError::Backpressure(
                "planner request completion fence changed".to_string(),
            ));
        }
        let mut completed = current.clone();
        completed.status = PlannerProjectionRequestStatus::Completed;
        completed.frame_id = Some(frame.identity.frame_id.clone());
        completed.updated_at_seq = completed_at_seq;
        completed.validate().map_err(to_contract_error)?;

        let expected = serde_json::to_vec(&current).map_err(to_storage_data)?;
        let completed_bytes = serde_json::to_vec(&completed).map_err(to_storage_data)?;
        let frame_bytes = serde_json::to_vec(&frame).map_err(to_storage_data)?;
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let current_status_key = request_status_key(&current);
        let completed_status_key = request_status_key(&completed);
        if let Some(capability) = capability {
            let retained = capability.hydration_fence.transaction_four_checked(
                &self.requests,
                &self.frames,
                &self.request_status,
                &self.request_owner_fences,
                |requests, frames, request_status, owner_fences, fence_current| {
                    require_transaction_value(
                        requests,
                        request_id.as_bytes(),
                        &expected,
                        "planner request",
                    )?;
                    require_transaction_value(
                        owner_fences,
                        request_id.as_bytes(),
                        &capability.owner_fence_bytes,
                        "planner request owner fence",
                    )?;
                    require_transaction_value(
                        request_status,
                        current_status_key.as_bytes(),
                        request_id.as_bytes(),
                        "planner request status index",
                    )?;
                    if !fence_current {
                        requests.remove(request_id.as_bytes())?;
                        request_status.remove(current_status_key.as_bytes())?;
                        owner_fences.remove(request_id.as_bytes())?;
                        return Ok(false);
                    }
                    if let Some(existing) = frames.get(frame.identity.frame_id.as_bytes())? {
                        if existing.as_ref() != frame_bytes.as_slice() {
                            return Err(ConflictableTransactionError::Abort(
                                "planner frame identity conflicts with durable state".to_string(),
                            ));
                        }
                    } else {
                        frames
                            .insert(frame.identity.frame_id.as_bytes(), frame_bytes.as_slice())?;
                    }
                    requests.insert(request_id.as_bytes(), completed_bytes.as_slice())?;
                    request_status.remove(current_status_key.as_bytes())?;
                    request_status
                        .insert(completed_status_key.as_bytes(), request_id.as_bytes())?;
                    Ok(true)
                },
            )?;
            if !retained {
                self.flush_durable("revoked stale planner request")?;
                return Err(StorageError::Backpressure(
                    "planner request owner fence changed and the stale request was revoked"
                        .to_string(),
                ));
            }
            self.flush_durable("completed fenced planner request")?;
            return Ok(completed);
        }
        // The immutable frame, request CAS, and status-index move commit as one
        // unit so a selector cannot rediscover a terminal request.
        (&self.requests, &self.frames, &self.request_status)
            .transaction(|(requests, frames, request_status)| {
                require_transaction_value(
                    requests,
                    request_id.as_bytes(),
                    &expected,
                    "planner request",
                )?;
                if let Some(existing) = frames.get(frame.identity.frame_id.as_bytes())? {
                    if existing.as_ref() != frame_bytes.as_slice() {
                        return Err(ConflictableTransactionError::Abort(
                            "planner frame identity conflicts with durable state".to_string(),
                        ));
                    }
                } else {
                    frames.insert(frame.identity.frame_id.as_bytes(), frame_bytes.as_slice())?;
                }
                require_transaction_value(
                    request_status,
                    current_status_key.as_bytes(),
                    request_id.as_bytes(),
                    "planner request status index",
                )?;
                requests.insert(request_id.as_bytes(), completed_bytes.as_slice())?;
                request_status.remove(current_status_key.as_bytes())?;
                request_status.insert(completed_status_key.as_bytes(), request_id.as_bytes())?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush_durable("completed planner request")?;
        Ok(completed)
    }

    /// Atomically fail one exact pending request with a bounded diagnostic.
    pub fn fail(
        &self,
        request_id: &str,
        expected_updated_at_seq: u64,
        error: impl Into<String>,
        failed_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        self.fail_inner(
            request_id,
            expected_updated_at_seq,
            error.into(),
            failed_at_seq,
            None,
        )
    }

    /// Fail one attested request through actor-only terminal authority.
    pub(super) fn fail_attested_for_actor(
        &self,
        capability: &PlannerProjectionTerminalCapability,
        request_id: &str,
        expected_updated_at_seq: u64,
        error: impl Into<String>,
        failed_at_seq: u64,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        self.fail_inner(
            request_id,
            expected_updated_at_seq,
            error.into(),
            failed_at_seq,
            Some(capability),
        )
    }

    fn fail_inner(
        &self,
        request_id: &str,
        expected_updated_at_seq: u64,
        error: String,
        failed_at_seq: u64,
        capability: Option<&PlannerProjectionTerminalCapability>,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        if error.trim().is_empty() || error.len() > 1024 {
            return Err(StorageError::InvalidPath(
                "planner failure detail must contain at most 1024 bytes".to_string(),
            ));
        }
        let expected_terminal_seq = expected_updated_at_seq.checked_add(1).ok_or_else(|| {
            StorageError::InvalidPath("planner request failure sequence cannot advance".to_string())
        })?;
        if failed_at_seq != expected_terminal_seq {
            return Err(StorageError::InvalidPath(
                "planner request failure sequence must be the exact request successor".to_string(),
            ));
        }
        let current = self.require_request(request_id)?;
        match (current.request.attested_belief.is_some(), capability) {
            (true, Some(capability)) => {
                capability.validate_call(&current, request_id, expected_updated_at_seq)?;
            }
            (true, None) => {
                return Err(StorageError::InvalidPath(
                    "attested planner failure requires actor terminal authority".to_string(),
                ));
            }
            (false, Some(_)) => {
                return Err(StorageError::InvalidPath(
                    "attested planner terminal authority cannot fail a public request".to_string(),
                ));
            }
            (false, None) => {}
        }
        if current.status == PlannerProjectionRequestStatus::Failed {
            if current.updated_at_seq == failed_at_seq
                && current.updated_at_seq > current.created_at_seq
                && current.last_error.as_deref() == Some(error.as_str())
            {
                self.flush_durable("failed planner request replay")?;
                return Ok(current);
            }
            return Err(StorageError::Backpressure(
                "failed planner request replay conflicts with durable state".to_string(),
            ));
        }
        if current.status != PlannerProjectionRequestStatus::Pending
            || current.updated_at_seq != expected_updated_at_seq
        {
            return Err(StorageError::Backpressure(
                "planner request failure fence changed".to_string(),
            ));
        }
        let mut failed = current.clone();
        failed.status = PlannerProjectionRequestStatus::Failed;
        failed.last_error = Some(error);
        failed.updated_at_seq = failed_at_seq;
        failed.validate().map_err(to_contract_error)?;
        let expected = serde_json::to_vec(&current).map_err(to_storage_data)?;
        let failed_bytes = serde_json::to_vec(&failed).map_err(to_storage_data)?;
        use sled::transaction::TransactionError;
        let current_status_key = request_status_key(&current);
        let failed_status_key = request_status_key(&failed);
        if let Some(capability) = capability {
            let retained = capability.hydration_fence.transaction_four_checked(
                &self.requests,
                &self.frames,
                &self.request_status,
                &self.request_owner_fences,
                |requests, _frames, request_status, owner_fences, fence_current| {
                    require_transaction_value(
                        requests,
                        request_id.as_bytes(),
                        &expected,
                        "planner request",
                    )?;
                    require_transaction_value(
                        owner_fences,
                        request_id.as_bytes(),
                        &capability.owner_fence_bytes,
                        "planner request owner fence",
                    )?;
                    require_transaction_value(
                        request_status,
                        current_status_key.as_bytes(),
                        request_id.as_bytes(),
                        "planner request status index",
                    )?;
                    if !fence_current {
                        requests.remove(request_id.as_bytes())?;
                        request_status.remove(current_status_key.as_bytes())?;
                        owner_fences.remove(request_id.as_bytes())?;
                        return Ok(false);
                    }
                    requests.insert(request_id.as_bytes(), failed_bytes.as_slice())?;
                    request_status.remove(current_status_key.as_bytes())?;
                    request_status.insert(failed_status_key.as_bytes(), request_id.as_bytes())?;
                    Ok(true)
                },
            )?;
            if !retained {
                self.flush_durable("revoked stale planner request")?;
                return Err(StorageError::Backpressure(
                    "planner request owner fence changed and the stale request was revoked"
                        .to_string(),
                ));
            }
            self.flush_durable("failed fenced planner request")?;
            return Ok(failed);
        }
        // Failure removes pending visibility in the same CAS transaction that
        // records the diagnostic, preserving retry and reopen ordering.
        (&self.requests, &self.request_status)
            .transaction(|(requests, request_status)| {
                require_transaction_value(
                    requests,
                    request_id.as_bytes(),
                    &expected,
                    "planner request",
                )?;
                require_transaction_value(
                    request_status,
                    current_status_key.as_bytes(),
                    request_id.as_bytes(),
                    "planner request status index",
                )?;
                requests.insert(request_id.as_bytes(), failed_bytes.as_slice())?;
                request_status.remove(current_status_key.as_bytes())?;
                request_status.insert(failed_status_key.as_bytes(), request_id.as_bytes())?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush_durable("failed planner request")?;
        Ok(failed)
    }

    /// Read one planner projection request record by identity.
    pub fn get_request(
        &self,
        request_id: &str,
    ) -> Result<Option<PlannerProjectionRequestRecord>, StorageError> {
        let record: Option<PlannerProjectionRequestRecord> = decode_optional(
            self.requests
                .get(request_id.as_bytes())
                .map_err(to_storage_io)?,
        )?;
        if let Some(record) = record.as_ref() {
            record.validate().map_err(to_contract_error)?;
            validate_store_record_sequence(record)?;
            if record.request.request_id != request_id {
                return Err(StorageError::InvalidPath(
                    "planner request tree key conflicts with embedded request identity".to_string(),
                ));
            }
        }
        Ok(record)
    }

    /// Read the exact planner-safe view frozen by an attested request.
    pub(crate) fn attested_view_for_request(
        &self,
        request: &PlannerProjectionRequest,
    ) -> Result<Option<BeliefView>, StorageError> {
        let Some(snapshot) = request.attested_belief.as_ref() else {
            return Ok(None);
        };
        let view: BeliefView = decode_optional(
            self.request_views
                .get(request.request_id.as_bytes())
                .map_err(to_storage_io)?,
        )?
        .ok_or_else(|| {
            StorageError::InvalidPath(
                "attested planner request is missing its frozen belief view".to_string(),
            )
        })?;
        if view.view_id != snapshot.view_id
            || view.current_revision_id.as_deref() != Some(snapshot.revision_id.as_str())
            || view.key.subject != request.subject
            || view.key.perspective != request.perspective
            || view.key.branch_scope != request.branch_scope
            || hash_readiness_view(&view)? != snapshot.view_hash
        {
            return Err(StorageError::InvalidPath(
                "attested planner request view conflicts with its frozen boundary".to_string(),
            ));
        }
        Ok(Some(view))
    }

    /// Read one completed planner projection frame by identity.
    pub fn get_frame(
        &self,
        frame_id: &str,
    ) -> Result<Option<PlannerProjectionFrame>, StorageError> {
        let frame: Option<PlannerProjectionFrame> = decode_optional(
            self.frames
                .get(frame_id.as_bytes())
                .map_err(to_storage_io)?,
        )?;
        if let Some(frame) = frame.as_ref() {
            frame.validate().map_err(to_contract_error)?;
            if frame.identity.frame_id != frame_id {
                return Err(StorageError::InvalidPath(
                    "planner frame tree key conflicts with embedded frame identity".to_string(),
                ));
            }
        }
        Ok(frame)
    }

    /// Select pending requests in stable creation and identity order.
    pub fn pending_requests_bounded(
        &self,
        max_items: usize,
    ) -> Result<PlannerPendingSelection, StorageError> {
        if max_items == 0 {
            return Err(StorageError::InvalidPath(
                "planner pending selection budget must be greater than zero".to_string(),
            ));
        }
        let mut records = Vec::new();
        for item in self
            .request_status
            .scan_prefix(PENDING_PREFIX)
            .take(max_items.saturating_add(1))
        {
            let (key, request_id) = item.map_err(to_storage_io)?;
            let request_id = std::str::from_utf8(request_id.as_ref()).map_err(|error| {
                StorageError::InvalidPath(format!(
                    "invalid planner request status identity: {error}"
                ))
            })?;
            let record = self.require_request(request_id)?;
            if record.status != PlannerProjectionRequestStatus::Pending
                || request_status_key(&record).as_bytes() != key.as_ref()
            {
                return Err(StorageError::InvalidPath(
                    "planner request status index conflicts with its request".to_string(),
                ));
            }
            records.push(record);
        }
        let budget_exhausted = records.len() > max_items;
        records.truncate(max_items);
        Ok(PlannerPendingSelection {
            records,
            budget_exhausted,
        })
    }

    /// Verify one exact immutable completed request and frame pair is durable.
    pub fn verify_completed_projection(
        &self,
        expected_request: &PlannerProjectionRequestRecord,
        expected_frame: &PlannerProjectionFrame,
    ) -> Result<(), StorageError> {
        expected_request.validate().map_err(to_contract_error)?;
        expected_frame.validate().map_err(to_contract_error)?;
        if expected_request.status != PlannerProjectionRequestStatus::Completed
            || expected_request.frame_id.as_deref()
                != Some(expected_frame.identity.frame_id.as_str())
            || expected_request.request.request_id != expected_frame.identity.request_id
            || expected_request.updated_at_seq != expected_frame.completed_at_seq
            || expected_request.updated_at_seq <= expected_request.created_at_seq
        {
            return Err(StorageError::InvalidPath(
                "planner readiness products do not form one completed projection".to_string(),
            ));
        }
        if self
            .get_request(&expected_request.request.request_id)?
            .as_ref()
            != Some(expected_request)
            || self.get_frame(&expected_frame.identity.frame_id)?.as_ref() != Some(expected_frame)
        {
            return Err(StorageError::Backpressure(
                "planner readiness products changed or are missing".to_string(),
            ));
        }
        Ok(())
    }

    /// Flush pending planner projection writes.
    pub fn flush(&self) -> Result<(), StorageError> {
        #[cfg(test)]
        {
            let mut probe = self.flush_probe.lock();
            if let Some(remaining) = probe.flushes_before_failure {
                if remaining == 0 {
                    probe.flushes_before_failure = None;
                    return Err(StorageError::IoError(io::Error::other(
                        "injected planner store flush failure",
                    )));
                }
                probe.flushes_before_failure = Some(remaining - 1);
            }
        }
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    fn require_request(
        &self,
        request_id: &str,
    ) -> Result<PlannerProjectionRequestRecord, StorageError> {
        self.get_request(request_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("unknown planner request '{request_id}'"))
        })
    }

    fn ensure_status_index(&self) -> Result<(), StorageError> {
        let schema = self
            .request_status
            .get(KEY_STATUS_SCHEMA)
            .map_err(to_storage_io)?;
        if let Some(schema) = schema.as_ref() {
            if schema.as_ref() != STATUS_SCHEMA_V1 {
                return Err(StorageError::InvalidPath(
                    "planner request status index schema conflicts with this runtime".to_string(),
                ));
            }
        }

        let mut expected = BTreeMap::new();
        for item in &self.requests {
            let (key, raw) = item.map_err(to_storage_io)?;
            let record: PlannerProjectionRequestRecord =
                serde_json::from_slice(raw.as_ref()).map_err(to_storage_data)?;
            record.validate().map_err(to_contract_error)?;
            validate_store_record_sequence(&record)?;
            if key.as_ref() != record.request.request_id.as_bytes() {
                return Err(StorageError::InvalidPath(
                    "planner request tree key conflicts with embedded request identity".to_string(),
                ));
            }
            expected.insert(
                request_status_key(&record).into_bytes(),
                record.request.request_id.into_bytes(),
            );
        }
        let mut actual = BTreeMap::new();
        for item in &self.request_status {
            let (key, value) = item.map_err(to_storage_io)?;
            if key.as_ref() != KEY_STATUS_SCHEMA {
                actual.insert(key.to_vec(), value.to_vec());
            }
        }
        if actual != expected && (schema.is_some() || !actual.is_empty()) {
            return Err(StorageError::InvalidPath(
                "planner request status index conflicts with deterministic backfill".to_string(),
            ));
        }

        if schema.is_some() {
            return Ok(());
        }

        // The marker is written in the same tree batch as every backfilled
        // entry, so reopen observes either a legacy store or a complete v1 index.
        let mut batch = sled::Batch::default();
        for (key, value) in expected {
            batch.insert(key, value);
        }
        batch.insert(KEY_STATUS_SCHEMA, STATUS_SCHEMA_V1);
        self.request_status
            .apply_batch(batch)
            .map_err(to_storage_io)?;
        self.flush_durable("planner request status index migration")
    }

    fn flush_durable(&self, product: &str) -> Result<(), StorageError> {
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!("{product} flush failed: {error}"))
        })
    }

    #[cfg(test)]
    fn fail_next_flush(&self) {
        self.flush_probe.lock().flushes_before_failure = Some(0);
    }

    #[cfg(test)]
    pub(crate) fn fail_flush_after(&self, successful_flushes: usize) {
        self.flush_probe.lock().flushes_before_failure = Some(successful_flushes);
    }
}

fn request_status_key(record: &PlannerProjectionRequestRecord) -> String {
    let status = match record.status {
        PlannerProjectionRequestStatus::Pending => "pending",
        PlannerProjectionRequestStatus::Completed => "completed",
        PlannerProjectionRequestStatus::Failed => "failed",
    };
    format!(
        "{status}::{:020}::{}",
        record.created_at_seq, record.request.request_id
    )
}

fn validate_store_record_sequence(
    record: &PlannerProjectionRequestRecord,
) -> Result<(), StorageError> {
    let valid = match record.status {
        PlannerProjectionRequestStatus::Pending => {
            if record.request.attested_belief.is_some() {
                record.updated_at_seq == record.created_at_seq.checked_add(2).unwrap_or(0)
            } else {
                record.updated_at_seq == record.created_at_seq
            }
        }
        PlannerProjectionRequestStatus::Completed | PlannerProjectionRequestStatus::Failed => {
            if record.request.attested_belief.is_some() {
                record.updated_at_seq == record.created_at_seq.checked_add(3).unwrap_or(0)
            } else {
                record.updated_at_seq == record.created_at_seq.checked_add(1).unwrap_or(0)
            }
        }
    };
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidPath(
            "planner request durable sequence does not match its lifecycle".to_string(),
        ))
    }
}

fn validate_prepared_request(
    prepared: &PreparedPlannerProjectionRequest,
) -> Result<(), StorageError> {
    prepared.request.validate().map_err(to_contract_error)?;
    if prepared.request.attested_belief.is_none() {
        return Err(StorageError::InvalidPath(
            "prepared planner request requires an attested belief boundary".to_string(),
        ));
    }
    if prepared.created_at_seq == 0 {
        return Err(StorageError::InvalidPath(
            "prepared planner request creation sequence must be greater than zero".to_string(),
        ));
    }
    validate_planner_owner_fence_hash(&prepared.owner_fence_hash)
}

fn validate_planner_owner_fence_hash(owner_fence_hash: &str) -> Result<(), StorageError> {
    if owner_fence_hash.len() != 64
        || !owner_fence_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(StorageError::InvalidPath(
            "planner request owner fence must be a lowercase BLAKE3 digest".to_string(),
        ));
    }
    Ok(())
}

fn validate_attested_request_snapshot(
    request: &PlannerProjectionRequest,
    snapshot: &BeliefReadinessSnapshot,
    created_at_seq: u64,
) -> Result<(), StorageError> {
    request.validate().map_err(to_contract_error)?;
    snapshot.validate()?;
    let frozen = request.attested_belief.as_ref().ok_or_else(|| {
        StorageError::InvalidPath(
            "hydration planner request must freeze an attested belief snapshot".to_string(),
        )
    })?;
    if frozen.attestation_id != snapshot.attestation.attestation_id
        || frozen.revision_id != snapshot.attestation.belief_revision_id
        || frozen.revision_hash != snapshot.attestation.belief_revision_hash
        || frozen.view_id != snapshot.attestation.belief_view_id
        || frozen.view_hash != snapshot.attestation.belief_view_hash
        || frozen.source_cursor_start != snapshot.attestation.source_cursor_start
        || frozen.source_cursor_end != snapshot.attestation.source_cursor_end
        || hash_readiness_view(&snapshot.view)? != frozen.view_hash
        || created_at_seq
            != snapshot
                .attestation
                .attested_at_seq
                .checked_add(2)
                .unwrap_or(0)
    {
        return Err(StorageError::InvalidPath(
            "planner request does not preserve its exact attested belief snapshot".to_string(),
        ));
    }
    Ok(())
}

fn require_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: &[u8],
    product: &str,
) -> Result<(), sled::transaction::ConflictableTransactionError<String>> {
    if tree.get(key)?.as_deref() != Some(expected) {
        return Err(sled::transaction::ConflictableTransactionError::Abort(
            format!("{product} changed during planner transition"),
        ));
    }
    Ok(())
}

fn insert_exact_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    value: &[u8],
    product: &str,
) -> Result<(), sled::transaction::ConflictableTransactionError<String>> {
    match tree.get(key)? {
        Some(current) if current.as_ref() != value => {
            Err(sled::transaction::ConflictableTransactionError::Abort(
                format!("{product} conflicts with durable state"),
            ))
        }
        Some(_) => Ok(()),
        None => {
            tree.insert(key, value)?;
            Ok(())
        }
    }
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, StorageError> {
    raw.map(|bytes| serde_json::from_slice(&bytes).map_err(to_storage_data))
        .transpose()
}

fn to_contract_error(error: impl ToString) -> StorageError {
    StorageError::InvalidPath(error.to_string())
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(error))
}

#[cfg(test)]
mod tests {
    use meld_lang::WorldState;

    use super::*;
    use crate::belief::BranchScope;
    use crate::events::DomainObjectRef;
    use crate::planner::{
        PlannerAttestedBeliefSnapshot, PlannerHydrationRefs, PlannerProjectionFrameIdentity,
        PlannerProjectionOutput, PlannerSourceRef, PLANNER_PROJECTION_VERSION,
    };
    use crate::world_state::graph::PerspectiveKey;

    fn request() -> PlannerProjectionRequest {
        PlannerProjectionRequest::identified(
            "execution-request-hash",
            "agent-docs",
            DomainObjectRef::new("workspace_fs", "node", "readme").unwrap(),
            PerspectiveKey::new("agent", "docs").unwrap(),
            BranchScope::main(),
            vec!["docs_freshness".to_string()],
            Vec::new(),
        )
        .unwrap()
    }

    fn request_named(source_hash: &str, subject_id: &str) -> PlannerProjectionRequest {
        PlannerProjectionRequest::identified(
            source_hash,
            "agent-docs",
            DomainObjectRef::new("workspace_fs", "node", subject_id).unwrap(),
            PerspectiveKey::new("agent", "docs").unwrap(),
            BranchScope::main(),
            vec!["docs_freshness".to_string()],
            Vec::new(),
        )
        .unwrap()
    }

    fn attested_request() -> PlannerProjectionRequest {
        PlannerProjectionRequest::identified_attested(
            "execution-request-hash",
            "agent-docs",
            DomainObjectRef::new("workspace_fs", "node", "readme").unwrap(),
            PerspectiveKey::new("agent", "docs").unwrap(),
            BranchScope::main(),
            vec!["docs_freshness".to_string()],
            Vec::new(),
            PlannerAttestedBeliefSnapshot {
                attestation_id: "attestation-a".to_string(),
                revision_id: "revision-a".to_string(),
                revision_hash: "revision-hash-a".to_string(),
                view_id: "view-a".to_string(),
                view_hash: "view-hash-a".to_string(),
                source_cursor_start: 1,
                source_cursor_end: 1,
            },
        )
        .unwrap()
    }

    fn frame(request: &PlannerProjectionRequest, seq: u64) -> PlannerProjectionFrame {
        let output = PlannerProjectionOutput {
            world_state: WorldState::new(Vec::new()).unwrap(),
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
            source_refs: vec![PlannerSourceRef::ProjectionRule {
                rule_id: "readiness".to_string(),
            }],
            hydration_refs: PlannerHydrationRefs::default(),
            warnings: Vec::new(),
        };
        PlannerProjectionFrame {
            identity: PlannerProjectionFrameIdentity::identified(request, &output).unwrap(),
            output,
            completed_at_seq: seq,
        }
    }

    #[test]
    fn request_transitions_are_atomic_replayable_and_reopenable() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("planner");
        let request = request();
        let completed;
        let durable_frame;
        {
            let store = PlannerProjectionStore::new(sled::open(&path).unwrap()).unwrap();
            let pending = store.put_pending(request.clone(), 3).unwrap();
            assert_eq!(pending.status, PlannerProjectionRequestStatus::Pending);
            durable_frame = frame(&request, 4);
            completed = store
                .complete(&request.request_id, 3, durable_frame.clone(), 4)
                .unwrap();
            assert_eq!(completed.status, PlannerProjectionRequestStatus::Completed);
            assert_eq!(
                store
                    .complete(&request.request_id, 3, durable_frame.clone(), 4)
                    .unwrap(),
                completed
            );
        }
        let store = PlannerProjectionStore::new(sled::open(&path).unwrap()).unwrap();
        assert_eq!(
            store.get_request(&request.request_id).unwrap(),
            Some(completed.clone())
        );
        store.fail_next_flush();
        store
            .verify_completed_projection(&completed, &durable_frame)
            .unwrap();
        assert!(store.flush().is_err());

        let mut missing_frame = durable_frame;
        missing_frame.identity.frame_id = "missing-frame".to_string();
        assert!(store
            .verify_completed_projection(&completed, &missing_frame)
            .is_err());
    }

    #[test]
    fn public_product_getters_reject_alias_keys_and_malformed_content() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let request = request();
        let record = store.put_pending(request.clone(), 3).unwrap();
        store
            .requests
            .insert(b"request-alias", serde_json::to_vec(&record).unwrap())
            .unwrap();
        store.fail_next_flush();
        assert!(matches!(
            store.get_request("request-alias"),
            Err(StorageError::InvalidPath(message))
                if message.contains("key conflicts with embedded request identity")
        ));

        let mut malformed_record = record;
        malformed_record.request.source_request_hash.clear();
        store
            .requests
            .insert(
                request.request_id.as_bytes(),
                serde_json::to_vec(&malformed_record).unwrap(),
            )
            .unwrap();
        assert!(store.get_request(&request.request_id).is_err());

        let frame = frame(&request, 4);
        store
            .frames
            .insert(b"frame-alias", serde_json::to_vec(&frame).unwrap())
            .unwrap();
        assert!(matches!(
            store.get_frame("frame-alias"),
            Err(StorageError::InvalidPath(message))
                if message.contains("key conflicts with embedded frame identity")
        ));

        let mut malformed_frame = frame;
        malformed_frame.completed_at_seq = 0;
        store
            .frames
            .insert(
                malformed_frame.identity.frame_id.as_bytes(),
                serde_json::to_vec(&malformed_frame).unwrap(),
            )
            .unwrap();
        assert!(store.get_frame(&malformed_frame.identity.frame_id).is_err());
        assert!(store.flush().is_err());
    }

    #[test]
    fn prepared_request_getter_rejects_alias_keys_and_malformed_content_without_flushing() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let attested = attested_request();
        let prepared = PreparedPlannerProjectionRequest {
            request: attested.clone(),
            created_at_seq: 3,
            owner_fence_hash: "a".repeat(64),
        };
        store
            .prepared_requests
            .insert(b"prepared-alias", serde_json::to_vec(&prepared).unwrap())
            .unwrap();
        store.fail_next_flush();
        assert!(matches!(
            store.get_prepared_request("prepared-alias"),
            Err(StorageError::InvalidPath(message))
                if message.contains("key conflicts with embedded request identity")
        ));

        let mut noncanonical_request = prepared.clone();
        noncanonical_request.request.source_request_hash.clear();
        store
            .prepared_requests
            .insert(
                attested.request_id.as_bytes(),
                serde_json::to_vec(&noncanonical_request).unwrap(),
            )
            .unwrap();
        assert!(store.get_prepared_request(&attested.request_id).is_err());

        let unattested_request = request();
        let missing_attestation = PreparedPlannerProjectionRequest {
            request: unattested_request.clone(),
            created_at_seq: 3,
            owner_fence_hash: "b".repeat(64),
        };
        store
            .prepared_requests
            .insert(
                unattested_request.request_id.as_bytes(),
                serde_json::to_vec(&missing_attestation).unwrap(),
            )
            .unwrap();
        assert!(matches!(
            store.get_prepared_request(&unattested_request.request_id),
            Err(StorageError::InvalidPath(message))
                if message.contains("requires an attested belief boundary")
        ));

        let mut zero_sequence = prepared.clone();
        zero_sequence.created_at_seq = 0;
        store
            .prepared_requests
            .insert(
                attested.request_id.as_bytes(),
                serde_json::to_vec(&zero_sequence).unwrap(),
            )
            .unwrap();
        assert!(matches!(
            store.get_prepared_request(&attested.request_id),
            Err(StorageError::InvalidPath(message))
                if message.contains("sequence must be greater than zero")
        ));

        let mut invalid_owner = prepared;
        invalid_owner.owner_fence_hash = "A".repeat(64);
        store
            .prepared_requests
            .insert(
                attested.request_id.as_bytes(),
                serde_json::to_vec(&invalid_owner).unwrap(),
            )
            .unwrap();
        assert!(matches!(
            store.get_prepared_request(&attested.request_id),
            Err(StorageError::InvalidPath(message))
                if message.contains("lowercase BLAKE3 digest")
        ));
        assert!(store.flush().is_err());
    }

    #[test]
    fn failed_request_replays_exactly_and_rejects_completion() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let request = request();
        store.put_pending(request.clone(), 3).unwrap();
        let failed = store
            .fail(&request.request_id, 3, "missing belief", 4)
            .unwrap();
        assert_eq!(
            store
                .fail(&request.request_id, 3, "missing belief", 4)
                .unwrap(),
            failed
        );
        assert!(store
            .complete(&request.request_id, 3, frame(&request, 4), 4)
            .is_err());
    }

    #[test]
    fn public_terminal_transition_rejects_attested_request() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let request = attested_request();
        let pending = PlannerProjectionRequestRecord {
            request: request.clone(),
            status: PlannerProjectionRequestStatus::Pending,
            frame_id: None,
            last_error: None,
            created_at_seq: 3,
            updated_at_seq: 5,
        };
        store
            .requests
            .insert(
                request.request_id.as_bytes(),
                serde_json::to_vec(&pending).unwrap(),
            )
            .unwrap();
        store
            .request_status
            .insert(
                request_status_key(&pending).as_bytes(),
                request.request_id.as_bytes(),
            )
            .unwrap();

        for result in [
            store.complete(&request.request_id, 5, frame(&request, 6), 6),
            store.fail(&request.request_id, 5, "terminal failure", 6),
        ] {
            assert!(matches!(
                result,
                Err(StorageError::InvalidPath(message))
                    if message.contains("actor terminal authority")
            ));
        }
        assert_eq!(
            store.get_request(&request.request_id).unwrap(),
            Some(pending)
        );
    }

    #[test]
    fn terminal_transitions_require_exact_successor_and_bound_frame_sequence() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let request = request();
        store.put_pending(request.clone(), 3).unwrap();

        assert!(matches!(
            store.complete(&request.request_id, 3, frame(&request, 5), 5),
            Err(StorageError::InvalidPath(_))
        ));
        assert!(matches!(
            store.fail(&request.request_id, 3, "skipped sequence", 5),
            Err(StorageError::InvalidPath(_))
        ));

        let durable_frame = frame(&request, 4);
        let completed = store
            .complete(&request.request_id, 3, durable_frame.clone(), 4)
            .unwrap();
        let mut malformed_record = completed.clone();
        malformed_record.updated_at_seq = 5;
        assert!(matches!(
            store.verify_completed_projection(&malformed_record, &durable_frame),
            Err(StorageError::InvalidPath(_))
        ));
        let mut malformed_frame = durable_frame;
        malformed_frame.completed_at_seq = 5;
        assert!(matches!(
            store.verify_completed_projection(&completed, &malformed_frame),
            Err(StorageError::InvalidPath(_))
        ));
    }

    #[test]
    fn exact_pending_replay_rejects_missing_status_index() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let request = request();
        let pending = store.put_pending(request.clone(), 3).unwrap();
        store
            .request_status
            .remove(request_status_key(&pending).as_bytes())
            .unwrap();

        assert!(matches!(
            store.put_pending(request, 3),
            Err(StorageError::Backpressure(message))
                if message.contains("status index conflicts")
        ));
    }

    #[test]
    fn exact_retry_reflushes_after_indeterminate_flush() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let request = request();
        store.fail_next_flush();
        assert!(matches!(
            store.put_pending(request.clone(), 3),
            Err(StorageError::DurabilityIndeterminate(_))
        ));
        assert_eq!(
            store.put_pending(request.clone(), 3).unwrap().request,
            request
        );
    }

    #[test]
    fn pending_selection_is_bounded_and_independent_of_insertion_order() {
        let first = request_named("source-a", "node-a");
        let second = request_named("source-b", "node-b");
        let expected = vec![second.request_id.clone(), first.request_id.clone()];
        for requests in [
            vec![(first.clone(), 8), (second.clone(), 7)],
            vec![(second.clone(), 7), (first.clone(), 8)],
        ] {
            let store =
                PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                    .unwrap();
            for (request, created_at_seq) in requests {
                store.put_pending(request, created_at_seq).unwrap();
            }
            let first_window = store.pending_requests_bounded(1).unwrap();
            assert!(first_window.budget_exhausted);
            assert_eq!(first_window.records[0].request.request_id, expected[0]);
            let full_window = store.pending_requests_bounded(2).unwrap();
            assert!(!full_window.budget_exhausted);
            assert_eq!(
                full_window
                    .records
                    .into_iter()
                    .map(|record| record.request.request_id)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn reopen_backfills_a_legacy_request_status_index() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("planner-backfill");
        let request = request();
        let record = PlannerProjectionRequestRecord {
            request: request.clone(),
            status: PlannerProjectionRequestStatus::Pending,
            frame_id: None,
            last_error: None,
            created_at_seq: 3,
            updated_at_seq: 3,
        };
        {
            let db = sled::open(&path).unwrap();
            db.open_tree(TREE_PROJECTION_REQUESTS)
                .unwrap()
                .insert(
                    request.request_id.as_bytes(),
                    serde_json::to_vec(&record).unwrap(),
                )
                .unwrap();
            db.flush().unwrap();
        }

        let store = PlannerProjectionStore::new(sled::open(&path).unwrap()).unwrap();
        assert_eq!(
            store.pending_requests_bounded(1).unwrap().records,
            vec![record]
        );
        assert_eq!(
            store
                .request_status
                .get(KEY_STATUS_SCHEMA)
                .unwrap()
                .unwrap(),
            STATUS_SCHEMA_V1
        );
    }

    #[test]
    fn reopen_rejects_a_conflicting_legacy_request_status_index() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("planner-conflict");
        let request = request();
        let record = PlannerProjectionRequestRecord {
            request: request.clone(),
            status: PlannerProjectionRequestStatus::Pending,
            frame_id: None,
            last_error: None,
            created_at_seq: 3,
            updated_at_seq: 3,
        };
        {
            let db = sled::open(&path).unwrap();
            db.open_tree(TREE_PROJECTION_REQUESTS)
                .unwrap()
                .insert(
                    request.request_id.as_bytes(),
                    serde_json::to_vec(&record).unwrap(),
                )
                .unwrap();
            db.open_tree(TREE_PROJECTION_REQUEST_STATUS)
                .unwrap()
                .insert("pending::00000000000000000003::wrong", "wrong")
                .unwrap();
            db.flush().unwrap();
        }

        assert!(matches!(
            PlannerProjectionStore::new(sled::open(&path).unwrap()),
            Err(StorageError::InvalidPath(message))
                if message.contains("conflicts with deterministic backfill")
        ));
    }

    #[test]
    fn reopen_rejects_a_legacy_request_under_the_wrong_tree_key() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("planner-key-conflict");
        let request = request();
        let record = PlannerProjectionRequestRecord {
            request,
            status: PlannerProjectionRequestStatus::Pending,
            frame_id: None,
            last_error: None,
            created_at_seq: 3,
            updated_at_seq: 3,
        };
        {
            let db = sled::open(&path).unwrap();
            db.open_tree(TREE_PROJECTION_REQUESTS)
                .unwrap()
                .insert("wrong-request-key", serde_json::to_vec(&record).unwrap())
                .unwrap();
            db.flush().unwrap();
        }

        assert!(matches!(
            PlannerProjectionStore::new(sled::open(&path).unwrap()),
            Err(StorageError::InvalidPath(message))
                if message.contains("tree key conflicts with embedded request identity")
        ));
    }

    #[test]
    fn reopen_rejects_a_marked_but_incomplete_status_index() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("planner-marked-conflict");
        let request = request();
        {
            let store = PlannerProjectionStore::new(sled::open(&path).unwrap()).unwrap();
            let record = store.put_pending(request, 3).unwrap();
            store
                .request_status
                .remove(request_status_key(&record))
                .unwrap();
            store.db.flush().unwrap();
        }

        assert!(matches!(
            PlannerProjectionStore::new(sled::open(&path).unwrap()),
            Err(StorageError::InvalidPath(message))
                if message.contains("conflicts with deterministic backfill")
        ));
    }

    #[test]
    fn stale_completion_cannot_replace_the_winning_frame() {
        let store =
            PlannerProjectionStore::new(sled::Config::new().temporary(true).open().unwrap())
                .unwrap();
        let request = request();
        store.put_pending(request.clone(), 3).unwrap();
        let winning = frame(&request, 4);
        let mut losing = frame(&request, 4);
        losing
            .output
            .source_refs
            .push(PlannerSourceRef::ProjectionRule {
                rule_id: "different".to_string(),
            });
        losing.output.source_refs.sort();
        losing.identity =
            PlannerProjectionFrameIdentity::identified(&request, &losing.output).unwrap();
        store
            .complete(&request.request_id, 3, winning.clone(), 4)
            .unwrap();

        assert!(matches!(
            store.complete(&request.request_id, 3, losing, 4),
            Err(StorageError::Backpressure(message))
                if message.contains("conflicts with durable state")
        ));
        assert_eq!(
            store.get_frame(&winning.identity.frame_id).unwrap(),
            Some(winning)
        );
    }
}
