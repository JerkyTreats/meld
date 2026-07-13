//! Bounded task outcome replay into belief-owned evidence state.
//!
//! The world model owns both the first docs-freshness outcome mapping and the
//! mutation policy that normalizes, assigns, and reassesses promoted evidence.
//! Root assembly supplies only an identity-bearing event replay capability.

use std::collections::BTreeMap;
use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::{
    DomainObjectRef, EventPage, EventRecord, LedgerCursor, LedgerIdentity, ReplayRequest,
};
use thiserror::Error;

use super::{
    ingest_promoted_evidence, BeliefRuntime, BeliefStore, BranchScope, ConfigSnapshot,
    EvidenceValue, PromotedEvidenceIngestionRequest, PromotedEvidenceIngestionResult,
    PromotedEvidenceRecord,
};
use crate::belief::ports::BeliefGraphQuery;
use crate::world_state::graph::PerspectiveKey;

/// Maximum events one task-evidence ingestion invocation may inspect.
pub const MAX_TASK_EVIDENCE_REPLAY_LIMIT: usize = 1024;

/// Identity-bearing event replay capability consumed by belief ingestion.
///
/// Root adapters implement this contract without interpreting event payloads,
/// choosing evidence values, or mutating belief state.
pub trait EvidenceEventReplaySource: Send + Sync {
    /// Return the durable ledger identity supplied by this source.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Replay one bounded page from the supplied identity-bearing cursor.
    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError>;
}

/// Request to map one docs task success fact into promoted evidence.
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

/// Request for one bounded docs task evidence ingestion invocation.
#[derive(Debug, Clone)]
pub struct DocsTaskEvidenceReplayRequest {
    /// Last event sequence already processed by the caller.
    pub after_seq: u64,
    /// Maximum event records to inspect.
    pub limit: usize,
    /// Workspace subject whose belief should receive support evidence.
    pub subject: DomainObjectRef,
    /// Runtime family configuration used for evidence normalization.
    pub config: ConfigSnapshot,
    /// Perspective for candidate belief keys.
    pub perspective: PerspectiveKey,
    /// Branch scope for candidate belief keys.
    pub branch_scope: BranchScope,
    /// Worker identity recorded on assessment leases.
    pub owner_id: String,
    /// Artifact type that marks a success event as applicable docs content.
    pub required_artifact_type_id: Option<String>,
}

/// Report from one bounded docs task evidence ingestion invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct DocsTaskEvidenceReplayReport {
    /// Replay cursor supplied by the caller.
    pub input_event_seq: u64,
    /// Highest event sequence inspected by this invocation.
    pub output_event_seq: u64,
    /// Event records inspected.
    pub events_attempted: usize,
    /// Promoted evidence records produced by the mapping.
    pub promoted_evidence_count: usize,
    /// Promoted evidence records rejected by belief config.
    pub rejected_evidence_count: usize,
    /// Evidence items produced by normalization.
    pub normalized_evidence_count: usize,
    /// New belief assignment edges inserted.
    pub new_assignment_count: usize,
    /// Per-ingestion results returned by the belief runtime.
    pub ingestions: Vec<PromotedEvidenceIngestionResult>,
}

/// Errors returned by docs task evidence mapping and bounded ingestion.
#[derive(Debug, Error)]
pub enum DocsTaskEvidenceError {
    /// Mapping or bounded request validation failed.
    #[error("invalid docs task evidence request: {0}")]
    InvalidRequest(String),
    /// Event replay failed before the requested page was available.
    #[error("docs task evidence replay failed: {0}")]
    Replay(String),
    /// Belief-owned storage or assessment failed.
    #[error("docs task evidence storage failed: {0}")]
    Storage(String),
}

/// Bounded world-model runtime for docs task outcome evidence.
///
/// Construction and invocation are explicit. Registering this runtime does not
/// make a recurring supervisor actor concrete or enabled.
#[derive(Clone)]
pub struct DocsTaskEvidenceIngestionRuntime {
    event_replay: Arc<dyn EvidenceEventReplaySource>,
    belief_store: Arc<BeliefStore>,
    graph_query: Arc<dyn BeliefGraphQuery>,
}

impl DocsTaskEvidenceIngestionRuntime {
    /// Bind event replay and the shared world-model stores without doing work.
    pub fn new<Q>(
        event_replay: Arc<dyn EvidenceEventReplaySource>,
        belief_store: Arc<BeliefStore>,
        graph_query: Arc<Q>,
    ) -> Self
    where
        Q: BeliefGraphQuery + 'static,
    {
        Self {
            event_replay,
            belief_store,
            graph_query,
        }
    }

    /// Bind an already erased graph query contract.
    pub fn from_graph_query(
        event_replay: Arc<dyn EvidenceEventReplaySource>,
        belief_store: Arc<BeliefStore>,
        graph_query: Arc<dyn BeliefGraphQuery>,
    ) -> Self {
        Self {
            event_replay,
            belief_store,
            graph_query,
        }
    }

    /// Replay a bounded event page and ingest applicable docs success evidence.
    pub fn ingest_after_limit(
        &self,
        request: DocsTaskEvidenceReplayRequest,
    ) -> Result<DocsTaskEvidenceReplayReport, DocsTaskEvidenceError> {
        validate_ingestion_request(&request)?;
        let page = self
            .event_replay
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: self.event_replay.ledger_identity(),
                    after_seq: request.after_seq,
                },
                limit: request.limit,
            })
            .map_err(|error| DocsTaskEvidenceError::Replay(error.to_string()))?;
        let runtime = BeliefRuntime::from_graph_query(
            Arc::clone(&self.belief_store),
            Arc::clone(&self.graph_query),
            request.config.clone(),
            request.perspective.clone(),
            request.branch_scope.clone(),
        );
        let mut report = DocsTaskEvidenceReplayReport {
            input_event_seq: request.after_seq,
            output_event_seq: request.after_seq,
            events_attempted: page.records.len(),
            promoted_evidence_count: 0,
            rejected_evidence_count: 0,
            normalized_evidence_count: 0,
            new_assignment_count: 0,
            ingestions: Vec::new(),
        };

        for event in page.records {
            report.output_event_seq = report.output_event_seq.max(event.seq);
            let mut evidence_request =
                DocsTaskSuccessEvidenceRequest::fresh_content(event, request.subject.clone());
            evidence_request.required_artifact_type_id = request.required_artifact_type_id.clone();
            let promoted = build_docs_task_success_evidence(evidence_request)?;
            let Some(record) = promoted else {
                continue;
            };
            report.promoted_evidence_count += 1;
            let ingestion = ingest_promoted_evidence(
                self.belief_store.as_ref(),
                &runtime,
                PromotedEvidenceIngestionRequest {
                    record,
                    config: request.config.clone(),
                    perspective: request.perspective.clone(),
                    branch_scope: request.branch_scope.clone(),
                    owner_id: &request.owner_id,
                },
            )
            .map_err(|error| DocsTaskEvidenceError::Storage(error.to_string()))?;
            if ingestion.rejected {
                report.rejected_evidence_count += 1;
            }
            report.normalized_evidence_count += ingestion.normalized_evidence_count;
            report.new_assignment_count += ingestion.new_assignment_count;
            report.ingestions.push(ingestion);
        }

        Ok(report)
    }
}

/// Build promoted evidence for a docs task success event.
pub fn build_docs_task_success_evidence(
    request: DocsTaskSuccessEvidenceRequest,
) -> Result<Option<PromotedEvidenceRecord>, DocsTaskEvidenceError> {
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
        .map_err(|error| DocsTaskEvidenceError::InvalidRequest(error.to_string()))?;
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
        // The event-spine prefix is a frozen stored identifier format. Existing
        // evidence records reference it, so it survives the ledger renaming.
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

fn validate_ingestion_request(
    request: &DocsTaskEvidenceReplayRequest,
) -> Result<(), DocsTaskEvidenceError> {
    if request.owner_id.trim().is_empty() {
        return Err(DocsTaskEvidenceError::InvalidRequest(
            "owner id must be non-empty".to_string(),
        ));
    }
    if !(1..=MAX_TASK_EVIDENCE_REPLAY_LIMIT).contains(&request.limit) {
        return Err(DocsTaskEvidenceError::InvalidRequest(format!(
            "event replay limit must be in 1..={MAX_TASK_EVIDENCE_REPLAY_LIMIT}, got {}",
            request.limit
        )));
    }
    request
        .subject
        .validate()
        .map_err(|error| DocsTaskEvidenceError::InvalidRequest(error.to_string()))
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

fn validate_probability(label: &str, value: f64) -> Result<(), DocsTaskEvidenceError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(DocsTaskEvidenceError::InvalidRequest(format!(
            "{label} must be a finite probability"
        )));
    }
    Ok(())
}

fn require_non_empty(label: &str, value: &str) -> Result<(), DocsTaskEvidenceError> {
    if value.trim().is_empty() {
        return Err(DocsTaskEvidenceError::InvalidRequest(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}
