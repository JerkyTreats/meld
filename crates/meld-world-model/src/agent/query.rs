//! Read-only agent query facade.

use crate::agent::contracts::{
    AgentCurationDecision, AgentCurationDedupeKey, AgentCurationOutcome, AgentDecisionOutboxRecord,
    AgentHydrationCheckpoint, AgentRecord, AgentSatisfactionCursorIdentity,
    AgentSatisfactionReview, AgentSatisfactionReviewCursor, AgentSemanticEnablementAudit,
    AgentStatus, AgentSubscriptionRecord,
};
use crate::agent::store::AgentStore;
use crate::error::StorageError;

/// Read-only facade for agent records, subscriptions, and decisions.
pub struct AgentQuery<'a> {
    store: &'a AgentStore,
}

impl<'a> AgentQuery<'a> {
    /// Create a query facade over durable agent storage.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Read one agent record by id.
    pub fn agent(&self, agent_id: &str) -> Result<Option<AgentRecord>, StorageError> {
        self.store.get_agent(agent_id)
    }

    /// List agents with a status in deterministic order.
    pub fn agents_by_status(&self, status: AgentStatus) -> Result<Vec<AgentRecord>, StorageError> {
        self.store.agents_by_status(status)
    }

    /// List subscriptions owned by an agent.
    pub fn subscriptions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentSubscriptionRecord>, StorageError> {
        self.store.subscriptions_for_agent(agent_id)
    }

    /// Read one subscription record by id.
    pub fn subscription(
        &self,
        subscription_id: &str,
    ) -> Result<Option<AgentSubscriptionRecord>, StorageError> {
        self.store.get_subscription(subscription_id)
    }

    /// List active subscriptions for an agent.
    pub fn pending_subscriptions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentSubscriptionRecord>, StorageError> {
        self.store.pending_subscriptions(agent_id)
    }

    /// Return the most recent curation decisions for an agent.
    pub fn recent_decisions(
        &self,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<AgentCurationDecision>, StorageError> {
        self.store.recent_decisions(agent_id, limit)
    }

    /// Return the latest decision for a dedupe key.
    pub fn decision_by_dedupe_key(
        &self,
        dedupe_key: &AgentCurationDedupeKey,
    ) -> Result<Option<AgentCurationDecision>, StorageError> {
        self.store.decision_by_dedupe_key(dedupe_key)
    }

    /// Return the decision recorded for one satisfaction review.
    pub fn decision_by_satisfaction_review(
        &self,
        review: &AgentSatisfactionReview,
    ) -> Result<Option<AgentCurationDecision>, StorageError> {
        self.store.decision_by_satisfaction_review(review)
    }

    /// Return one complete durable outcome including its exact command payload.
    pub fn curation_outcome(
        &self,
        decision_id: &str,
    ) -> Result<AgentCurationOutcome, StorageError> {
        self.store.outcome_for_decision(decision_id)
    }

    /// Return the durable command outbox for one decision.
    pub fn decision_outbox(
        &self,
        decision_id: &str,
    ) -> Result<Option<AgentDecisionOutboxRecord>, StorageError> {
        self.store.decision_outbox(decision_id)
    }

    /// Read one independent satisfaction-review cursor.
    pub fn satisfaction_review_cursor(
        &self,
        identity: &AgentSatisfactionCursorIdentity,
    ) -> Result<Option<AgentSatisfactionReviewCursor>, StorageError> {
        self.store.satisfaction_review_cursor(identity)
    }

    /// Read one resumable hydration checkpoint.
    pub fn hydration_checkpoint(
        &self,
        hydration_id: &str,
    ) -> Result<Option<AgentHydrationCheckpoint>, StorageError> {
        self.store.hydration_checkpoint(hydration_id)
    }

    /// Audit durable state before recurring semantic actors are enabled.
    pub fn semantic_enablement_audit(&self) -> Result<AgentSemanticEnablementAudit, StorageError> {
        self.store.audit_semantic_enablement()
    }
}
