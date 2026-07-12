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

use crate::error::{EventAuthorityError, StorageError};
use crate::events::authority::{EventPage, LedgerCursor};
use crate::events::registry::{ConsumerCursor, EventCursorRegistry};
use crate::events::store::EventStore;
use crate::events::writer::CommitWatermark;
use crate::events::{DomainObjectRef, EventRecord, EventRecordRef, LedgerIdentity};

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

    /// Newest bounded records selected by record count rather than sequence
    /// distance. This compatibility surface keeps sparse imported ledgers
    /// honest until direct CLI reads move to the product authority in E5.
    /// TODO compat-shim: E5 removes this method after
    /// `product_event_authority_cutover` proves direct CLI tailing through the
    /// authority replay capability on sparse history.
    fn newest_page(&self, limit: usize) -> Result<EventPage, StorageError>;
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
// TODO compat-shim: E5 replaces this identity-less request after
// product_event_authority_cutover proves direct CLI paging passes a
// LedgerCursor through the resolved authority.
pub struct EventPageRequest {
    /// Compatibility cursor: only events with higher sequences are returned.
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
// TODO compat-shim: E5 removes this identity-less wire type after
// product_event_authority_cutover proves every direct CLI page uses
// authority::EventPage. Canonical observability no longer constructs it.
pub struct LegacyEventPage {
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
    /// Ledger summarized by this report.
    pub ledger_id: LedgerIdentity,
    /// Highest persisted ledger sequence.
    pub tip_seq: u64,
    /// Highest writer-committed sequence inside this report's frozen tip;
    /// direct consumer-side appends may place the tip above it.
    pub committed_watermark: u64,
    /// First retained sequence; one means full history.
    pub retained_from: u64,
    /// Best-effort events dropped by backpressure since process start.
    pub dropped_events: u64,
    /// Per-consumer positions and lag against the watermark.
    pub consumers: Vec<ConsumerLagReport>,
    /// Trailing append rate per domain.
    pub append_rates: Vec<DomainAppendRate>,
    /// Durable range inspected for append-rate calculation.
    pub append_rate_coverage: EventReadCoverage,
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
    /// Ledger summarized by this report.
    pub ledger_id: LedgerIdentity,
    /// Durable trailing range inspected for flow counts.
    pub coverage: EventReadCoverage,
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
    /// Durable range inspected to identify silent domains.
    pub silent_domain_coverage: EventReadCoverage,
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
    /// The domain's last sequence within `silent_domain_coverage`.
    pub last_seq: u64,
    /// Recorded time of that last event.
    pub last_recorded_at: String,
}

/// Causal chain for one trace subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTraceReport {
    /// Ledger summarized by this report.
    pub ledger_id: LedgerIdentity,
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
    SourceRecord {
        /// Identity-bearing source record carrying the provenance edge.
        record: EventRecordRef,
    },
    /// The hop shares the subject stream.
    Stream,
}

/// One session's ledger records in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTimelineReport {
    /// Ledger summarized by this report.
    pub ledger_id: LedgerIdentity,
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
    ledger_id: LedgerIdentity,
    store: Arc<EventStore>,
    watermark: Arc<CommitWatermark>,
    registry: EventCursorRegistry,
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
        // TODO compat-shim: E5 removes this infallible raw construction after
        // product_event_authority_cutover and observability_contracts cover
        // typed identity corruption through `try_new` and EventAuthority.
        Self::try_new(store, watermark, registry, dropped)
            .expect("legacy observability backing requires a valid persisted ledger identity")
    }

    /// Compatibility constructor that establishes or validates the ledger's
    /// persisted identity without inventing an identity per report.
    pub fn try_new(
        store: Arc<EventStore>,
        watermark: Arc<CommitWatermark>,
        registry: EventCursorRegistry,
        dropped: Arc<AtomicU64>,
    ) -> Result<Self, StorageError> {
        let ledger_id = stable_ledger_identity(&store)?;
        Ok(Self {
            ledger_id,
            store,
            watermark,
            registry,
            dropped,
        })
    }

    pub(crate) fn from_authority(
        ledger_id: LedgerIdentity,
        store: Arc<EventStore>,
        watermark: Arc<CommitWatermark>,
        registry: EventCursorRegistry,
        dropped: Arc<AtomicU64>,
    ) -> Self {
        Self {
            ledger_id,
            store,
            watermark,
            registry,
            dropped,
        }
    }

    pub(crate) fn ledger_identity(&self) -> LedgerIdentity {
        self.ledger_id
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
        let ledger_id = self.ledger_identity();
        let cursor = LedgerCursor {
            ledger_id,
            after_seq: request.after_seq,
        };
        let mut page = read_page(self.store(), ledger_id, cursor, request.limit)?;
        if page.records.is_empty() && request.timeout_ms > 0 {
            self.watermark
                .wait_past(request.after_seq, Duration::from_millis(request.timeout_ms));
            page = read_page(self.store(), ledger_id, cursor, request.limit)?;
        }
        Ok(page)
    }

    fn newest_page(&self, limit: usize) -> Result<EventPage, StorageError> {
        validate_nonzero_bound("event page limit", limit, MAX_EVENT_PAGE_LIMIT)?;
        read_newest_page(self.store(), self.ledger_identity(), limit)
    }
}

impl crate::events::authority::EventObservabilityCapability {
    /// Computes health after validating the requested ledger identity.
    pub fn health(
        &self,
        ledger_id: LedgerIdentity,
    ) -> Result<EventHealthReport, EventAuthorityError> {
        self.validate_observability_identity(ledger_id)?;
        health::compute(&self.as_ledger_observability()).map_err(Into::into)
    }

    /// Computes one bounded flow report after validating identity.
    pub fn flow(
        &self,
        ledger_id: LedgerIdentity,
        window: FlowWindow,
    ) -> Result<EventFlowReport, EventAuthorityError> {
        self.validate_observability_identity(ledger_id)?;
        validate_nonzero_bound(
            "event flow window",
            window.max_events,
            MAX_FLOW_WINDOW_EVENTS,
        )
        .map_err(EventAuthorityError::from)?;
        session::compute_flow(&self.as_ledger_observability(), window).map_err(Into::into)
    }

    /// Computes one bounded session report after validating identity.
    pub fn session(
        &self,
        ledger_id: LedgerIdentity,
        session_id: &str,
    ) -> Result<SessionTimelineReport, EventAuthorityError> {
        self.validate_observability_identity(ledger_id)?;
        session::compute_timeline(&self.as_ledger_observability(), session_id).map_err(Into::into)
    }

    /// Computes one bounded structural trace after validating identity.
    pub fn trace(
        &self,
        ledger_id: LedgerIdentity,
        subject: TraceSubject,
    ) -> Result<EventTraceReport, EventAuthorityError> {
        self.validate_observability_identity(ledger_id)?;
        trace::compute(&self.as_ledger_observability(), subject).map_err(Into::into)
    }

    fn validate_observability_identity(
        &self,
        actual: LedgerIdentity,
    ) -> Result<(), EventAuthorityError> {
        let expected = self.ledger_identity();
        if expected == actual {
            Ok(())
        } else {
            Err(EventAuthorityError::IdentityMismatch { expected, actual })
        }
    }

    fn as_ledger_observability(&self) -> LedgerObservability {
        LedgerObservability::from_authority(
            self.ledger_identity(),
            self.store_handle(),
            self.watermark_handle(),
            self.registry_handle(),
            self.dropped_handle(),
        )
    }
}

fn stable_ledger_identity(store: &EventStore) -> Result<LedgerIdentity, StorageError> {
    let bytes = match store.ledger_identity_bytes()? {
        Some(raw) => raw,
        None => {
            let candidate = LedgerIdentity::new();
            store.establish_ledger_identity_bytes(&candidate.encode())?
        }
    };
    LedgerIdentity::decode(&bytes).map_err(|error| {
        StorageError::InvalidPath(format!(
            "ledger_identity must contain one 16-byte UUID: {error}"
        ))
    })
}

fn read_page(
    store: &EventStore,
    ledger_id: LedgerIdentity,
    cursor: LedgerCursor,
    limit: usize,
) -> Result<EventPage, StorageError> {
    let tip_seq = store.tip_seq()?;
    let retained_from = store.retained_lower_boundary()?;
    let mut records =
        store.read_all_events_between_limit(cursor.after_seq, tip_seq, limit.saturating_add(1))?;
    let truncated_after = records.len() > limit;
    records.truncate(limit);
    let scanned_from_seq = records.first().map(|record| record.seq);
    let scanned_through_seq = records.last().map(|record| record.seq);
    let truncated_before = retained_from > 1 || (tip_seq > 0 && cursor.after_seq >= retained_from);
    let next_after_seq = scanned_through_seq.unwrap_or(cursor.after_seq);
    Ok(EventPage {
        ledger_id,
        records,
        next_cursor: LedgerCursor {
            ledger_id,
            after_seq: next_after_seq,
        },
        coverage: EventReadCoverage {
            retained_from,
            tip_seq,
            scanned_from_seq,
            scanned_through_seq,
            truncation: coverage_truncation(truncated_before, truncated_after),
        },
    })
}

fn read_newest_page(
    store: &EventStore,
    ledger_id: LedgerIdentity,
    limit: usize,
) -> Result<EventPage, StorageError> {
    let tip_seq = store.tip_seq()?;
    let retained_from = store.retained_lower_boundary()?;
    let mut records = store.read_newest_events_through(tip_seq, limit.saturating_add(1))?;
    let truncated_before = retained_from > 1 || records.len() > limit;
    if records.len() > limit {
        records.remove(0);
    }
    let scanned_from_seq = records.first().map(|record| record.seq);
    let scanned_through_seq = records.last().map(|record| record.seq);
    let after_seq = scanned_through_seq.unwrap_or_else(|| retained_from.saturating_sub(1));
    Ok(EventPage {
        ledger_id,
        records,
        next_cursor: LedgerCursor {
            ledger_id,
            after_seq,
        },
        coverage: EventReadCoverage {
            retained_from,
            tip_seq,
            scanned_from_seq,
            scanned_through_seq,
            truncation: coverage_truncation(truncated_before, false),
        },
    })
}

pub(super) fn coverage_truncation(
    truncated_before: bool,
    truncated_after: bool,
) -> CoverageTruncation {
    match (truncated_before, truncated_after) {
        (false, false) => CoverageTruncation::None,
        (true, false) => CoverageTruncation::Before,
        (false, true) => CoverageTruncation::After,
        (true, true) => CoverageTruncation::Both,
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
