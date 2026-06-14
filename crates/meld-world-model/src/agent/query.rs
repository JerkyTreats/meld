//! Read-only agent query facade.

use crate::agent::contracts::{
    AgentCurationDecision, AgentCurationDedupeKey, AgentRecord, AgentSatisfactionReview,
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
}
