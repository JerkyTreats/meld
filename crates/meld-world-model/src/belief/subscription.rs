//! Belief-owned acceptance of Agent subscription requests.

use serde::{Deserialize, Serialize};

use crate::agent::AgentSubscriptionRequestV1;
use crate::error::StorageError;

use super::{BeliefFamilyRevision, BeliefStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceSubscriptionAcceptanceV1 {
    pub acceptance_id: String,
    pub request_id: String,
    pub source_owner: String,
    pub source_contract_revision: crate::belief::TheoryRevisionRef,
}

impl SourceSubscriptionAcceptanceV1 {
    fn new(request: &AgentSubscriptionRequestV1) -> Result<Self, StorageError> {
        let acceptance_id = stable_hash(&(
            request.request_id.as_str(),
            request.source_owner.as_str(),
            &request.source_contract_revision,
        ))?;
        Ok(Self {
            acceptance_id,
            request_id: request.request_id.clone(),
            source_owner: request.source_owner.clone(),
            source_contract_revision: request.source_contract_revision.clone(),
        })
    }

    fn verify_identity(&self) -> Result<(), StorageError> {
        let expected = stable_hash(&(
            self.request_id.as_str(),
            self.source_owner.as_str(),
            &self.source_contract_revision,
        ))?;
        if self.acceptance_id != expected {
            return Err(StorageError::InvalidPath(
                "source subscription acceptance identity mismatch".to_string(),
            ));
        }
        Ok(())
    }
}

/// Opaque proof that Belief durably accepted one exact subscription request.
///
/// Only [`BeliefSubscriptionAuthority`] can construct this proof. Agent
/// genesis may match its request identity but cannot manufacture or persist
/// Belief's decision.
#[derive(Debug)]
pub struct BeliefSubscriptionAcceptanceProof {
    acceptance: SourceSubscriptionAcceptanceV1,
}

impl BeliefSubscriptionAcceptanceProof {
    pub(crate) fn request_id(&self) -> &str {
        &self.acceptance.request_id
    }

    pub(crate) fn acceptance_id(&self) -> &str {
        &self.acceptance.acceptance_id
    }
}

/// Canonical Belief owner for durable subscription acceptance.
pub struct BeliefSubscriptionAuthority<'a> {
    store: &'a BeliefStore,
}

impl<'a> BeliefSubscriptionAuthority<'a> {
    pub fn new(store: &'a BeliefStore) -> Self {
        Self { store }
    }

    pub fn accept(
        &self,
        request: &AgentSubscriptionRequestV1,
        revision: &BeliefFamilyRevision,
    ) -> Result<BeliefSubscriptionAcceptanceProof, StorageError> {
        if request.source_owner != "belief"
            || request.source_contract_revision.registry != "belief_family"
            || request.belief_key.dimension_id != request.source_contract_revision.id
        {
            return Err(StorageError::InvalidPath(
                "Belief subscription request does not cite this owner and family".to_string(),
            ));
        }
        if revision.family_id != request.source_contract_revision.id
            || revision.content_hash != request.source_contract_revision.content_hash
            || revision.config.dimension_id != request.belief_key.dimension_id
            || revision.config.predicate_id != request.belief_key.predicate_id
            || revision.config.evidence_policy_id != request.belief_key.evidence_policy_id
        {
            return Err(StorageError::InvalidPath(
                "Belief subscription key differs from its exact family contract".to_string(),
            ));
        }
        if let Some(existing) = self.store.subscription_acceptance(&request.request_id)? {
            existing.verify_identity()?;
            if existing.request_id == request.request_id
                && existing.source_owner == request.source_owner
                && existing.source_contract_revision == request.source_contract_revision
            {
                return Ok(BeliefSubscriptionAcceptanceProof {
                    acceptance: existing,
                });
            }
            return Err(StorageError::InvalidPath(
                "Belief subscription request conflicts with its durable acceptance".to_string(),
            ));
        }
        let acceptance = SourceSubscriptionAcceptanceV1::new(request)?;
        self.store.put_subscription_acceptance(&acceptance)?;
        Ok(BeliefSubscriptionAcceptanceProof { acceptance })
    }
}

fn stable_hash(value: &impl Serialize) -> Result<String, StorageError> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| StorageError::InvalidPath(failure.to_string()))
}

#[cfg(test)]
mod tests {
    use meld_events::DomainObjectRef;

    use super::*;
    use crate::belief::{
        configured_belief_key, BeliefFamilyConfig, BeliefFamilyRegistry, BeliefFamilyRegistryStore,
        BranchScope,
    };
    use crate::world_state::graph::PerspectiveKey;

    #[test]
    fn belief_accepts_only_the_exact_family_key_and_reuses_its_receipt() {
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path()).unwrap();
        let mut registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let config: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/belief_family.docs_freshness.json"
        ))
        .unwrap();
        let revision = registry.install(config, 1).unwrap().1;
        let key = configured_belief_key(
            &revision,
            &DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            &PerspectiveKey::new("default", "default").unwrap(),
            &BranchScope::main(),
        );
        let request = AgentSubscriptionRequestV1::new(
            "agent-a".to_string(),
            "belief".to_string(),
            revision.revision_ref(),
            key,
            "from_genesis".to_string(),
        )
        .unwrap();
        let store = BeliefStore::new(db).unwrap();
        let first = {
            let authority = BeliefSubscriptionAuthority::new(&store);
            let first = authority.accept(&request, &revision).unwrap();
            let second = authority.accept(&request, &revision).unwrap();
            assert_eq!(first.acceptance_id(), second.acceptance_id());

            let mut mismatched = request;
            mismatched.belief_key.predicate_id = "other".to_string();
            assert!(authority.accept(&mismatched, &revision).is_err());
            first.acceptance
        };
        drop(store);
        drop(registry);

        let reopened = BeliefStore::new(sled::open(root.path()).unwrap()).unwrap();
        assert_eq!(
            reopened.subscription_acceptance(&first.request_id).unwrap(),
            Some(first)
        );
    }
}
