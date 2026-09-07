//! Named requests to reconcile an installed intent, without granting work authority.

use serde::{Deserialize, Serialize};

use super::{AgentGenesisIntentV1, AgentReconciliationIntent};
use crate::error::StorageError;

/// A caller's stable key distinguishes requests beneath one exact native genesis.
/// Repeating the key returns the original request, including after completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReconciliationRequest {
    pub request_id: String,
    pub agent_id: String,
    pub genesis_intent_id: String,
    pub request_key: String,
}

/// Native completion evidence projected beside its original intake record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReconciliationRequestStatus {
    pub request: AgentReconciliationRequest,
    pub completed: bool,
}

impl AgentReconciliationRequest {
    pub(crate) fn new(
        genesis: &AgentGenesisIntentV1,
        request_key: String,
    ) -> Result<Self, StorageError> {
        if request_key.trim().is_empty() || genesis.registration.maintained_condition.is_none() {
            return Err(StorageError::InvalidPath(
                "reconciliation request requires a named key and an installed maintained intent"
                    .into(),
            ));
        }
        if genesis
            .registration
            .maintained_condition
            .as_ref()
            .is_some_and(|binding| {
                binding.condition.observation_scope == super::AgentObservationScope::AdmissionEpoch
            })
        {
            return Err(StorageError::InvalidPath(
                "admission-epoch observations are initiated by lifecycle, not explicit requests"
                    .into(),
            ));
        }
        let bytes = serde_json::to_vec(&(&genesis.intent_id, &request_key))
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        Ok(Self {
            request_id: format!("agent-request-v1::{}", blake3::hash(&bytes).to_hex()),
            agent_id: genesis.registration.agent_id.clone(),
            genesis_intent_id: genesis.intent_id.clone(),
            request_key,
        })
    }

    pub fn validate_for(&self, genesis: &AgentGenesisIntentV1) -> Result<(), StorageError> {
        if Self::new(genesis, self.request_key.clone())? != *self {
            return Err(StorageError::InvalidPath(
                "reconciliation request differs from its exact native genesis".into(),
            ));
        }
        Ok(())
    }

    pub fn goal_id(&self, intent: &AgentReconciliationIntent) -> String {
        intent.goal_id(&self.agent_id, &self.request_id)
    }
}
