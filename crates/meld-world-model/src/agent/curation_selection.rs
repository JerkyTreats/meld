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
        self.select_next(authority, None)
    }

    fn select_next(
        &self,
        authority: &CurationAuthority,
        after: Option<&crate::belief::TheoryRevisionRef>,
    ) -> Result<CurationRuleSelection, StorageError> {
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
        let intent = AgentReconciliationIntent::MaintainedCondition(condition);
        let goal_id = super::AgentEpochSpecification::goal_identity(
            &intent,
            &genesis,
            &authority.activation_generation,
            authority.admission_epoch.as_deref(),
        );
        let mut rules = Vec::new();
        for products in self.store.epoch_products_for_agent(&self.agent_id)? {
            if let Some(request) = &products.specification.request {
                if self.store.reconciliation_request_completed(request)? {
                    continue;
                }
            } else if products.specification.goal_id != goal_id {
                continue;
            }
            if (!products.specification.is_prepared_request()
                && products.specification.authority != *authority)
                || products.specification.intent != intent
                || products.specification.genesis != genesis
            {
                return Err(StorageError::InvalidPath(
                    "retained Curation selection names another epoch lineage".into(),
                ));
            }
            rules.push(products.curation_rule);
        }
        rules.sort_by(|left, right| {
            (&left.rule_id, &left.content_hash).cmp(&(&right.rule_id, &right.content_hash))
        });
        if !rules.is_empty() {
            let index = after
                .and_then(|after| {
                    rules.iter().position(|rule| {
                        (&rule.rule_id, &rule.content_hash) > (&after.id, &after.content_hash)
                    })
                })
                .unwrap_or(0);
            return Ok(CurationRuleSelection::Selected(Box::new(
                rules.swap_remove(index),
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

    fn template_refs(&self) -> Result<Vec<crate::belief::TheoryRevisionRef>, StorageError> {
        Ok(self
            .genesis()?
            .installed_owner_revisions
            .into_iter()
            .filter(|reference| {
                reference.registry == crate::curation::CURATION_TEMPLATE_REGISTRY_ID
            })
            .collect())
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
