use std::collections::BTreeMap;

use meld_events::{AppendReceipt, DomainObjectRef, EventEnvelope, EventWatermark, LedgerCursor};
use serde::{Deserialize, Serialize};

use crate::belief::{BranchScope, TheoryRevisionRef};
use crate::error::StorageError;
use crate::world_state::graph::contracts::{
    BoundedTraversalRequest, OwnerPublicationExclusion, OwnerPublicationOperation,
    OwnerPublicationScope, PerspectiveKey, TraversalBounds, TraversalCut, TraversalCutRequest,
    TraversalDirection, TraversalFrontierEntry, TraversalResult,
};

/// Semantic owner used for standing Curation publications.
pub const CURATION_OWNER_ID: &str = "curation";
/// Producer-owned terminal result Event type.
pub const CURATION_RESULT_EVENT_TYPE: &str = "world_model.curation.result.v1";
/// Registry identity for installed standing Curation rules.
pub const CURATION_RULE_REGISTRY_ID: &str = "epistemic_curation_rule";
/// Standing Curation waits on an exact complete Traversal cut.
pub const CURATION_CUT_INCOMPLETE: &str = "curation_cut_incomplete";
/// Standing Curation is quiet until a declared selection changes.
pub const CURATION_SELECTION_UNCHANGED: &str = "curation_selection_unchanged";
/// Standing Curation has durable intent whose Event append remains pending.
pub const CURATION_PUBLICATION_PENDING: &str = "curation_publication_pending";

/// One narrow installed rule for Curation-owned expected state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingCurationRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_event_route: Option<crate::world_state::graph::admission::OwnerEventSourceRef>,
    /// Exact judgment authority when observing an independently owned source scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judgment_scope: Option<CurationJudgmentScope>,
    pub rule_id: String,
    pub agent_id: String,
    pub source_owner_id: String,
    pub scope: OwnerPublicationScope,
    pub roots: Vec<DomainObjectRef>,
    pub traversal_direction: TraversalDirection,
    pub bounds: TraversalBounds,
    pub expected_object_kind: String,
    pub expected_object_id: String,
    pub relation_type: String,
    pub output_policy_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realization: Option<CurationRealizationRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<super::CurationCoverageRule>,
}

/// Agent judgment context, independent from the source owner's publication scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurationJudgmentScope {
    pub subject: DomainObjectRef,
    pub perspective: PerspectiveKey,
    pub branch_scope: BranchScope,
}

impl CurationJudgmentScope {
    pub fn validate(&self) -> Result<(), StorageError> {
        self.subject.validate()?;
        self.perspective.validate()?;
        require_non_empty("Curation judgment branch", &self.branch_scope.branch_id)
    }

    fn matches(&self, authority: &CurationAuthority) -> bool {
        self.subject == authority.subject
            && self.perspective == authority.perspective
            && self.branch_scope == authority.branch_scope
    }
}

/// Installed owner-qualified predicate assessed under a complete bounded cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationRealizationRule {
    pub observed_object: DomainObjectRef,
    #[serde(default)]
    pub required_qualifications: BTreeMap<String, String>,
    pub realized_relation_type: String,
    pub not_realized_relation_type: String,
}

impl StandingCurationRule {
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("standing Curation rule id", &self.rule_id)?;
        require_non_empty("standing Curation agent id", &self.agent_id)?;
        require_non_empty("standing Curation source owner", &self.source_owner_id)?;
        self.scope.validate()?;
        if let Some(source) = &self.source_event_route {
            source.validate()?;
        }
        if let Some(scope) = &self.judgment_scope {
            scope.validate()?;
        }
        if self.roots.is_empty() {
            return invalid("standing Curation rule requires at least one root");
        }
        for root in &self.roots {
            root.validate()?;
            if root.domain_id != self.source_owner_id {
                return invalid("standing Curation roots must belong to the source owner");
            }
        }
        self.bounds.validate_for_curation()?;
        if let Some(coverage) = &self.coverage {
            coverage.validate()?;
        }
        if let Some(realization) = &self.realization {
            realization.observed_object.validate()?;
            if !self.roots.contains(&realization.observed_object) {
                return invalid("Curation realization target must be an explicit traversal root");
            }
            if realization.observed_object.domain_id != self.source_owner_id {
                return invalid("Curation realization must refer to its declared source owner");
            }
            require_non_empty(
                "Curation realized relation",
                &realization.realized_relation_type,
            )?;
            require_non_empty(
                "Curation not-realized relation",
                &realization.not_realized_relation_type,
            )?;
            if realization.realized_relation_type == realization.not_realized_relation_type {
                return invalid("Curation realization dispositions must have distinct relations");
            }
        }
        require_non_empty(
            "standing Curation expected object kind",
            &self.expected_object_kind,
        )?;
        require_non_empty(
            "standing Curation expected object id",
            &self.expected_object_id,
        )?;
        require_non_empty("standing Curation relation type", &self.relation_type)?;
        require_non_empty(
            "standing Curation output policy revision",
            &self.output_policy_revision,
        )
    }

    /// Validate a caller's exact Agent scope before preparing work for this rule.
    pub fn validate_authority(&self, authority: &CurationAuthority) -> Result<(), StorageError> {
        self.validate()?;
        authority.validate()?;
        if self.agent_id != authority.agent_id {
            return invalid("standing Curation rule belongs to another Agent");
        }
        if let Some(reason) = self.authority_mismatch(authority) {
            return invalid(reason);
        }
        Ok(())
    }

    fn authority_mismatch(&self, authority: &CurationAuthority) -> Option<&'static str> {
        if let Some(scope) = &self.judgment_scope {
            return (!scope.matches(authority))
                .then_some("initiating Agent differs from the declared judgment scope");
        }
        if !self.roots.contains(&authority.subject) {
            Some("initiating Agent subject is outside the standing rule roots")
        } else if self.scope.branch_id.as_deref() != Some(authority.branch_scope.branch_id.as_str())
        {
            Some("initiating Agent branch differs from the standing rule scope")
        } else if self.scope.perspective_id.as_deref()
            != Some(authority.perspective.perspective_id.as_str())
        {
            Some("initiating Agent perspective differs from the standing rule scope")
        } else {
            None
        }
    }

    pub fn expected_object(&self) -> Result<DomainObjectRef, StorageError> {
        DomainObjectRef::new(
            CURATION_OWNER_ID,
            self.expected_object_kind.clone(),
            self.expected_object_id.clone(),
        )
    }

    /// Realization and coverage judge selected source state; consumers must
    /// establish that their derived evidence still names that source revision.
    pub fn publishes_source_judgments(&self) -> bool {
        self.realization.is_some() || self.coverage.is_some()
    }

    /// Consumers must wait for the same source evidence Curation requires for coverage.
    pub fn source_readiness_requirements(
        &self,
    ) -> Vec<crate::world_state::graph::contracts::OwnerObjectQualificationRequirement> {
        self.coverage.as_ref().map_or_else(Vec::new, |coverage| {
            self.roots
                .iter()
                .map(|root| {
                    crate::world_state::graph::contracts::OwnerObjectQualificationRequirement {
                        object_ref: root.clone(),
                        qualifications: coverage.source_required_qualifications.clone(),
                    }
                })
                .collect()
        })
    }

    pub fn traversal_request(&self) -> BoundedTraversalRequest {
        BoundedTraversalRequest {
            roots: self.roots.clone(),
            direction: self.traversal_direction,
            relation_types: None,
            bounds: self.bounds.clone(),
        }
    }
}

/// Exact installed revision of one standing Curation rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingCurationRuleRevision {
    pub rule_id: String,
    pub content_hash: String,
    pub installed_at_seq: u64,
    pub rule: StandingCurationRule,
}

impl StandingCurationRuleRevision {
    pub fn validate(&self) -> Result<(), StorageError> {
        self.rule.validate()?;
        if self.rule_id != self.rule.rule_id {
            return invalid("standing Curation revision rule id differs from its body");
        }
        if self.content_hash != stable_identity("standing-curation-rule-v1", &self.rule)? {
            return invalid("standing Curation revision content hash is not canonical");
        }
        Ok(())
    }

    pub fn revision_ref(&self) -> TheoryRevisionRef {
        TheoryRevisionRef {
            registry: CURATION_RULE_REGISTRY_ID.to_string(),
            id: self.rule_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }
}

/// Agent-owned identity and generation fence accepted by Curation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationAuthority {
    pub agent_id: String,
    pub perspective: PerspectiveKey,
    pub branch_scope: BranchScope,
    pub activation_generation: String,
    /// Exact admission epoch, absent only in legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_epoch: Option<String>,
    pub subject: DomainObjectRef,
}

impl CurationAuthority {
    /// Durable ownership scope shared by this Agent's successive admission epochs.
    pub(crate) fn same_owner_scope(&self, other: &Self) -> bool {
        self.agent_id == other.agent_id
            && self.subject == other.subject
            && self.perspective == other.perspective
            && self.branch_scope == other.branch_scope
    }

    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("Curation authority agent id", &self.agent_id)?;
        self.perspective.validate()?;
        require_non_empty("Curation authority branch id", &self.branch_scope.branch_id)?;
        require_non_empty(
            "Curation authority activation generation",
            &self.activation_generation,
        )?;
        self.subject.validate()
    }
}

/// One deterministic standing operation over an immutable cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationOperation {
    pub operation_id: String,
    pub selection_id: String,
    pub authority: CurationAuthority,
    pub rule_revision: TheoryRevisionRef,
    pub source_cut: TraversalCut,
    pub traversal_request: BoundedTraversalRequest,
    /// Caller-owned request distinguishes planned intake from standing selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Agent authorization attached only at planned publication time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planned_authorization: Option<CurationPlannedAuthorization>,
}

/// Exact Agent product authority Curation validates before planned acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationPlannedAuthorization {
    pub authorization_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub plan_revision_id: String,
    pub product_id: String,
    pub operation_id: String,
    pub context_id: String,
    pub authority_scope_id: String,
    pub activation_generation: String,
    /// Exact admission epoch, absent only in legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_epoch: Option<String>,
    pub idempotency_key: String,
}

impl CurationPlannedAuthorization {
    pub fn validate(&self, operation: &CurationOperation) -> Result<(), StorageError> {
        for value in [
            &self.authorization_id,
            &self.agent_id,
            &self.goal_id,
            &self.plan_revision_id,
            &self.product_id,
            &self.operation_id,
            &self.context_id,
            &self.authority_scope_id,
            &self.activation_generation,
            &self.idempotency_key,
        ] {
            require_non_empty("planned Curation authorization field", value)?;
        }
        if self.operation_id != operation.operation_id
            || self.agent_id != operation.authority.agent_id
            || self.activation_generation != operation.authority.activation_generation
            || self.admission_epoch != operation.authority.admission_epoch
        {
            return invalid("planned Curation authorization does not match its operation fence");
        }
        let expected = stable_identity(
            "agent-product-authorization-v1",
            &(
                &self.agent_id,
                &self.goal_id,
                &self.plan_revision_id,
                &self.product_id,
                &self.operation_id,
                &self.context_id,
                &self.authority_scope_id,
                &self.activation_generation,
                &self.idempotency_key,
            ),
        )?;
        if self.authorization_id != expected {
            return invalid("planned Curation authorization identity is not canonical");
        }
        Ok(())
    }
}

impl CurationOperation {
    pub fn reconstruct(
        authority: CurationAuthority,
        rule_revision: TheoryRevisionRef,
        source_cut: TraversalCut,
        traversal_request: BoundedTraversalRequest,
    ) -> Result<Self, StorageError> {
        authority.validate()?;
        rule_revision.validate_for_registry(CURATION_RULE_REGISTRY_ID)?;
        source_cut.validate_identity()?;
        traversal_request.validate_for_curation()?;
        let traversal_request = traversal_request.normalized();
        let selection_id = stable_identity(
            "standing-curation-selection-v1",
            &(
                &authority,
                &rule_revision,
                &source_cut.owners,
                &source_cut
                    .receipts
                    .iter()
                    .map(|receipt| receipt.semantic_basis())
                    .collect::<Vec<_>>(),
                &source_cut.scope,
                source_cut.currentness,
                &traversal_request,
            ),
        )?;
        let operation_id = stable_identity(
            "standing-curation-operation-v1",
            &(&authority, &rule_revision, &source_cut, &traversal_request),
        )?;
        Ok(Self {
            operation_id,
            selection_id,
            authority,
            rule_revision,
            source_cut,
            traversal_request,
            request_id: None,
            planned_authorization: None,
        })
    }

    /// Freeze a distinct request while retaining transport-insensitive replay.
    pub fn for_request(mut self, request_id: String) -> Result<Self, StorageError> {
        if request_id.trim().is_empty() || self.planned_authorization.is_some() {
            return invalid("Curation request must be named before authorization");
        }
        let base = Self::reconstruct(
            self.authority.clone(),
            self.rule_revision.clone(),
            self.source_cut.clone(),
            self.traversal_request.clone(),
        )?;
        self.selection_id = stable_identity(
            "requested-curation-selection-v1",
            &(&base.selection_id, &request_id),
        )?;
        self.operation_id = stable_identity(
            "requested-curation-operation-v1",
            &(&base.operation_id, &request_id),
        )?;
        self.request_id = Some(request_id);
        Ok(self)
    }

    /// Attach one exact durable Agent authorization without changing semantic identity.
    pub fn with_planned_authorization(
        mut self,
        authorization: CurationPlannedAuthorization,
    ) -> Result<Self, StorageError> {
        authorization.validate(&self)?;
        self.planned_authorization = Some(authorization);
        Ok(self)
    }

    /// Return the immutable semantic operation persisted by Curation.
    pub(crate) fn semantic_operation(&self) -> Self {
        let mut operation = self.clone();
        operation.planned_authorization = None;
        operation
    }

    pub fn validate(&self) -> Result<(), StorageError> {
        let mut expected = Self::reconstruct(
            self.authority.clone(),
            self.rule_revision.clone(),
            self.source_cut.clone(),
            self.traversal_request.clone(),
        )?;
        if let Some(request_id) = &self.request_id {
            expected = expected.for_request(request_id.clone())?;
        }
        if expected.operation_id != self.operation_id || expected.selection_id != self.selection_id
        {
            return invalid("standing Curation operation identity is not canonical");
        }
        if let Some(authorization) = &self.planned_authorization {
            authorization.validate(self)?;
        }
        Ok(())
    }
}

/// Durable pre-execution admission outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurationAdmissionDecision {
    Admitted,
    Rejected,
}

/// Exact acceptance or rejection receipt for one attempted operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationAcceptanceRecord {
    pub acceptance_id: String,
    pub operation_id: String,
    pub decision: CurationAdmissionDecision,
    pub reason: String,
    pub authority: CurationAuthority,
    /// Exact current authority observed when rejecting an obsolete offer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_authority: Option<CurationAuthority>,
    pub rule_revision: TheoryRevisionRef,
    pub source_cut_id: String,
}

impl CurationAcceptanceRecord {
    /// Decide new intake against the exact currently open native authority.
    pub fn for_current_authority(
        operation: &CurationOperation,
        rule: &StandingCurationRuleRevision,
        current: &CurationAuthority,
    ) -> Result<Self, StorageError> {
        current.validate()?;
        let mut receipt = Self::for_operation(operation, rule)?;
        if &operation.authority != current {
            receipt.decision = CurationAdmissionDecision::Rejected;
            receipt.reason = "operation authority does not match the open admission epoch".into();
            receipt.observed_authority = Some(current.clone());
            receipt.acceptance_id = stable_identity(
                "curation-epoch-acceptance-v1",
                &(
                    &operation.operation_id,
                    &receipt.decision,
                    &receipt.reason,
                    current,
                ),
            )?;
        }
        Ok(receipt)
    }

    pub fn for_operation(
        operation: &CurationOperation,
        rule: &StandingCurationRuleRevision,
    ) -> Result<Self, StorageError> {
        operation.validate()?;
        rule.validate()?;
        let mismatch = if operation.authority.agent_id != rule.rule.agent_id {
            Some("initiating Agent does not own the standing rule")
        } else if operation.rule_revision != rule.revision_ref() {
            Some("operation rule revision is not the installed revision")
        } else if operation.traversal_request != rule.rule.traversal_request().normalized() {
            Some("operation traversal request differs from the installed rule")
        } else if operation.source_cut.scope != rule.rule.scope {
            Some("operation source cut differs from the installed rule scope")
        } else if operation.source_cut.owners != expected_cut_owners(rule) {
            Some("operation source cut has undeclared owner requirements")
        } else if !complete_cut_matches_declared_owners(&operation.source_cut) {
            Some("operation source cut is not complete for its declared owners")
        } else if let Some(reason) = rule.rule.authority_mismatch(&operation.authority) {
            Some(reason)
        } else if let Some(authorization) = &operation.planned_authorization {
            authorization
                .validate(operation)
                .err()
                .map(|_| "planned authorization is invalid")
        } else {
            None
        };
        let decision = if mismatch.is_some() {
            CurationAdmissionDecision::Rejected
        } else {
            CurationAdmissionDecision::Admitted
        };
        let reason = mismatch
            .unwrap_or("standing operation admitted")
            .to_string();
        let acceptance_id = stable_identity(
            "standing-curation-acceptance-v1",
            &(&operation.operation_id, &decision, &reason),
        )?;
        Ok(Self {
            acceptance_id,
            operation_id: operation.operation_id.clone(),
            decision,
            reason,
            authority: operation.authority.clone(),
            observed_authority: None,
            rule_revision: operation.rule_revision.clone(),
            source_cut_id: operation.source_cut.cut_id.clone(),
        })
    }
}

pub(crate) fn expected_cut_owners(
    rule: &StandingCurationRuleRevision,
) -> Vec<crate::world_state::graph::contracts::TraversalOwnerRequirement> {
    let mut owners = vec![
        crate::world_state::graph::contracts::TraversalOwnerRequirement {
            event_source: rule.rule.source_event_route.clone(),
            owner_id: rule.rule.source_owner_id.clone(),
            scope: rule.rule.scope.clone(),
            required: true,
        },
        crate::world_state::graph::contracts::TraversalOwnerRequirement {
            event_source: None,
            owner_id: CURATION_OWNER_ID.to_string(),
            scope: rule.rule.scope.clone(),
            required: false,
        },
    ];
    owners.sort();
    owners
}

fn complete_cut_matches_declared_owners(cut: &TraversalCut) -> bool {
    if cut.status != crate::world_state::graph::contracts::TraversalCutStatus::Complete
        || !cut.issues.is_empty()
        || cut.event_position.ledger_id != cut.graph_position.ledger_id
        || cut.graph_position.after_seq > cut.event_position.after_seq
    {
        return false;
    }
    for requirement in &cut.owners {
        let matching = cut
            .receipts
            .iter()
            .filter(|receipt| {
                receipt.owner_id == requirement.owner_id && receipt.scope == requirement.scope
            })
            .count();
        if requirement.required && matching != 1 || !requirement.required && matching > 1 {
            return false;
        }
    }
    cut.receipts.iter().all(|receipt| {
        cut.owners.iter().any(|requirement| {
            requirement.owner_id == receipt.owner_id
                && requirement.scope == receipt.scope
                && requirement.event_source.as_ref()
                    == receipt
                        .event_coverage
                        .as_ref()
                        .map(|coverage| &coverage.source)
        }) && receipt.completeness.status
            == crate::world_state::graph::contracts::OwnerCompletenessStatus::Complete
            && receipt.completeness.scope == receipt.scope
            && receipt.source_event.is_none_or(|event| {
                event.ledger_id == cut.event_position.ledger_id
                    && event.seq > 0
                    && event.seq <= cut.event_position.after_seq
            })
            && match &receipt.event_coverage {
                Some(coverage) => {
                    coverage.through == cut.event_position
                        && cut.owners.iter().any(|owner| {
                            owner.owner_id == receipt.owner_id
                                && owner.scope == receipt.scope
                                && owner.event_source.as_ref() == Some(&coverage.source)
                        })
                }
                None => receipt.source_event.is_some(),
            }
            && receipt.projection_position.ledger_id == cut.graph_position.ledger_id
            && receipt.projection_position.after_seq <= cut.graph_position.after_seq
    })
}

/// Terminal disposition for admitted standing work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurationTerminalDisposition {
    Applied,
    Unchanged,
    Abstained,
    Incomplete,
    Conflicted,
    Failed,
}

/// Durable terminal result and complete publication intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationResult {
    pub result_id: String,
    pub operation_id: String,
    pub disposition: CurationTerminalDisposition,
    pub reason: String,
    pub source_cut_id: String,
    pub source_event_position: LedgerCursor,
    pub semantic_publication: Option<OwnerPublicationOperation>,
    pub cited_publication_ids: Vec<String>,
    pub frontier: Vec<TraversalFrontierEntry>,
    pub exclusions: Vec<OwnerPublicationExclusion>,
    pub failures: Vec<String>,
}

impl CurationResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation: &CurationOperation,
        disposition: CurationTerminalDisposition,
        reason: impl Into<String>,
        semantic_publication: Option<OwnerPublicationOperation>,
        mut cited_publication_ids: Vec<String>,
        mut frontier: Vec<TraversalFrontierEntry>,
        mut exclusions: Vec<OwnerPublicationExclusion>,
        mut failures: Vec<String>,
    ) -> Result<Self, StorageError> {
        operation.validate()?;
        let reason = reason.into();
        require_non_empty("Curation terminal reason", &reason)?;
        cited_publication_ids.sort();
        cited_publication_ids.dedup();
        frontier.sort();
        frontier.dedup();
        exclusions.sort();
        exclusions.dedup();
        failures.sort();
        failures.dedup();
        let result_id = stable_identity(
            "standing-curation-result-v1",
            &(&operation.operation_id, disposition),
        )?;
        let result = Self {
            result_id,
            operation_id: operation.operation_id.clone(),
            disposition,
            reason,
            source_cut_id: operation.source_cut.cut_id.clone(),
            source_event_position: operation.source_cut.event_position,
            semantic_publication,
            cited_publication_ids,
            frontier,
            exclusions,
            failures,
        };
        result.validate(operation)?;
        Ok(result)
    }

    pub fn validate(&self, operation: &CurationOperation) -> Result<(), StorageError> {
        operation.validate()?;
        if self.operation_id != operation.operation_id
            || self.source_cut_id != operation.source_cut.cut_id
            || self.source_event_position != operation.source_cut.event_position
        {
            return invalid("Curation result does not match its admitted operation");
        }
        if self.result_id
            != stable_identity(
                "standing-curation-result-v1",
                &(&operation.operation_id, self.disposition),
            )?
        {
            return invalid("Curation result identity is not canonical");
        }
        require_non_empty("Curation terminal reason", &self.reason)?;
        if self.disposition == CurationTerminalDisposition::Incomplete
            && self.frontier.is_empty()
            && self.failures.is_empty()
        {
            return invalid("incomplete Curation result must explain its bound");
        }
        if self.disposition == CurationTerminalDisposition::Failed && self.failures.is_empty() {
            return invalid("failed Curation result must name an operational failure");
        }
        match self.disposition {
            CurationTerminalDisposition::Applied => {
                let Some(publication) = &self.semantic_publication else {
                    return invalid("applied Curation result requires semantic publication");
                };
                publication.validate()?;
                let mut publication_ids = publication
                    .batch
                    .objects
                    .iter()
                    .map(|object| object.publication_id.clone())
                    .chain(
                        publication
                            .batch
                            .relations
                            .iter()
                            .map(|occurrence| occurrence.occurrence_id.clone()),
                    )
                    .collect::<Vec<_>>();
                publication_ids.sort();
                publication_ids.dedup();
                if publication.batch.owner_id != CURATION_OWNER_ID
                    || publication.batch.revision_id != self.result_id
                    || publication.batch.scope != operation.source_cut.scope
                    || publication_ids != self.cited_publication_ids
                    || publication
                        .batch
                        .objects
                        .iter()
                        .any(|object| object.source_product_ref != operation.operation_id)
                    || publication
                        .batch
                        .relations
                        .iter()
                        .any(|occurrence| occurrence.source_product_ref != operation.operation_id)
                {
                    return invalid(
                        "Curation semantic publication does not match its result lineage",
                    );
                }
            }
            CurationTerminalDisposition::Unchanged => {
                if self.semantic_publication.is_some() || self.cited_publication_ids.is_empty() {
                    return invalid(
                        "unchanged Curation result must cite state without republishing it",
                    );
                }
            }
            CurationTerminalDisposition::Abstained
            | CurationTerminalDisposition::Incomplete
            | CurationTerminalDisposition::Conflicted
            | CurationTerminalDisposition::Failed => {
                if self.semantic_publication.is_some() {
                    return invalid(
                        "non-authoring terminal result cannot carry semantic publication",
                    );
                }
            }
        }
        Ok(())
    }

    pub fn event_record_id(&self) -> String {
        format!("curation-result::{}", self.result_id)
    }

    pub fn event_envelope(&self, session_id: &str) -> Result<EventEnvelope, StorageError> {
        let data = serde_json::to_value(self).map_err(to_storage_data)?;
        let objects = self
            .semantic_publication
            .as_ref()
            .map(|publication| {
                publication
                    .batch
                    .objects
                    .iter()
                    .map(|object| object.object_ref.clone())
                    .collect()
            })
            .unwrap_or_default();
        Ok(EventEnvelope::with_now_domain(
            session_id,
            CURATION_OWNER_ID,
            self.operation_id.clone(),
            CURATION_RESULT_EVENT_TYPE,
            None,
            data,
        )
        .with_record_id(self.event_record_id())
        .with_graph(objects, Vec::new()))
    }
}

/// Kind of deterministic Event publication required by a terminal result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurationPublicationKind {
    Semantic,
    Terminal,
}

/// Durable receipt recorded after the Event authority accepts one publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationPublicationReceipt {
    pub result_id: String,
    pub kind: CurationPublicationKind,
    pub event_record_id: String,
    pub event_position: LedgerCursor,
}

/// Event port owned by Curation and implemented by root composition.
pub trait CurationEventPort: Send + Sync {
    fn watermark(&self) -> Result<EventWatermark, String>;
    fn append_idempotent(&self, envelope: EventEnvelope) -> Result<AppendReceipt, String>;
}

/// Exact Traversal query port consumed by Curation.
pub trait CurationTraversalPort: Send + Sync {
    fn cut(&self, request: &TraversalCutRequest) -> Result<TraversalCut, StorageError>;

    fn traverse(
        &self,
        cut: &TraversalCut,
        request: &BoundedTraversalRequest,
    ) -> Result<TraversalResult, StorageError>;
}

/// Diagnostic account for one bounded standing Curation step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurationStepReport {
    pub actor_id: String,
    pub input_after_seq: u64,
    pub output_after_seq: u64,
    pub operations_attempted: usize,
    pub acceptances_persisted: usize,
    pub results_persisted: usize,
    pub publications_appended: usize,
    pub reused_results: usize,
    pub retryable_errors: Vec<String>,
    pub fatal_errors: Vec<String>,
    pub waiting_on: Vec<crate::WaitingOnDeclaration>,
}

impl CurationStepReport {
    pub(crate) fn new(actor_id: &str) -> Self {
        Self {
            actor_id: actor_id.to_string(),
            input_after_seq: 0,
            output_after_seq: 0,
            operations_attempted: 0,
            acceptances_persisted: 0,
            results_persisted: 0,
            publications_appended: 0,
            reused_results: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            waiting_on: Vec::new(),
        }
    }
}

pub(crate) fn stable_identity(
    namespace: &str,
    value: &impl Serialize,
) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(value).map_err(to_storage_data)?;
    Ok(format!("{namespace}::{}", blake3::hash(&bytes).to_hex()))
}

pub(crate) fn qualifications(
    operation: &CurationOperation,
    rule: &StandingCurationRuleRevision,
) -> BTreeMap<String, String> {
    let mut qualifications = BTreeMap::from([
        ("agent_id".to_string(), operation.authority.agent_id.clone()),
        (
            "perspective".to_string(),
            operation.authority.perspective.index_key(),
        ),
        (
            "branch_id".to_string(),
            operation.authority.branch_scope.branch_id.clone(),
        ),
        (
            "activation_generation".to_string(),
            operation.authority.activation_generation.clone(),
        ),
        ("rule_revision".to_string(), rule.content_hash.clone()),
        (
            "source_cut".to_string(),
            operation.source_cut.cut_id.clone(),
        ),
        (
            "output_policy_revision".to_string(),
            rule.rule.output_policy_revision.clone(),
        ),
    ]);
    if let Some(epoch) = &operation.authority.admission_epoch {
        qualifications.insert("admission_epoch".into(), epoch.clone());
    }
    qualifications
}

fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        invalid(&format!("{label} must be non-empty"))
    } else {
        Ok(())
    }
}

fn invalid<T>(message: &str) -> Result<T, StorageError> {
    Err(StorageError::InvalidPath(message.to_string()))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::InvalidPath(format!("cannot encode Curation identity: {error}"))
}

trait CurationTraversalValidation {
    fn validate_for_curation(&self) -> Result<(), StorageError>;
}

impl CurationTraversalValidation for TraversalBounds {
    fn validate_for_curation(&self) -> Result<(), StorageError> {
        if self.max_depth == 0
            || self.max_objects == 0
            || self.max_occurrences == 0
            || self.max_paths == 0
        {
            invalid("standing Curation traversal bounds must be greater than zero")
        } else {
            Ok(())
        }
    }
}

impl CurationTraversalValidation for BoundedTraversalRequest {
    fn validate_for_curation(&self) -> Result<(), StorageError> {
        if self.roots.is_empty() {
            return invalid("standing Curation traversal requires at least one root");
        }
        for root in &self.roots {
            root.validate()?;
        }
        self.bounds.validate_for_curation()
    }
}
