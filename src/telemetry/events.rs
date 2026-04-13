//! Event schema for telemetry.

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::telemetry::contracts::{DomainObjectRef, EventRelation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub ts: String,
    #[serde(default)]
    pub recorded_at: String,
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
pub struct ProgressEnvelope {
    pub ts: String,
    pub recorded_at: String,
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

impl ProgressEnvelope {
    pub fn new(ts: String, session: String, event_type: impl Into<String>, data: Value) -> Self {
        Self::new_domain(ts, session.clone(), default_domain_id(), session, event_type, None, data)
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
}

impl ProgressEvent {
    pub fn from_envelope(envelope: ProgressEnvelope, seq: u64) -> Self {
        Self {
            ts: envelope.ts,
            recorded_at: envelope.recorded_at,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartedData {
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndedData {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEventData {
    pub node_id: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatsEventData {
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLifecycleEventData {
    pub node_id: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptContextLineageEventData {
    pub node_id: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub prompt_link_id: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub system_prompt_artifact_id: String,
    pub user_prompt_template_artifact_id: String,
    pub rendered_prompt_artifact_id: String,
    pub context_artifact_id: String,
    pub lineage_failure_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadataValidationEventData {
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub prompt_link_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_frame_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_prompt_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_context_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_prompt_link_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_seq: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTargetEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_frame_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turns_completed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reused_existing_head: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTurnEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub turn_seq: u32,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub attempt: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_frame_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowForceResetEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_frame_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryEventData {
    pub command: String,
    pub ok: bool,
    pub duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_chars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_chars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn event_round_trip() {
        let event = ProgressEvent {
            ts: "2026-02-14T12:34:56.789Z".to_string(),
            recorded_at: "2026-02-14T12:34:56.789Z".to_string(),
            session: "s1".to_string(),
            seq: 1,
            domain_id: "telemetry".to_string(),
            stream_id: "s1".to_string(),
            event_type: "session_started".to_string(),
            occurred_at: None,
            content_hash: None,
            objects: Vec::new(),
            relations: Vec::new(),
            data: json!({ "command": "scan" }),
        };
        let serialized = serde_json::to_string(&event).unwrap();
        let parsed: ProgressEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed.session, "s1");
        assert_eq!(parsed.seq, 1);
        assert_eq!(parsed.domain_id, "telemetry");
        assert_eq!(parsed.stream_id, "s1");
        assert_eq!(parsed.event_type, "session_started");
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let raw = r#"{"ts":"2026-02-14T12:34:56.789Z","session":"s1","seq":1,"type":"session_started","data":{"command":"scan"},"future":"ok"}"#;
        let parsed = serde_json::from_str::<ProgressEvent>(raw)
            .unwrap()
            .normalize_legacy_defaults();
        assert_eq!(parsed.session, "s1");
        assert_eq!(parsed.recorded_at, parsed.ts);
        assert_eq!(parsed.domain_id, "telemetry");
        assert_eq!(parsed.stream_id, "s1");
        assert!(parsed.objects.is_empty());
        assert!(parsed.relations.is_empty());
    }

    #[test]
    fn timestamp_is_iso_8601_with_milliseconds() {
        let env = ProgressEnvelope::with_now("s1", "session_started", json!({}));
        let parsed = chrono::DateTime::parse_from_rfc3339(&env.ts).unwrap();
        assert_eq!(env.ts.len(), 24);
        assert_eq!(env.recorded_at, env.ts);
        assert_eq!(env.ts.chars().nth(19), Some('.'));
        assert!(env.ts.ends_with('Z'));
        assert!(parsed.timestamp_subsec_millis() <= 999);
    }
}
