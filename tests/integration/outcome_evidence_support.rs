//! Task outcome to promoted evidence integration mapping.
//!
//! This adapter owns the first docs-writer success mapping at the root crate
//! boundary. World model ingestion stays generic and execution publication
//! remains task focused.

use std::collections::BTreeMap;

use meld_events::{DomainObjectRef, EventRecord};
use meld_world_model::belief::{EvidenceValue, PromotedEvidenceRecord};
use thiserror::Error;

/// Request to map one docs task success fact into generic promoted evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct DocsTaskSuccessEvidenceRequest {
    /// Published event ledger record.
    pub event: EventRecord,
    /// Workspace subject whose belief should receive the promoted evidence.
    pub subject: DomainObjectRef,
    /// Runtime value for the configured stale probability field.
    pub stale_probability: f64,
    /// Runtime value for the configured review probability field.
    pub review_probability: f64,
    /// Configured promoted source kind.
    pub source_kind: String,
    /// Artifact type that marks this success as applicable docs content.
    pub required_artifact_type_id: Option<String>,
}

impl DocsTaskSuccessEvidenceRequest {
    /// Build the first-slice docs content success mapping.
    pub fn fresh_content(event: EventRecord, subject: DomainObjectRef) -> Self {
        Self {
            event,
            subject,
            stale_probability: 0.0,
            review_probability: 0.2,
            source_kind: "content_written".to_string(),
            required_artifact_type_id: Some("docs_patch".to_string()),
        }
    }
}

/// Errors returned by the docs outcome evidence mapper.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DocsTaskSuccessEvidenceError {
    /// Mapping request failed validation.
    #[error("invalid docs task success evidence request: {0}")]
    InvalidRequest(String),
}

/// Build promoted evidence for a docs task success event.
pub fn build_docs_task_success_evidence(
    request: DocsTaskSuccessEvidenceRequest,
) -> Result<Option<PromotedEvidenceRecord>, DocsTaskSuccessEvidenceError> {
    if request.event.envelope.domain_id != "execution" {
        return Ok(None);
    }
    if request.event.envelope.event_type != "execution.task.succeeded" {
        return Ok(None);
    }
    if let Some(required_artifact_type_id) = request.required_artifact_type_id.as_deref() {
        require_non_empty("required artifact type id", required_artifact_type_id)?;
        if !event_has_artifact_type(&request.event, required_artifact_type_id) {
            return Ok(None);
        }
    }

    request
        .subject
        .validate()
        .map_err(|error| DocsTaskSuccessEvidenceError::InvalidRequest(error.to_string()))?;
    require_non_empty("source kind", &request.source_kind)?;
    validate_probability("stale probability", request.stale_probability)?;
    validate_probability("review probability", request.review_probability)?;

    let mut fields = BTreeMap::new();
    fields.insert(
        "stale_probability".to_string(),
        EvidenceValue::Scalar(request.stale_probability),
    );
    fields.insert(
        "review_probability".to_string(),
        EvidenceValue::Scalar(request.review_probability),
    );

    let event = request.event;
    let source_id = event
        .envelope
        .record_id
        .clone()
        // The event-spine:: prefix is a frozen stored-identifier format:
        // existing evidence records reference it, so it survives the ledger
        // renaming.
        .unwrap_or_else(|| format!("event-spine::{}", event.seq));
    Ok(Some(PromotedEvidenceRecord {
        source_kind: request.source_kind,
        source_id,
        subject: request.subject,
        source_fact_ids: vec![format!("event-spine::{}", event.seq)],
        graph_anchor_ids: Vec::new(),
        objects: event.envelope.objects.clone(),
        relations: event.envelope.relations.clone(),
        source_cursor_start: event.seq,
        source_cursor_end: event.seq,
        reference_time: event
            .envelope
            .occurred_at
            .clone()
            .or_else(|| Some(event.envelope.recorded_at.clone())),
        transaction_seq: event.seq,
        content_hash: event.envelope.content_hash.clone(),
        fields,
    }))
}

fn event_has_artifact_type(event: &EventRecord, artifact_type_id: &str) -> bool {
    event
        .envelope
        .data
        .get("artifact_records")
        .and_then(|records| records.as_array())
        .is_some_and(|records| {
            records.iter().any(|record| {
                record
                    .get("artifact_type_id")
                    .and_then(|value| value.as_str())
                    == Some(artifact_type_id)
            })
        })
}

fn validate_probability(label: &str, value: f64) -> Result<(), DocsTaskSuccessEvidenceError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(DocsTaskSuccessEvidenceError::InvalidRequest(format!(
            "{label} must be a finite probability"
        )));
    }
    Ok(())
}

fn require_non_empty(label: &str, value: &str) -> Result<(), DocsTaskSuccessEvidenceError> {
    if value.trim().is_empty() {
        return Err(DocsTaskSuccessEvidenceError::InvalidRequest(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}

use std::sync::Arc;

use meld::runtime::ports::ProductEventReplayPort;
use meld_world_model::belief::{
    ingest_promoted_evidence, ConfigSnapshot, PromotedEvidenceIngestionRequest,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::PerspectiveKey;
use meld_world_model::{
    BeliefRuntime, BeliefStore, BranchScope, PromotedEvidenceIngestionResult,
};

/// Test-only replay of docs task success events into belief evidence.
///
/// Lifted verbatim from the removed `DocsTaskEvidenceReplayPort` so the
/// reopen contract keeps its evidence-injection driver. The production
/// evidence path is the world-model interpretation mapping; this helper and
/// its hardcoded probabilities exist only for characterized contract tests.
pub struct DocsTaskEvidenceReplayRequest {
    pub after_seq: u64,
    pub limit: usize,
    pub subject: DomainObjectRef,
    pub config: ConfigSnapshot,
    pub perspective: PerspectiveKey,
    pub branch_scope: BranchScope,
    pub owner_id: String,
    pub required_artifact_type_id: Option<String>,
}

pub struct DocsTaskEvidenceReplayReport {
    pub input_event_seq: u64,
    pub output_event_seq: u64,
    pub events_attempted: usize,
    pub promoted_evidence_count: usize,
    pub rejected_evidence_count: usize,
    pub normalized_evidence_count: usize,
    pub new_assignment_count: usize,
    pub ingestions: Vec<PromotedEvidenceIngestionResult>,
}

pub fn ingest_after_limit(
    event_replay: &ProductEventReplayPort,
    belief_store: &Arc<BeliefStore>,
    traversal_store: &Arc<TraversalStore>,
    request: DocsTaskEvidenceReplayRequest,
) -> Result<DocsTaskEvidenceReplayReport, String> {
    if request.owner_id.trim().is_empty() {
        return Err("owner id must be non-empty".to_string());
    }
    request
        .subject
        .validate()
        .map_err(|error| error.to_string())?;
    let events = event_replay
        .read_after_limit(request.after_seq, request.limit)
        .map_err(|error| error.to_string())?;
    let runtime = BeliefRuntime::new(
        Arc::clone(belief_store),
        Arc::clone(traversal_store),
        request.config.clone(),
        request.perspective.clone(),
        request.branch_scope.clone(),
    );
    let mut report = DocsTaskEvidenceReplayReport {
        input_event_seq: request.after_seq,
        output_event_seq: request.after_seq,
        events_attempted: events.len(),
        promoted_evidence_count: 0,
        rejected_evidence_count: 0,
        normalized_evidence_count: 0,
        new_assignment_count: 0,
        ingestions: Vec::new(),
    };

    for event in events {
        report.output_event_seq = report.output_event_seq.max(event.seq);
        let promoted = build_docs_task_success_evidence(DocsTaskSuccessEvidenceRequest {
            event,
            subject: request.subject.clone(),
            stale_probability: 0.0,
            review_probability: 0.2,
            source_kind: "content_written".to_string(),
            required_artifact_type_id: request.required_artifact_type_id.clone(),
        })
        .map_err(|error| error.to_string())?;
        let Some(record) = promoted else {
            continue;
        };
        report.promoted_evidence_count += 1;
        let ingestion = ingest_promoted_evidence(
            belief_store.as_ref(),
            &runtime,
            PromotedEvidenceIngestionRequest {
                record,
                config: request.config.clone(),
                perspective: request.perspective.clone(),
                branch_scope: request.branch_scope.clone(),
                owner_id: &request.owner_id,
            },
        )
        .map_err(|error| error.to_string())?;
        if ingestion.rejected {
            report.rejected_evidence_count += 1;
        }
        report.normalized_evidence_count += ingestion.normalized_evidence_count;
        report.new_assignment_count += ingestion.new_assignment_count;
        report.ingestions.push(ingestion);
    }

    Ok(report)
}
