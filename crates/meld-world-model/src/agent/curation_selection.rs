//! Agent-owned selection of the Curation rule retained for an admission epoch.

use std::sync::Arc;

use super::{AgentReconciliationIntent, AgentStore};
use crate::curation::{CurationAuthority, CurationRuleSelection, CurationRuleSelectionPort};
use crate::error::StorageError;
use crate::waiting::{StructuralWakeAddress, WaitingOnDeclaration};

pub struct AgentEpochCurationSource {
    store: Arc<AgentStore>,
    agent_id: String,
}

impl AgentEpochCurationSource {
    pub fn new(store: Arc<AgentStore>, agent_id: String) -> Result<Self, StorageError> {
        let source = Self { store, agent_id };
        source.genesis()?;
        Ok(source)
    }

    fn genesis(&self) -> Result<super::AgentGenesisIntentV1, StorageError> {
        self.store
            .genesis_intent_for_agent(&self.agent_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "epoch Curation source has no native Agent genesis".into(),
                )
            })
    }

    fn address(&self) -> String {
        format!("agent-epoch-products::{}", self.agent_id)
    }
}

impl CurationRuleSelectionPort for AgentEpochCurationSource {
    fn select(&self, authority: &CurationAuthority) -> Result<CurationRuleSelection, StorageError> {
        authority.validate()?;
        let genesis = self.genesis()?;
        let registration = &genesis.registration;
        if authority.agent_id != self.agent_id
            || authority.subject != registration.subject
            || authority.perspective != registration.perspective_key
            || authority.branch_scope != registration.branch_scope
        {
            return Err(StorageError::InvalidPath(
                "Curation selection names another Agent judgment scope".into(),
            ));
        }
        let condition = registration.maintained_condition.clone().ok_or_else(|| {
            StorageError::InvalidPath("epoch Curation source has no maintained condition".into())
        })?;
        let scope = super::AgentAuthorizationFence::scope_for(
            &authority.activation_generation,
            authority.admission_epoch.as_deref(),
        );
        let goal_id = AgentReconciliationIntent::MaintainedCondition(condition)
            .goal_id(&self.agent_id, &scope);
        if let Some(products) = self.store.epoch_products(&goal_id)? {
            if products.specification.authority != *authority
                || products.specification.genesis != genesis
            {
                return Err(StorageError::InvalidPath(
                    "retained Curation selection names another epoch lineage".into(),
                ));
            }
            return Ok(CurationRuleSelection::Selected(Box::new(
                products.curation_rule,
            )));
        }
        Ok(CurationRuleSelection::Waiting(WaitingOnDeclaration::about(
            "agent_epoch_preparation_pending",
            goal_id,
            "standing Curation awaits the Agent's exact epoch specification",
            vec![StructuralWakeAddress::OwnerRevision(format!(
                "world-model::{}::{}::after::{}",
                self.store.resource_id(),
                self.address(),
                self.store.reconciliation_position(),
            ))],
        )))
    }

    fn binding_refs(&self) -> Result<Vec<String>, StorageError> {
        let genesis = self.genesis()?;
        Ok(vec![
            genesis.intent_id,
            format!(
                "world-model::{}::{}::after::{}",
                self.store.resource_id(),
                self.address(),
                self.store.reconciliation_position()
            ),
        ])
    }

    fn resolves_wake(&self, wake: &StructuralWakeAddress) -> Result<bool, String> {
        let StructuralWakeAddress::OwnerRevision(value) = wake else {
            return Ok(false);
        };
        let Some(value) = crate::waiting::bound_address(value, self.store.resource_id()) else {
            return Ok(false);
        };
        Ok(crate::waiting::after_position(value, &self.address())
            && self
                .genesis()
                .map_err(|error| error.to_string())?
                .registration
                .agent_id
                == self.agent_id)
    }
}
