//! Canonical event domain contracts and compatibility surface.

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod compat;
pub mod contracts;
pub mod ingress;
pub mod query;
pub mod runtime;
pub mod store;
pub mod subscription;

pub use contracts::{DomainObjectRef, EventRelation};
pub use ingress::{EventBus, EventIngestor, SharedIngestor};
pub use runtime::EventRuntime;
pub use store::EventStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub ts: String,
    #[serde(default)]
    pub recorded_at: String,
    #[serde(default)]
    pub record_id: Option<String>,
    pub session: String,
    pub seq: u64,
    #[serde(default = "default_domain_id")]
    pub domain_id: String,
    #[serde(default)]
    pub stream_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub occurred_at: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub objects: Vec<DomainObjectRef>,
    #[serde(default)]
    pub relations: Vec<EventRelation>,
    pub data: Value,
}

#[derive(Debug, Clone)]
pub struct EventEnvelope {
    pub ts: String,
    pub recorded_at: String,
    pub record_id: Option<String>,
    pub session: String,
    pub domain_id: String,
    pub stream_id: String,
    pub event_type: String,
    pub occurred_at: Option<String>,
    pub content_hash: Option<String>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
    pub data: Value,
}

impl EventEnvelope {
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

    pub fn with_graph(
        mut self,
        objects: Vec<DomainObjectRef>,
        relations: Vec<EventRelation>,
    ) -> Self {
        self.objects = objects;
        self.relations = relations;
        self
    }

    pub fn with_occurred_at(mut self, occurred_at: impl Into<String>) -> Self {
        self.occurred_at = Some(occurred_at.into());
        self
    }

    pub fn with_record_id(mut self, record_id: impl Into<String>) -> Self {
        self.record_id = Some(record_id.into());
        self
    }
}

impl EventRecord {
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
