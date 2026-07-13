//! Durable request and frame authority for planner projection actors.

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

#[cfg(test)]
#[derive(Default)]
struct FlushProbe {
    fail_next: bool,
}

/// World-model-owned durable planner request and frame store.
#[derive(Clone)]
pub struct PlannerProjectionStore {
    db: Db,
    requests: Tree,
    frames: Tree,
    #[cfg(test)]
    flush_probe: Arc<Mutex<FlushProbe>>,
}

impl PlannerProjectionStore {
    /// Open planner projection trees in the shared world-model database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            requests: db
                .open_tree(TREE_PROJECTION_REQUESTS)
                .map_err(to_storage_io)?,
            frames: db
                .open_tree(TREE_PROJECTION_FRAMES)
                .map_err(to_storage_io)?,
            #[cfg(test)]
            flush_probe: Arc::new(Mutex::new(FlushProbe::default())),
            db,
        })
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
        (&self.requests,)
            .transaction(|(requests,)| {
                if let Some(existing) = requests.get(key)? {
                    if existing.as_ref() != encoded.as_slice() {
                        return Err(ConflictableTransactionError::Abort(
                            "planner request identity conflicts with durable state".to_string(),
                        ));
                    }
                } else {
                    requests.insert(key, encoded.as_slice())?;
                }
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
        if completed_at_seq == 0 || frame.completed_at_seq != completed_at_seq {
            return Err(StorageError::InvalidPath(
                "planner frame completion sequence must match the transition".to_string(),
            ));
        }
        let current = self.require_request(request_id)?;
        if current.status == PlannerProjectionRequestStatus::Completed {
            let durable_frame = self.get_frame(&frame.identity.frame_id)?;
            if current.updated_at_seq == completed_at_seq
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
            || completed_at_seq <= expected_updated_at_seq
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
        (&self.requests, &self.frames)
            .transaction(|(requests, frames)| {
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
                requests.insert(request_id.as_bytes(), completed_bytes.as_slice())?;
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
        let current = self.require_request(request_id)?;
        if current.status == PlannerProjectionRequestStatus::Failed {
            if current.updated_at_seq == failed_at_seq
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
            || failed_at_seq <= expected_updated_at_seq
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
        (&self.requests,)
            .transaction(|(requests,)| {
                require_transaction_value(
                    requests,
                    request_id.as_bytes(),
                    &expected,
                    "planner request",
                )?;
                requests.insert(request_id.as_bytes(), failed_bytes.as_slice())?;
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
        decode_optional(
            self.requests
                .get(request_id.as_bytes())
                .map_err(to_storage_io)?,
        )
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
            if probe.fail_next {
                probe.fail_next = false;
                return Err(StorageError::IoError(io::Error::other(
                    "injected planner store flush failure",
                )));
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

    fn flush_durable(&self, product: &str) -> Result<(), StorageError> {
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!("{product} flush failed: {error}"))
        })
    }

    #[cfg(test)]
    fn fail_next_flush(&self) {
        self.flush_probe.lock().fail_next = true;
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
}
