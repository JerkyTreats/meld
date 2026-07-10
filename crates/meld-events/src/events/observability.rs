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

/// Largest page a caller may request from the streaming surface.
pub const MAX_EVENT_PAGE_LIMIT: usize = 1_024;
/// Largest page wait accepted by the streaming surface, in milliseconds.
pub const MAX_EVENT_PAGE_TIMEOUT_MS: u64 = 30_000;
/// Largest trailing event window accepted by the flow surface.
pub const MAX_FLOW_WINDOW_EVENTS: usize = 100_000;
/// Number of newest retained records a trace may inspect.
pub const MAX_TRACE_SCAN_EVENTS: usize = 100_000;
/// Number of newest retained records a session report may inspect.
pub const MAX_SESSION_SCAN_EVENTS: usize = 100_000;

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

/// Which side of a bounded read omitted ledger history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageTruncation {
    /// The report covers all retained records through its frozen tip.
    None,
    /// Records before the scanned range were omitted.
    Before,
    /// Records after the scanned range but no later than the frozen tip were omitted.
    After,
    /// Records on both sides of the scanned range were omitted.
    Both,
}

/// Exact durable range considered by one bounded ledger read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventReadCoverage {
    /// First sequence the ledger declares retained.
    pub retained_from: u64,
    /// Durable ledger tip frozen before the scan began.
    pub tip_seq: u64,
    /// First record included in the computed window, absent when it is empty.
    pub scanned_from_seq: Option<u64>,
    /// Last record included in the computed window, absent when it is empty.
    pub scanned_through_seq: Option<u64>,
    /// Whether the bounded scan omitted records before or after its range.
    pub truncation: CoverageTruncation,
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
    /// Durable range inspected while computing the chain.
    pub coverage: EventReadCoverage,
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
    /// Recorded time of the first returned event, absent for empty sessions.
    pub observed_started_at: Option<String>,
    /// Recorded time of the last returned event, absent for empty sessions.
    pub observed_ended_at: Option<String>,
    /// Number of session events returned from the bounded scan.
    pub events_returned: u64,
    /// Durable ledger range inspected for this session.
    pub coverage: EventReadCoverage,
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

    pub(crate) fn store(&self) -> &EventStore {
        &self.store
    }

    pub(crate) fn watermark(&self) -> &CommitWatermark {
        &self.watermark
    }

    pub(crate) fn consumer_snapshot(&self) -> Result<Vec<ConsumerCursor>, StorageError> {
        self.registry.snapshot()
    }

    pub(crate) fn dropped_events(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl EventObservabilityPort for LedgerObservability {
    fn health(&self) -> Result<EventHealthReport, StorageError> {
        health::compute(self)
    }

    fn flow(&self, window: FlowWindow) -> Result<EventFlowReport, StorageError> {
        validate_nonzero_bound(
            "event flow window",
            window.max_events,
            MAX_FLOW_WINDOW_EVENTS,
        )?;
        session::compute_flow(self, window)
    }

    fn trace(&self, subject: TraceSubject) -> Result<EventTraceReport, StorageError> {
        trace::compute(self, subject)
    }

    fn session(&self, session_id: &str) -> Result<SessionTimelineReport, StorageError> {
        session::compute_timeline(self, session_id)
    }

    fn next_page(&self, request: EventPageRequest) -> Result<EventPage, StorageError> {
        validate_nonzero_bound("event page limit", request.limit, MAX_EVENT_PAGE_LIMIT)?;
        validate_upper_bound(
            "event page timeout_ms",
            request.timeout_ms,
            MAX_EVENT_PAGE_TIMEOUT_MS,
        )?;
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

fn validate_nonzero_bound(name: &str, value: usize, maximum: usize) -> Result<(), StorageError> {
    if !(1..=maximum).contains(&value) {
        return Err(invalid_request(format!(
            "{name} must be in 1..={maximum}, got {value}"
        )));
    }
    Ok(())
}

fn validate_upper_bound(name: &str, value: u64, maximum: u64) -> Result<(), StorageError> {
    if value > maximum {
        return Err(invalid_request(format!(
            "{name} must be in 0..={maximum}, got {value}"
        )));
    }
    Ok(())
}

fn invalid_request(message: String) -> StorageError {
    // TODO compat-shim: E2 replaces this legacy StorageError::InvalidPath
    // mapping with EventAuthorityError::InvalidRequest. Remove it after the
    // observability_contracts invalid-limit parity cases pass through the
    // authority contract.
    StorageError::InvalidPath(message)
}
