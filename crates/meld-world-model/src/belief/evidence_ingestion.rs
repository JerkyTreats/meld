//! Bounded evidence ingestion actor over the canonical event ledger.
//!
//! Owner: world model belief domain. One step reads the durable cursor for
//! [`EVIDENCE_CONSUMER_ID`], replays a bounded batch of canonical records
//! after it through the domain-owned replay port, interprets each record
//! through the frozen [`OutcomeEvidenceMapping`] contract, and ingests
//! applicable evidence through the existing promoted-evidence path.
//!
//! Durability ordering invariant: the durable cursor is advanced only after
//! every absorbed record's domain state — evidence, assignments, committed
//! revisions, and invalid-record rejections — is flushed. Understood
//! non-applicable records and durably recorded invalid records may advance
//! the cursor. A record whose absorption fails stops the batch before it,
//! so the cursor never passes unabsorbed evidence; a reopen between
//! evidence commit and cursor commit replays idempotently because promoted
//! evidence identity is deterministic from the publication record and the
//! installed mapping.
//!
//! The request and report pair is domain-owned. Root runtime contracts
//! adapt this report behind their own bounded-step surface; this crate does
//! not depend on them.

use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::{
    CoverageTruncation, DurableConsumerCursor, EventPage, EventRecord, LedgerCursor,
    LedgerIdentity, ReplayRequest, MAX_REPLAY_LIMIT,
};

use crate::belief::config::{stable_hash_hex, ConfigSnapshot};
use crate::belief::contracts::{BranchScope, EvidenceRejection};
use crate::belief::ingestion::{ingest_promoted_evidence, PromotedEvidenceIngestionRequest};
use crate::belief::outcome::mapping::{
    OutcomeEvidenceMapping, OutcomeMappingDisposition, OutcomeMappingInput, EVIDENCE_CONSUMER_ID,
};
use crate::belief::registry::BeliefFamilyRegistry;
use crate::belief::runtime::BeliefRuntime;
use crate::belief::store::BeliefStore;
use crate::waiting::WaitingOnDeclaration;
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::PerspectiveKey;

/// Identity-bearing source of bounded canonical event pages for evidence.
///
/// The belief domain owns its replay requirement; product composition
/// supplies an adapter backed by the event authority without exposing event
/// storage to this domain.
pub trait EvidenceEventReplaySource: Send + Sync {
    /// Returns the ledger whose sequence space this source replays.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Replays one bounded page after an identity-bearing cursor.
    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError>;
}

/// Bounded evidence ingestion step request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceIngestionRequest {
    /// Maximum ledger records replayed and interpreted during one step.
    pub max_events: usize,
}

/// Diagnostic issue produced by one ingestion step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceIngestionIssue {
    /// Record sequence or evidence identity the issue concerns, when known.
    pub item_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Domain report from one bounded evidence ingestion step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceIngestionReport {
    /// Stable actor identifier.
    pub actor_id: String,
    /// Durable cursor position read before work.
    pub input_after_seq: u64,
    /// Durable cursor position after the step.
    pub output_after_seq: u64,
    /// Canonical records replayed this step.
    pub events_replayed: usize,
    /// Records mapped to promoted evidence.
    pub applicable_count: usize,
    /// Records understood as carrying no evidence.
    pub not_applicable_count: usize,
    /// Matched records whose content was rejected durably.
    pub invalid_count: usize,
    /// Evidence assignment edges newly inserted this step.
    pub new_assignment_count: usize,
    /// Belief revisions durably committed this step.
    pub revisions_committed: usize,
    /// Rejection ids recorded for invalid records this step.
    pub recorded_rejection_ids: Vec<String>,
    /// Retryable diagnostics observed during the step.
    pub retryable_errors: Vec<EvidenceIngestionIssue>,
    /// Fatal diagnostics observed during the step.
    pub fatal_errors: Vec<EvidenceIngestionIssue>,
    /// True when more replayable records remained beyond the budget.
    pub more_available: bool,
    /// What would make quiet or blocked ingestion eligible (DBG-016).
    ///
    /// Derived from the replay window and dispositions this step already
    /// computed; emission never gates or reorders ingestion.
    pub waiting_on: Vec<WaitingOnDeclaration>,
}

/// Bounded actor that discovers evidence work from the durable cursor.
pub struct EvidenceIngestionActor {
    actor_id: String,
    store: Arc<BeliefStore>,
    traversal: Arc<TraversalStore>,
    registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
    family_id: String,
    replay: Arc<dyn EvidenceEventReplaySource>,
    cursor: Arc<dyn DurableConsumerCursor + Send + Sync>,
    mapping: Arc<dyn OutcomeEvidenceMapping + Send + Sync>,
    mapping_id: String,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}

impl EvidenceIngestionActor {
    /// Bind the actor to durable stores, its ports, and configured scope.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_id: impl Into<String>,
        store: Arc<BeliefStore>,
        traversal: Arc<TraversalStore>,
        registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
        family_id: impl Into<String>,
        replay: Arc<dyn EvidenceEventReplaySource>,
        cursor: Arc<dyn DurableConsumerCursor + Send + Sync>,
        mapping: Arc<dyn OutcomeEvidenceMapping + Send + Sync>,
        mapping_id: impl Into<String>,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        Self {
            actor_id: actor_id.into(),
            store,
            traversal,
            registry,
            family_id: family_id.into(),
            replay,
            cursor,
            mapping,
            mapping_id: mapping_id.into(),
            perspective,
            branch_scope,
        }
    }

    /// Stable actor identity carried in reports and ingestion ownership.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Run one bounded ingestion step from the durable cursor.
    ///
    /// Sequencing: read the durable cursor, replay a bounded page after it,
    /// interpret record by record in ascending sequence order, absorb each
    /// disposition durably, then flush domain state and request cursor
    /// advancement through the highest contiguously absorbed sequence.
    pub fn bounded_step(&mut self, request: &EvidenceIngestionRequest) -> EvidenceIngestionReport {
        let mut report = EvidenceIngestionReport {
            actor_id: self.actor_id.clone(),
            input_after_seq: 0,
            output_after_seq: 0,
            events_replayed: 0,
            applicable_count: 0,
            not_applicable_count: 0,
            invalid_count: 0,
            new_assignment_count: 0,
            revisions_committed: 0,
            recorded_rejection_ids: Vec::new(),
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            more_available: false,
            waiting_on: Vec::new(),
        };
        if request.max_events == 0 {
            report.fatal_errors.push(issue(
                None,
                "invalid_budget",
                "ingestion budget must be greater than zero",
            ));
            return report;
        }

        let ledger_id = self.replay.ledger_identity();
        let after_seq = match self.cursor.consumer_cursor(EVIDENCE_CONSUMER_ID) {
            Ok(Some(state)) => {
                if state.ledger_id != ledger_id {
                    report.fatal_errors.push(issue(
                        None,
                        "cursor_ledger_mismatch",
                        &format!(
                            "durable cursor is bound to ledger {} but replay serves {}",
                            state.ledger_id, ledger_id
                        ),
                    ));
                    return report;
                }
                state.after_seq
            }
            Ok(None) => 0,
            Err(error) => {
                let target = if error.retryable {
                    &mut report.retryable_errors
                } else {
                    &mut report.fatal_errors
                };
                target.push(issue(None, "cursor_read_failed", &error.message));
                return report;
            }
        };
        report.input_after_seq = after_seq;
        report.output_after_seq = after_seq;

        let page = match self.replay.replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id,
                after_seq,
            },
            limit: request.max_events.min(MAX_REPLAY_LIMIT),
        }) {
            Ok(page) => page,
            Err(error) => {
                report
                    .retryable_errors
                    .push(issue(None, "replay_failed", &error.to_string()));
                return report;
            }
        };
        report.events_replayed = page.records.len();
        report.more_available = matches!(
            page.coverage.truncation,
            CoverageTruncation::After | CoverageTruncation::Both
        );
        if page.records.is_empty() {
            return report;
        }

        // Resolved per step, like the assessment actor, so ingestion always
        // runs under the currently installed theory revision. Without an
        // installed family the batch is left untouched: advancing the
        // cursor here would permanently skip applicable evidence.
        let revision = match self.registry.current(&self.family_id) {
            Ok(Some(revision)) => revision,
            Ok(None) => {
                report.retryable_errors.push(issue(
                    Some(self.family_id.clone()),
                    "family_not_installed",
                    "no current registry revision for configured family",
                ));
                return report;
            }
            Err(error) => {
                report.fatal_errors.push(issue(
                    Some(self.family_id.clone()),
                    "registry_read_failed",
                    &error.to_string(),
                ));
                return report;
            }
        };
        let snapshot = ConfigSnapshot {
            config: revision.config.clone(),
            hash: revision.content_hash.clone(),
        };
        let runtime = BeliefRuntime::from_family_revision(
            Arc::clone(&self.store),
            Arc::clone(&self.traversal),
            &revision,
            self.perspective.clone(),
            self.branch_scope.clone(),
        );

        let mut durable_through = after_seq;
        for record in page.records {
            let seq = record.seq;
            if seq <= durable_through {
                // Replay pages are ascending; a stale or duplicate sequence
                // is already covered by the cursor.
                continue;
            }
            let disposition = self.mapping.map_outcome(&OutcomeMappingInput {
                record: record.clone(),
                mapping_id: self.mapping_id.clone(),
            });
            match disposition {
                OutcomeMappingDisposition::Applicable {
                    evidence_id,
                    record: promoted,
                } => {
                    report.applicable_count += 1;
                    // The ingest path dedupes on evidence identity derived
                    // from the disposition's frozen evidence_id (carried as
                    // the promoted source_id), so replaying an already
                    // absorbed record inserts nothing new.
                    match ingest_promoted_evidence(
                        self.store.as_ref(),
                        &runtime,
                        PromotedEvidenceIngestionRequest {
                            record: *promoted,
                            config: snapshot.clone(),
                            perspective: self.perspective.clone(),
                            branch_scope: self.branch_scope.clone(),
                            owner_id: &self.actor_id,
                        },
                    ) {
                        Ok(result) => {
                            report.new_assignment_count += result.new_assignment_count;
                            report.revisions_committed += result.committed.len();
                            if result.rejected {
                                // Configuration rejected the promoted record;
                                // the rejection is already durable, so the
                                // cursor may pass this record.
                                report.invalid_count += 1;
                            }
                            durable_through = seq;
                        }
                        Err(error) => {
                            // Absorption is indeterminate, so the cursor must
                            // stop before this record; replay is idempotent.
                            report.retryable_errors.push(issue(
                                Some(evidence_id),
                                "ingestion_failed",
                                &error.to_string(),
                            ));
                            break;
                        }
                    }
                }
                OutcomeMappingDisposition::NotApplicable { .. } => {
                    report.not_applicable_count += 1;
                    durable_through = seq;
                }
                OutcomeMappingDisposition::Invalid { reason } => {
                    let rejection = invalid_outcome_rejection(&record, &self.mapping_id, &reason);
                    match self.store.put_rejection(&rejection) {
                        Ok(()) => {
                            report.invalid_count += 1;
                            report.recorded_rejection_ids.push(rejection.rejection_id);
                            durable_through = seq;
                        }
                        Err(error) => {
                            report.retryable_errors.push(issue(
                                Some(rejection.rejection_id),
                                "rejection_persist_failed",
                                &error.to_string(),
                            ));
                            break;
                        }
                    }
                }
            }
        }

        if durable_through > after_seq {
            // Domain state durable before cursor advance, always. The flush
            // covers rejections and any ingestion writes not yet flushed by
            // the belief runtime's own commits.
            if let Err(error) = self.store.flush() {
                report
                    .retryable_errors
                    .push(issue(None, "flush_failed", &error.to_string()));
                return report;
            }
            match self
                .cursor
                .advance_consumer_cursor(EVIDENCE_CONSUMER_ID, durable_through)
            {
                Ok(state) => report.output_after_seq = state.after_seq,
                Err(error) => {
                    // Evidence is already durable; a re-run replays the same
                    // window idempotently and retries the advancement.
                    report.retryable_errors.push(issue(
                        None,
                        "cursor_advance_failed",
                        &error.message,
                    ));
                }
            }
        }
        // The hardened DBG-016 rule: a step that absorbed nothing states
        // what would change that. A quiet ledger waits on new committed
        // events; a window where every record fell outside the installed
        // mapping waits on a vocabulary intersection — the survey's
        // publisher-to-mapping mismatch surfaces exactly here.
        if report.events_replayed == 0 {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                "ledger_quiet_past_cursor",
                format!("no committed events past cursor {}", report.input_after_seq),
            ));
        } else if report.applicable_count == 0 && report.invalid_count == 0 {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                "no_mappable_events",
                format!(
                    "{} replayed records matched no source mapping of '{}'",
                    report.events_replayed, self.mapping_id
                ),
            ));
        }
        report
    }
}

/// Durable audit rejection for one invalid mapped record.
///
/// The identity is deterministic over the publication identity, mapping
/// identity, and reason so reopen replay re-records the same rejection
/// instead of accumulating duplicates.
fn invalid_outcome_rejection(
    record: &EventRecord,
    mapping_id: &str,
    reason: &str,
) -> EvidenceRejection {
    let source_id = record
        .envelope()
        .record_id
        .clone()
        .unwrap_or_else(|| format!("event-spine::{}", record.seq));
    EvidenceRejection {
        rejection_id: format!(
            "rejection-{}",
            stable_hash_hex(format!("outcome::{source_id}::{mapping_id}::{reason}").as_bytes())
        ),
        source_id,
        reason: reason.to_string(),
        source_cursor_start: record.seq,
        source_cursor_end: record.seq,
    }
}

fn issue(item_id: Option<String>, code: &str, message: &str) -> EvidenceIngestionIssue {
    EvidenceIngestionIssue {
        item_id,
        code: code.to_string(),
        message: message.to_string(),
    }
}
