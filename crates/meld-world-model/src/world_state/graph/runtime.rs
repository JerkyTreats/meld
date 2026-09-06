//! Runtime wrapper for graph event reduction.
//!
//! `GraphRuntime` consumes event-authority ports and owns traversal projection
//! coordination. Reads call `catch_up` before querying so graph indexes observe
//! all durable events exposed by the supplied replay capability.
//!
//! Product composition supplies all three authority-derived ports to
//! [`GraphRuntime::from_ports`].

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::StorageError;
use meld_events::error::EventAuthorityError;

use crate::events::observability::CoverageTruncation;
use crate::events::{
    AppendDisposition, AppendReceipt, EventPage, LedgerCursor, LedgerIdentity, ReplayRequest,
    MAX_REPLAY_LIMIT,
};
use crate::world_state::graph::cursor::GraphProjectionCursor;
use crate::world_state::graph::outbox::GraphDerivedOutbox;
use crate::world_state::graph::ports::{
    GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource,
};
use crate::world_state::graph::reducer::TraversalReducer;
use crate::world_state::graph::store::TraversalStore;

pub(super) const GRAPH_ACTOR_ID: &str = "world_state.graph.reducer";

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
    /// Independent owner-history checkpoint when this tick bootstraps a late route.
    pub source_replay: Option<OwnerEventReplayTick>,
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
    /// Owner-authored conditions that can make Graph replay eligible again.
    pub waiting_on: Vec<crate::waiting::WaitingOnDeclaration>,
}

/// Native progress in a historical owner-source scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerEventReplayTick {
    pub source: super::admission::OwnerEventSourceRef,
    pub before: LedgerCursor,
    pub after: LedgerCursor,
    pub target: LedgerCursor,
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
    lifecycle: crate::lifecycle::NativeLifecycle,
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
        Ok(Self {
            replay,
            derived_sink,
            cursor_reporter,
            ledger_id,
            traversal,
            cursor,
            derived_outbox,
            catch_up_lock: Mutex::new(()),
            lifecycle: crate::lifecycle::NativeLifecycle::new("world_state.graph_replay"),
        })
    }

    /// Resolve only positions in the Event authority this Graph actually replays.
    pub fn resolves_wake(
        &self,
        wake: &crate::waiting::StructuralWakeAddress,
    ) -> Result<bool, String> {
        Ok(match wake {
            crate::waiting::StructuralWakeAddress::EventPosition(value) => {
                crate::waiting::after_position(value, &format!("event-ledger::{}", self.ledger_id))
            }
            crate::waiting::StructuralWakeAddress::OwnerRevision(value) => {
                crate::waiting::bound_address(value, self.traversal.resource_id()).is_some_and(
                    |value| value == format!("graph-projection::{}::successor", self.ledger_id),
                )
            }
            _ => false,
        })
    }

    /// Read the Graph-owned cursor, subscription, and durable publication account.
    pub fn lifecycle_evidence(&self) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let _guard = self.catch_up_lock.lock();
        self.lifecycle_evidence_inner()
    }

    fn lifecycle_evidence_inner(
        &self,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let cursor = self
            .durable_event_cursor()
            .map_err(|error| error.to_string())?;
        let pending = self
            .derived_outbox
            .pending()
            .map_err(|error| error.to_string())?;
        let source_replays = self
            .traversal
            .owner_event_replay_states(cursor.ledger_id)
            .map_err(|error| error.to_string())?;
        self.traversal.flush().map_err(|error| error.to_string())?;
        let checkpoint_ref = format!(
            "graph-projection::{}::{}",
            cursor.ledger_id, cursor.after_seq
        );
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: {
                let mut revisions = vec!["graph-projection-schema::v1".into()];
                for route in self
                    .traversal
                    .owner_event_routes()
                    .map_err(|error| error.to_string())?
                {
                    revisions.push(crate::lifecycle::evidence_ref(
                        "graph-owner-event-route",
                        &route.revision_ref().map_err(|error| error.to_string())?,
                    )?);
                }
                revisions
            },
            binding_refs: vec![
                format!("graph-traversal::{}", cursor.ledger_id),
                crate::lifecycle::evidence_ref("graph-owner-source-progress", &source_replays)?,
            ],
            subscription_refs: vec![format!("event-ledger::{}", cursor.ledger_id)],
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: crate::lifecycle::evidence_ref(
                "graph-native-work",
                &(
                    &pending,
                    source_replays
                        .iter()
                        .filter(|state| !state.covered)
                        .collect::<Vec<_>>(),
                ),
            )?,
        })
    }

    /// Author native start evidence under the Graph work lock.
    pub fn lifecycle_start(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.catch_up_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .start(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native safe point evidence under the Graph work lock.
    pub fn lifecycle_safe_point(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.catch_up_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .safe_point(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native stop evidence under the Graph work lock.
    pub fn lifecycle_stop(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.catch_up_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .stop(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native release evidence under the Graph work lock.
    pub fn lifecycle_release(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.catch_up_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .release(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
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
        let mut combined = self.catch_up_page(max_items, true)?;
        while combined.source_replay.is_none()
            && combined.events_attempted < max_items
            && combined.budget_exhausted
            && combined.retryable_errors.is_empty()
            && combined.fatal_errors.is_empty()
        {
            // Newly derived records consume the remainder of this same work budget.
            // Historical source scans retain their independent reporting boundary.
            let next = self.catch_up_page(max_items - combined.events_attempted, false)?;
            combined.output_event_seq = next.output_event_seq;
            combined.events_attempted += next.events_attempted;
            combined.traversal_events_applied += next.traversal_events_applied;
            combined.derived_events_appended += next.derived_events_appended;
            combined.retryable_errors.extend(next.retryable_errors);
            combined.fatal_errors.extend(next.fatal_errors);
            combined.budget_exhausted = next.budget_exhausted;
            combined.waiting_on = next.waiting_on;
            if next.events_attempted == 0 {
                break;
            }
        }
        Ok(combined)
    }

    fn catch_up_page(
        &self,
        max_items: usize,
        allow_source_replay: bool,
    ) -> Result<GraphCatchUpReport, StorageError> {
        let (mut derived_events_appended, mut derived_through) = self.drain_derived_outbox()?;
        let cursor = self.cursor.get()?;
        let after_seq = cursor.after_seq;
        if allow_source_replay && after_seq > 0 {
            if let Some(mut report) = super::source_replay::replay_pending_source(
                self.traversal.as_ref(),
                self.replay.as_ref(),
                self.cursor_reporter.as_ref(),
                cursor,
                max_items,
            )? {
                report.derived_events_appended += derived_events_appended;
                return Ok(report);
            }
        }
        // Snapshot before replay so a route installed mid-page cannot claim prior coverage.
        let genesis_sources = if after_seq == 0 {
            self.traversal
                .owner_event_routes()?
                .into_iter()
                .filter(|route| route.complete_event_source)
                .map(|route| route.source_ref())
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        let read = self
            .replay
            .replay(ReplayRequest {
                cursor,
                limit: max_items,
            })
            .and_then(|page| {
                if page.coverage.retained_from > after_seq.saturating_add(1) {
                    Err(EventAuthorityError::RetentionGap {
                        ledger_id: self.ledger_id,
                        after_seq,
                        retained_from: page.coverage.retained_from,
                    })
                } else {
                    Ok(page)
                }
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
                if after_seq == 0 {
                    self.traversal
                        .record_genesis_event_sources(self.ledger_id, &genesis_sources)?;
                }
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
                    source_replay: None,
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
                    waiting_on: Vec::new(),
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
            let reducer = TraversalReducer::replay_records(
                self.traversal.as_ref(),
                self.ledger_id,
                durable_cursor.after_seq,
                std::iter::once(event),
            )?;
            traversal_events_applied += reducer.applied_events;
            fail_after_projection_before_outbox()?;
            self.derived_outbox.replace(&reducer.emitted_envelopes)?;
            let (appended, through) = self.drain_derived_outbox()?;
            derived_events_appended += appended;
            derived_through = derived_through.max(through);
            self.traversal.flush()?;
            durable_cursor = self.cursor.advance(event_seq)?;
        }
        // Cursor registry publication follows the local durable cursor. If
        // reporting fails, the next tick reports the same or a later cursor;
        // replay never regresses to the registry's observational mirror.
        self.cursor_reporter
            .report_graph_cursor(durable_cursor)
            .map_err(authority_error_to_storage)?;
        for state in self
            .traversal
            .owner_event_replay_states(self.ledger_id)?
            .into_iter()
            .filter(|state| state.covered)
        {
            self.cursor_reporter
                .report_owner_source_cursor(&state.source, durable_cursor)
                .map_err(authority_error_to_storage)?;
        }
        let budget_exhausted = budget_exhausted || derived_through > durable_cursor.after_seq;
        let waiting_on = if events_attempted == 0 && !budget_exhausted {
            vec![crate::waiting::WaitingOnDeclaration::broad(
                "ledger_quiet_past_graph_cursor",
                format!(
                    "no committed events past Graph cursor {}",
                    durable_cursor.after_seq
                ),
                vec![crate::waiting::StructuralWakeAddress::EventPosition(
                    format!(
                        "event-ledger::{}::after::{}",
                        self.ledger_id, durable_cursor.after_seq
                    ),
                )],
            )]
        } else {
            Vec::new()
        };
        Ok(GraphCatchUpReport {
            source_replay: None,
            actor_id: GRAPH_ACTOR_ID.to_string(),
            input_event_seq: after_seq,
            output_event_seq: durable_cursor.after_seq,
            events_attempted,
            traversal_events_applied,
            derived_events_appended,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted,
            waiting_on,
        })
    }

    fn drain_derived_outbox(&self) -> Result<(usize, u64), StorageError> {
        let mut pending = self.derived_outbox.pending()?;
        let mut inserted = 0;
        let mut through = 0;
        while let Some(envelope) = pending.first().cloned() {
            let receipt = self
                .derived_sink
                .append_derived(envelope)
                .map_err(authority_error_to_storage)?;
            validate_receipt_identity(self.ledger_id, receipt)?;
            through = through.max(receipt.seq);
            if receipt.disposition == AppendDisposition::Inserted {
                inserted += 1;
            }
            pending.remove(0);
            self.derived_outbox.replace(&pending)?;
        }
        Ok((inserted, through))
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

pub(crate) fn validate_replay_page(
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
fn fail_after_projection_before_outbox() -> Result<(), StorageError> {
    if FAIL_AFTER_PROJECTION_BEFORE_OUTBOX.swap(false, Ordering::SeqCst) {
        return Err(StorageError::Unavailable(
            "injected crash after projection flush and before outbox persistence".to_string(),
        ));
    }
    Ok(())
}

#[cfg(not(test))]
fn fail_after_projection_before_outbox() -> Result<(), StorageError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::events::{DomainObjectRef, EventEnvelope};
    use crate::world_state::graph::test_support::GraphRuntimeTestFixture; // boundary-allow: event-test

    #[test]
    fn lifecycle_evidence_accounts_for_the_actual_durable_outbox() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let fixture = GraphRuntimeTestFixture::open(db).unwrap();
        let runtime = fixture.runtime();
        let valid = crate::waiting::StructuralWakeAddress::EventPosition(format!(
            "event-ledger::{}::after::0",
            runtime.ledger_id
        ));
        assert!(runtime.resolves_wake(&valid).unwrap());
        for invalid in [
            format!("event-ledger::{}::after::0", LedgerIdentity::new()),
            format!("event-ledger::{}::after::not-a-position", runtime.ledger_id),
            format!("event-ledger::{}::after::0::extra", runtime.ledger_id),
        ] {
            assert!(!runtime
                .resolves_wake(&crate::waiting::StructuralWakeAddress::EventPosition(
                    invalid
                ))
                .unwrap());
        }
        let identity =
            crate::lifecycle::NativeLifecycleIdentity::new("generation-a".into(), "graph-a".into())
                .unwrap();
        let (ready, transition) = runtime.lifecycle_start(identity.clone()).unwrap();
        assert_eq!(transition.checkpoint_ref, ready.proof_position_ref);
        let envelope = EventEnvelope::with_now_domain(
            "session",
            "graph",
            "pending",
            "test.pending",
            None,
            json!({}),
        );
        runtime.derived_outbox.replace(&[envelope]).unwrap();
        let (safe, transition) = runtime.lifecycle_safe_point(identity).unwrap();
        assert_eq!(safe.checkpoint_ref, ready.checkpoint_ref);
        assert_eq!(transition.checkpoint_ref, safe.proof_position_ref);
        assert_ne!(
            safe.unresolved_operation_summary_ref,
            ready.unresolved_operation_summary_ref
        );
        assert_eq!(runtime.derived_outbox.pending().unwrap().len(), 1);
    }

    #[test]
    fn derived_event_replay_uses_remaining_budget_and_reports_exhaustion_exactly() {
        for budget in [1, 16] {
            let temp = tempfile::tempdir().unwrap();
            let fixture = GraphRuntimeTestFixture::open(sled::open(temp.path()).unwrap()).unwrap();
            let runtime = fixture.runtime();
            fixture
                .append(
                    EventEnvelope::with_now_domain(
                        "session-a",
                        "context",
                        "stream-a",
                        "context.head_selected",
                        None,
                        json!({"ok":true}),
                    )
                    .with_graph(
                        vec![
                            DomainObjectRef::new("context", "head", "node-a::analysis").unwrap(),
                            DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
                            DomainObjectRef::new("context", "frame", "frame-a").unwrap(),
                        ],
                        Vec::new(),
                    ),
                )
                .unwrap();
            let first = runtime
                .catch_up_bounded(GraphCatchUpBudget { max_items: budget })
                .unwrap();
            assert!(first.events_attempted <= budget);
            assert_eq!(first.derived_events_appended, 1);
            let records = fixture.records().unwrap();
            assert_eq!(records.len(), 2);
            if budget == 1 {
                assert_eq!(first.events_attempted, 1);
                assert_eq!(first.output_event_seq, records[0].seq);
                assert!(first.budget_exhausted);
                let second = runtime
                    .catch_up_bounded(GraphCatchUpBudget { max_items: 1 })
                    .unwrap();
                assert_eq!(second.events_attempted, 1);
                assert_eq!(second.derived_events_appended, 0);
                assert_eq!(second.output_event_seq, records[1].seq);
                assert!(!second.budget_exhausted);
            } else {
                assert_eq!(first.events_attempted, 2);
                assert_eq!(first.output_event_seq, records[1].seq);
                assert!(!first.budget_exhausted);
            }
            assert_eq!(
                runtime.durable_event_cursor().unwrap().after_seq,
                records[1].seq
            );
            assert_eq!(fixture.records().unwrap().len(), 2);
        }
    }

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
}
