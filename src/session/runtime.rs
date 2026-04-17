use std::sync::Arc;

use crate::error::{ApiError, StorageError};
use crate::session::contracts::{SessionMeta, SessionRecord};
use crate::session::policy::PrunePolicy;
use crate::session::storage::SessionStore;
use crate::telemetry::{new_session_id, now_millis};

#[derive(Clone)]
pub struct SessionRuntime {
    store: Arc<SessionStore>,
}

impl SessionRuntime {
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    pub fn start_command_session(&self, command_name: String) -> Result<String, ApiError> {
        let session_id = new_session_id();
        let started = now_millis();
        let record = SessionRecord::command(session_id.clone(), command_name, started);
        self.store.put_session(&record)?;
        self.store
            .put_meta(&session_id, &SessionMeta::active(started))?;
        Ok(session_id)
    }

    pub fn finish_command_session(
        &self,
        session_id: &str,
        success: bool,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let status = if success {
            crate::session::policy::SessionStatus::Completed
        } else {
            crate::session::policy::SessionStatus::Failed
        };
        let mut record = self.store.get_session(session_id)?.ok_or_else(|| {
            ApiError::StorageError(StorageError::InvalidPath(
                "session record missing".to_string(),
            ))
        })?;
        record.status = status;
        record.status_text = status.as_str().to_string();
        record.ended_at_ms = Some(now_millis());
        record.error = error;
        self.store.put_session(&record)?;
        if let Some(mut meta) = self.store.get_meta(session_id)? {
            meta.latest_status = status;
            meta.updated_at_ms = now_millis();
            self.store.put_meta(session_id, &meta)?;
        }
        Ok(())
    }

    pub fn mark_interrupted_sessions(&self) -> Result<usize, ApiError> {
        self.store
            .mark_interrupted_sessions()
            .map_err(ApiError::StorageError)
    }

    pub fn prune(&self, policy: PrunePolicy) -> Result<usize, ApiError> {
        self.store
            .prune_completed(policy.max_completed, policy.max_age_ms, now_millis())
            .map_err(ApiError::StorageError)
    }

    pub fn store(&self) -> &SessionStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::policy::SessionStatus;

    #[test]
    fn session_lifecycle_round_trips() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = SessionStore::shared(db).unwrap();
        let runtime = SessionRuntime::new(store.clone());

        let session_id = runtime.start_command_session("scan".to_string()).unwrap();
        let started = store.get_session(&session_id).unwrap().unwrap();
        assert_eq!(started.command, "scan");
        assert_eq!(started.status, SessionStatus::Active);

        runtime
            .finish_command_session(&session_id, true, None)
            .unwrap();
        let finished = store.get_session(&session_id).unwrap().unwrap();
        assert_eq!(finished.status, SessionStatus::Completed);
        assert!(finished.ended_at_ms.is_some());
    }

    #[test]
    fn interrupted_sessions_are_marked() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = SessionStore::shared(db).unwrap();
        let runtime = SessionRuntime::new(store.clone());

        let session_id = runtime
            .start_command_session("workspace.watch".to_string())
            .unwrap();
        let changed = runtime.mark_interrupted_sessions().unwrap();
        assert_eq!(changed, 1);

        let record = store.get_session(&session_id).unwrap().unwrap();
        assert_eq!(record.status, SessionStatus::Interrupted);
    }
}
