//! Agent subscription commands and cursor rules.

use crate::agent::contracts::{
    deterministic_id, AgentSubscriptionRecord, AgentSubscriptionStatus, SubscribeAgentCommand,
};
use crate::agent::store::AgentStore;
use crate::error::StorageError;

/// Write facade for subscription binding and cursor advancement.
pub struct AgentSubscription<'a> {
    store: &'a AgentStore,
}

impl<'a> AgentSubscription<'a> {
    /// Create a subscription facade over durable agent storage.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Bind an agent to a belief stream idempotently.
    pub fn subscribe(
        &self,
        command: SubscribeAgentCommand,
    ) -> Result<AgentSubscriptionRecord, StorageError> {
        command.validate()?;
        if self.store.get_agent(&command.agent_id)?.is_none() {
            return Err(StorageError::InvalidPath(format!(
                "unknown agent '{}'",
                command.agent_id
            )));
        }
        if let Some(existing) = self
            .store
            .subscription_by_agent_and_key(&command.agent_id, &command.belief_key)?
        {
            return Ok(existing);
        }
        let natural_key =
            AgentSubscriptionRecord::natural_key(&command.agent_id, &command.belief_key);
        let record = AgentSubscriptionRecord {
            subscription_id: deterministic_id("subscription", &natural_key),
            agent_id: command.agent_id,
            belief_key: command.belief_key,
            status: AgentSubscriptionStatus::Active,
            last_delivered_revision_id: None,
            last_delivered_seq: 0,
            created_at_seq: command.created_at_seq,
            updated_at_seq: command.created_at_seq,
        };
        self.store.put_subscription(&record)?;
        Ok(record)
    }
}
