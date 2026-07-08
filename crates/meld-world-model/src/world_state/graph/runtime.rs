//! Runtime wrapper for graph event reduction.
//!
//! `GraphRuntime` owns the event ledger and traversal store that share one sled
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
    /// Event ledger sequence read before replay.
    pub input_event_seq: u64,
    /// Event ledger sequence durably reduced after replay.
    pub output_event_seq: u64,
    /// Event records selected for this tick.
    pub events_attempted: usize,
    /// Source events that produced graph facts.
    pub traversal_events_applied: usize,
    /// Derived graph events appended idempotently to the ledger.
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
    ledger: Arc<EventStore>,
    traversal: Arc<TraversalStore>,
    catch_up_lock: Mutex<()>,
}

impl GraphRuntime {
    /// Open the event ledger and traversal store against one shared database.
    pub fn new(db: sled::Db) -> Result<Self, StorageError> {
        Ok(Self {
            ledger: EventStore::shared(db.clone())?,
            traversal: TraversalStore::shared(db)?,
            catch_up_lock: Mutex::new(()),
        })
    }

    /// Build graph projection runtime from already opened product stores.
    ///
    /// This is used when the product runtime keeps the event ledger and world
    /// model graph stores in separate physical databases while graph replay
    /// ownership remains inside the world model domain.
    pub fn from_stores(ledger: Arc<EventStore>, traversal: Arc<TraversalStore>) -> Self {
        Self {
            ledger,
            traversal,
            catch_up_lock: Mutex::new(()),
        }
    }

    /// Reduce new ledger events into traversal indexes.
    ///
    /// A retention gap propagates as its typed error so CLI catch-up
    /// callers can never mistake a stranded cursor for zero progress.
    pub fn catch_up(&self) -> Result<usize, StorageError> {
        let report = self.catch_up_unbounded()?;
        if report
            .fatal_errors
            .iter()
            .any(|issue| issue.code == "retention_gap")
        {
            return Err(StorageError::RetentionGap {
                after_seq: self.traversal.last_reduced_seq()?,
                retained_from: self.ledger.retained_lower_boundary()?,
            });
        }
        Ok(report.traversal_events_applied)
    }

    /// Reduce a bounded number of new ledger events into traversal indexes.
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
        let read = match max_items {
            Some(max_items) => self
                .ledger
                .read_all_events_after_limit(after_seq, max_items.saturating_add(1)),
            None => self.ledger.read_all_events_after(after_seq),
        };
        let (events, budget_exhausted) = match read {
            Ok(mut events) => match max_items {
                Some(max_items) => {
                    let budget_exhausted = events.len() > max_items;
                    if budget_exhausted {
                        events.truncate(max_items);
                    }
                    (events, budget_exhausted)
                }
                None => (events, false),
            },
            // A retention gap means compaction pruned events this cursor has
            // not reduced. Replaying through the gap would corrupt the
            // projection, so the tick reports a fatal diagnostic without
            // moving the cursor; the operator rebuilds from a genesis fact.
            Err(StorageError::RetentionGap {
                after_seq: gap_cursor,
                retained_from,
            }) => {
                return Ok(GraphCatchUpReport {
                    actor_id: GRAPH_ACTOR_ID.to_string(),
                    input_event_seq: after_seq,
                    output_event_seq: after_seq,
                    events_attempted: 0,
                    traversal_events_applied: 0,
                    derived_events_appended: 0,
                    retryable_errors: Vec::new(),
                    fatal_errors: vec![GraphWorkerIssue {
                        item_id: Some(format!("event_spine_seq::{gap_cursor}")),
                        code: "retention_gap".to_string(),
                        message: format!(
                            "traversal cursor {gap_cursor} predates retained history starting \
                             at {retained_from}; rebuild the projection from a genesis fact"
                        ),
                    }],
                    budget_exhausted: false,
                });
            }
            Err(error) => return Err(error),
        };
        let events_attempted = events.len();
        let reducer = TraversalReducer::replay_events(self.traversal.as_ref(), after_seq, events)?;
        // The cursor never advances past the highest processed source event.
        // Producers may append between this tick's read and its derived-event
        // appends, so advancing to a derived sequence would skip those source
        // events permanently. Re-reading own derived events next tick is safe:
        // they are not traversal source events and their appends are
        // idempotent.
        let last_persisted_seq = reducer.last_seen_seq;
        let derived_events_appended = reducer.emitted_envelopes.len();
        for envelope in reducer.emitted_envelopes {
            self.ledger.append_envelope_idempotent(envelope)?;
        }
        self.ledger.flush()?;
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

    /// Append a source event to the ledger.
    pub fn append_envelope(&self, envelope: EventEnvelope) -> Result<u64, StorageError> {
        let seq = self.ledger.append_envelope(envelope)?;
        self.ledger.flush()?;
        Ok(seq)
    }
}
