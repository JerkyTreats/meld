//! Durable request and frame authority for planner projection actors.

use std::collections::BTreeMap;
use std::io;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use parking_lot::Mutex;
use sled::{transaction::Transactional, Db, Tree};

use crate::error::StorageError;
use crate::planner::{
    PlannerProjectionFrame, PlannerProjectionRequest, PlannerProjectionRequestRecord,
    PlannerProjectionRequestStatus,
};

const TREE_PROJECTION_REQUESTS: &str = "planner_projection_requests";
const TREE_PROJECTION_FRAMES: &str = "planner_projection_frames";
const TREE_PROJECTION_REQUEST_STATUS: &str = "planner_projection_request_status";
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

    /// Atomically complete one exact pending request with one durable frame.
    pub fn complete(
        &self,
        request_id: &str,
        expected_updated_at_seq: u64,
        frame: PlannerProjectionFrame,
        completed_at_seq: u64,
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
        if current.status == PlannerProjectionRequestStatus::Completed {
            let durable_frame = self.get_frame(&frame.identity.frame_id)?;
            if current.updated_at_seq == completed_at_seq
                && current.updated_at_seq == current.created_at_seq.checked_add(1).unwrap_or(0)
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
            || current.updated_at_seq != current.created_at_seq
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
        let error = error.into();
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
        if current.status == PlannerProjectionRequestStatus::Failed {
            if current.updated_at_seq == failed_at_seq
                && current.updated_at_seq == current.created_at_seq.checked_add(1).unwrap_or(0)
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
            || current.updated_at_seq != current.created_at_seq
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
        }
        Ok(record)
    }

    /// Read one completed planner projection frame by identity.
    pub fn get_frame(
        &self,
        frame_id: &str,
    ) -> Result<Option<PlannerProjectionFrame>, StorageError> {
        decode_optional(
            self.frames
                .get(frame_id.as_bytes())
                .map_err(to_storage_io)?,
        )
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
            || expected_request.updated_at_seq
                != expected_request.created_at_seq.checked_add(1).unwrap_or(0)
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
        self.flush_durable("planner readiness verification")
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
        PlannerProjectionRequestStatus::Pending => record.updated_at_seq == record.created_at_seq,
        PlannerProjectionRequestStatus::Completed | PlannerProjectionRequestStatus::Failed => {
            record.updated_at_seq == record.created_at_seq.checked_add(1).unwrap_or(0)
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
        PlannerHydrationRefs, PlannerProjectionFrameIdentity, PlannerProjectionOutput,
        PlannerSourceRef, PLANNER_PROJECTION_VERSION,
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
        store
            .verify_completed_projection(&completed, &durable_frame)
            .unwrap();

        let mut missing_frame = durable_frame;
        missing_frame.identity.frame_id = "missing-frame".to_string();
        assert!(store
            .verify_completed_projection(&completed, &missing_frame)
            .is_err());
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
