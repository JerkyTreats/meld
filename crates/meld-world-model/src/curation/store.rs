use std::io;

use serde::{de::DeserializeOwned, Serialize};
use sled::{Db, Tree};

use crate::curation::{
    stable_identity, CurationAcceptanceRecord, CurationAdmissionDecision, CurationAuthority,
    CurationOperation, CurationPlannedAuthorization, CurationPublicationKind,
    CurationPublicationReceipt, CurationResult, StandingCurationRule, StandingCurationRuleRevision,
};
use crate::error::StorageError;

const TREE_RULES: &str = "curation_rule_revisions";
const TREE_ACTIVE_RULES: &str = "curation_active_rules";
const TREE_OPERATIONS: &str = "curation_operations";
const TREE_OPERATION_BY_SELECTION: &str = "curation_operation_by_selection";
const TREE_PLANNED_AUTHORIZATIONS: &str = "curation_planned_authorizations";
const TREE_ACCEPTANCES: &str = "curation_acceptances";
const TREE_ACCEPTANCE_BY_OPERATION: &str = "curation_acceptance_by_operation";
const TREE_RESULTS: &str = "curation_results";
const TREE_RESULT_BY_OPERATION: &str = "curation_result_by_operation";
const TREE_PUBLICATION_RECEIPTS: &str = "curation_publication_receipts";

/// Curation-owned durable state inside the shared world-model database.
#[derive(Clone)]
pub struct CurationStore {
    resource_id: String,
    db: Db,
    rules: Tree,
    active_rules: Tree,
    operations: Tree,
    operation_by_selection: Tree,
    planned_authorizations: Tree,
    acceptances: Tree,
    acceptance_by_operation: Tree,
    results: Tree,
    result_by_operation: Tree,
    publication_receipts: Tree,
}

impl CurationStore {
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            resource_id: crate::waiting::resource_identity(&db)?,
            rules: db.open_tree(TREE_RULES).map_err(to_storage_io)?,
            active_rules: db.open_tree(TREE_ACTIVE_RULES).map_err(to_storage_io)?,
            operations: db.open_tree(TREE_OPERATIONS).map_err(to_storage_io)?,
            operation_by_selection: db
                .open_tree(TREE_OPERATION_BY_SELECTION)
                .map_err(to_storage_io)?,
            planned_authorizations: db
                .open_tree(TREE_PLANNED_AUTHORIZATIONS)
                .map_err(to_storage_io)?,
            acceptances: db.open_tree(TREE_ACCEPTANCES).map_err(to_storage_io)?,
            acceptance_by_operation: db
                .open_tree(TREE_ACCEPTANCE_BY_OPERATION)
                .map_err(to_storage_io)?,
            results: db.open_tree(TREE_RESULTS).map_err(to_storage_io)?,
            result_by_operation: db
                .open_tree(TREE_RESULT_BY_OPERATION)
                .map_err(to_storage_io)?,
            publication_receipts: db
                .open_tree(TREE_PUBLICATION_RECEIPTS)
                .map_err(to_storage_io)?,
            db,
        })
    }

    /// Exact durable world-model resource bound to this store instance.
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Persist one exact grounded rule without selecting a live runtime.
    fn persist_rule(
        &self,
        rule: StandingCurationRule,
        installed_at_seq: u64,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        rule.validate()?;
        let content_hash = stable_identity("standing-curation-rule-v1", &rule)?;
        let candidate = StandingCurationRuleRevision {
            rule_id: rule.rule_id.clone(),
            content_hash,
            installed_at_seq,
            rule,
        };
        candidate.validate()?;
        let key = rule_revision_key(&candidate.rule_id, &candidate.content_hash);
        let revision = match self.rules.get(&key).map_err(to_storage_io)? {
            Some(raw) => {
                let existing: StandingCurationRuleRevision = decode(&raw)?;
                existing.validate()?;
                if existing.rule != candidate.rule {
                    return Err(StorageError::InvalidPath(
                        "standing Curation rule identity collision".to_string(),
                    ));
                }
                existing
            }
            None => {
                self.rules
                    .insert(&key, encode(&candidate)?)
                    .map_err(to_storage_io)?;
                candidate
            }
        };
        self.flush()?;
        Ok(revision)
    }

    /// Legacy explicit selection retained for existing unprepared callers.
    pub fn install_rule(
        &self,
        rule: StandingCurationRule,
        installed_at_seq: u64,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        let revision = self.persist_rule(rule, installed_at_seq)?;
        if let Some(current) = self.active_rule(&revision.rule.agent_id)? {
            if current.installed_at_seq > revision.installed_at_seq
                || current.installed_at_seq == revision.installed_at_seq
                    && current.content_hash != revision.content_hash
            {
                return Err(StorageError::InvalidPath(
                    "standing Curation active rule cannot regress or fork its install sequence"
                        .to_string(),
                ));
            }
        }
        self.active_rules
            .insert(
                revision.rule.agent_id.as_bytes(),
                revision_key_bytes(&revision),
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(revision)
    }

    /// Install portable theory without selecting or starting any runtime owner.
    pub fn install_template(
        &self,
        template: super::CurationRuleTemplate,
        installed_at_seq: u64,
    ) -> Result<super::CurationTemplateRevision, StorageError> {
        template.validate()?;
        let content_hash = stable_identity("curation-rule-template-v1", &template)?;
        let reference = crate::belief::TheoryRevisionRef {
            registry: super::CURATION_TEMPLATE_REGISTRY_ID.into(),
            id: template.rule_id.clone(),
            content_hash: content_hash.clone(),
        };
        if let Some(existing) = self.resolve_template(&reference)? {
            return Ok(existing);
        }
        let revision = super::CurationTemplateRevision {
            template,
            content_hash,
            installed_at_seq,
        };
        put_exact(
            &self.rules,
            &format!("template::{}::{}", reference.id, reference.content_hash),
            &revision,
        )?;
        self.flush()?;
        Ok(revision)
    }

    pub fn resolve_template(
        &self,
        reference: &crate::belief::TheoryRevisionRef,
    ) -> Result<Option<super::CurationTemplateRevision>, StorageError> {
        reference.validate_for_registry(super::CURATION_TEMPLATE_REGISTRY_ID)?;
        let revision: Option<super::CurationTemplateRevision> = get_optional(
            &self.rules,
            &format!("template::{}::{}", reference.id, reference.content_hash),
        )?;
        if let Some(revision) = &revision {
            revision.validate()?;
            if revision.revision_ref() != *reference {
                return Err(StorageError::InvalidPath(
                    "Curation template reference differs".into(),
                ));
            }
        }
        Ok(revision)
    }

    /// Ground exact installed semantic content for one structural assignment.
    pub fn prepare_rule(
        &self,
        reference: &crate::belief::TheoryRevisionRef,
        binding: &super::CurationRuleBinding,
        installed_at_seq: u64,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        let template = self.resolve_template(reference)?.ok_or_else(|| {
            StorageError::InvalidPath("Curation template revision is not installed".into())
        })?;
        self.persist_rule(template.template.ground(binding)?, installed_at_seq)
    }

    /// Ground an installed template against an independent owner source and Agent judgment.
    pub fn prepare_rule_for_source(
        &self,
        reference: &crate::belief::TheoryRevisionRef,
        binding: &super::CurationRuleBinding,
        source: &super::CurationSourceBinding,
        judgment: &super::CurationJudgmentScope,
        installed_at_seq: u64,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        let template = self.resolve_template(reference)?.ok_or_else(|| {
            StorageError::InvalidPath("Curation template revision is not installed".into())
        })?;
        self.persist_rule(
            template
                .template
                .ground_for_source(binding, source, judgment)?,
            installed_at_seq,
        )
    }

    /// Reconstruct the exact source grounding without consulting the mutable active head.
    pub fn resolve_source_bound_rule(
        &self,
        template_ref: &crate::belief::TheoryRevisionRef,
        binding: &super::CurationRuleBinding,
        source: &super::CurationSourceBinding,
        judgment: &super::CurationJudgmentScope,
        rule_ref: &crate::belief::TheoryRevisionRef,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        let template = self.resolve_template(template_ref)?.ok_or_else(|| {
            StorageError::InvalidPath("prepared Curation template is missing".into())
        })?;
        let rule = self.resolve_rule(rule_ref)?;
        if rule.rule
            != template
                .template
                .ground_for_source(binding, source, judgment)?
        {
            return Err(StorageError::InvalidPath(
                "prepared Curation rule differs from its exact source and judgment context".into(),
            ));
        }
        Ok(rule)
    }

    pub fn resolve_bound_rule(
        &self,
        template_ref: &crate::belief::TheoryRevisionRef,
        binding: &super::CurationRuleBinding,
        rule_ref: &crate::belief::TheoryRevisionRef,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        let template = self.resolve_template(template_ref)?.ok_or_else(|| {
            StorageError::InvalidPath("prepared Curation template is missing".into())
        })?;
        let rule = self.resolve_rule(rule_ref)?;
        if rule.rule != template.template.ground(binding)? {
            return Err(StorageError::InvalidPath(
                "prepared Curation rule differs from its installed template and assignment".into(),
            ));
        }
        Ok(rule)
    }

    pub fn resolve_rule(
        &self,
        reference: &crate::belief::TheoryRevisionRef,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        reference.validate_for_registry(super::CURATION_RULE_REGISTRY_ID)?;
        self.rule_revision(&reference.id, &reference.content_hash)
    }

    pub fn active_rule(
        &self,
        agent_id: &str,
    ) -> Result<Option<StandingCurationRuleRevision>, StorageError> {
        let Some(key) = self
            .active_rules
            .get(agent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision: StandingCurationRuleRevision = self
            .rules
            .get(key)
            .map_err(to_storage_io)?
            .map(|raw| decode(&raw))
            .transpose()?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "standing Curation active rule points at a missing revision".to_string(),
                )
            })?;
        revision.validate()?;
        if revision.rule.agent_id != agent_id {
            return Err(StorageError::InvalidPath(
                "standing Curation active rule points at another Agent".to_string(),
            ));
        }
        Ok(Some(revision))
    }

    pub fn put_operation(&self, operation: &CurationOperation) -> Result<(), StorageError> {
        operation.validate()?;
        let semantic_operation = operation.semantic_operation();
        if let Some(existing) = self.operation_for_selection(&semantic_operation.selection_id)? {
            if existing != semantic_operation {
                return Err(StorageError::InvalidPath(
                    "one standing Curation selection cannot create divergent operations"
                        .to_string(),
                ));
            }
            return Ok(());
        }
        put_exact(
            &self.operations,
            &semantic_operation.operation_id,
            &semantic_operation,
        )?;
        self.operation_by_selection
            .insert(
                semantic_operation.selection_id.as_bytes(),
                semantic_operation.operation_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Durably accept an Agent-authorized planned operation for the canonical actor.
    pub fn submit_planned(&self, operation: &CurationOperation) -> Result<(), StorageError> {
        let Some(authorization) = &operation.planned_authorization else {
            return Err(StorageError::InvalidPath(
                "planned Curation submission requires Agent product authorization".to_string(),
            ));
        };
        operation.validate()?;
        self.put_operation(operation)?;
        put_exact(
            &self.planned_authorizations,
            &operation.operation_id,
            authorization,
        )?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    /// Return the first durable planned operation not yet accepted.
    pub fn next_planned_operation(
        &self,
        agent_id: &str,
    ) -> Result<Option<CurationOperation>, StorageError> {
        let mut pending = Vec::new();
        for row in self.planned_authorizations.iter() {
            let (operation_id, raw) = row.map_err(to_storage_io)?;
            let operation_id = std::str::from_utf8(&operation_id).map_err(|error| {
                StorageError::InvalidPath(format!(
                    "invalid planned Curation authorization index: {error}"
                ))
            })?;
            let authorization: CurationPlannedAuthorization = decode(&raw)?;
            let operation = self.operation(operation_id)?.ok_or_else(|| {
                StorageError::InvalidPath(
                    "planned Curation authorization references a missing operation".to_string(),
                )
            })?;
            authorization.validate(&operation)?;
            if operation.authority.agent_id == agent_id {
                if self
                    .acceptance_for_operation(&operation.operation_id)?
                    .is_some_and(|receipt| receipt.decision == CurationAdmissionDecision::Rejected)
                {
                    continue;
                }
                let complete = if let Some(result) =
                    self.result_for_operation(&operation.operation_id)?
                {
                    let terminal = self
                        .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)?
                        .is_some();
                    let semantic = result.semantic_publication.is_none()
                        || self
                            .publication_receipt(
                                &result.result_id,
                                CurationPublicationKind::Semantic,
                            )?
                            .is_some();
                    terminal && semantic
                } else {
                    false
                };
                if !complete {
                    pending.push(operation.with_planned_authorization(authorization)?);
                }
            }
        }
        pending.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
        Ok(pending.into_iter().next())
    }

    pub fn acceptance_for_planned_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
        if self
            .planned_authorization_for_operation(operation_id)?
            .is_none()
        {
            return Ok(None);
        }
        self.acceptance_for_operation(operation_id)
    }

    pub fn planned_authorization_for_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationPlannedAuthorization>, StorageError> {
        let authorization: Option<CurationPlannedAuthorization> =
            get_optional(&self.planned_authorizations, operation_id)?;
        if let Some(authorization) = &authorization {
            let operation = self.operation(operation_id)?.ok_or_else(|| {
                StorageError::InvalidPath(
                    "planned Curation authorization references a missing operation".to_string(),
                )
            })?;
            authorization.validate(&operation)?;
        }
        Ok(authorization)
    }

    /// Resolve an unchanged semantic selection to its original frozen operation.
    /// Advancing transport positions alone must not recreate completed work.
    pub fn resolve_operation(
        &self,
        candidate: CurationOperation,
    ) -> Result<CurationOperation, StorageError> {
        candidate.validate()?;
        Ok(self
            .operation_for_selection(&candidate.selection_id)?
            .unwrap_or(candidate))
    }

    pub fn operation_for_selection(
        &self,
        selection_id: &str,
    ) -> Result<Option<CurationOperation>, StorageError> {
        let operation_id = self
            .operation_by_selection
            .get(selection_id.as_bytes())
            .map_err(to_storage_io)?;
        let Some(operation_id) = operation_id else {
            for row in self.operations.iter() {
                let (_, raw) = row.map_err(to_storage_io)?;
                let operation: CurationOperation = decode(&raw)?;
                operation.validate()?;
                if operation.selection_id == selection_id {
                    self.operation_by_selection
                        .insert(selection_id.as_bytes(), operation.operation_id.as_bytes())
                        .map_err(to_storage_io)?;
                    return Ok(Some(operation));
                }
            }
            return Ok(None);
        };
        let operation_id = std::str::from_utf8(&operation_id).map_err(|error| {
            StorageError::InvalidPath(format!("invalid Curation operation index: {error}"))
        })?;
        let operation: Option<CurationOperation> = get_optional(&self.operations, operation_id)?;
        if let Some(operation) = &operation {
            operation.validate()?;
        }
        Ok(operation)
    }

    pub fn put_acceptance(
        &self,
        acceptance: &CurationAcceptanceRecord,
    ) -> Result<(), StorageError> {
        self.validate_acceptance(acceptance)?;
        if let Some(existing) = self.acceptance_for_operation(&acceptance.operation_id)? {
            if existing != *acceptance {
                return Err(StorageError::InvalidPath(
                    "one Curation operation cannot have divergent admission decisions".to_string(),
                ));
            }
            return Ok(());
        }
        put_exact(&self.acceptances, &acceptance.acceptance_id, acceptance)?;
        self.acceptance_by_operation
            .insert(
                acceptance.operation_id.as_bytes(),
                acceptance.acceptance_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    pub fn acceptance(
        &self,
        acceptance_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
        let acceptance: Option<CurationAcceptanceRecord> =
            get_optional(&self.acceptances, acceptance_id)?;
        if let Some(acceptance) = &acceptance {
            self.validate_acceptance(acceptance)?;
        }
        Ok(acceptance)
    }

    /// Persist one terminal result together with its complete publication intent.
    pub fn put_result(&self, result: &CurationResult) -> Result<CurationResult, StorageError> {
        let operation = self.operation(&result.operation_id)?.ok_or_else(|| {
            StorageError::InvalidPath("Curation result requires its durable operation".to_string())
        })?;
        result.validate(&operation)?;
        let acceptance = self
            .acceptance_for_operation(&result.operation_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation result requires a durable admission decision".to_string(),
                )
            })?;
        let rule = self.rule_revision(
            &acceptance.rule_revision.id,
            &acceptance.rule_revision.content_hash,
        )?;
        if acceptance != CurationAcceptanceRecord::for_operation(&operation, &rule)? {
            return Err(StorageError::InvalidPath(
                "Curation result references an invalid admission decision".to_string(),
            ));
        }
        if acceptance.decision != CurationAdmissionDecision::Admitted {
            return Err(StorageError::InvalidPath(
                "rejected Curation operation cannot acquire a terminal result".to_string(),
            ));
        }
        if let Some(existing) = self.result_for_operation(&result.operation_id)? {
            if existing != *result {
                return Err(StorageError::InvalidPath(
                    "one admitted Curation operation cannot have divergent terminal results"
                        .to_string(),
                ));
            }
            return Ok(existing);
        }
        put_exact(&self.results, &result.result_id, result)?;
        self.result_by_operation
            .insert(result.operation_id.as_bytes(), result.result_id.as_bytes())
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(result.clone())
    }

    pub(crate) fn result(&self, result_id: &str) -> Result<Option<CurationResult>, StorageError> {
        let result: Option<CurationResult> = get_optional(&self.results, result_id)?;
        if let Some(result) = &result {
            let operation = self.operation(&result.operation_id)?.ok_or_else(|| {
                StorageError::InvalidPath("Curation result has no retained operation".into())
            })?;
            result.validate(&operation)?;
        }
        Ok(result)
    }

    pub fn result_for_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationResult>, StorageError> {
        let result_id = self
            .result_by_operation
            .get(operation_id.as_bytes())
            .map_err(to_storage_io)?;
        let Some(result_id) = result_id else {
            for row in self.results.iter() {
                let (_, raw) = row.map_err(to_storage_io)?;
                let result: CurationResult = decode(&raw)?;
                if result.operation_id == operation_id {
                    let operation = self.operation(operation_id)?.ok_or_else(|| {
                        StorageError::InvalidPath(
                            "Curation result references a missing operation".to_string(),
                        )
                    })?;
                    result.validate(&operation)?;
                    self.result_by_operation
                        .insert(operation_id.as_bytes(), result.result_id.as_bytes())
                        .map_err(to_storage_io)?;
                    return Ok(Some(result));
                }
            }
            return Ok(None);
        };
        let result_id = std::str::from_utf8(&result_id).map_err(|error| {
            StorageError::InvalidPath(format!("invalid Curation result index: {error}"))
        })?;
        let result: Option<CurationResult> = get_optional(&self.results, result_id)?;
        if let Some(result) = &result {
            let operation = self.operation(operation_id)?.ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation result references a missing operation".to_string(),
                )
            })?;
            result.validate(&operation)?;
        }
        Ok(result)
    }

    pub fn put_publication_receipt(
        &self,
        receipt: &CurationPublicationReceipt,
    ) -> Result<(), StorageError> {
        self.validate_publication_receipt(receipt)?;
        let key = publication_receipt_key(&receipt.result_id, receipt.kind);
        put_exact(&self.publication_receipts, &key, receipt)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    pub fn publication_receipt(
        &self,
        result_id: &str,
        kind: CurationPublicationKind,
    ) -> Result<Option<CurationPublicationReceipt>, StorageError> {
        let receipt: Option<CurationPublicationReceipt> = get_optional(
            &self.publication_receipts,
            &publication_receipt_key(result_id, kind),
        )?;
        if let Some(receipt) = &receipt {
            if receipt.result_id != result_id || receipt.kind != kind {
                return Err(StorageError::InvalidPath(
                    "Curation publication receipt index has divergent identity".to_string(),
                ));
            }
            self.validate_publication_receipt(receipt)?;
        }
        Ok(receipt)
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    /// Finish accepted native work under its original authority after admission closes.
    pub(crate) fn resumable_operation(
        &self,
        authority: &CurationAuthority,
    ) -> Result<Option<CurationOperation>, StorageError> {
        for (operation_id, _) in self.unresolved_operations(authority)? {
            if !self
                .acceptance_for_operation(&operation_id)?
                .is_some_and(|receipt| receipt.decision == CurationAdmissionDecision::Admitted)
            {
                continue;
            }
            let operation = self.operation(&operation_id)?.ok_or_else(|| {
                StorageError::InvalidPath("accepted Curation operation is absent".into())
            })?;
            return match self.planned_authorization_for_operation(&operation_id)? {
                Some(authorization) => operation
                    .with_planned_authorization(authorization)
                    .map(Some),
                None => Ok(Some(operation)),
            };
        }
        Ok(None)
    }

    pub(crate) fn unresolved_operations(
        &self,
        authority: &super::CurationAuthority,
    ) -> Result<Vec<(String, String)>, StorageError> {
        let mut pending = Vec::new();
        for row in &self.operations {
            let (_, raw) = row.map_err(to_storage_io)?;
            let operation: CurationOperation = decode(&raw)?;
            if !operation.authority.same_owner_scope(authority) {
                continue;
            }
            if self
                .acceptance_for_operation(&operation.operation_id)?
                .is_some_and(|acceptance| {
                    acceptance.decision == super::CurationAdmissionDecision::Rejected
                })
            {
                continue;
            }
            let Some(result) = self.result_for_operation(&operation.operation_id)? else {
                pending.push((operation.operation_id, "result_pending".into()));
                continue;
            };
            for kind in [
                CurationPublicationKind::Semantic,
                CurationPublicationKind::Terminal,
            ] {
                if kind == CurationPublicationKind::Semantic
                    && result.semantic_publication.is_none()
                {
                    continue;
                }
                if self.publication_receipt(&result.result_id, kind)?.is_none() {
                    pending.push((
                        operation.operation_id.clone(),
                        format!("{kind:?}_publication_pending"),
                    ));
                }
            }
        }
        Ok(pending)
    }

    pub(super) fn operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationOperation>, StorageError> {
        let operation: Option<CurationOperation> = get_optional(&self.operations, operation_id)?;
        if let Some(operation) = &operation {
            operation.validate()?;
        }
        Ok(operation)
    }

    fn acceptance_for_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
        let acceptance_id = self
            .acceptance_by_operation
            .get(operation_id.as_bytes())
            .map_err(to_storage_io)?;
        let Some(acceptance_id) = acceptance_id else {
            for row in self.acceptances.iter() {
                let (_, raw) = row.map_err(to_storage_io)?;
                let acceptance: CurationAcceptanceRecord = decode(&raw)?;
                self.validate_acceptance(&acceptance)?;
                if acceptance.operation_id == operation_id {
                    self.acceptance_by_operation
                        .insert(operation_id.as_bytes(), acceptance.acceptance_id.as_bytes())
                        .map_err(to_storage_io)?;
                    return Ok(Some(acceptance));
                }
            }
            return Ok(None);
        };
        let acceptance_id = std::str::from_utf8(&acceptance_id).map_err(|error| {
            StorageError::InvalidPath(format!("invalid Curation acceptance index: {error}"))
        })?;
        let acceptance: Option<CurationAcceptanceRecord> =
            get_optional(&self.acceptances, acceptance_id)?;
        if let Some(acceptance) = &acceptance {
            self.validate_acceptance(acceptance)?;
        }
        Ok(acceptance)
    }

    fn validate_acceptance(
        &self,
        acceptance: &CurationAcceptanceRecord,
    ) -> Result<(), StorageError> {
        let operation = self.operation(&acceptance.operation_id)?.ok_or_else(|| {
            StorageError::InvalidPath(
                "Curation acceptance requires its durable operation".to_string(),
            )
        })?;
        let rule = self.rule_revision(
            &acceptance.rule_revision.id,
            &acceptance.rule_revision.content_hash,
        )?;
        let expected = match &acceptance.observed_authority {
            Some(current) => {
                CurationAcceptanceRecord::for_current_authority(&operation, &rule, current)?
            }
            None => CurationAcceptanceRecord::for_operation(&operation, &rule)?,
        };
        if expected != *acceptance {
            return Err(StorageError::InvalidPath(
                "Curation acceptance does not match its exact operation and rule".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_publication_receipt(
        &self,
        receipt: &CurationPublicationReceipt,
    ) -> Result<(), StorageError> {
        let result: CurationResult =
            get_optional(&self.results, &receipt.result_id)?.ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation publication receipt requires its durable result".to_string(),
                )
            })?;
        let operation = self.operation(&result.operation_id)?.ok_or_else(|| {
            StorageError::InvalidPath(
                "Curation publication receipt references a missing operation".to_string(),
            )
        })?;
        result.validate(&operation)?;
        let expected_record_id = match receipt.kind {
            CurationPublicationKind::Semantic => result
                .semantic_publication
                .as_ref()
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "semantic receipt requires semantic publication intent".to_string(),
                    )
                })?
                .event_record_id(),
            CurationPublicationKind::Terminal => result.event_record_id(),
        };
        if receipt.event_record_id != expected_record_id
            || receipt.event_position.ledger_id != result.source_event_position.ledger_id
            || receipt.event_position.after_seq <= result.source_event_position.after_seq
        {
            return Err(StorageError::InvalidPath(
                "Curation publication receipt does not match its Event intent".to_string(),
            ));
        }
        Ok(())
    }

    fn rule_revision(
        &self,
        rule_id: &str,
        content_hash: &str,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        let key = rule_revision_key(rule_id, content_hash);
        let revision: StandingCurationRuleRevision = self
            .rules
            .get(key)
            .map_err(to_storage_io)?
            .map(|raw| decode(&raw))
            .transpose()?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation operation references a missing rule revision".to_string(),
                )
            })?;
        revision.validate()?;
        Ok(revision)
    }
}

fn rule_revision_key(rule_id: &str, content_hash: &str) -> Vec<u8> {
    format!("{rule_id}::{content_hash}").into_bytes()
}

fn revision_key_bytes(revision: &StandingCurationRuleRevision) -> Vec<u8> {
    rule_revision_key(&revision.rule_id, &revision.content_hash)
}

fn publication_receipt_key(result_id: &str, kind: CurationPublicationKind) -> String {
    format!("{result_id}::{kind:?}")
}

fn put_exact<T: Serialize + DeserializeOwned + PartialEq>(
    tree: &Tree,
    key: &str,
    value: &T,
) -> Result<(), StorageError> {
    if let Some(raw) = tree.get(key.as_bytes()).map_err(to_storage_io)? {
        let existing: T = decode(&raw)?;
        if existing != *value {
            return Err(StorageError::InvalidPath(format!(
                "Curation durable identity '{key}' has divergent content"
            )));
        }
        return Ok(());
    }
    tree.insert(key.as_bytes(), encode(value)?)
        .map_err(to_storage_io)?;
    Ok(())
}

fn get_optional<T: DeserializeOwned>(tree: &Tree, key: &str) -> Result<Option<T>, StorageError> {
    tree.get(key.as_bytes())
        .map_err(to_storage_io)?
        .map(|raw| decode(&raw))
        .transpose()
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(value).map_err(to_storage_data)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, StorageError> {
    serde_json::from_slice(bytes).map_err(to_storage_data)
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(error.to_string()))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
}
