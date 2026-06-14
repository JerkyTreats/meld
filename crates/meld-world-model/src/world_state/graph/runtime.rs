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

const GRAPH_ACTOR_ID: &str = "world_state.graph.reducer";

/// Bounded graph catch-up request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCatchUpBudget {
    /// Maximum event records to attempt during one tick.
    pub max_items: usize,
}

/// Diagnostic issue produced by the graph worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphWorkerIssue {
    /// Optional event sequence or derived record identifier.
    pub item_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Diagnostic report from one bounded graph catch-up tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCatchUpReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Event spine sequence read before replay.
    pub input_event_seq: u64,
    /// Event spine sequence durably reduced after replay.
    pub output_event_seq: u64,
    /// Event records selected for this tick.
    pub events_attempted: usize,
    /// Source events that produced graph facts.
    pub traversal_events_applied: usize,
    /// Derived graph events appended idempotently to the spine.
    pub derived_events_appended: usize,
    /// Retryable diagnostics observed during the tick.
    pub retryable_errors: Vec<GraphWorkerIssue>,
    /// Fatal diagnostics observed during the tick.
    pub fatal_errors: Vec<GraphWorkerIssue>,
    /// True when more event records remain after the selected budget.
    pub budget_exhausted: bool,
}

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
        let report = self.catch_up_unbounded()?;
        Ok(report.traversal_events_applied)
    }

    /// Reduce a bounded number of new spine events into traversal indexes.
    pub fn catch_up_bounded(
        &self,
        budget: GraphCatchUpBudget,
    ) -> Result<GraphCatchUpReport, StorageError> {
        if budget.max_items == 0 {
            return Err(StorageError::InvalidPath(
                "graph catch-up budget must be greater than zero".to_string(),
            ));
        }
        self.catch_up_with_limit(Some(budget.max_items))
    }

    fn catch_up_unbounded(&self) -> Result<GraphCatchUpReport, StorageError> {
        self.catch_up_with_limit(None)
    }

    fn catch_up_with_limit(
        &self,
        max_items: Option<usize>,
    ) -> Result<GraphCatchUpReport, StorageError> {
        let _guard = self.catch_up_lock.lock();
        let after_seq = self.traversal.last_reduced_seq()?;
        let (events, budget_exhausted) = match max_items {
            Some(max_items) => {
                let mut events = self
                    .spine
                    .read_all_events_after_limit(after_seq, max_items.saturating_add(1))?;
                let budget_exhausted = events.len() > max_items;
                if budget_exhausted {
                    events.truncate(max_items);
                }
                (events, budget_exhausted)
            }
            None => (self.spine.read_all_events_after(after_seq)?, false),
        };
        let events_attempted = events.len();
        let reducer = TraversalReducer::replay_events(self.traversal.as_ref(), after_seq, events)?;
        let mut last_persisted_seq = reducer.last_seen_seq;
        let derived_events_appended = reducer.emitted_envelopes.len();
        for envelope in reducer.emitted_envelopes {
            let seq = self.spine.append_envelope_idempotent(envelope)?;
            // When source input remains past this tick, the traversal cursor must
            // stay on the processed source event. Derived events are idempotent
            // replay products, but advancing to their later spine sequence would
            // skip unprocessed source events after restart.
            if !budget_exhausted {
                last_persisted_seq = last_persisted_seq.max(seq);
            }
        }
        self.spine.flush()?;
        self.traversal.set_last_reduced_seq(last_persisted_seq)?;
        self.traversal.flush()?;
        Ok(GraphCatchUpReport {
            actor_id: GRAPH_ACTOR_ID.to_string(),
            input_event_seq: after_seq,
            output_event_seq: last_persisted_seq,
            events_attempted,
            traversal_events_applied: reducer.applied_events,
            derived_events_appended,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted,
        })
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
