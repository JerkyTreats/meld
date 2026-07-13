//! Runtime wrapper for graph event reduction.
//!
//! `GraphRuntime` consumes event-authority ports and owns traversal projection
//! coordination. Reads call `catch_up` before querying so graph indexes observe
//! all durable events exposed by the supplied replay capability.
//!
//! Product composition supplies all three authority-derived ports to
//! [`GraphRuntime::from_ports`].

use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::Mutex;

use crate::error::StorageError;
use meld_events::error::EventAuthorityError;

use crate::events::observability::CoverageTruncation;
use crate::events::{
    AppendDisposition, AppendReceipt, EventPage, EventRecord, LedgerCursor, LedgerIdentity,
    ReplayRequest, MAX_REPLAY_LIMIT,
};
use crate::world_state::graph::cursor::GraphProjectionCursor;
use crate::world_state::graph::outbox::GraphDerivedOutbox;
use crate::world_state::graph::ports::{
    GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource,
};
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
    replay: Arc<dyn GraphEventReplaySource>,
    derived_sink: Arc<dyn GraphDerivedEventSink>,
    cursor_reporter: Arc<dyn GraphConsumerCursorReporter>,
    ledger_id: LedgerIdentity,
    traversal: Arc<TraversalStore>,
    cursor: GraphProjectionCursor,
    derived_outbox: GraphDerivedOutbox,
    catch_up_lock: Mutex<()>,
    mutation_lock: Arc<Mutex<()>>,
    owner_id: String,
}

impl GraphRuntime {
    /// Builds a graph projection from product-owned event authority ports.
    pub fn from_ports(
        replay: Arc<dyn GraphEventReplaySource>,
        derived_sink: Arc<dyn GraphDerivedEventSink>,
        cursor_reporter: Arc<dyn GraphConsumerCursorReporter>,
        traversal: Arc<TraversalStore>,
    ) -> Result<Self, StorageError> {
        let ledger_id = replay.ledger_identity();
        for actual in [
            derived_sink.ledger_identity(),
            cursor_reporter.ledger_identity(),
        ] {
            if actual != ledger_id {
                return Err(StorageError::IdentityMismatch {
                    expected: ledger_id,
                    actual,
                });
            }
        }
        let cursor = GraphProjectionCursor::open(traversal.as_ref(), ledger_id)?;
        let derived_outbox = GraphDerivedOutbox::open(traversal.db())?;
        let mutation_lock = graph_mutation_lock(ledger_id);
        Ok(Self {
            replay,
            derived_sink,
            cursor_reporter,
            ledger_id,
            traversal,
            cursor,
            derived_outbox,
            catch_up_lock: Mutex::new(()),
            mutation_lock,
            owner_id: format!("graph-runtime-{}", LedgerIdentity::new()),
        })
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
            return match self.replay.replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: self.ledger_id,
                    after_seq: report.input_event_seq,
                },
                limit: 1,
            }) {
                Err(error) => Err(authority_error_to_storage(error)),
                Ok(_) => Err(StorageError::InvalidPath(
                    "graph replay retention gap changed while producing its report".to_string(),
                )),
            };
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
        if budget.max_items > MAX_REPLAY_LIMIT {
            return Err(StorageError::InvalidPath(format!(
                "graph catch-up budget must be in 1..={MAX_REPLAY_LIMIT}"
            )));
        }
        self.catch_up_with_limit(budget.max_items)
    }

    fn catch_up_unbounded(&self) -> Result<GraphCatchUpReport, StorageError> {
        let mut combined = self.catch_up_with_limit(MAX_REPLAY_LIMIT)?;
        while combined.budget_exhausted {
            let next = self.catch_up_with_limit(MAX_REPLAY_LIMIT)?;
            combined.output_event_seq = next.output_event_seq;
            combined.events_attempted += next.events_attempted;
            combined.traversal_events_applied += next.traversal_events_applied;
            combined.derived_events_appended += next.derived_events_appended;
            combined.retryable_errors.extend(next.retryable_errors);
            combined.fatal_errors.extend(next.fatal_errors);
            combined.budget_exhausted = next.budget_exhausted;
            if !combined.fatal_errors.is_empty() {
                break;
            }
        }
        Ok(combined)
    }

    fn catch_up_with_limit(&self, max_items: usize) -> Result<GraphCatchUpReport, StorageError> {
        let _guard = self.catch_up_lock.lock();
        let mut derived_events_appended = {
            let _mutation_guard = self.mutation_lock.lock();
            // TODO compat-shim: remove after all supported graph stores were
            // created by intent-aware runtimes. An outbox without an intent
            // can only come from the former cursor-last crash protocol.
            if self.cursor.pending_intent()?.is_none() {
                self.drain_derived_outbox()?
            } else {
                0
            }
        };
        let cursor = self.cursor.get()?;
        let after_seq = cursor.after_seq;
        let read = self.replay.replay(ReplayRequest {
            cursor,
            limit: max_items,
        });
        let (events, budget_exhausted) = match read {
            Ok(page) => {
                if page.ledger_id != self.ledger_id {
                    return Err(StorageError::IdentityMismatch {
                        expected: self.ledger_id,
                        actual: page.ledger_id,
                    });
                }
                if page.next_cursor.ledger_id != self.ledger_id {
                    return Err(StorageError::IdentityMismatch {
                        expected: self.ledger_id,
                        actual: page.next_cursor.ledger_id,
                    });
                }
                validate_replay_page(after_seq, max_items, &page)?;
                let budget_exhausted = matches!(
                    page.coverage.truncation,
                    CoverageTruncation::After | CoverageTruncation::Both
                );
                (page.records, budget_exhausted)
            }
            // A retention gap means compaction pruned events this cursor has
            // not reduced. Replaying through the gap would corrupt the
            // projection, so the tick reports a fatal diagnostic without
            // moving the cursor; the operator rebuilds from a genesis fact.
            Err(EventAuthorityError::RetentionGap {
                after_seq: gap_cursor,
                retained_from,
                ..
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
            Err(error) => return Err(authority_error_to_storage(error)),
        };
        let events_attempted = events.len();
        let mut traversal_events_applied = 0;
        let mut durable_cursor = cursor;
        // Apply one source record at a time. This makes a replay after a crash
        // observe projection state from exactly that source record, allowing
        // the reducer to reconstruct the same derived envelopes before the
        // cursor advances. Processing a whole page before publication would
        // let later projection mutations alter an earlier envelope's payload.
        for event in events {
            let event_seq = event.seq;
            let event_identity = graph_event_identity(&event)?;
            pause_after_replay_before_admission()?;
            let _mutation_guard = self.mutation_lock.lock();
            let intent = self.cursor.admit(
                durable_cursor.after_seq,
                event_seq,
                &event_identity,
                &self.owner_id,
            )?;
            let reducer = TraversalReducer::replay_records(
                self.traversal.as_ref(),
                self.ledger_id,
                durable_cursor.after_seq,
                std::iter::once(event),
            )?;
            traversal_events_applied += reducer.applied_events;
            fail_after_projection_before_outbox()?;
            self.derived_outbox.replace(&reducer.emitted_envelopes)?;
            derived_events_appended += self.drain_derived_outbox()?;
            self.traversal.flush()?;
            durable_cursor = self.cursor.complete(&intent)?;
        }
        // Cursor registry publication follows the local durable cursor. If
        // reporting fails, the next tick reports the same or a later cursor;
        // replay never regresses to the registry's observational mirror.
        self.cursor_reporter
            .report_graph_cursor(durable_cursor)
            .map_err(authority_error_to_storage)?;
        Ok(GraphCatchUpReport {
            actor_id: GRAPH_ACTOR_ID.to_string(),
            input_event_seq: after_seq,
            output_event_seq: durable_cursor.after_seq,
            events_attempted,
            traversal_events_applied,
            derived_events_appended,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted,
        })
    }

    fn drain_derived_outbox(&self) -> Result<usize, StorageError> {
        let mut pending = self.derived_outbox.pending()?;
        let mut inserted = 0;
        while let Some(envelope) = pending.first().cloned() {
            let receipt = self
                .derived_sink
                .append_derived(envelope)
                .map_err(authority_error_to_storage)?;
            validate_receipt_identity(self.ledger_id, receipt)?;
            if receipt.disposition == AppendDisposition::Inserted {
                inserted += 1;
            }
            pending.remove(0);
            self.derived_outbox.replace(&pending)?;
        }
        Ok(inserted)
    }

    /// Clone the shared traversal store.
    pub fn traversal_store(&self) -> Arc<TraversalStore> {
        Arc::clone(&self.traversal)
    }

    /// Returns the persisted identity of the source event ledger.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.ledger_id
    }

    /// Returns the identity-bearing graph cursor durably persisted locally.
    pub fn durable_event_cursor(&self) -> Result<LedgerCursor, StorageError> {
        self.cursor.get()
    }
}

fn graph_mutation_lock(ledger_id: LedgerIdentity) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<LedgerIdentity, Weak<Mutex<()>>>>> = OnceLock::new();
    let mut locks = LOCKS.get_or_init(|| Mutex::new(HashMap::new())).lock();
    if let Some(lock) = locks.get(&ledger_id).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(ledger_id, Arc::downgrade(&lock));
    lock
}

fn graph_event_identity(event: &EventRecord) -> Result<String, StorageError> {
    let encoded = serde_json::to_vec(event).map_err(|error| {
        StorageError::InvalidPath(format!("invalid graph replay event payload: {error}"))
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"meld.world_state.graph.replay_event.v1\0");
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

fn validate_receipt_identity(
    expected: LedgerIdentity,
    receipt: AppendReceipt,
) -> Result<(), StorageError> {
    if receipt.ledger_id == expected {
        Ok(())
    } else {
        Err(StorageError::IdentityMismatch {
            expected,
            actual: receipt.ledger_id,
        })
    }
}

fn validate_replay_page(
    after_seq: u64,
    max_items: usize,
    page: &EventPage,
) -> Result<(), StorageError> {
    if page.records.len() > max_items {
        return Err(StorageError::InvalidPath(format!(
            "graph replay returned {} records for a {max_items}-record request",
            page.records.len()
        )));
    }
    let mut prior = after_seq;
    for record in &page.records {
        if record.seq <= prior || record.seq > page.coverage.tip_seq {
            return Err(StorageError::InvalidPath(format!(
                "graph replay returned out-of-order or out-of-coverage sequence {}",
                record.seq
            )));
        }
        prior = record.seq;
    }
    let expected_next = page.records.last().map_or(after_seq, |record| record.seq);
    if page.next_cursor.after_seq != expected_next {
        return Err(StorageError::InvalidPath(format!(
            "graph replay next cursor {} does not match returned sequence {expected_next}",
            page.next_cursor.after_seq
        )));
    }
    Ok(())
}

fn authority_error_to_storage(error: EventAuthorityError) -> StorageError {
    match error {
        EventAuthorityError::AppendValidation {
            code,
            field,
            message,
        } => StorageError::InvalidPath(format!(
            "event append validation {code:?} at {field}: {message}"
        )),
        EventAuthorityError::InvalidRequest { message }
        | EventAuthorityError::Internal { message }
        | EventAuthorityError::CorruptPersistedIdentity { message }
        | EventAuthorityError::MigrationConflict { message } => StorageError::InvalidPath(message),
        EventAuthorityError::IdentityMismatch { expected, actual } => {
            StorageError::IdentityMismatch { expected, actual }
        }
        EventAuthorityError::RetentionGap {
            after_seq,
            retained_from,
            ..
        } => StorageError::RetentionGap {
            after_seq,
            retained_from,
        },
        EventAuthorityError::Backpressure { message } => StorageError::Backpressure(message),
        EventAuthorityError::DurabilityIndeterminate { message } => {
            StorageError::DurabilityIndeterminate(message)
        }
        EventAuthorityError::Unavailable { message } => StorageError::Unavailable(message),
        EventAuthorityError::Persistence { message } => {
            StorageError::IoError(std::io::Error::other(message))
        }
        EventAuthorityError::DuplicateAuthorityBinding { ledger_id } => StorageError::IoError(
            std::io::Error::other(format!("duplicate authority binding for {ledger_id}")),
        ),
    }
}

#[cfg(test)]
static FAIL_AFTER_PROJECTION_BEFORE_OUTBOX: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static FAILPOINT_TEST_LOCK: Mutex<()> = Mutex::new(());
#[cfg(test)]
static PAUSE_AFTER_REPLAY_BEFORE_ADMISSION: std::sync::Mutex<
    Option<(
        std::sync::mpsc::SyncSender<()>,
        std::sync::mpsc::Receiver<()>,
    )>,
> = std::sync::Mutex::new(None);

#[cfg(test)]
fn fail_after_projection_before_outbox() -> Result<(), StorageError> {
    if FAIL_AFTER_PROJECTION_BEFORE_OUTBOX.swap(false, Ordering::SeqCst) {
        return Err(StorageError::Unavailable(
            "injected crash after projection flush and before outbox persistence".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
fn pause_after_replay_before_admission() -> Result<(), StorageError> {
    let pause = PAUSE_AFTER_REPLAY_BEFORE_ADMISSION
        .lock()
        .map_err(|_| StorageError::Unavailable("graph replay pause lock poisoned".to_string()))?
        .take();
    if let Some((reached, resume)) = pause {
        reached.send(()).map_err(|_| {
            StorageError::Unavailable("graph replay pause observer disappeared".to_string())
        })?;
        resume.recv().map_err(|_| {
            StorageError::Unavailable("graph replay pause controller disappeared".to_string())
        })?;
    }
    Ok(())
}

#[cfg(not(test))]
fn fail_after_projection_before_outbox() -> Result<(), StorageError> {
    Ok(())
}

#[cfg(not(test))]
fn pause_after_replay_before_admission() -> Result<(), StorageError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::events::{DomainObjectRef, EventEnvelope};
    use crate::world_state::graph::test_support::GraphRuntimeTestFixture; // boundary-allow: event-test

    #[test]
    fn reopen_recovers_crash_between_projection_flush_and_outbox_persistence() {
        let _failpoint_guard = FAILPOINT_TEST_LOCK.lock();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("graph-runtime");
        let db = sled::open(&path).unwrap();
        let fixture = GraphRuntimeTestFixture::open(db.clone()).unwrap();
        let runtime = fixture.runtime();
        let head = DomainObjectRef::new("context", "head", "node-a::analysis").unwrap();
        let node = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        let frame = DomainObjectRef::new("context", "frame", "frame-a").unwrap();
        fixture
            .append(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "context",
                    "stream-a",
                    "context.head_selected",
                    None,
                    json!({ "ok": true }),
                )
                .with_graph(vec![head, node, frame], Vec::new()),
            )
            .unwrap();
        FAIL_AFTER_PROJECTION_BEFORE_OUTBOX.store(true, Ordering::SeqCst);

        assert!(matches!(
            runtime.catch_up(),
            Err(StorageError::Unavailable(message))
                if message.contains("before outbox persistence")
        ));
        assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, 0);
        drop(runtime);
        drop(fixture);
        drop(db);

        let reopened_db = sled::open(&path).unwrap();
        let reopened_fixture = GraphRuntimeTestFixture::open(reopened_db.clone()).unwrap();
        let reopened = reopened_fixture.runtime();
        reopened.catch_up().unwrap();
        let derived: Vec<_> = reopened_fixture
            .records()
            .unwrap()
            .into_iter()
            .filter(|record| record.event_type == "world_state.anchor_selected")
            .collect();

        assert_eq!(derived.len(), 1);
        assert!(reopened.durable_event_cursor().unwrap().after_seq >= 1);
    }

    #[test]
    fn reopen_recovers_one_selected_and_one_superseded_fact_after_projection_crash() {
        let _failpoint_guard = FAILPOINT_TEST_LOCK.lock();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("graph-runtime-supersession");
        let db = sled::open(&path).unwrap();
        let fixture = GraphRuntimeTestFixture::open(db.clone()).unwrap();
        let runtime = fixture.runtime();
        let ledger_id = runtime.ledger_identity();
        let head = DomainObjectRef::new("context", "head", "node-a::analysis").unwrap();
        let node = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        let first_frame = DomainObjectRef::new("context", "frame", "frame-a").unwrap();
        let second_frame = DomainObjectRef::new("context", "frame", "frame-b").unwrap();
        fixture
            .append(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "context",
                    "stream-a",
                    "context.head_selected",
                    None,
                    json!({ "generation": 1 }),
                )
                .with_graph(
                    vec![head.clone(), node.clone(), first_frame.clone()],
                    Vec::new(),
                ),
            )
            .unwrap();
        runtime.catch_up().unwrap();
        let second_source_seq = fixture
            .append(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "context",
                    "stream-a",
                    "context.head_selected",
                    None,
                    json!({ "generation": 2 }),
                )
                .with_graph(vec![head.clone(), node, second_frame.clone()], Vec::new()),
            )
            .unwrap()
            .seq;
        FAIL_AFTER_PROJECTION_BEFORE_OUTBOX.store(true, Ordering::SeqCst);

        assert!(matches!(
            runtime.catch_up(),
            Err(StorageError::Unavailable(message))
                if message.contains("before outbox persistence")
        ));
        assert!(runtime.durable_event_cursor().unwrap().after_seq < second_source_seq);
        drop(runtime);
        drop(fixture);
        drop(db);

        let reopened_db = sled::open(&path).unwrap();
        let reopened_fixture = GraphRuntimeTestFixture::open(reopened_db.clone()).unwrap();
        let reopened = reopened_fixture.runtime();
        reopened.catch_up().unwrap();
        let recovered: Vec<_> = reopened_fixture
            .records()
            .unwrap()
            .into_iter()
            .filter(|record| {
                record.envelope.provenance.source_records
                    == vec![crate::events::EventRecordRef {
                        ledger_id,
                        seq: second_source_seq,
                    }]
            })
            .collect();
        let selected: Vec<_> = recovered
            .iter()
            .filter(|record| record.event_type == "world_state.anchor_selected")
            .collect();
        let superseded: Vec<_> = recovered
            .iter()
            .filter(|record| record.event_type == "world_state.anchor_superseded")
            .collect();

        assert_eq!(selected.len(), 1);
        assert_eq!(superseded.len(), 1);
        assert_eq!(
            selected[0].data["anchor"]["target"]["object_id"],
            second_frame.object_id
        );
        assert_eq!(
            selected[0].data["anchor"]["selected_at_seq"],
            second_source_seq
        );
        assert_eq!(
            superseded[0].data["anchor"]["target"]["object_id"],
            first_frame.object_id
        );
        assert_eq!(
            superseded[0].data["anchor"]["ended_at_seq"],
            second_source_seq
        );
        assert_eq!(
            superseded[0].data["anchor"]["ended_by_anchor_id"],
            format!("anchor::{}::{second_source_seq}", head.index_key())
        );
        assert!(reopened.durable_event_cursor().unwrap().after_seq >= second_source_seq);
    }

    #[test]
    fn stale_runtime_cannot_end_a_newer_anchor_or_append_a_derived_event() {
        let _failpoint_guard = FAILPOINT_TEST_LOCK.lock();
        let db = sled::Config::new().temporary(true).open().unwrap();
        let fixture = GraphRuntimeTestFixture::open(db).unwrap();
        let winner = fixture.runtime();
        let head = DomainObjectRef::new("context", "head", "node-a::analysis").unwrap();
        let node = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        let first_frame = DomainObjectRef::new("context", "frame", "frame-a").unwrap();
        let newer_frame = DomainObjectRef::new("context", "frame", "frame-b").unwrap();
        fixture
            .append(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "context",
                    "stream-a",
                    "context.head_selected",
                    None,
                    json!({ "generation": 1 }),
                )
                .with_graph(vec![head.clone(), node.clone(), first_frame], Vec::new()),
            )
            .unwrap();
        winner.catch_up().unwrap();
        let stale = fixture.additional_runtime().unwrap();
        fixture
            .append(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "context",
                    "stream-a",
                    "context.head_tombstoned",
                    None,
                    json!({ "reason": "removed" }),
                )
                .with_graph(vec![head.clone()], Vec::new()),
            )
            .unwrap();

        let (reached_tx, reached_rx) = std::sync::mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);
        *PAUSE_AFTER_REPLAY_BEFORE_ADMISSION.lock().unwrap() = Some((reached_tx, resume_rx));
        let stale_thread = std::thread::spawn(move || stale.catch_up());
        reached_rx.recv().unwrap();

        let newer_source_seq = fixture
            .append(
                EventEnvelope::with_now_domain(
                    "session-a",
                    "context",
                    "stream-a",
                    "context.head_selected",
                    None,
                    json!({ "generation": 2 }),
                )
                .with_graph(vec![head.clone(), node, newer_frame.clone()], Vec::new()),
            )
            .unwrap()
            .seq;
        winner.catch_up().unwrap();
        let records_after_winner = fixture.records().unwrap().len();
        resume_tx.send(()).unwrap();
        assert!(matches!(
            stale_thread.join().unwrap(),
            Err(StorageError::Backpressure(message))
                if message.contains("before replay admission")
        ));

        let current = winner
            .traversal_store()
            .current_anchor(&head)
            .unwrap()
            .expect("newer selection remains current");
        assert_eq!(current.target, newer_frame);
        assert_eq!(current.selected_at_seq, newer_source_seq);
        assert_eq!(current.ended_at_seq, None);
        assert_eq!(fixture.records().unwrap().len(), records_after_winner);
    }
}
