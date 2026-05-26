//! Runtime wrapper for graph event reduction.
//!
//! `GraphRuntime` owns the event spine and traversal store that share one sled
//! database. Reads call `catch_up` before querying so graph indexes observe all
//! durable events seen by the runtime.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::graph::runtime::GraphRuntime;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let runtime = GraphRuntime::new(sled::open(temp.path()).unwrap()).unwrap();
//! assert_eq!(runtime.catch_up().unwrap(), 0);
//! ```

use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::StorageError;
use crate::events::store::EventStore;
use crate::events::EventEnvelope;
use crate::world_state::graph::reducer::TraversalReducer;
use crate::world_state::graph::store::TraversalStore;

/// Event-backed graph projection runtime.
pub struct GraphRuntime {
    spine: Arc<EventStore>,
    traversal: Arc<TraversalStore>,
    catch_up_lock: Mutex<()>,
}

impl GraphRuntime {
    /// Open the event spine and traversal store against one shared database.
    pub fn new(db: sled::Db) -> Result<Self, StorageError> {
        Ok(Self {
            spine: EventStore::shared(db.clone())?,
            traversal: TraversalStore::shared(db)?,
            catch_up_lock: Mutex::new(()),
        })
    }

    /// Reduce new spine events into traversal indexes.
    pub fn catch_up(&self) -> Result<usize, StorageError> {
        let _guard = self.catch_up_lock.lock();
        let after_seq = self.traversal.last_reduced_seq()?;
        let reducer = TraversalReducer::replay_from_spine(
            self.spine.as_ref(),
            self.traversal.as_ref(),
            after_seq,
        )?;
        let mut last_persisted_seq = reducer.last_seen_seq;
        for envelope in reducer.emitted_envelopes {
            let seq = self.spine.append_envelope_idempotent(envelope)?;
            last_persisted_seq = last_persisted_seq.max(seq);
        }
        self.spine.flush()?;
        self.traversal.set_last_reduced_seq(last_persisted_seq)?;
        self.traversal.flush()?;
        Ok(reducer.applied_events)
    }

    /// Clone the shared traversal store.
    pub fn traversal_store(&self) -> Arc<TraversalStore> {
        Arc::clone(&self.traversal)
    }

    /// Append a source event to the spine.
    pub fn append_envelope(&self, envelope: EventEnvelope) -> Result<u64, StorageError> {
        let seq = self.spine.append_envelope(envelope)?;
        self.spine.flush()?;
        Ok(seq)
    }
}
