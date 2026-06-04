//! Canonical event domain contracts and compatibility surface.
//!
//! This domain owns unsequenced envelopes, persisted event records, graph
//! materialization references, in-process ingestion, synchronous runtime
//! emission, and append-only event storage.
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

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Compatibility aliases for pre-extraction event callers.
pub mod compat;
/// Domain object and relation records carried by event envelopes.
pub mod contracts;
/// Non-blocking event bus and queue drainers.
pub mod ingress;
/// Synchronous event emission facade.
pub mod runtime;
/// Append-only sled-backed event store.
pub mod store;
/// Subscription compatibility surface for event bus callers.
pub mod subscription;

pub use contracts::{DomainObjectRef, EventRelation};
pub use ingress::{EventBus, EventIngestor, SharedIngestor};
pub use runtime::EventRuntime;

/// Persisted event record in the global event spine.
///
/// Records are append-only after a sequence is assigned. Legacy fields keep
/// defaults so older telemetry events can still be read through the same API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    /// Producer timestamp retained for legacy callers.
    pub ts: String,
    /// Storage timestamp for the record.
    #[serde(default)]
    pub recorded_at: String,
    /// Optional idempotency key supplied by the producer.
    #[serde(default)]
    pub record_id: Option<String>,
    /// Session-level read partition.
    pub session: String,
    /// Runtime-wide monotonically increasing sequence.
    pub seq: u64,
    /// Domain that owns the event meaning.
    #[serde(default = "default_domain_id")]
    pub domain_id: String,
    /// Domain-local stream that groups related records.
    #[serde(default)]
    pub stream_id: String,
    /// Stable event type identifier.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Optional source occurrence time when it differs from record time.
    #[serde(default)]
    pub occurred_at: Option<String>,
    /// Optional content hash for deduplication or lineage outside this store.
    #[serde(default)]
    pub content_hash: Option<String>,
    /// Domain objects referenced by this event.
    #[serde(default)]
    pub objects: Vec<DomainObjectRef>,
    /// Directed relations between referenced objects.
    #[serde(default)]
    pub relations: Vec<EventRelation>,
    /// Domain payload owned by the event producer.
    pub data: Value,
}

/// Unsequenced event ready to be emitted or appended to the store.
#[derive(Debug, Clone, PartialEq)]
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
    pub event_type: String,
    /// Optional source occurrence time when it differs from record time.
    pub occurred_at: Option<String>,
    /// Optional content hash for deduplication or lineage outside this store.
    pub content_hash: Option<String>,
    /// Domain objects referenced by this event.
    pub objects: Vec<DomainObjectRef>,
    /// Directed relations between referenced objects.
    pub relations: Vec<EventRelation>,
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
            data,
        }
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
    /// Assigns a spine sequence to an envelope without changing producer data.
    pub fn from_envelope(envelope: EventEnvelope, seq: u64) -> Self {
        Self {
            ts: envelope.ts,
            recorded_at: envelope.recorded_at,
            record_id: envelope.record_id,
            session: envelope.session,
            seq,
            domain_id: envelope.domain_id,
            stream_id: envelope.stream_id,
            event_type: envelope.event_type,
            occurred_at: envelope.occurred_at,
            content_hash: envelope.content_hash,
            objects: envelope.objects,
            relations: envelope.relations,
            data: envelope.data,
        }
    }

    /// Fills fields that did not exist on legacy telemetry records.
    pub fn normalize_legacy_defaults(mut self) -> Self {
        if self.recorded_at.is_empty() {
            self.recorded_at = if self.ts.is_empty() {
                default_timestamp()
            } else {
                self.ts.clone()
            };
        }
        if self.domain_id.is_empty() {
            self.domain_id = default_domain_id();
        }
        if self.stream_id.is_empty() {
            self.stream_id = self.session.clone();
        }
        self
    }
}

fn default_domain_id() -> String {
    "telemetry".to_string()
}

fn default_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
