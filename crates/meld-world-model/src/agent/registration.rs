//! Agent registration commands.

use crate::agent::contracts::AgentHydrationCheckpoint;
use crate::agent::contracts::{AgentRecord, AgentStatus, SeedAgentRegistration};
use crate::agent::hydration::{
    AgentProcessHydrationRecord, FailAgentHydrationCommand, MarkAgentOperationalCommand,
    StartAgentHydrationCommand,
};
use crate::agent::store::{AgentHydrationOwnerFence, AgentStore};
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
            let matches = existing.perspective_key == request.perspective_key
                && existing.subject == request.subject
                && existing.branch_scope == request.branch_scope
                && existing.observation_scope == request.observation_scope
                && existing.directive_id == request.directive_id
                && existing.seed_provenance == request.seed_provenance;
            if matches {
                return Ok(existing);
            }
            return Err(StorageError::InvalidPath(format!(
                "seed agent '{}' registration conflict",
                request.agent_id
            )));
        }
        let record = AgentRecord {
            agent_id: request.agent_id,
            perspective_key: request.perspective_key,
            subject: request.subject,
            branch_scope: request.branch_scope,
            observation_scope: request.observation_scope,
            directive_id: request.directive_id,
            seed_provenance: request.seed_provenance,
            status: AgentStatus::Registered,
            created_at_seq: request.created_at_seq,
            updated_at_seq: request.created_at_seq,
        };
        self.store.put_agent(&record)?;
        Ok(record)
    }

    pub(crate) fn mark_operational_fenced(
        &self,
        command: &MarkAgentOperationalCommand,
        owner: &AgentHydrationOwnerFence,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<AgentRecord, StorageError> {
        self.store
            .mark_agent_operational_fenced(command, owner, checkpoint)
    }

    pub(crate) fn start_hydration_fenced(
        &self,
        command: &StartAgentHydrationCommand,
        owner: &AgentHydrationOwnerFence,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        self.store.start_process_hydration_fenced(command, owner)
    }

    pub(crate) fn fail_hydration_fenced(
        &self,
        command: &FailAgentHydrationCommand,
        owner: &AgentHydrationOwnerFence,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        self.store
            .fail_process_hydration_fenced(command, owner, checkpoint)
    }
}
