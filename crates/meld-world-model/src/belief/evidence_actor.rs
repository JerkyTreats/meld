//! Durable bounded event-evidence actor owned by the belief domain.

use std::sync::Arc;

use meld_events::error::EventAuthorityError;

use crate::belief::{
    build_docs_task_success_evidence, ingest_promoted_evidence, BeliefRuntime, BeliefStore,
    BranchScope, ConfigSnapshot, DocsTaskEvidenceError, DocsTaskSuccessEvidenceRequest,
    EvidenceConsumerCursor, EvidenceEventReplaySource, EvidenceIngestionReceipt,
    EvidenceIngestionReceiptDisposition, EvidenceIngestionReceiptIdentity,
    EvidenceIngestionReceiptWriteDisposition, PromotedEvidenceIngestionRequest,
};
use crate::error::StorageError;
use crate::events::observability::CoverageTruncation;
use crate::events::{DomainObjectRef, EventPage, EventRecordRef, LedgerCursor, ReplayRequest};
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::PerspectiveKey;

/// Canonical recurring evidence ingestion runtime identity.
pub const EVIDENCE_INGESTION_ACTOR_ID: &str = "world_model.evidence_ingestion";

/// Maximum canonical source records one evidence tick may inspect.
pub const MAX_EVIDENCE_INGESTION_ITEMS: usize = 1024;

/// Request for one bounded durable event-evidence tick.
#[derive(Debug, Clone)]
pub struct EvidenceIngestionActorRequest {
    /// Workspace subject receiving applicable docs evidence.
    pub subject: DomainObjectRef,
    /// Belief-family configuration used for mapping and assessment.
    pub config: ConfigSnapshot,
    /// Perspective used for evidence assignment.
    pub perspective: PerspectiveKey,
    /// Branch used for evidence assignment.
    pub branch_scope: BranchScope,
    /// Active supervisor lease id recorded on assessment leases.
    pub lease_owner_id: String,
    /// Maximum canonical event records to inspect.
    pub max_items: usize,
    /// Artifact type required for docs-success promotion.
    pub required_artifact_type_id: Option<String>,
}

/// Stable diagnostic issue from one evidence ingestion tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceIngestionIssue {
    /// Optional canonical event record identity.
    pub item_id: Option<String>,
    /// Stable machine-readable category.
    pub code: String,
    /// Human-readable diagnostic detail.
    pub message: String,
}

/// Durable receipt disposition observed for one canonical source record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceiptReport {
    /// Canonical event sequence covered by the receipt.
    pub event_sequence: u64,
    /// Semantic mapping result retained durably.
    pub disposition: EvidenceIngestionReceiptDisposition,
    /// Whether this tick inserted or exactly replayed the receipt.
    pub write_disposition: EvidenceIngestionReceiptWriteDisposition,
}

/// Bounded diagnostic report from one evidence ingestion tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceIngestionActorReport {
    /// Canonical actor identity.
    pub actor_id: String,
    /// Durable event sequence observed before replay.
    pub input_event_sequence: u64,
    /// Durable event sequence covered after this tick.
    pub output_event_sequence: u64,
    /// Canonical source records inspected.
    pub events_attempted: usize,
    /// Records mapped into promoted evidence candidates.
    pub promoted_record_count: usize,
    /// Records rejected by belief normalization.
    pub rejected_record_count: usize,
    /// Records intentionally outside this mapping.
    pub irrelevant_record_count: usize,
    /// Normalized evidence items produced.
    pub normalized_evidence_count: usize,
    /// New durable assignment edges inserted.
    pub new_assignment_count: usize,
    /// Belief revisions committed during ingestion.
    pub committed_revision_count: usize,
    /// Durable semantic receipt results in event order.
    pub receipts: Vec<EvidenceReceiptReport>,
    /// Retryable issues that left the current event unacknowledged.
    pub retryable_errors: Vec<EvidenceIngestionIssue>,
    /// Fatal issues that require configuration or data repair.
    pub fatal_errors: Vec<EvidenceIngestionIssue>,
    /// True when replay reported additional records beyond this budget.
    pub budget_exhausted: bool,
}

impl EvidenceIngestionActorReport {
    fn new(input_event_sequence: u64) -> Self {
        Self {
            actor_id: EVIDENCE_INGESTION_ACTOR_ID.to_string(),
            input_event_sequence,
            output_event_sequence: input_event_sequence,
            events_attempted: 0,
            promoted_record_count: 0,
            rejected_record_count: 0,
            irrelevant_record_count: 0,
            normalized_evidence_count: 0,
            new_assignment_count: 0,
            committed_revision_count: 0,
            receipts: Vec::new(),
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }
}

/// Domain-owned actor that selects replay from its durable belief cursor.
pub struct EvidenceIngestionActor {
    replay: Arc<dyn EvidenceEventReplaySource>,
    store: Arc<BeliefStore>,
    traversal: Arc<TraversalStore>,
}

impl EvidenceIngestionActor {
    /// Bind identity-bearing event replay and durable world-model stores.
    pub fn new(
        replay: Arc<dyn EvidenceEventReplaySource>,
        store: Arc<BeliefStore>,
        traversal: Arc<TraversalStore>,
    ) -> Self {
        Self {
            replay,
            store,
            traversal,
        }
    }

    /// Select, ingest, receipt, and advance one bounded canonical event window.
    pub fn tick(&self, request: EvidenceIngestionActorRequest) -> EvidenceIngestionActorReport {
        let identity = self.cursor_identity(&request, 0);
        let current = match self.store.evidence_consumer_cursor(&identity) {
            Ok(current) => current,
            Err(error) => {
                let mut report = EvidenceIngestionActorReport::new(0);
                push_storage_issue(&mut report, None, error);
                return report;
            }
        };
        let input_sequence = current
            .as_ref()
            .map_or(0, |cursor| cursor.ledger_cursor.after_seq);
        let mut report = EvidenceIngestionActorReport::new(input_sequence);
        if let Err(message) = validate_request(&request) {
            report
                .fatal_errors
                .push(issue(None, "invalid_request", message));
            return report;
        }
        let runtime = BeliefRuntime::new(
            Arc::clone(&self.store),
            Arc::clone(&self.traversal),
            request.config.clone(),
            request.perspective.clone(),
            request.branch_scope.clone(),
        );
        if let Err(error) = runtime.persist_config() {
            push_storage_issue(&mut report, None, error);
            return report;
        }
        let page = match self.replay.replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: self.replay.ledger_identity(),
                after_seq: input_sequence,
            },
            limit: request.max_items,
        }) {
            Ok(page) => page,
            Err(error) => {
                push_replay_issue(&mut report, error);
                return report;
            }
        };
        if let Err(message) = validate_page(
            &page,
            self.replay.ledger_identity(),
            input_sequence,
            request.max_items,
        ) {
            report
                .fatal_errors
                .push(issue(None, "invalid_replay_page", message));
            return report;
        }
        report.budget_exhausted = matches!(
            page.coverage.truncation,
            CoverageTruncation::After | CoverageTruncation::Both
        );
        let mapping_hash = source_mapping_hash(&request);
        let mut expected = current;
        for event in page.records {
            report.events_attempted += 1;
            let item_id = Some(format!("{}::{}", self.replay.ledger_identity(), event.seq));
            let mapped = match build_docs_task_success_evidence(DocsTaskSuccessEvidenceRequest {
                event: event.clone(),
                subject: request.subject.clone(),
                stale_probability: 0.0,
                review_probability: 0.2,
                source_kind: "content_written".to_string(),
                required_artifact_type_id: request.required_artifact_type_id.clone(),
            }) {
                Ok(mapped) => mapped,
                Err(error) => {
                    push_mapping_issue(&mut report, item_id, error);
                    break;
                }
            };
            let mut evidence_ids = Vec::new();
            let disposition = match mapped {
                None => {
                    report.irrelevant_record_count += 1;
                    EvidenceIngestionReceiptDisposition::Irrelevant
                }
                Some(record) => {
                    report.promoted_record_count += 1;
                    match ingest_promoted_evidence(
                        self.store.as_ref(),
                        &runtime,
                        PromotedEvidenceIngestionRequest {
                            record,
                            config: request.config.clone(),
                            perspective: request.perspective.clone(),
                            branch_scope: request.branch_scope.clone(),
                            owner_id: &request.lease_owner_id,
                        },
                    ) {
                        Ok(ingestion) => {
                            evidence_ids = ingestion.evidence_ids;
                            report.normalized_evidence_count += ingestion.normalized_evidence_count;
                            report.new_assignment_count += ingestion.new_assignment_count;
                            report.committed_revision_count += ingestion.committed.len();
                            if ingestion.rejected {
                                report.rejected_record_count += 1;
                                EvidenceIngestionReceiptDisposition::Rejected
                            } else {
                                EvidenceIngestionReceiptDisposition::Promoted
                            }
                        }
                        Err(error) => {
                            push_storage_issue(&mut report, item_id, error);
                            break;
                        }
                    }
                }
            };
            let next = EvidenceConsumerCursor {
                consumer_id: EVIDENCE_INGESTION_ACTOR_ID.to_string(),
                ledger_cursor: LedgerCursor {
                    ledger_id: self.replay.ledger_identity(),
                    after_seq: event.seq,
                },
                family_config_hash: request.config.hash.clone(),
                source_mapping_hash: mapping_hash.clone(),
                perspective: request.perspective.clone(),
                branch_scope: request.branch_scope.clone(),
            };
            let receipt = EvidenceIngestionReceipt {
                identity: EvidenceIngestionReceiptIdentity {
                    consumer_id: next.consumer_id.clone(),
                    source_record: EventRecordRef {
                        ledger_id: next.ledger_cursor.ledger_id,
                        seq: event.seq,
                    },
                    family_config_hash: next.family_config_hash.clone(),
                    source_mapping_hash: next.source_mapping_hash.clone(),
                    perspective: next.perspective.clone(),
                    branch_scope: next.branch_scope.clone(),
                },
                disposition,
                evidence_ids,
            };
            // Semantic writes are replay-safe and become durable before this
            // atomic receipt plus cursor barrier acknowledges the source fact.
            match self
                .store
                .record_evidence_receipt_and_advance(expected.as_ref(), &receipt, &next)
            {
                Ok(write_disposition) => {
                    report.receipts.push(EvidenceReceiptReport {
                        event_sequence: event.seq,
                        disposition,
                        write_disposition,
                    });
                    report.output_event_sequence = event.seq;
                    expected = Some(next);
                }
                Err(error) => {
                    push_storage_issue(&mut report, item_id, error);
                    break;
                }
            }
        }
        if let Ok(Some(durable)) = self.store.evidence_consumer_cursor(&identity) {
            report.output_event_sequence = durable.ledger_cursor.after_seq;
        }
        report
    }

    /// Read the durable belief-owned cursor for this complete mapping identity.
    pub fn durable_cursor(
        &self,
        request: &EvidenceIngestionActorRequest,
    ) -> Result<Option<EvidenceConsumerCursor>, StorageError> {
        self.store
            .evidence_consumer_cursor(&self.cursor_identity(request, 0))
    }

    fn cursor_identity(
        &self,
        request: &EvidenceIngestionActorRequest,
        after_seq: u64,
    ) -> EvidenceConsumerCursor {
        EvidenceConsumerCursor {
            consumer_id: EVIDENCE_INGESTION_ACTOR_ID.to_string(),
            ledger_cursor: LedgerCursor {
                ledger_id: self.replay.ledger_identity(),
                after_seq,
            },
            family_config_hash: request.config.hash.clone(),
            source_mapping_hash: source_mapping_hash(request),
            perspective: request.perspective.clone(),
            branch_scope: request.branch_scope.clone(),
        }
    }
}

fn validate_request(request: &EvidenceIngestionActorRequest) -> Result<(), String> {
    if request.lease_owner_id.trim().is_empty() {
        return Err("supervisor lease owner id must be non-empty".to_string());
    }
    if !(1..=MAX_EVIDENCE_INGESTION_ITEMS).contains(&request.max_items) {
        return Err(format!(
            "evidence ingestion budget must be in 1..={MAX_EVIDENCE_INGESTION_ITEMS}"
        ));
    }
    request
        .subject
        .validate()
        .map_err(|error| error.to_string())?;
    if request
        .required_artifact_type_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err("required artifact type id must be non-empty".to_string());
    }
    Ok(())
}

fn validate_page(
    page: &EventPage,
    ledger_id: crate::events::LedgerIdentity,
    after_seq: u64,
    max_items: usize,
) -> Result<(), String> {
    if page.ledger_id != ledger_id || page.next_cursor.ledger_id != ledger_id {
        return Err("event page did not preserve the requested ledger identity".to_string());
    }
    if page.records.len() > max_items {
        return Err(format!(
            "event replay returned {} records for a {max_items}-record request",
            page.records.len()
        ));
    }
    let mut expected = after_seq.saturating_add(1);
    for record in &page.records {
        if record.seq != expected {
            return Err(format!(
                "event replay sequence {} did not match expected contiguous sequence {expected}",
                record.seq
            ));
        }
        expected = expected.saturating_add(1);
    }
    let output = page.records.last().map_or(after_seq, |record| record.seq);
    if page.next_cursor.after_seq != output {
        return Err(format!(
            "event replay next cursor {} did not match selected output {output}",
            page.next_cursor.after_seq
        ));
    }
    Ok(())
}

fn source_mapping_hash(request: &EvidenceIngestionActorRequest) -> String {
    let bytes = serde_json::to_vec(&(
        &request.config.config.source_mappings,
        &request.required_artifact_type_id,
        &request.subject,
        "docs_task_success_v1",
        0.0f64.to_bits(),
        0.2f64.to_bits(),
    ))
    .expect("validated belief mapping must serialize");
    blake3::hash(&bytes).to_hex().to_string()
}

fn push_mapping_issue(
    report: &mut EvidenceIngestionActorReport,
    item_id: Option<String>,
    error: DocsTaskEvidenceError,
) {
    let retryable = matches!(
        error,
        DocsTaskEvidenceError::Replay(_) | DocsTaskEvidenceError::Storage(_)
    );
    let target = if retryable {
        &mut report.retryable_errors
    } else {
        &mut report.fatal_errors
    };
    target.push(issue(item_id, "evidence_mapping_failed", error.to_string()));
}

fn push_replay_issue(report: &mut EvidenceIngestionActorReport, error: EventAuthorityError) {
    let retryable = matches!(
        error,
        EventAuthorityError::Backpressure { .. }
            | EventAuthorityError::Unavailable { .. }
            | EventAuthorityError::DurabilityIndeterminate { .. }
            | EventAuthorityError::Persistence { .. }
            | EventAuthorityError::Internal { .. }
    );
    let code = match &error {
        EventAuthorityError::RetentionGap { .. } => "retention_gap",
        EventAuthorityError::IdentityMismatch { .. }
        | EventAuthorityError::CorruptPersistedIdentity { .. } => "ledger_identity_mismatch",
        EventAuthorityError::Backpressure { .. } => "event_replay_backpressure",
        EventAuthorityError::Unavailable { .. } => "event_replay_unavailable",
        EventAuthorityError::DurabilityIndeterminate { .. } => "durability_indeterminate",
        EventAuthorityError::Persistence { .. } | EventAuthorityError::Internal { .. } => {
            "event_replay_failed"
        }
        _ => "event_replay_invalid",
    };
    let target = if retryable {
        &mut report.retryable_errors
    } else {
        &mut report.fatal_errors
    };
    target.push(issue(None, code, error.to_string()));
}

fn push_storage_issue(
    report: &mut EvidenceIngestionActorReport,
    item_id: Option<String>,
    error: StorageError,
) {
    let retryable = matches!(
        error,
        StorageError::Backpressure(_)
            | StorageError::Unavailable(_)
            | StorageError::DurabilityIndeterminate(_)
            | StorageError::IoError(_)
    );
    let code = match &error {
        StorageError::Backpressure(_) => "cursor_or_assessment_conflict",
        StorageError::Unavailable(_) => "storage_unavailable",
        StorageError::DurabilityIndeterminate(_) => "durability_indeterminate",
        StorageError::IoError(_) => "storage_io",
        _ => "evidence_ingestion_invalid",
    };
    let target = if retryable {
        &mut report.retryable_errors
    } else {
        &mut report.fatal_errors
    };
    target.push(issue(item_id, code, error.to_string()));
}

fn issue(
    item_id: Option<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) -> EvidenceIngestionIssue {
    EvidenceIngestionIssue {
        item_id,
        code: code.into(),
        message: message.into(),
    }
}
