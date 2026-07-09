//! Observability port and report contracts over the event ledger.
//!
//! Owner: event observability.
//! Inputs: the ledger store, the writer's commit watermark, and the consumer
//! cursor registry.
//! Outputs: serializable, presentation-free reports served through one port
//! so a CLI, a TUI, and an HTTP dashboard are equally thin adapters, plus one
//! cursor-paged stream shape every follow loop shares.
//! Does not own: event payload meaning. Reports summarize structure the
//! events domain owns: sequences, identity, references, and provenance;
//! semantic rollups belong downstream.
//!
//! Report structs are wire contracts: their serialized field names are pinned
//! by contract tests, and shape changes are breaking changes for adapters.

/// Health surface computation.
mod health;
/// Session timeline and flow computation.
mod session;
/// Causal trace computation.
mod trace;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::events::registry::{ConsumerCursor, EventCursorRegistry};
use crate::events::store::EventStore;
use crate::events::subscription::EventSubscription;
use crate::events::writer::CommitWatermark;
use crate::events::{DomainObjectRef, EventRecord};

/// One port for every observability read.
///
/// Point-in-time queries answer health, flow, causality, and session
/// questions; `next_page` is the universal stream primitive a tail command,
/// a TUI render loop, and a server-sent-events adapter all share.
pub trait EventObservabilityPort {
    /// Ledger health: tip, watermark, retention, drops, consumer lag, rates.
    fn health(&self) -> Result<EventHealthReport, StorageError>;

    /// Event flow over a trailing window, including silent domains.
    fn flow(&self, window: FlowWindow) -> Result<EventFlowReport, StorageError>;

    /// Causal chain for one object, stream, or record.
    fn trace(&self, subject: TraceSubject) -> Result<EventTraceReport, StorageError>;

    /// One session's records in order with step timing.
    fn session(&self, session_id: &str) -> Result<SessionTimelineReport, StorageError>;

    /// Bounded page after a cursor, blocking until events commit or the
    /// timeout elapses; an empty page means the timeout expired. Zero
    /// limits are rejected rather than blocked on.
    fn next_page(&self, request: EventPageRequest) -> Result<EventPage, StorageError>;
}

/// Trailing window selector for flow reports.
///
/// Windows are event-count bounded because sequence order is the ledger's
/// native clock; reports carry the observed wall-clock span alongside.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowWindow {
    /// Maximum trailing events the window covers.
    pub max_events: usize,
}

/// Subject selector for causal traces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceSubject {
    /// Every event referencing one domain object.
    Object(DomainObjectRef),
    /// Every event in one domain-local stream.
    Stream {
        /// Domain owning the stream.
        domain_id: String,
        /// Domain-local stream identifier.
        stream_id: String,
    },
    /// The causal neighborhood of one ledger record.
    Record {
        /// Ledger sequence of the record.
        seq: u64,
    },
}

/// Cursor-paged stream request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventPageRequest {
    /// Cursor: only events with higher sequences are returned.
    pub after_seq: u64,
    /// Maximum records in the page.
    pub limit: usize,
    /// How long to block waiting for new commits.
    pub timeout_ms: u64,
}

/// One bounded page of ledger records plus the next cursor.
///
/// Records travel intact as canonical products; the page adds only the
/// cursor operational metadata around them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPage {
    /// Records in sequence order.
    pub records: Vec<EventRecord>,
    /// Cursor for the next request; unchanged when the page is empty.
    pub next_after_seq: u64,
}

/// Ledger health snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventHealthReport {
    /// Highest persisted ledger sequence.
    pub tip_seq: u64,
    /// Highest writer-committed sequence; direct consumer-side appends may
    /// place the tip above it.
    pub committed_watermark: u64,
    /// First retained sequence; one means full history.
    pub retained_from: u64,
    /// Best-effort events dropped by backpressure since process start.
    pub dropped_events: u64,
    /// Per-consumer positions and lag against the watermark.
    pub consumers: Vec<ConsumerLagReport>,
    /// Trailing append rate per domain.
    pub append_rates: Vec<DomainAppendRate>,
}

/// One consumer's lag against the commit watermark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerLagReport {
    /// Stable consumer name from the registry.
    pub name: String,
    /// Highest sequence the consumer reported as durably reduced.
    pub reported_seq: u64,
    /// Watermark minus reported sequence, zero when caught up.
    pub lag: u64,
}

/// Trailing append rate for one domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainAppendRate {
    /// Domain that owns the events.
    pub domain_id: String,
    /// Events observed in the trailing window.
    pub events_in_window: u64,
    /// Observed wall-clock span of the window in seconds, absent when
    /// timestamps were unparseable.
    pub window_seconds: Option<u64>,
}

/// Event flow over a trailing window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventFlowReport {
    /// Events actually covered by the window.
    pub window_events: u64,
    /// Observed wall-clock span of the window in seconds, absent when
    /// timestamps were unparseable.
    pub span_seconds: Option<u64>,
    /// Counts by owning domain, descending by count.
    pub by_domain: Vec<DomainFlow>,
    /// Counts by event type, descending by count.
    pub by_type: Vec<TypeFlow>,
    /// Domains with history that emitted nothing inside the window.
    pub silent_domains: Vec<SilentDomain>,
}

/// Window counts for one domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainFlow {
    /// Domain that owns the events.
    pub domain_id: String,
    /// Events inside the window.
    pub count: u64,
    /// Highest sequence the domain reached inside the window.
    pub last_seq: u64,
}

/// Window counts for one event type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeFlow {
    /// Stable event type identifier.
    pub event_type: String,
    /// Events inside the window.
    pub count: u64,
}

/// A domain that has gone quiet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SilentDomain {
    /// Domain that owns the events.
    pub domain_id: String,
    /// The domain's last sequence anywhere in history.
    pub last_seq: u64,
    /// Recorded time of that last event.
    pub last_recorded_at: String,
}

/// Causal chain for one trace subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTraceReport {
    /// Subject the trace was computed for.
    pub subject: TraceSubject,
    /// Chain hops in sequence order.
    pub hops: Vec<TraceHop>,
}

/// One event in a causal chain and the reference that linked it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceHop {
    /// Ledger sequence of the event.
    pub seq: u64,
    /// Storage timestamp of the event.
    pub recorded_at: String,
    /// Domain that owns the event.
    pub domain_id: String,
    /// Domain-local stream of the event.
    pub stream_id: String,
    /// Stable event type identifier.
    pub event_type: String,
    /// Session partition of the event.
    pub session_id: String,
    /// Why this event belongs to the chain.
    pub link: TraceLink,
}

/// The reference connecting a hop to the trace subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceLink {
    /// The hop is the subject record itself.
    Subject,
    /// The hop carries the subject object in its object references.
    ObjectRef,
    /// The hop relates the subject through a typed relation edge.
    Relation {
        /// Domain-specific relationship name.
        relation_type: String,
    },
    /// The hop's stored provenance references the subject record.
    SourceFact {
        /// Stored fact identifier carrying the reference.
        fact_id: String,
    },
    /// The hop shares the subject stream.
    Stream,
}

/// One session's ledger records in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTimelineReport {
    /// Session partition the timeline covers.
    pub session_id: String,
    /// Recorded time of the first event, absent for empty sessions.
    pub started_at: Option<String>,
    /// Recorded time of the last event, absent for empty sessions.
    pub ended_at: Option<String>,
    /// Total events in the session.
    pub total_events: u64,
    /// Ordered steps with inter-step timing.
    pub steps: Vec<SessionStep>,
}

/// One event inside a session timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStep {
    /// Ledger sequence of the event.
    pub seq: u64,
    /// Storage timestamp of the event.
    pub recorded_at: String,
    /// Domain that owns the event.
    pub domain_id: String,
    /// Stable event type identifier.
    pub event_type: String,
    /// Milliseconds since the previous step, absent for the first step or
    /// unparseable timestamps.
    pub gap_ms: Option<u64>,
}

/// In-process backing for the observability port.
///
/// Serves one-shot CLI commands and any surface hosted inside the process
/// that owns the ledger database. Out-of-process backings implement the same
/// port over the daemon edge later; adapters never learn which they got.
pub struct LedgerObservability {
    store: Arc<EventStore>,
    watermark: Arc<CommitWatermark>,
    registry: EventCursorRegistry,
    subscription: EventSubscription,
    dropped: Arc<AtomicU64>,
}

impl LedgerObservability {
    /// Binds the backing to an opened store, its watermark, the registry,
    /// and the writer's shared drop counter.
    pub fn new(
        store: Arc<EventStore>,
        watermark: Arc<CommitWatermark>,
        registry: EventCursorRegistry,
        dropped: Arc<AtomicU64>,
    ) -> Self {
        let subscription = EventSubscription::new(Arc::clone(&store), Arc::clone(&watermark));
        Self {
            store,
            watermark,
            registry,
            subscription,
            dropped,
        }
    }

    // Consumed by the surface unit modules as they land in the fan-out
    // phase; the allowances retire with the last stub.
    #[allow(dead_code)]
    pub(crate) fn store(&self) -> &EventStore {
        &self.store
    }

    #[allow(dead_code)]
    pub(crate) fn watermark(&self) -> &CommitWatermark {
        &self.watermark
    }

    #[allow(dead_code)]
    pub(crate) fn consumer_snapshot(&self) -> Result<Vec<ConsumerCursor>, StorageError> {
        self.registry.snapshot()
    }

    #[allow(dead_code)]
    pub(crate) fn dropped_events(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl EventObservabilityPort for LedgerObservability {
    fn health(&self) -> Result<EventHealthReport, StorageError> {
        health::compute(self)
    }

    fn flow(&self, window: FlowWindow) -> Result<EventFlowReport, StorageError> {
        session::compute_flow(self, window)
    }

    fn trace(&self, subject: TraceSubject) -> Result<EventTraceReport, StorageError> {
        trace::compute(self, subject)
    }

    fn session(&self, session_id: &str) -> Result<SessionTimelineReport, StorageError> {
        session::compute_timeline(self, session_id)
    }

    fn next_page(&self, request: EventPageRequest) -> Result<EventPage, StorageError> {
        if request.limit == 0 {
            return Err(StorageError::InvalidPath(
                "event page limit must be greater than zero".to_string(),
            ));
        }
        let records = self.subscription.next_batch(
            request.after_seq,
            request.limit,
            Duration::from_millis(request.timeout_ms),
        )?;
        let next_after_seq = records
            .last()
            .map(|record| record.seq)
            .unwrap_or(request.after_seq);
        Ok(EventPage {
            records,
            next_after_seq,
        })
    }
}

pub(crate) fn surface_not_implemented(surface: &str) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("event observability surface {surface} is not implemented yet"),
    ))
}
