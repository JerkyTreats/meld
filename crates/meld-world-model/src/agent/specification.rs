//! Native Agent identity for an epoch-specific desired observation and frozen inputs.

use meld_events::DomainObjectRef;
use meld_lang::TaskInput;
use serde::{Deserialize, Serialize};

use super::{AgentAuthorizationFence, AgentGenesisIntentV1, AgentReconciliationIntent};
use crate::curation::{CurationAuthority, StandingCurationRuleRevision};
use crate::error::StorageError;

/// Reasoning identity exists before either Goal inception or executable work.
/// Source owners receive opaque correlations; they do not decide the Agent's Goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentEpochSpecification {
    pub specification_id: String,
    pub goal_id: String,
    pub intent: AgentReconciliationIntent,
    pub authority: CurationAuthority,
    pub fence: AgentAuthorizationFence,
    pub genesis: AgentGenesisIntentV1,
}

impl AgentEpochSpecification {
    /// Agent-authored opaque correlation references for independently owned effects.
    pub fn effect_correlations(&self) -> Vec<String> {
        vec![
            format!("assignment::{}", self.genesis.assignment_id),
            format!(
                "compilation::{}",
                self.genesis.product_compilation_receipt_id
            ),
            format!("goal::{}", self.goal_id),
            self.specification_id.clone(),
        ]
    }

    pub(crate) fn new(
        intent: AgentReconciliationIntent,
        authority: CurationAuthority,
        fence: AgentAuthorizationFence,
        genesis: AgentGenesisIntentV1,
    ) -> Result<Self, StorageError> {
        let goal_id = intent.goal_id(&authority.agent_id, &fence.reconciliation_scope());
        let specification_id =
            specification_identity(&goal_id, &intent, &authority, &fence, &genesis)?;
        let value = Self {
            specification_id,
            goal_id,
            intent,
            authority,
            fence,
            genesis,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), StorageError> {
        self.authority.validate()?;
        let AgentReconciliationIntent::MaintainedCondition(condition) = &self.intent else {
            return Err(invalid(
                "epoch specification requires a maintained condition",
            ));
        };
        condition.validate()?;
        let canonical_genesis = AgentGenesisIntentV1::new(
            self.genesis.assignment_id.clone(),
            self.genesis.product_compilation_receipt_id.clone(),
            self.genesis.topology_position_id.clone(),
            self.genesis.installed_owner_revisions.clone(),
            self.genesis.registration.clone(),
            self.genesis.required_subscriptions.clone(),
        )?;
        if self.genesis != canonical_genesis
            || self
                .fence
                .admission_epoch
                .as_deref()
                .is_none_or(|epoch| epoch.trim().is_empty())
            || self.fence.authority_policy_content_hash.trim().is_empty()
            || self.authority.activation_generation != self.fence.activation_generation
            || self.authority.admission_epoch != self.fence.admission_epoch
            || self.genesis.registration.agent_id != self.authority.agent_id
            || self.genesis.registration.subject != self.authority.subject
            || self.genesis.registration.perspective_key != self.authority.perspective
            || self.genesis.registration.branch_scope != self.authority.branch_scope
            || !self
                .genesis
                .installed_owner_revisions
                .contains(&condition.revision)
            || self.genesis.registration.maintained_condition.as_ref() != Some(condition)
            || self.goal_id
                != self
                    .intent
                    .goal_id(&self.authority.agent_id, &self.fence.reconciliation_scope())
            || self.specification_id
                != specification_identity(
                    &self.goal_id,
                    &self.intent,
                    &self.authority,
                    &self.fence,
                    &self.genesis,
                )?
        {
            return Err(invalid(
                "Agent epoch specification differs from its native lineage",
            ));
        }
        Ok(())
    }
}

/// Owner-prepared values selected for this exact reasoning identity.
/// Observation does not replace the subject on which the Agent can act.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentEpochProducts {
    pub specification: AgentEpochSpecification,
    pub observation_subject: DomainObjectRef,
    pub task_inputs: Vec<TaskInput>,
    pub curation_rule: StandingCurationRuleRevision,
}

impl AgentEpochProducts {
    /// Rebind declared source relationships to this epoch's exact observation.
    pub fn subscription_requests(
        &self,
    ) -> Result<Vec<super::AgentSubscriptionRequestV1>, StorageError> {
        self.validate()?;
        self.specification
            .genesis
            .required_subscriptions
            .iter()
            .map(|request| {
                request.validate()?;
                let mut key = request.belief_key.clone();
                key.subject = self.observation_subject.clone();
                super::AgentSubscriptionRequestV1::new(
                    request.agent_id.clone(),
                    request.source_owner.clone(),
                    request.source_contract_revision.clone(),
                    key,
                    request.initial_cursor_policy.clone(),
                )
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), StorageError> {
        self.specification.validate()?;
        self.observation_subject.validate()?;
        self.curation_rule.validate()?;
        self.curation_rule
            .rule
            .validate_authority(&self.specification.authority)?;
        let mut slots = std::collections::BTreeSet::new();
        for input in &self.task_inputs {
            input.validate().map_err(StorageError::InvalidPath)?;
            if !slots.insert((&input.step_id, &input.slot_id)) {
                return Err(invalid("epoch inputs repeat a consumer slot"));
            }
        }
        Ok(())
    }
}

/// Prepare owner products without executing their Task or judging desired state.
pub trait AgentEpochPreparationPort: Send + Sync {
    fn prepare(
        &self,
        specification: &AgentEpochSpecification,
    ) -> Result<AgentEpochProducts, StorageError>;
}

/// Source-owned acceptance of a native Agent's exact observation relationship.
pub trait AgentEpochSubscriptionPort: Send + Sync {
    fn subscribe(
        &self,
        request: &super::AgentSubscriptionRequestV1,
    ) -> Result<crate::belief::BeliefSubscriptionAcceptanceProof, StorageError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEpochSubscriptionReceipt {
    pub request_id: String,
    pub acceptance_id: String,
    pub agent_id: String,
    pub belief_key: crate::belief::BeliefKey,
}

/// One source of reconciliation products, selected before the actor starts.
#[derive(Clone)]
pub enum AgentPreparation {
    InstalledRule(Box<StandingCurationRuleRevision>),
    Epoch {
        products: std::sync::Arc<dyn AgentEpochPreparationPort>,
        subscriptions: std::sync::Arc<dyn AgentEpochSubscriptionPort>,
    },
}

impl From<StandingCurationRuleRevision> for AgentPreparation {
    fn from(rule: StandingCurationRuleRevision) -> Self {
        Self::InstalledRule(Box::new(rule))
    }
}

fn invalid(detail: &str) -> StorageError {
    StorageError::InvalidPath(detail.into())
}

fn specification_identity(
    goal_id: &str,
    intent: &AgentReconciliationIntent,
    authority: &CurationAuthority,
    fence: &AgentAuthorizationFence,
    genesis: &AgentGenesisIntentV1,
) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(&(goal_id, intent, authority, fence, genesis))
        .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
    Ok(format!(
        "agent-epoch-specification-v1::{}",
        blake3::hash(&bytes).to_hex()
    ))
}
