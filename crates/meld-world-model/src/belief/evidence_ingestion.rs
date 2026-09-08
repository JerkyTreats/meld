//! Bounded evidence ingestion actor over the canonical event ledger.
//!
//! Owner: world model belief domain. One step reads the durable cursor for
//! its exact installed family and mapping selection, replays canonical records
//! after it through the domain-owned replay port, interprets each record
//! through the frozen [`OutcomeEvidenceMapping`] contract, and ingests
//! applicable evidence through every matching installed family. Historical
//! unpinned callers retain the [`EVIDENCE_CONSUMER_ID`] cursor.
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
use crate::waiting::{conditions, StructuralWakeAddress, WaitingOnDeclaration};
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
    registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
    family_id: String,
    replay: Arc<dyn EvidenceEventReplaySource>,
    cursor: Arc<dyn DurableConsumerCursor + Send + Sync>,
    mapping: Arc<dyn OutcomeEvidenceMapping + Send + Sync>,
    mapping_id: String,
    mapping_revision: Option<crate::belief::TheoryRevisionRef>,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
    family_revisions: Option<Vec<crate::belief::BeliefFamilyRevision>>,
    lifecycle: crate::lifecycle::NativeLifecycle,
    work_lock: parking_lot::Mutex<()>,
}

impl EvidenceIngestionActor {
    /// Bind the actor to durable stores, its ports, and configured scope.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_id: impl Into<String>,
        store: Arc<BeliefStore>,
        registry: Arc<dyn BeliefFamilyRegistry + Send + Sync>,
        family_id: impl Into<String>,
        replay: Arc<dyn EvidenceEventReplaySource>,
        cursor: Arc<dyn DurableConsumerCursor + Send + Sync>,
        mapping: Arc<dyn OutcomeEvidenceMapping + Send + Sync>,
        mapping_id: impl Into<String>,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        let actor_id = actor_id.into();
        Self {
            lifecycle: crate::lifecycle::NativeLifecycle::new(actor_id.clone()),
            work_lock: parking_lot::Mutex::new(()),
            actor_id,
            store,
            registry,
            family_id: family_id.into(),
            replay,
            cursor,
            mapping,
            mapping_id: mapping_id.into(),
            mapping_revision: None,
            perspective,
            branch_scope,
            family_revisions: None,
        }
    }

    /// Stamp exact mapping lineage on promoted evidence and rejections.
    pub fn with_mapping_revision(mut self, revision: crate::belief::TheoryRevisionRef) -> Self {
        self.mapping_revision = Some(revision);
        self
    }

    /// Freeze the exact belief-family revision for this actor lifetime.
    pub fn with_family_revision(self, revision: crate::belief::BeliefFamilyRevision) -> Self {
        self.with_pinned_families(vec![revision])
    }

    /// Freeze every family consumed before this actor advances its shared cursor.
    pub fn with_pinned_families(
        mut self,
        mut revisions: Vec<crate::belief::BeliefFamilyRevision>,
    ) -> Self {
        revisions.sort_by(|left, right| left.family_id.cmp(&right.family_id));
        self.family_revisions = Some(revisions);
        self
    }

    /// Exact installed semantics own their replay position. A changed selection
    /// must revisit retained publications rather than inherit an unrelated cursor.
    pub fn consumer_id(&self) -> String {
        match &self.family_revisions {
            Some(families) => format!(
                "{EVIDENCE_CONSUMER_ID}::{}",
                stable_hash_hex(
                    &serde_json::to_vec(&(
                        families
                            .iter()
                            .map(|family| family.revision_ref())
                            .collect::<Vec<_>>(),
                        &self.mapping_id,
                        &self.mapping_revision,
                        &self.perspective,
                        &self.branch_scope
                    ))
                    .expect("installed evidence selection serializes")
                )
            ),
            None => EVIDENCE_CONSUMER_ID.into(),
        }
    }

    fn resolve_families(&self) -> Result<Vec<crate::belief::BeliefFamilyRevision>, String> {
        if let Some(families) = &self.family_revisions {
            if families.is_empty()
                || families
                    .windows(2)
                    .any(|pair| pair[0].family_id == pair[1].family_id)
            {
                return Err("evidence requires distinct installed families".into());
            }
            return Ok(families.clone());
        }
        self.registry
            .current(&self.family_id)
            .map_err(|error| error.to_string())?
            .map(|family| vec![family])
            .ok_or_else(|| format!("Evidence family {} is not installed", self.family_id))
    }

    /// Stable actor identity carried in reports and ingestion ownership.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Return the exact durable ledger position used by lifecycle recovery.
    pub fn lifecycle_cursor(&self) -> Result<LedgerCursor, String> {
        let ledger_id = self.replay.ledger_identity();
        match self
            .cursor
            .consumer_cursor(&self.consumer_id())
            .map_err(|error| error.message)?
        {
            Some(cursor) if cursor.ledger_id == ledger_id => Ok(LedgerCursor {
                ledger_id,
                after_seq: cursor.after_seq,
            }),
            Some(cursor) => Err(format!(
                "durable evidence cursor is bound to ledger {} but replay serves {}",
                cursor.ledger_id, ledger_id
            )),
            None => Ok(LedgerCursor {
                ledger_id,
                after_seq: 0,
            }),
        }
    }

    /// Resolve a wake against this ingestion instance's actual Event source.
    pub fn resolves_wake(&self, wake: &StructuralWakeAddress) -> Result<bool, String> {
        // Installed mappings are frozen for this instance. A new mapping needs
        // replacement activation; only the live replay source advances here.
        Ok(matches!(wake, StructuralWakeAddress::EventPosition(value)
            if crate::waiting::after_position(value, &format!("event-ledger::{}", self.replay.ledger_identity()))))
    }

    /// Read lifecycle evidence from this owner's bound stores and installed inputs.
    pub fn lifecycle_evidence(&self) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let _guard = self.work_lock.lock();
        self.lifecycle_evidence_inner()
    }

    fn lifecycle_evidence_inner(
        &self,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let cursor = self.lifecycle_cursor()?;
        let families = self.resolve_families()?;
        let mapping = self
            .mapping_revision
            .as_ref()
            .ok_or("Evidence mapping has no installed revision")?;
        self.store.flush().map_err(|error| error.to_string())?;
        let checkpoint_ref = format!(
            "evidence-consumer::{}::{}",
            cursor.ledger_id, cursor.after_seq
        );
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: families
                .iter()
                .map(|family| {
                    crate::lifecycle::evidence_ref("belief-family", &family.revision_ref())
                })
                .chain(std::iter::once(crate::lifecycle::evidence_ref(
                    "evidence-mapping",
                    mapping,
                )))
                .collect::<Result<Vec<_>, _>>()?,
            binding_refs: vec![format!("evidence-ledger::{}", cursor.ledger_id)],
            subscription_refs: vec![format!(
                "evidence-consumer::{}::{}",
                cursor.ledger_id,
                self.consumer_id()
            )],
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: format!(
                "evidence-replay::{}::after::{}",
                cursor.ledger_id, cursor.after_seq
            ),
        })
    }

    /// Author native start evidence with bounded work excluded.
    pub fn lifecycle_start(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .start(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native safe point evidence with bounded work excluded.
    pub fn lifecycle_safe_point(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .safe_point(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native stop evidence with bounded work excluded.
    pub fn lifecycle_stop(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .stop(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native release evidence with bounded work excluded.
    pub fn lifecycle_release(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .release(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Run one bounded ingestion step from the durable cursor.
    ///
    /// Sequencing: read the durable cursor, replay a bounded page after it,
    /// interpret record by record in ascending sequence order, absorb each
    /// disposition durably, then flush domain state and request cursor
    /// advancement through the highest contiguously absorbed sequence.
    pub fn bounded_step(&mut self, request: &EvidenceIngestionRequest) -> EvidenceIngestionReport {
        let _guard = self.work_lock.lock();
        let mut report = self.bounded_step_inner(request);
        crate::waiting::bind_waits(&mut report.waiting_on, self.store.resource_id());
        report
    }

    fn bounded_step_inner(&self, request: &EvidenceIngestionRequest) -> EvidenceIngestionReport {
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
        let after_seq = match self.cursor.consumer_cursor(&self.consumer_id()) {
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
        if page.coverage.retained_from > after_seq.saturating_add(1) {
            report.fatal_errors.push(issue(
                None,
                "replay_history_unavailable",
                "retained Event history does not cover this evidence selection's cursor",
            ));
            return report;
        }
        report.events_replayed = page.records.len();
        report.more_available = matches!(
            page.coverage.truncation,
            CoverageTruncation::After | CoverageTruncation::Both
        );
        if page.records.is_empty() {
            // The hardened DBG-016 rule applies to the quiet path too: an
            // empty page is a step that absorbed nothing, and it states
            // what would change that before returning.
            report.waiting_on.push(WaitingOnDeclaration::broad(
                conditions::LEDGER_QUIET_PAST_CURSOR,
                format!("no committed events past cursor {}", report.input_after_seq),
                vec![StructuralWakeAddress::EventPosition(format!(
                    "event-ledger::{}::after::{}",
                    self.replay.ledger_identity(),
                    report.input_after_seq
                ))],
            ));
            return report;
        }

        // No record may pass the cursor before every selected family resolves.
        let families = match self.resolve_families() {
            Ok(families) => families,
            Err(error) => {
                report
                    .retryable_errors
                    .push(issue(None, "family_not_installed", &error));
                return report;
            }
        };
        let runtimes: Vec<_> = families
            .iter()
            .map(|revision| {
                (
                    ConfigSnapshot {
                        config: revision.config.clone(),
                        hash: revision.content_hash.clone(),
                    },
                    BeliefRuntime::from_family_revision(
                        Arc::clone(&self.store),
                        revision,
                        self.perspective.clone(),
                        self.branch_scope.clone(),
                    ),
                )
            })
            .collect();

        let mut durable_through = after_seq;
        'records: for record in page.records {
            let seq = record.seq;
            if seq <= durable_through {
                // Replay pages are ascending; a stale or duplicate sequence
                // is already covered by the cursor.
                continue;
            }
            let disposition = self.mapping.map_outcome(&OutcomeMappingInput {
                record: record.clone(),
                mapping_id: self.mapping_id.clone(),
                mapping_revision: self.mapping_revision.clone(),
            });
            match disposition {
                OutcomeMappingDisposition::Applicable {
                    evidence_id,
                    record: promoted,
                } => {
                    report.applicable_count += 1;
                    let targets: Vec<_> = runtimes
                        .iter()
                        .filter(|(snapshot, _)| {
                            snapshot
                                .config
                                .source_mappings
                                .iter()
                                .any(|mapping| mapping.source_kind == promoted.source_kind)
                        })
                        .collect();
                    if targets.is_empty() {
                        let rejection = invalid_outcome_rejection(
                            &record,
                            &self.mapping_id,
                            self.mapping_revision.clone(),
                            "missing promoted source mapping in installed families",
                        );
                        if let Err(error) = self.store.put_rejection(&rejection) {
                            report.retryable_errors.push(issue(
                                Some(rejection.rejection_id),
                                "rejection_persist_failed",
                                &error.to_string(),
                            ));
                            break;
                        }
                        report.invalid_count += 1;
                        report.recorded_rejection_ids.push(rejection.rejection_id);
                    }
                    let mut rejected = false;
                    for (snapshot, runtime) in targets {
                        match ingest_promoted_evidence(
                            self.store.as_ref(),
                            runtime,
                            PromotedEvidenceIngestionRequest {
                                record: (*promoted).clone(),
                                config: snapshot.clone(),
                                perspective: self.perspective.clone(),
                                branch_scope: self.branch_scope.clone(),
                                owner_id: &self.actor_id,
                            },
                        ) {
                            Ok(result) => {
                                report.new_assignment_count += result.new_assignment_count;
                                report.revisions_committed += result.committed.len();
                                rejected |= result.rejected;
                            }
                            Err(error) => {
                                report.retryable_errors.push(issue(
                                    Some(evidence_id.clone()),
                                    "ingestion_failed",
                                    &error.to_string(),
                                ));
                                break 'records;
                            }
                        }
                    }
                    report.invalid_count += usize::from(rejected);
                    durable_through = seq;
                }
                OutcomeMappingDisposition::NotApplicable { .. } => {
                    report.not_applicable_count += 1;
                    durable_through = seq;
                }
                OutcomeMappingDisposition::Invalid { reason } => {
                    let rejection = invalid_outcome_rejection(
                        &record,
                        &self.mapping_id,
                        self.mapping_revision.clone(),
                        &reason,
                    );
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
                .advance_consumer_cursor(&self.consumer_id(), durable_through)
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
        // what would change that. The quiet-ledger declaration lands on the
        // empty-page return above; a window where every record fell outside
        // the installed mapping waits on a vocabulary intersection — the
        // survey's publisher-to-mapping mismatch surfaces exactly here.
        if report.applicable_count == 0 && report.invalid_count == 0 {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                conditions::NO_MAPPABLE_EVENTS,
                format!(
                    "{} replayed records matched no source mapping of '{}'",
                    report.events_replayed, self.mapping_id
                ),
                vec![StructuralWakeAddress::EventPosition(format!(
                    "event-ledger::{}::after::{}",
                    self.replay.ledger_identity(),
                    report.output_after_seq
                ))],
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
    mapping_revision: Option<crate::belief::TheoryRevisionRef>,
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
        outcome_mapping_revision: mapping_revision.map(Box::new),
    }
}

fn issue(item_id: Option<String>, code: &str, message: &str) -> EvidenceIngestionIssue {
    EvidenceIngestionIssue {
        item_id,
        code: code.to_string(),
        message: message.to_string(),
    }
}
