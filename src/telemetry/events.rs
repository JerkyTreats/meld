//! Event schema for telemetry.

use serde::{Deserialize, Serialize};

pub use crate::events::compat::{ProgressEnvelope, ProgressEvent};

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

pub use meld_execution::generation::{
    FrameMetadataValidationProgressEventData as FrameMetadataValidationEventData,
    PromptContextLineageProgressEventData as PromptContextLineageEventData,
};
pub use meld_execution::workflow::{
    WorkflowForceResetProgressEventData as WorkflowForceResetEventData,
    WorkflowTargetProgressEventData as WorkflowTargetEventData,
    WorkflowTurnProgressEventData as WorkflowTurnEventData,
};

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
            record_id: None,
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
