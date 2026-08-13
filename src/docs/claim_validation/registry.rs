//! Durable exact revisions of docs-owned claim policies.

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use super::DocsClaimPolicy;
use crate::error::ApiError;

const TREE_REVISIONS: &str = "docs_claim_policy_registry_revisions";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsClaimPolicyRevisionRef {
    pub policy_id: String,
    pub content_identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsClaimPolicyRevision {
    pub policy: DocsClaimPolicy,
    pub content_identity: String,
    pub installed_at_seq: u64,
}

impl DocsClaimPolicyRevision {
    pub fn revision_ref(&self) -> DocsClaimPolicyRevisionRef {
        DocsClaimPolicyRevisionRef {
            policy_id: self.policy.policy_id.clone(),
            content_identity: self.content_identity.clone(),
        }
    }
}

#[derive(Clone)]
pub struct DocsClaimPolicyRegistryStore {
    db: Db,
    revisions: Tree,
}

impl DocsClaimPolicyRegistryStore {
    pub fn new(db: Db) -> Result<Self, ApiError> {
        Ok(Self {
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_api)?,
            db,
        })
    }

    pub fn install(
        &self,
        policy: DocsClaimPolicy,
        installed_at_seq: u64,
    ) -> Result<(bool, DocsClaimPolicyRevision), ApiError> {
        policy.validate()?;
        let reference = DocsClaimPolicyRevisionRef {
            policy_id: policy.policy_id.clone(),
            content_identity: policy.content_identity(),
        };
        if let Some(existing) = self.resolve(&reference)? {
            return Ok((false, existing));
        }
        let revision = DocsClaimPolicyRevision {
            policy,
            content_identity: reference.content_identity,
            installed_at_seq,
        };
        self.revisions
            .insert(key(&revision.revision_ref())?, encode(&revision)?)
            .map_err(to_api)?;
        self.db.flush().map_err(to_api)?;
        Ok((true, revision))
    }

    pub fn resolve(
        &self,
        reference: &DocsClaimPolicyRevisionRef,
    ) -> Result<Option<DocsClaimPolicyRevision>, ApiError> {
        let Some(raw) = self.revisions.get(key(reference)?).map_err(to_api)? else {
            return Ok(None);
        };
        let revision: DocsClaimPolicyRevision = decode(&raw)?;
        revision.policy.validate()?;
        if revision.policy.policy_id != reference.policy_id
            || revision.content_identity != reference.content_identity
            || revision.policy.content_identity() != revision.content_identity
        {
            return Err(ApiError::ConfigError(
                "docs claim policy revision content identity is corrupt".to_string(),
            ));
        }
        Ok(Some(revision))
    }
}

fn key(reference: &DocsClaimPolicyRevisionRef) -> Result<Vec<u8>, ApiError> {
    encode(&(
        reference.policy_id.as_str(),
        reference.content_identity.as_str(),
    ))
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, ApiError> {
    serde_json::to_vec(value).map_err(|error| ApiError::ConfigError(error.to_string()))
}

fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(bytes).map_err(|error| ApiError::ConfigError(error.to_string()))
}

fn to_api(error: sled::Error) -> ApiError {
    ApiError::ConfigError(format!("docs claim policy storage failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DocsClaimPolicy {
        serde_json::from_str(include_str!(
            "../../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
        ))
        .unwrap()
    }

    #[test]
    fn exact_policy_revisions_are_idempotent_and_historical() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = DocsClaimPolicyRegistryStore::new(db).unwrap();
        let (_, first) = store.install(policy(), 3).unwrap();
        let (same, replay) = store.install(policy(), 8).unwrap();
        let mut changed = policy();
        changed.minimum_groundedness = 0.91;
        let (_, second) = store.install(changed, 9).unwrap();

        assert!(!same);
        assert_eq!(replay.installed_at_seq, 3);
        assert_ne!(first.content_identity, second.content_identity);
        assert_eq!(store.resolve(&first.revision_ref()).unwrap(), Some(first));
    }
}
