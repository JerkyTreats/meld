//! Canonical event domain contracts and compatibility surface.
//!
//! This domain owns unsequenced envelopes, persisted event records, graph
//! materialization references, single-writer ingestion with group commit,
//! durable and best-effort emission, and append-only event storage.
//!
//! Inputs are producer envelopes from telemetry, execution, workflow, world
//! model, and compatibility callers. Outputs are sequenced event records,
//! session-scoped reads, runtime-wide cursor reads, and object references for
//! downstream graph materializers.
//!
//! This domain does not own event payload semantics, external telemetry sink
//! routing, task execution, workflow orchestration, or graph materialization.
//! Producers own payload meaning and consumers adapt through explicit records.
//!
//! # Example
//!
//! ```rust
//! use meld_events::{EventEnvelope, EventRecord};
//! use serde_json::json;
//!
//! let envelope = EventEnvelope::new_domain(
//!     "2026-04-26T16:00:00Z".to_string(),
//!     "session-a",
//!     "execution",
//!     "workflow-a",
//!     "execution.artifact.available",
//!     Some("sha256:abc".to_string()),
//!     json!({ "artifact": "artifact-a" }),
//! );
//! let record = EventRecord::from_envelope(envelope, 7);
//!
//! assert_eq!(record.stream_id, "workflow-a");
//! assert_eq!(record.content_hash.as_deref(), Some("sha256:abc"));
//! ```

use std::ops::{Deref, DerefMut};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Identity-bound event authority and its derived capabilities.
pub mod authority;
/// Compatibility aliases for pre-extraction event callers.
pub mod compat;
/// Domain object and relation records carried by event envelopes.
pub mod contracts;
/// Durable ledger identity contract.
pub mod identity;
/// Recoverable migration from frozen legacy ledgers into one authority.
pub mod migration;
/// Observability port, report contracts, and in-process backing.
pub mod observability;
/// Named consumer cursor registry for lag observability.
pub(crate) mod registry;
/// Transport-neutral authority requests, responses, and client contract.
pub mod remote;
/// Legacy event emission facade retained for internal compatibility tests.
#[cfg(any(test, feature = "test-support"))]
pub(crate) mod runtime;
/// Append-only sled-backed event store.
pub(crate) mod store;
/// Consumer subscription surface and durable cursor helper.
#[cfg(any(test, feature = "test-support"))]
pub(crate) mod subscription;
/// Explicit raw-ledger fixtures for low-level tests, benches, and fuzzing.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
/// Single-writer ingress engine with group commit and watermark.
pub(crate) mod writer;

pub use authority::{
    AppendDisposition, AppendMode, AppendReceipt, BestEffortAppendReceipt, ConsumerCursorPosition,
    EventAppendCapability, EventAuthority, EventAuthorityOpenOptions,
    EventConsumerRegistryCapability, EventFinalBarrier, EventIngressFenceSnapshot,
    EventIngressFenceState, EventObservabilityCapability, EventPage, EventReplayCapability,
    EventSubscriptionCapability, EventWatermark, EventWatermarkCapability, LedgerCursor,
    ReplayRequest, SubscriptionPollRequest, MAX_REPLAY_LIMIT, MAX_SUBSCRIPTION_TIMEOUT_MS,
};
pub use contracts::{
    validate_event_append_envelope, validate_event_structural_identifier, DomainObjectRef,
    EventAppendValidationCode, EventAppendValidationIssue, EventRelation,
    EventStructuralIdentifierKind, MAX_EVENT_NAMESPACE_IDENTIFIER_BYTES,
    MAX_EVENT_OPAQUE_IDENTIFIER_BYTES,
};
pub use identity::LedgerIdentity;
pub use migration::{
    LegacyEventCutoverMarker, LegacyEventMigrationMapping, LegacyEventMigrationOptions,
    LegacyEventMigrationReport, LegacyEventMigrationSource,
};
pub use observability::{
    ConsumerLagReport, CoverageTruncation, DomainAppendRate, DomainFlow, EventFlowReport,
    EventHealthReport, EventReadCoverage, EventTraceReport, FlowWindow, SessionStep,
    SessionTimelineReport, SilentDomain, TraceHop, TraceLink, TraceSubject, TypeFlow,
    MAX_EVENT_PAGE_LIMIT, MAX_EVENT_PAGE_TIMEOUT_MS, MAX_FLOW_WINDOW_EVENTS,
    MAX_SESSION_SCAN_EVENTS, MAX_TRACE_SCAN_EVENTS,
};
#[cfg(any(test, feature = "test-support"))]
pub use observability::{EventObservabilityPort, EventPageRequest, LegacyEventPage};
pub use registry::ConsumerCursor;
pub use remote::{
    BestEffortAppendRequest, DurableAppendRequest, EventAuthorityContract, FlowRequest,
    HealthRequest, SessionRequest, TraceRequest, WatermarkRequest,
};

/// Persisted event record in the global event ledger.
///
/// Records are append-only after a sequence is assigned. The record owns store
/// sequence metadata and contains the prepared producer envelope unchanged.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventRecord {
    /// Runtime-wide monotonically increasing sequence.
    pub seq: u64,
    /// Prepared event envelope supplied by the producer.
    pub envelope: EventEnvelope,
}

/// Identity-bearing reference to one canonical event record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventRecordRef {
    /// Ledger that owns the referenced sequence.
    pub ledger_id: LedgerIdentity,
    /// Canonical sequence within that ledger.
    pub seq: u64,
}

/// Structural source-record provenance carried by a derived event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventProvenance {
    /// Canonical records used to derive this event.
    pub source_records: Vec<EventRecordRef>,
}

impl EventProvenance {
    /// Returns true when the envelope carries no source-record references.
    pub fn is_empty(&self) -> bool {
        self.source_records.is_empty()
    }
}

/// Unsequenced event ready to be emitted or appended to the store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Producer timestamp retained for legacy callers.
    pub ts: String,
    /// Storage timestamp to persist with the record.
    pub recorded_at: String,
    /// Optional idempotency key supplied by the producer.
    pub record_id: Option<String>,
    /// Session-level read partition.
    pub session: String,
    /// Domain that owns the event meaning.
    pub domain_id: String,
    /// Domain-local stream that groups related records.
    pub stream_id: String,
    /// Stable event type identifier.
    #[serde(rename = "type", alias = "event_type")]
    pub event_type: String,
    /// Optional source occurrence time when it differs from record time.
    pub occurred_at: Option<String>,
    /// Optional content hash for deduplication or lineage outside this store.
    pub content_hash: Option<String>,
    /// Domain objects referenced by this event.
    pub objects: Vec<DomainObjectRef>,
    /// Directed relations between referenced objects.
    pub relations: Vec<EventRelation>,
    /// Structural provenance linking derived events to canonical sources.
    #[serde(default, skip_serializing_if = "EventProvenance::is_empty")]
    pub provenance: EventProvenance,
    /// Domain payload owned by the event producer.
    pub data: Value,
}

impl EventEnvelope {
    /// Creates a legacy telemetry envelope in the session stream.
    pub fn new(ts: String, session: String, event_type: impl Into<String>, data: Value) -> Self {
        Self::new_domain(
            ts,
            session.clone(),
            default_domain_id(),
            session,
            event_type,
            None,
            data,
        )
    }

    /// Creates a domain envelope with explicit stream ownership.
    pub fn new_domain(
        ts: String,
        session: impl Into<String>,
        domain_id: impl Into<String>,
        stream_id: impl Into<String>,
        event_type: impl Into<String>,
        content_hash: Option<String>,
        data: Value,
    ) -> Self {
        let recorded_at = if ts.is_empty() {
            default_timestamp()
        } else {
            ts.clone()
        };
        Self {
            ts,
            recorded_at,
            record_id: None,
            session: session.into(),
            domain_id: domain_id.into(),
            stream_id: stream_id.into(),
            event_type: event_type.into(),
            occurred_at: None,
            content_hash,
            objects: Vec::new(),
            relations: Vec::new(),
            provenance: EventProvenance::default(),
            data,
        }
    }

    /// Creates a legacy telemetry envelope with the current timestamp.
    pub fn with_now(
        session: impl Into<String>,
        event_type: impl Into<String>,
        data: Value,
    ) -> Self {
        let session = session.into();
        Self::with_now_domain(
            session.clone(),
            default_domain_id(),
            session,
            event_type,
            None,
            data,
        )
    }

    /// Creates a domain envelope with the current timestamp.
    pub fn with_now_domain(
        session: impl Into<String>,
        domain_id: impl Into<String>,
        stream_id: impl Into<String>,
        event_type: impl Into<String>,
        content_hash: Option<String>,
        data: Value,
    ) -> Self {
        let ts = default_timestamp();
        Self {
            ts: ts.clone(),
            recorded_at: ts,
            record_id: None,
            session: session.into(),
            domain_id: domain_id.into(),
            stream_id: stream_id.into(),
            event_type: event_type.into(),
            occurred_at: None,
            content_hash,
            objects: Vec::new(),
            relations: Vec::new(),
            provenance: EventProvenance::default(),
            data,
        }
    }

    /// Creates a genesis fact marking a projection rebuilt from a snapshot.
    ///
    /// Replay for the stream may start at this fact plus later deltas
    /// instead of from the beginning of history; `basis_seq` records the
    /// highest source sequence the snapshot covers, and the idempotency key
    /// makes re-recording the same genesis safe.
    pub fn genesis_domain(
        session: impl Into<String>,
        domain_id: impl Into<String>,
        stream_id: impl Into<String>,
        basis_seq: u64,
        data: Value,
    ) -> Self {
        let domain_id = domain_id.into();
        let stream_id = stream_id.into();
        let event_type = format!("{domain_id}.genesis");
        let record_id = format!("genesis::{domain_id}::{stream_id}::{basis_seq}");
        Self::with_now_domain(session, domain_id, stream_id, event_type, None, data)
            .with_record_id(record_id)
    }

    /// Attaches graph materialization references to the envelope.
    pub fn with_graph(
        mut self,
        objects: Vec<DomainObjectRef>,
        relations: Vec<EventRelation>,
    ) -> Self {
        self.objects = objects;
        self.relations = relations;
        self
    }

    /// Attaches canonical source-record provenance to this envelope.
    pub fn with_source_records(mut self, source_records: Vec<EventRecordRef>) -> Self {
        self.provenance = EventProvenance { source_records };
        self
    }

    /// Sets the source occurrence timestamp.
    pub fn with_occurred_at(mut self, occurred_at: impl Into<String>) -> Self {
        self.occurred_at = Some(occurred_at.into());
        self
    }

    /// Sets the producer idempotency key.
    pub fn with_record_id(mut self, record_id: impl Into<String>) -> Self {
        self.record_id = Some(record_id.into());
        self
    }
}

impl EventRecord {
    /// Assigns a ledger sequence to an envelope without changing producer data.
    pub fn from_envelope(envelope: EventEnvelope, seq: u64) -> Self {
        Self { seq, envelope }
    }

    /// Fills fields that did not exist on legacy telemetry records.
    pub fn normalize_legacy_defaults(mut self) -> Self {
        if self.envelope.recorded_at.is_empty() {
            self.envelope.recorded_at = if self.envelope.ts.is_empty() {
                default_timestamp()
            } else {
                self.envelope.ts.clone()
            };
        }
        if self.envelope.domain_id.is_empty() {
            self.envelope.domain_id = default_domain_id();
        }
        if self.envelope.stream_id.is_empty() {
            self.envelope.stream_id = self.envelope.session.clone();
        }
        self
    }

    /// Returns the contained producer envelope.
    pub fn envelope(&self) -> &EventEnvelope {
        &self.envelope
    }

    /// Returns the contained producer envelope for mutation.
    pub fn envelope_mut(&mut self) -> &mut EventEnvelope {
        &mut self.envelope
    }

    /// Splits the record into its store sequence and producer envelope.
    pub fn into_parts(self) -> (u64, EventEnvelope) {
        (self.seq, self.envelope)
    }
}

impl Deref for EventRecord {
    type Target = EventEnvelope;

    fn deref(&self) -> &Self::Target {
        &self.envelope
    }
}

impl DerefMut for EventRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.envelope
    }
}

impl<'de> Deserialize<'de> for EventRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum EventRecordWire {
            Current { seq: u64, envelope: EventEnvelope },
            Legacy(LegacyEventRecord),
        }

        match EventRecordWire::deserialize(deserializer)? {
            EventRecordWire::Current { seq, envelope } => Ok(EventRecord { seq, envelope }),
            EventRecordWire::Legacy(legacy) => Ok(legacy.into_record()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LegacyEventRecord {
    ts: String,
    #[serde(default)]
    recorded_at: String,
    #[serde(default)]
    record_id: Option<String>,
    session: String,
    seq: u64,
    #[serde(default = "default_domain_id")]
    domain_id: String,
    #[serde(default)]
    stream_id: String,
    #[serde(rename = "type", alias = "event_type")]
    event_type: String,
    #[serde(default)]
    occurred_at: Option<String>,
    #[serde(default)]
    content_hash: Option<String>,
    #[serde(default)]
    objects: Vec<DomainObjectRef>,
    #[serde(default)]
    relations: Vec<EventRelation>,
    #[serde(default)]
    provenance: EventProvenance,
    data: Value,
}

impl LegacyEventRecord {
    fn into_record(self) -> EventRecord {
        EventRecord {
            seq: self.seq,
            envelope: EventEnvelope {
                ts: self.ts,
                recorded_at: self.recorded_at,
                record_id: self.record_id,
                session: self.session,
                domain_id: self.domain_id,
                stream_id: self.stream_id,
                event_type: self.event_type,
                occurred_at: self.occurred_at,
                content_hash: self.content_hash,
                objects: self.objects,
                relations: self.relations,
                provenance: self.provenance,
                data: self.data,
            },
        }
    }
}

fn default_domain_id() -> String {
    "telemetry".to_string()
}

fn default_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
