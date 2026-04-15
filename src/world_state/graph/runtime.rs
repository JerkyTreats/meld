use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::StorageError;
use crate::telemetry::sinks::store::ProgressStore;
use crate::world_state::graph::reducer::TraversalReducer;
use crate::world_state::graph::store::TraversalStore;

pub struct GraphRuntime {
    spine: Arc<ProgressStore>,
    traversal: Arc<TraversalStore>,
    catch_up_lock: Mutex<()>,
}

impl GraphRuntime {
    pub fn new(db: sled::Db) -> Result<Self, StorageError> {
        Ok(Self {
            spine: ProgressStore::shared(db.clone())?,
            traversal: TraversalStore::shared(db)?,
            catch_up_lock: Mutex::new(()),
        })
    }

    pub fn catch_up(&self) -> Result<usize, StorageError> {
        let _guard = self.catch_up_lock.lock();
        let after_seq = self.traversal.last_reduced_seq()?;
        let reducer = TraversalReducer::replay_from_spine(
            self.spine.as_ref(),
            self.traversal.as_ref(),
            after_seq,
        )?;
        Ok(reducer.applied_events)
    }

    pub fn traversal_store(&self) -> Arc<TraversalStore> {
        Arc::clone(&self.traversal)
    }
}

