//! Factory for durable task artifact repositories.

use super::{TaskArtifactRepo, TaskArtifactRepoError};

/// Opens task-scoped artifact repositories from a caller-owned database.
///
/// Product assembly owns the database path. The task domain owns repo scoping,
/// durable record validation, and replay behavior.
#[derive(Clone)]
pub struct TaskArtifactRepoFactory {
    db: sled::Db,
}

impl TaskArtifactRepoFactory {
    /// Create a factory from a caller-owned sled database.
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }

    /// Open one task artifact repository scoped by repo id.
    pub fn open_repo(
        &self,
        repo_id: impl Into<String>,
    ) -> Result<TaskArtifactRepo, TaskArtifactRepoError> {
        TaskArtifactRepo::open_sled(self.db.clone(), repo_id)
    }

    /// Flush all task artifact repository writes in the shared database.
    pub fn flush(&self) -> Result<(), TaskArtifactRepoError> {
        self.db
            .flush()
            .map_err(|error| TaskArtifactRepoError::Storage(error.to_string()))?;
        Ok(())
    }
}
