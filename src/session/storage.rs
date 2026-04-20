use std::io;
use std::sync::Arc;

use sled::{Db, Tree};

use crate::error::StorageError;
use crate::session::contracts::{SessionMeta, SessionRecord};
use crate::session::policy::SessionStatus;
use crate::telemetry::now_millis;

const TREE_SESSIONS: &str = "obs_sessions";
const TREE_META: &str = "obs_session_meta";

#[derive(Clone)]
pub struct SessionStore {
    sessions: Tree,
    meta: Tree,
}

impl SessionStore {
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            sessions: db.open_tree(TREE_SESSIONS).map_err(to_storage_io)?,
            meta: db.open_tree(TREE_META).map_err(to_storage_io)?,
        })
    }

    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    pub fn put_session(&self, record: &SessionRecord) -> Result<(), StorageError> {
        let value = serde_json::to_vec(record).map_err(to_storage_data)?;
        self.sessions
            .insert(record.session_id.as_bytes(), value)
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>, StorageError> {
        let Some(raw) = self
            .sessions
            .get(session_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRecord>, StorageError> {
        let mut out = Vec::new();
        for result in self.sessions.iter() {
            let (_, value) = result.map_err(to_storage_io)?;
            let record = serde_json::from_slice(&value).map_err(to_storage_data)?;
            out.push(record);
        }
        out.sort_by_key(|record: &SessionRecord| std::cmp::Reverse(record.started_at_ms));
        Ok(out)
    }

    pub fn put_meta(&self, session_id: &str, meta: &SessionMeta) -> Result<(), StorageError> {
        let value = serde_json::to_vec(meta).map_err(to_storage_data)?;
        self.meta
            .insert(session_id.as_bytes(), value)
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn get_meta(&self, session_id: &str) -> Result<Option<SessionMeta>, StorageError> {
        let Some(raw) = self
            .meta
            .get(session_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
    }

    pub fn mark_interrupted_sessions(&self) -> Result<usize, StorageError> {
        let mut changed = 0usize;
        for mut session in self.list_sessions()? {
            if session.status == SessionStatus::Active {
                session.status = SessionStatus::Interrupted;
                session.status_text = SessionStatus::Interrupted.as_str().to_string();
                self.put_session(&session)?;
                if let Some(mut meta) = self.get_meta(&session.session_id)? {
                    meta.latest_status = SessionStatus::Interrupted;
                    meta.updated_at_ms = now_millis();
                    self.put_meta(&session.session_id, &meta)?;
                }
                changed += 1;
            }
        }
        Ok(changed)
    }

    pub fn prune_completed(
        &self,
        max_completed: usize,
        max_age_ms: u64,
        now_ms: u64,
    ) -> Result<usize, StorageError> {
        let mut completed: Vec<SessionRecord> = self
            .list_sessions()?
            .into_iter()
            .filter(|session| {
                session.status == SessionStatus::Completed
                    || session.status == SessionStatus::Failed
            })
            .collect();
        completed.sort_by_key(|session| session.started_at_ms);

        let mut removed = 0usize;
        for session in &completed {
            let ended = session.ended_at_ms.unwrap_or(session.started_at_ms);
            if now_ms.saturating_sub(ended) > max_age_ms {
                self.delete_session(session.session_id.as_str())?;
                removed += 1;
            }
        }

        let mut remaining: Vec<SessionRecord> = self
            .list_sessions()?
            .into_iter()
            .filter(|session| {
                session.status == SessionStatus::Completed
                    || session.status == SessionStatus::Failed
            })
            .collect();
        remaining.sort_by_key(|session| std::cmp::Reverse(session.started_at_ms));
        if remaining.len() > max_completed {
            for session in remaining.iter().skip(max_completed) {
                self.delete_session(session.session_id.as_str())?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), StorageError> {
        self.sessions
            .remove(session_id.as_bytes())
            .map_err(to_storage_io)?;
        self.meta
            .remove(session_id.as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}
