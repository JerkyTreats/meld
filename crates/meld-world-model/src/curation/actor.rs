use std::sync::Arc;

use crate::curation::{
    expected_cut_owners, qualifications, stable_identity, CurationAcceptanceRecord,
    CurationAdmissionDecision, CurationAuthority, CurationEventPort, CurationOperation,
    CurationPublicationKind, CurationPublicationReceipt, CurationResult, CurationStepReport,
    CurationStore, CurationTerminalDisposition, CurationTraversalPort,
    StandingCurationRuleRevision, CURATION_CUT_INCOMPLETE, CURATION_OWNER_ID,
    CURATION_PUBLICATION_PENDING, CURATION_SELECTION_UNCHANGED,
};
use crate::error::StorageError;
use crate::waiting::WaitingOnDeclaration;
use crate::world_state::graph::contracts::{
    HydrationReference, OwnerCompletenessReceipt, OwnerCompletenessStatus, OwnerCurrentnessPolicy,
    OwnerObjectPublication, OwnerPublicationBatch, OwnerPublicationOperation,
    OwnerPublicationState, OwnerRelationOccurrence, TraversalCutRequest, TraversalCutStatus,
    TraversalResult,
};
use crate::world_state::graph::events::owner_publication_envelope;

/// One concrete standing Curation participant.
pub struct StandingCurationActor {
    actor_id: String,
    session_id: String,
    authority: CurationAuthority,
    rule: StandingCurationRuleRevision,
    store: Arc<CurationStore>,
    traversal: Arc<dyn CurationTraversalPort>,
    events: Arc<dyn CurationEventPort>,
}

impl StandingCurationActor {
    pub fn new(
        actor_id: impl Into<String>,
        session_id: impl Into<String>,
        authority: CurationAuthority,
        rule: StandingCurationRuleRevision,
        store: Arc<CurationStore>,
        traversal: Arc<dyn CurationTraversalPort>,
        events: Arc<dyn CurationEventPort>,
    ) -> Result<Self, StorageError> {
        authority.validate()?;
        rule.validate()?;
        let actor_id = actor_id.into();
        let session_id = session_id.into();
        if actor_id.trim().is_empty() || session_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "standing Curation actor and session ids must be non-empty".to_string(),
            ));
        }
        Ok(Self {
            actor_id,
            session_id,
            authority,
            rule,
            store,
            traversal,
            events,
        })
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Run at most one standing operation from the current durable Event tip.
    pub fn bounded_step(&self, max_items: usize) -> CurationStepReport {
        let mut report = CurationStepReport::new(&self.actor_id);
        if max_items == 0 {
            report
                .fatal_errors
                .push("standing Curation budget must be greater than zero".to_string());
            return report;
        }
        let watermark = match self.events.watermark() {
            Ok(watermark) => watermark,
            Err(error) => {
                report.retryable_errors.push(error);
                return report;
            }
        };
        report.input_after_seq = watermark.committed_seq;
        report.output_after_seq = watermark.committed_seq;
        let candidate = match self.store.next_planned_operation(&self.authority.agent_id) {
            Ok(Some(operation)) => operation,
            Ok(None) => {
                let cut_request = TraversalCutRequest {
                    owners: expected_cut_owners(&self.rule),
                    scope: self.rule.rule.scope.clone(),
                    currentness: OwnerCurrentnessPolicy::LatestComplete,
                    event_position: crate::events::LedgerCursor {
                        ledger_id: watermark.ledger_id,
                        after_seq: watermark.committed_seq,
                    },
                };
                let cut = match self.traversal.cut(&cut_request) {
                    Ok(cut) => cut,
                    Err(error) => {
                        report.retryable_errors.push(error.to_string());
                        return report;
                    }
                };
                if cut.status != TraversalCutStatus::Complete {
                    report.waiting_on.push(WaitingOnDeclaration::broad(
                        CURATION_CUT_INCOMPLETE,
                        format!("Curation awaits a complete cut: {:?}", cut.issues),
                    ));
                    return report;
                }
                match CurationOperation::reconstruct(
                    self.authority.clone(),
                    self.rule.revision_ref(),
                    cut,
                    self.rule.rule.traversal_request(),
                ) {
                    Ok(operation) => operation,
                    Err(error) => {
                        report.fatal_errors.push(error.to_string());
                        return report;
                    }
                }
            }
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        report.operations_attempted = 1;
        let operation = match self.store.operation_for_selection(&candidate.selection_id) {
            Ok(Some(operation)) => match candidate.planned_authorization.clone() {
                Some(authorization) => match operation.with_planned_authorization(authorization) {
                    Ok(operation) => operation,
                    Err(error) => {
                        report.fatal_errors.push(error.to_string());
                        return report;
                    }
                },
                None => operation,
            },
            Ok(None) => candidate,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        if let Err(error) = self.store.put_operation(&operation) {
            report.fatal_errors.push(error.to_string());
            return report;
        }
        let acceptance = match CurationAcceptanceRecord::for_operation(&operation, &self.rule) {
            Ok(acceptance) => acceptance,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        match self.store.acceptance(&acceptance.acceptance_id) {
            Ok(Some(existing)) if existing == acceptance => {}
            Ok(Some(_)) => {
                report.fatal_errors.push(
                    "standing Curation acceptance identity has divergent content".to_string(),
                );
                return report;
            }
            Ok(None) => {
                if let Err(error) = self.store.put_acceptance(&acceptance) {
                    report.fatal_errors.push(error.to_string());
                    return report;
                }
                report.acceptances_persisted = 1;
            }
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        }
        if acceptance.decision == CurationAdmissionDecision::Rejected {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                CURATION_SELECTION_UNCHANGED,
                "rejected standing selection waits on changed authority, rule, or owner input",
            ));
            return report;
        }

        let result = match self.store.result_for_operation(&operation.operation_id) {
            Ok(Some(result)) => {
                report.reused_results = 1;
                result
            }
            Ok(None) => match self.execute(&operation) {
                Ok(result) => match self.store.put_result(&result) {
                    Ok(result) => {
                        report.results_persisted = 1;
                        result
                    }
                    Err(error) => {
                        report.fatal_errors.push(error.to_string());
                        return report;
                    }
                },
                Err(error) => {
                    let failed = match CurationResult::new(
                        &operation,
                        CurationTerminalDisposition::Failed,
                        "bounded traversal failed after admission",
                        None,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![error.to_string()],
                    ) {
                        Ok(result) => result,
                        Err(result_error) => {
                            report.fatal_errors.push(result_error.to_string());
                            return report;
                        }
                    };
                    match self.store.put_result(&failed) {
                        Ok(result) => {
                            report.results_persisted = 1;
                            result
                        }
                        Err(store_error) => {
                            report.fatal_errors.push(store_error.to_string());
                            return report;
                        }
                    }
                }
            },
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        self.publish(&result, &mut report);
        if report.reused_results == 1
            && report.publications_appended == 0
            && report.retryable_errors.is_empty()
            && report.fatal_errors.is_empty()
        {
            report.waiting_on.push(WaitingOnDeclaration::broad(
                CURATION_SELECTION_UNCHANGED,
                format!(
                    "standing selection '{}' is already terminal and fully published",
                    operation.selection_id
                ),
            ));
        }
        report
    }

    fn execute(&self, operation: &CurationOperation) -> Result<CurationResult, StorageError> {
        let traversal = self
            .traversal
            .traverse(&operation.source_cut, &operation.traversal_request)?;
        if traversal.truncation.is_truncated() || !traversal.frontier.is_empty() {
            let failures = truncation_reasons(&traversal);
            let exclusions = source_exclusions(&traversal);
            return CurationResult::new(
                operation,
                CurationTerminalDisposition::Incomplete,
                "bounded traversal did not cover the declared source",
                None,
                Vec::new(),
                traversal.frontier,
                exclusions,
                failures,
            );
        }
        let expected = self.rule.rule.expected_object()?;
        let root = self.rule.rule.roots[0].clone();
        let object_id = stable_identity(
            "curation-object-publication-v1",
            &(
                &self.rule.content_hash,
                &expected,
                &operation.authority.perspective,
            ),
        )?;
        let occurrence_id = stable_identity(
            "curation-relation-occurrence-v1",
            &(
                &self.rule.content_hash,
                &self.rule.rule.relation_type,
                &expected,
                &root,
                &operation.authority.perspective,
            ),
        )?;
        if semantic_state_exists(&traversal, &object_id, &occurrence_id) {
            return CurationResult::new(
                operation,
                CurationTerminalDisposition::Unchanged,
                "installed Curation-owned state already holds",
                None,
                vec![object_id, occurrence_id],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
        }

        let result_id = stable_identity(
            "standing-curation-result-v1",
            &(
                &operation.operation_id,
                CurationTerminalDisposition::Applied,
            ),
        )?;
        let hydration = HydrationReference {
            owner_id: CURATION_OWNER_ID.to_string(),
            product_kind: "standing_curation_result".to_string(),
            product_id: result_id.clone(),
            revision_id: result_id.clone(),
            role: "authored_semantic_product".to_string(),
        };
        let qualifications = qualifications(operation, &self.rule);
        let batch = OwnerPublicationBatch {
            owner_id: CURATION_OWNER_ID.to_string(),
            revision_id: result_id,
            scope: self.rule.rule.scope.clone(),
            objects: vec![OwnerObjectPublication {
                publication_id: object_id.clone(),
                object_ref: expected.clone(),
                state: OwnerPublicationState::Observed,
                source_product_ref: operation.operation_id.clone(),
                hydration: hydration.clone(),
                provenance_refs: source_provenance(&traversal),
                qualifications: qualifications.clone(),
            }],
            relations: vec![OwnerRelationOccurrence {
                occurrence_id: occurrence_id.clone(),
                relation_type: self.rule.rule.relation_type.clone(),
                src: expected,
                dst: root,
                source_product_ref: operation.operation_id.clone(),
                hydration,
                qualifications,
                provenance_refs: source_provenance(&traversal),
            }],
            completeness: OwnerCompletenessReceipt {
                receipt_id: stable_identity(
                    "curation-completeness-v1",
                    &(&operation.operation_id, &object_id, &occurrence_id),
                )?,
                scope: self.rule.rule.scope.clone(),
                included_ids: vec![object_id.clone(), occurrence_id.clone()],
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        };
        let publication = OwnerPublicationOperation::reconstruct(
            self.rule.rule.output_policy_revision.clone(),
            batch,
        )?;
        CurationResult::new(
            operation,
            CurationTerminalDisposition::Applied,
            "installed Curation rule authored expected state",
            Some(publication),
            vec![object_id, occurrence_id],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    fn publish(&self, result: &CurationResult, report: &mut CurationStepReport) {
        if let Some(publication) = &result.semantic_publication {
            let semantic_receipted =
                self.receipt_exists(result, CurationPublicationKind::Semantic, report);
            if !report.fatal_errors.is_empty() {
                return;
            }
            if !semantic_receipted {
                match owner_publication_envelope(&self.session_id, publication)
                    .map_err(|error| error.to_string())
                    .and_then(|envelope| self.events.append_idempotent(envelope))
                {
                    Ok(receipt) => {
                        let appended_seq = receipt.seq;
                        match self.record_receipt(
                            result,
                            CurationPublicationKind::Semantic,
                            publication.event_record_id(),
                            receipt,
                        ) {
                            Ok(()) => {
                                report.publications_appended += 1;
                                report.output_after_seq = report.output_after_seq.max(appended_seq);
                            }
                            Err(error) => report.fatal_errors.push(error.to_string()),
                        }
                    }
                    Err(error) => {
                        publication_pending(report, error);
                        return;
                    }
                }
                if !report.fatal_errors.is_empty() {
                    return;
                }
            }
        }
        let terminal_receipted =
            self.receipt_exists(result, CurationPublicationKind::Terminal, report);
        if !report.fatal_errors.is_empty() {
            return;
        }
        if !terminal_receipted {
            match result
                .event_envelope(&self.session_id)
                .map_err(|error| error.to_string())
                .and_then(|envelope| self.events.append_idempotent(envelope))
            {
                Ok(receipt) => {
                    let appended_seq = receipt.seq;
                    match self.record_receipt(
                        result,
                        CurationPublicationKind::Terminal,
                        result.event_record_id(),
                        receipt,
                    ) {
                        Ok(()) => {
                            report.publications_appended += 1;
                            report.output_after_seq = report.output_after_seq.max(appended_seq);
                        }
                        Err(error) => report.fatal_errors.push(error.to_string()),
                    }
                }
                Err(error) => publication_pending(report, error),
            }
        }
    }

    fn receipt_exists(
        &self,
        result: &CurationResult,
        kind: CurationPublicationKind,
        report: &mut CurationStepReport,
    ) -> bool {
        match self.store.publication_receipt(&result.result_id, kind) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                true
            }
        }
    }

    fn record_receipt(
        &self,
        result: &CurationResult,
        kind: CurationPublicationKind,
        event_record_id: String,
        receipt: crate::events::AppendReceipt,
    ) -> Result<(), StorageError> {
        self.store
            .put_publication_receipt(&CurationPublicationReceipt {
                result_id: result.result_id.clone(),
                kind,
                event_record_id,
                event_position: crate::events::LedgerCursor {
                    ledger_id: receipt.ledger_id,
                    after_seq: receipt.seq,
                },
            })
    }
}

fn truncation_reasons(traversal: &TraversalResult) -> Vec<String> {
    [
        (traversal.truncation.depth, "depth bound reached"),
        (traversal.truncation.objects, "object bound reached"),
        (traversal.truncation.occurrences, "occurrence bound reached"),
        (traversal.truncation.paths, "path bound reached"),
    ]
    .into_iter()
    .filter(|(reached, _)| *reached)
    .map(|(_, reason)| reason.to_string())
    .collect()
}

fn publication_pending(report: &mut CurationStepReport, error: String) {
    report.retryable_errors.push(error.clone());
    report.waiting_on.push(WaitingOnDeclaration::broad(
        CURATION_PUBLICATION_PENDING,
        format!("standing Curation publication remains pending: {error}"),
    ));
}

fn semantic_state_exists(
    traversal: &TraversalResult,
    object_id: &str,
    occurrence_id: &str,
) -> bool {
    traversal
        .objects
        .iter()
        .any(|object| object.publication_id == object_id)
        && traversal
            .occurrences
            .iter()
            .any(|occurrence| occurrence.occurrence_id == occurrence_id)
}

fn source_provenance(traversal: &TraversalResult) -> Vec<String> {
    traversal
        .receipts
        .iter()
        .map(|receipt| {
            format!(
                "{}::{}::{}",
                receipt.source_event.ledger_id, receipt.source_event.seq, receipt.revision_id
            )
        })
        .collect()
}

fn source_exclusions(
    traversal: &TraversalResult,
) -> Vec<crate::world_state::graph::contracts::OwnerPublicationExclusion> {
    let mut exclusions = traversal
        .receipts
        .iter()
        .flat_map(|receipt| receipt.completeness.exclusions.iter().cloned())
        .collect::<Vec<_>>();
    exclusions.sort();
    exclusions.dedup();
    exclusions
}
