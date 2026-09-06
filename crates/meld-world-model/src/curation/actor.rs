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
use crate::waiting::{StructuralWakeAddress, WaitingOnDeclaration};
use crate::world_state::graph::contracts::{
    HydrationReference, OwnerCompletenessReceipt, OwnerCompletenessStatus, OwnerCurrentnessPolicy,
    OwnerObjectPublication, OwnerPublicationBatch, OwnerPublicationOperation,
    OwnerPublicationState, OwnerRelationOccurrence, TraversalCutRequest, TraversalCutStatus,
    TraversalResult,
};
use crate::world_state::graph::events::owner_publication_envelope;

/// Current authority supplied through the structural admission boundary.
pub trait CurationAuthorityPort: Send + Sync {
    fn observe(&self) -> Result<Option<CurationAuthority>, StorageError>;
}

/// One concrete standing Curation participant.
pub struct StandingCurationActor {
    actor_id: String,
    session_id: String,
    authority: CurationAuthority,
    authority_port: Option<Arc<dyn CurationAuthorityPort>>,
    rule: super::CurationRuleSource,
    store: Arc<CurationStore>,
    traversal: Arc<dyn CurationTraversalPort>,
    events: Arc<dyn CurationEventPort>,
    lifecycle: crate::lifecycle::NativeLifecycle,
    work_lock: parking_lot::Mutex<()>,
}

impl StandingCurationActor {
    pub fn new(
        actor_id: impl Into<String>,
        session_id: impl Into<String>,
        authority: CurationAuthority,
        rule: impl Into<super::CurationRuleSource>,
        store: Arc<CurationStore>,
        traversal: Arc<dyn CurationTraversalPort>,
        events: Arc<dyn CurationEventPort>,
    ) -> Result<Self, StorageError> {
        authority.validate()?;
        let rule = rule.into();
        rule.binding_refs()?;
        let actor_id = actor_id.into();
        let session_id = session_id.into();
        if actor_id.trim().is_empty() || session_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "standing Curation actor and session ids must be non-empty".to_string(),
            ));
        }
        Ok(Self {
            lifecycle: crate::lifecycle::NativeLifecycle::new(actor_id.clone()),
            work_lock: parking_lot::Mutex::new(()),
            actor_id,
            session_id,
            authority,
            authority_port: None,
            rule,
            store,
            traversal,
            events,
        })
    }

    /// Bind standing selection and new planned intake to the live admission epoch.
    pub fn with_authority_port(mut self, authority: Arc<dyn CurationAuthorityPort>) -> Self {
        self.authority_port = Some(authority);
        self
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Return the native durable source position used by lifecycle recovery.
    pub fn lifecycle_watermark(&self) -> Result<crate::events::EventWatermark, String> {
        self.events.watermark()
    }

    /// Resolve only this Curation resource's live input and durable operations.
    pub fn resolves_wake(&self, wake: &StructuralWakeAddress) -> Result<bool, String> {
        if let super::CurationRuleSource::Producer(port) = &self.rule {
            if port.resolves_wake(wake)? {
                return Ok(true);
            }
        }
        match wake {
            StructuralWakeAddress::EventPosition(value) => Ok(crate::waiting::after_position(
                value,
                &format!("event-ledger::{}", self.events.watermark()?.ledger_id),
            )),
            StructuralWakeAddress::DurableOperation(value) => {
                let Some(value) = crate::waiting::bound_address(value, self.store.resource_id())
                else {
                    return Ok(false);
                };
                if let Some(id) = value
                    .strip_prefix("curation-operation::")
                    .and_then(|tail| tail.strip_suffix("::completion"))
                {
                    return Ok(self
                        .store
                        .operation(id)
                        .map_err(|error| error.to_string())?
                        .is_some_and(|operation| {
                            operation.authority.same_owner_scope(&self.authority)
                        }));
                }
                Ok(
                    crate::waiting::after_position(value, "curation-publication")
                        && !self
                            .store
                            .unresolved_operations(&self.authority)
                            .map_err(|error| error.to_string())?
                            .is_empty(),
                )
            }
            _ => Ok(false),
        }
    }

    /// Read lifecycle evidence from this owner's bound stores and installed inputs.
    pub fn lifecycle_evidence(&self) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let _guard = self.work_lock.lock();
        self.lifecycle_evidence_inner()
    }

    fn lifecycle_evidence_inner(
        &self,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let source_refs = self
            .rule
            .binding_refs()
            .map_err(|error| error.to_string())?;
        let mut installed_refs = match &self.rule {
            super::CurationRuleSource::Installed(rule) => vec![crate::lifecycle::evidence_ref(
                "curation-rule",
                &rule.revision_ref(),
            )?],
            super::CurationRuleSource::Producer(port) => {
                let mut refs = Vec::new();
                for reference in port.template_refs().map_err(|error| error.to_string())? {
                    let template = self
                        .store
                        .resolve_template(&reference)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| {
                            "Curation producer cites an uninstalled template".to_string()
                        })?;
                    refs.push(crate::lifecycle::evidence_ref(
                        "curation-template",
                        &template.revision_ref(),
                    )?);
                }
                refs
            }
        };
        let current_authority = match &self.authority_port {
            Some(port) => port.observe().map_err(|error| error.to_string())?,
            None => Some(self.authority.clone()),
        };
        if let Some(authority) = &current_authority {
            if let super::CurationRuleSelection::Selected(rule) = self
                .rule
                .select(authority)
                .map_err(|error| error.to_string())?
            {
                installed_refs.push(crate::lifecycle::evidence_ref(
                    "curation-rule",
                    &rule.revision_ref(),
                )?);
            }
        }
        self.authority
            .validate()
            .map_err(|error| error.to_string())?;
        let watermark = self.events.watermark()?;
        let pending = self
            .store
            .unresolved_operations(&self.authority)
            .map_err(|error| error.to_string())?;
        for (operation_id, _) in &pending {
            if let Some(operation) = self
                .store
                .operation(operation_id)
                .map_err(|error| error.to_string())?
            {
                installed_refs.push(crate::lifecycle::evidence_ref(
                    "curation-rule",
                    &operation.rule_revision,
                )?);
            }
        }
        installed_refs.sort();
        installed_refs.dedup();
        self.store.flush().map_err(|error| error.to_string())?;
        let checkpoint_ref = format!(
            "standing-curation::{}::{}::{}",
            self.authority.agent_id, watermark.ledger_id, watermark.committed_seq
        );
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: installed_refs,
            binding_refs: source_refs
                .into_iter()
                .chain([
                    crate::lifecycle::evidence_ref("curation-prepared-authority", &self.authority)?,
                    crate::lifecycle::evidence_ref(
                        "curation-current-admission",
                        &current_authority,
                    )?,
                ])
                .collect(),
            subscription_refs: vec![format!("event-ledger::{}", watermark.ledger_id)],
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: crate::lifecycle::evidence_ref(
                "curation-unresolved",
                &pending,
            )?,
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

    /// Run at most one standing operation from the current durable Event tip.
    pub fn bounded_step(&self, max_items: usize) -> CurationStepReport {
        let _guard = self.work_lock.lock();
        let mut report = self.bounded_step_inner(max_items);
        crate::waiting::bind_waits(&mut report.waiting_on, self.store.resource_id());
        report
    }

    fn bounded_step_inner(&self, max_items: usize) -> CurationStepReport {
        let mut report = CurationStepReport::new(&self.actor_id);
        if max_items == 0 {
            report
                .fatal_errors
                .push("standing Curation budget must be greater than zero".to_string());
            return report;
        }
        let resumed = match self.store.resumable_operation(&self.authority) {
            Ok(operation) => operation,
            Err(error) => {
                report.retryable_errors.push(error.to_string());
                return report;
            }
        };
        let resuming = resumed.is_some();
        let authority = if let Some(operation) = &resumed {
            operation.authority.clone()
        } else {
            match &self.authority_port {
                Some(port) => match port.observe() {
                    Ok(Some(authority)) => authority,
                    Ok(None) => {
                        report.waiting_on.push(WaitingOnDeclaration::broad(
                            "curation_admission_closed",
                            "Curation has no open admission epoch",
                            vec![StructuralWakeAddress::BindingRecovery(format!(
                                "curation-authority::{}",
                                self.authority.agent_id
                            ))],
                        ));
                        return report;
                    }
                    Err(error) => {
                        report.retryable_errors.push(error.to_string());
                        return report;
                    }
                },
                None => self.authority.clone(),
            }
        };
        let queued = if resumed.is_none() {
            match self.store.next_planned_operation(&authority.agent_id) {
                Ok(queued) => queued,
                Err(error) => {
                    report.fatal_errors.push(error.to_string());
                    return report;
                }
            }
        } else {
            None
        };
        let historical = resumed.as_ref().or_else(|| {
            queued
                .as_ref()
                .filter(|operation| operation.authority != authority)
        });
        let selected_rule = if let Some(operation) = historical {
            self.store.resolve_rule(&operation.rule_revision)
        } else {
            match self.rule.select(&authority) {
                Ok(super::CurationRuleSelection::Selected(rule)) => Ok(*rule),
                Ok(super::CurationRuleSelection::Waiting(wait)) => {
                    report.waiting_on.push(wait);
                    return report;
                }
                Err(error) => Err(error),
            }
        };
        let selected_rule = match selected_rule {
            Ok(rule) => rule,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        let watermark = match self.events.watermark() {
            Ok(watermark) => watermark,
            Err(error) => {
                report.retryable_errors.push(error);
                return report;
            }
        };
        report.input_after_seq = watermark.committed_seq;
        report.output_after_seq = watermark.committed_seq;
        let candidate = if let Some(operation) = resumed {
            operation
        } else {
            match queued {
                Some(operation) => operation,
                None => {
                    let cut_request = TraversalCutRequest {
                        owners: expected_cut_owners(&selected_rule),
                        scope: selected_rule.rule.scope.clone(),
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
                            vec![
                                StructuralWakeAddress::EventPosition(format!(
                                    "event-ledger::{}::after::{}",
                                    watermark.ledger_id, watermark.committed_seq
                                )),
                                StructuralWakeAddress::OwnerRevision(format!(
                                    "graph-projection::{}::successor",
                                    watermark.ledger_id
                                )),
                            ],
                        ));
                        return report;
                    }
                    match CurationOperation::reconstruct(
                        authority.clone(),
                        selected_rule.revision_ref(),
                        cut,
                        selected_rule.rule.traversal_request(),
                    ) {
                        Ok(operation) => operation,
                        Err(error) => {
                            report.fatal_errors.push(error.to_string());
                            return report;
                        }
                    }
                }
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
        if let Some(port) = self.authority_port.as_ref().filter(|_| !resuming) {
            match port.observe() {
                Ok(Some(current)) if current == authority => {}
                Ok(_) => return report,
                Err(error) => {
                    report.retryable_errors.push(error.to_string());
                    return report;
                }
            }
        }
        let operation_rule = selected_rule;
        let acceptance = match CurationAcceptanceRecord::for_current_authority(
            &operation,
            &operation_rule,
            &authority,
        ) {
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
                vec![
                    StructuralWakeAddress::EventPosition(format!(
                        "event-ledger::{}::after::{}",
                        watermark.ledger_id, watermark.committed_seq
                    )),
                    StructuralWakeAddress::OwnerRevision(format!(
                        "curation-rule::{}::after::{}",
                        operation_rule.rule_id, operation_rule.content_hash
                    )),
                    StructuralWakeAddress::BindingRecovery(format!(
                        "curation-authority::{}",
                        self.authority.agent_id
                    )),
                ],
            ));
            return report;
        }

        let result = match self.store.result_for_operation(&operation.operation_id) {
            Ok(Some(result)) => {
                report.reused_results = 1;
                result
            }
            Ok(None) => match self.execute(&operation, &operation_rule) {
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
                vec![StructuralWakeAddress::EventPosition(format!(
                    "event-ledger::{}::after::{}",
                    watermark.ledger_id, watermark.committed_seq
                ))],
            ));
        }
        report
    }

    fn execute(
        &self,
        operation: &CurationOperation,
        installed_rule: &StandingCurationRuleRevision,
    ) -> Result<CurationResult, StorageError> {
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
        let realization = if let Some(rule) = &installed_rule.rule.realization {
            let source_complete = traversal.receipts.iter().any(|receipt| {
                receipt.owner_id == installed_rule.rule.source_owner_id
                    && receipt.scope == installed_rule.rule.scope
                    && receipt.completeness.status == OwnerCompletenessStatus::Complete
                    && receipt.completeness.exclusions.is_empty()
                    && receipt.completeness.failures.is_empty()
            });
            if !source_complete {
                return CurationResult::new(
                    operation,
                    CurationTerminalDisposition::Incomplete,
                    "realization assessment requires complete declared owner coverage",
                    None,
                    Vec::new(),
                    traversal.frontier.clone(),
                    source_exclusions(&traversal),
                    vec!["source_owner_coverage_unproved".into()],
                );
            }
            let observed = traversal.objects.iter().find(|object| {
                object.object_ref == rule.observed_object
                    && object.state == OwnerPublicationState::Observed
                    && rule
                        .required_qualifications
                        .iter()
                        .all(|(key, value)| object.qualifications.get(key) == Some(value))
            });
            let relation_type = if observed.is_some() {
                &rule.realized_relation_type
            } else {
                &rule.not_realized_relation_type
            };
            let source_receipts: Vec<_> = traversal
                .receipts
                .iter()
                .filter(|receipt| receipt.owner_id == installed_rule.rule.source_owner_id)
                .map(|receipt| receipt.semantic_basis())
                .collect();
            let realization_id = stable_identity(
                "curation-realization-v1",
                &(
                    &installed_rule.content_hash,
                    &operation.authority,
                    &rule.observed_object,
                    relation_type,
                    &source_receipts,
                    observed.map(|object| &object.publication_id),
                ),
            )?;
            Some((rule, observed, realization_id))
        } else {
            None
        };
        let expected = installed_rule.rule.expected_object()?;
        let root = installed_rule.rule.roots[0].clone();
        let object_id = stable_identity(
            "curation-object-publication-v1",
            &(
                &installed_rule.content_hash,
                &expected,
                &operation.authority.perspective,
            ),
        )?;
        let occurrence_id = stable_identity(
            "curation-relation-occurrence-v1",
            &(
                &installed_rule.content_hash,
                &installed_rule.rule.relation_type,
                &expected,
                &root,
                &operation.authority.perspective,
            ),
        )?;
        // A named request records its own observation even when standing state agrees.
        // Replays still resolve to the same operation and never publish it twice.
        if operation.request_id.is_none()
            && semantic_state_exists(&traversal, &object_id, &occurrence_id)
            && realization.as_ref().is_none_or(|(_, _, id)| {
                traversal
                    .occurrences
                    .iter()
                    .any(|occurrence| &occurrence.occurrence_id == id)
            })
        {
            let mut retained_ids = vec![object_id, occurrence_id];
            if let Some((_, _, realization_id)) = realization {
                retained_ids.push(realization_id);
            }
            return CurationResult::new(
                operation,
                CurationTerminalDisposition::Unchanged,
                "installed Curation-owned state already holds",
                None,
                retained_ids,
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
        let base_qualifications = qualifications(operation, installed_rule);
        let mut batch = OwnerPublicationBatch {
            owner_id: CURATION_OWNER_ID.to_string(),
            revision_id: result_id,
            scope: installed_rule.rule.scope.clone(),
            objects: vec![OwnerObjectPublication {
                publication_id: object_id.clone(),
                object_ref: expected.clone(),
                state: OwnerPublicationState::Observed,
                source_product_ref: operation.operation_id.clone(),
                hydration: hydration.clone(),
                provenance_refs: source_provenance(&traversal),
                qualifications: base_qualifications.clone(),
            }],
            relations: vec![OwnerRelationOccurrence {
                occurrence_id: occurrence_id.clone(),
                relation_type: installed_rule.rule.relation_type.clone(),
                src: expected,
                dst: root,
                source_product_ref: operation.operation_id.clone(),
                hydration,
                qualifications: base_qualifications,
                provenance_refs: source_provenance(&traversal),
            }],
            completeness: OwnerCompletenessReceipt {
                receipt_id: stable_identity(
                    "curation-completeness-v1",
                    &(&operation.operation_id, &object_id, &occurrence_id),
                )?,
                scope: installed_rule.rule.scope.clone(),
                included_ids: vec![object_id.clone(), occurrence_id.clone()],
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        };
        if let Some((rule, observed, realization_id)) = realization {
            let relation_type = if observed.is_some() {
                &rule.realized_relation_type
            } else {
                &rule.not_realized_relation_type
            };
            let mut evidence = source_provenance(&traversal);
            if let Some(observed) = observed {
                evidence.push(observed.publication_id.clone());
                evidence.push(observed.source_product_ref.clone());
            }
            let mut qualifications = qualifications(operation, installed_rule);
            qualifications.insert(
                "realization".into(),
                if observed.is_some() {
                    "realized"
                } else {
                    "not_realized"
                }
                .into(),
            );
            batch.objects[0]
                .qualifications
                .extend(qualifications.clone());
            batch.relations.push(OwnerRelationOccurrence {
                occurrence_id: realization_id.clone(),
                relation_type: relation_type.clone(),
                src: installed_rule.rule.expected_object()?,
                dst: rule.observed_object.clone(),
                source_product_ref: operation.operation_id.clone(),
                hydration: batch.objects[0].hydration.clone(),
                qualifications,
                provenance_refs: evidence,
            });
            batch.completeness.included_ids.push(realization_id);
        }
        let authored_ids = batch.completeness.included_ids.clone();
        let publication = OwnerPublicationOperation::reconstruct(
            installed_rule.rule.output_policy_revision.clone(),
            batch,
        )?;
        CurationResult::new(
            operation,
            CurationTerminalDisposition::Applied,
            "installed Curation rule authored bounded epistemic state",
            Some(publication),
            authored_ids,
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
        vec![StructuralWakeAddress::DurableOperation(format!(
            "curation-publication::after::{}",
            report.output_after_seq
        ))],
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
        .map(|receipt| match receipt.source_event {
            Some(event) => format!(
                "{}::{}::{}",
                event.ledger_id, event.seq, receipt.revision_id
            ),
            None => format!(
                "event-source-coverage::{}::{}::{}",
                receipt.projection_position.ledger_id,
                receipt.projection_position.after_seq,
                receipt.revision_id
            ),
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
