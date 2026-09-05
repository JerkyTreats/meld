//! Agent registration commands.

use crate::agent::contracts::{AgentRecord, AgentStatus, SeedAgentRegistration};
use crate::agent::store::AgentStore;
use crate::error::StorageError;

/// Write facade for seed agent registration and readiness transitions.
pub struct AgentRegistration<'a> {
    store: &'a AgentStore,
}

impl<'a> AgentRegistration<'a> {
    /// Create a registration facade over durable agent storage.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Register a seed agent idempotently.
    pub fn register_seed_agent(
        &self,
        request: SeedAgentRegistration,
    ) -> Result<AgentRecord, StorageError> {
        request.validate()?;
        if let Some(existing) = self.store.get_agent(&request.agent_id)? {
            if existing.perspective_key != request.perspective_key
                || existing.subject != request.subject
                || existing.branch_scope != request.branch_scope
                || existing.observation_scope != request.observation_scope
                || existing.directive != request.directive
                || existing.seed_provenance != request.seed_provenance
                || existing.curation_rule != request.curation_rule
                || existing.curation_rule_revision != request.curation_rule_revision
                || existing.maintained_condition != request.maintained_condition
                || existing.maintained_condition_revision != request.maintained_condition_revision
            {
                return Err(StorageError::InvalidPath(format!(
                    "Agent '{}' already exists with a different immutable genesis identity",
                    request.agent_id
                )));
            }
            return Ok(existing);
        }
        let record = AgentRecord {
            agent_id: request.agent_id,
            perspective_key: request.perspective_key,
            subject: request.subject,
            branch_scope: request.branch_scope,
            observation_scope: request.observation_scope,
            directive: request.directive,
            seed_provenance: request.seed_provenance,
            curation_rule: request.curation_rule,
            curation_rule_revision: request.curation_rule_revision,
            maintained_condition: request.maintained_condition,
            maintained_condition_revision: request.maintained_condition_revision,
            status: AgentStatus::Registered,
            created_at_seq: request.created_at_seq,
            updated_at_seq: request.created_at_seq,
        };
        self.store.put_agent(&record)?;
        Ok(record)
    }

    /// Mark an agent operational after it has at least one active subscription.
    pub fn mark_operational(
        &self,
        agent_id: &str,
        updated_at_seq: u64,
    ) -> Result<AgentRecord, StorageError> {
        let Some(mut record) = self.store.get_agent(agent_id)? else {
            return Err(StorageError::InvalidPath(format!(
                "unknown agent '{agent_id}'"
            )));
        };
        if self.store.pending_subscriptions(agent_id)?.is_empty() {
            return Err(StorageError::InvalidPath(format!(
                "agent '{agent_id}' has no active subscriptions"
            )));
        }
        record.status = AgentStatus::Operational;
        record.updated_at_seq = updated_at_seq;
        self.store.put_agent(&record)?;
        Ok(record)
    }
}
