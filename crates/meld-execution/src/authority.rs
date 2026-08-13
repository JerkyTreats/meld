//! Durable authority policies and independent execution revalidation.

use meld_events::DomainObjectRef;
use meld_lang::{
    evaluate_authority, AuthorityDecision, AuthorityDenial, AuthorityPolicy,
    AuthorityPolicyBinding, Composition,
};
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use thiserror::Error;

const TREE_REVISIONS: &str = "authority_policy_registry_revisions";

/// Exact reference to one execution-owned authority policy revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityPolicyRevisionRef {
    /// Stable authority policy identity.
    pub policy_id: String,
    /// Canonical identity derived from the complete policy body.
    pub content_hash: String,
}

/// One installed execution-owned authority policy revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityPolicyRevision {
    /// Intact authority policy body.
    pub policy: AuthorityPolicy,
    /// Canonical identity derived from the complete policy body.
    pub content_hash: String,
    /// Sequence observed on first installation.
    pub installed_at_seq: u64,
}

impl AuthorityPolicyRevision {
    /// Return the exact durable reference for this revision.
    pub fn revision_ref(&self) -> AuthorityPolicyRevisionRef {
        AuthorityPolicyRevisionRef {
            policy_id: self.policy.policy_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }

    /// Return the exact shared policy binding used by semantic runtimes.
    pub fn binding(&self) -> Result<AuthorityPolicyBinding, AuthorityPolicyStoreError> {
        AuthorityPolicyBinding::new(self.policy.clone(), self.content_hash.clone())
            .map_err(|error| AuthorityPolicyStoreError::Invalid(error.to_string()))
    }
}

/// Failure to install or resolve an authority policy revision.
#[derive(Debug, Error)]
pub enum AuthorityPolicyStoreError {
    /// The supplied policy failed owner validation.
    #[error("authority policy is invalid: {0}")]
    Invalid(String),
    /// Stored content no longer matches its exact identity.
    #[error("authority policy revision content identity is corrupt")]
    Corrupt,
    /// Physical persistence or serialization failed.
    #[error("authority policy storage failed: {0}")]
    Storage(String),
}

/// Append-only execution-owned store for exact authority policy revisions.
#[derive(Clone)]
pub struct AuthorityPolicyRegistryStore {
    db: Db,
    revisions: Tree,
}

impl AuthorityPolicyRegistryStore {
    /// Open the execution-owned registry tree.
    pub fn new(db: Db) -> Result<Self, AuthorityPolicyStoreError> {
        Ok(Self {
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage)?,
            db,
        })
    }

    /// Install or reuse one exact policy revision.
    pub fn install(
        &self,
        policy: AuthorityPolicy,
        installed_at_seq: u64,
    ) -> Result<(bool, AuthorityPolicyRevision), AuthorityPolicyStoreError> {
        policy
            .validate()
            .map_err(|error| AuthorityPolicyStoreError::Invalid(error.to_string()))?;
        let content_hash = policy
            .content_hash()
            .map_err(|error| AuthorityPolicyStoreError::Invalid(error.to_string()))?;
        if let Some(existing) = self.resolve(&policy.policy_id, &content_hash)? {
            return Ok((false, existing));
        }
        let revision = AuthorityPolicyRevision {
            policy,
            content_hash,
            installed_at_seq,
        };
        self.revisions
            .insert(
                revision_key(&revision.policy.policy_id, &revision.content_hash)?,
                encode(&revision)?,
            )
            .map_err(to_storage)?;
        self.db.flush().map_err(to_storage)?;
        Ok((true, revision))
    }

    /// Resolve and integrity-check one exact historical policy revision.
    pub fn resolve(
        &self,
        policy_id: &str,
        content_hash: &str,
    ) -> Result<Option<AuthorityPolicyRevision>, AuthorityPolicyStoreError> {
        let Some(raw) = self
            .revisions
            .get(revision_key(policy_id, content_hash)?)
            .map_err(to_storage)?
        else {
            return Ok(None);
        };
        let revision: AuthorityPolicyRevision = decode(&raw)?;
        if revision.policy.policy_id != policy_id
            || revision.content_hash != content_hash
            || revision
                .policy
                .content_hash()
                .map_err(|error| AuthorityPolicyStoreError::Invalid(error.to_string()))?
                != revision.content_hash
        {
            return Err(AuthorityPolicyStoreError::Corrupt);
        }
        Ok(Some(revision))
    }
}

/// Revalidate retained authority against one active exact policy.
pub fn revalidate_authority(
    binding: &AuthorityPolicyBinding,
    decision: &AuthorityDecision,
    composition: &Composition,
    subject: &DomainObjectRef,
) -> Result<(), AuthorityDenial> {
    decision.validate()?;
    if decision.policy_id != binding.policy.policy_id
        || decision.policy_content_hash != binding.content_hash
        || decision.principal_id != binding.policy.principal_id
    {
        return Err(AuthorityDenial::StalePolicy);
    }
    if &decision.subject != subject {
        return Err(AuthorityDenial::ScopeMismatch);
    }
    let recalculated = evaluate_authority(
        binding,
        &decision.requested_action_ids,
        composition,
        subject,
    )?;
    if &recalculated != decision {
        return Err(AuthorityDenial::InvalidDecision(
            "authority decision does not match the active policy intersection".to_string(),
        ));
    }
    Ok(())
}

/// Revalidate one task action against retained authority and the active policy.
pub fn revalidate_action_authority(
    binding: &AuthorityPolicyBinding,
    decision: &AuthorityDecision,
    action_id: &str,
) -> Result<(), AuthorityDenial> {
    binding.validate()?;
    decision.validate()?;
    if action_id.trim().is_empty() {
        return Err(AuthorityDenial::InvalidDecision(
            "task action identity must be non-empty".to_string(),
        ));
    }
    if decision.policy_id != binding.policy.policy_id
        || decision.policy_content_hash != binding.content_hash
        || decision.principal_id != binding.policy.principal_id
    {
        return Err(AuthorityDenial::StalePolicy);
    }
    if decision.subject != binding.policy.subject {
        return Err(AuthorityDenial::ScopeMismatch);
    }
    if !decision
        .requested_action_ids
        .iter()
        .any(|requested| requested == action_id)
    {
        return Err(AuthorityDenial::NotRequested(action_id.to_string()));
    }
    if !decision
        .authorized_action_ids
        .iter()
        .any(|authorized| authorized == action_id)
    {
        return Err(AuthorityDenial::InvalidDecision(format!(
            "task action '{action_id}' is absent from effective authority"
        )));
    }
    if !binding
        .policy
        .principal_granted_action_ids
        .iter()
        .any(|granted| granted == action_id)
    {
        return Err(AuthorityDenial::NotGranted(action_id.to_string()));
    }
    if !binding
        .policy
        .runtime_allowed_action_ids
        .iter()
        .any(|allowed| allowed == action_id)
    {
        return Err(AuthorityDenial::RuntimeDenied(action_id.to_string()));
    }
    if binding
        .policy
        .restricted_action_ids
        .iter()
        .any(|restricted| restricted == action_id)
    {
        return Err(AuthorityDenial::Restricted(action_id.to_string()));
    }
    Ok(())
}

fn revision_key(id: &str, hash: &str) -> Result<Vec<u8>, AuthorityPolicyStoreError> {
    encode(&(id, hash))
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, AuthorityPolicyStoreError> {
    serde_json::to_vec(value).map_err(|error| AuthorityPolicyStoreError::Storage(error.to_string()))
}

fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, AuthorityPolicyStoreError> {
    serde_json::from_slice(bytes)
        .map_err(|error| AuthorityPolicyStoreError::Storage(error.to_string()))
}

fn to_storage(error: sled::Error) -> AuthorityPolicyStoreError {
    AuthorityPolicyStoreError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> AuthorityPolicy {
        AuthorityPolicy {
            policy_id: "docs-local".to_string(),
            principal_id: "workspace-owner".to_string(),
            subject: DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            principal_granted_action_ids: vec!["docs.publish".to_string()],
            runtime_allowed_action_ids: vec!["docs.publish".to_string()],
            restricted_action_ids: Vec::new(),
        }
    }

    #[test]
    fn exact_policy_revisions_are_append_only_and_historical() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = AuthorityPolicyRegistryStore::new(db).unwrap();
        let first = store.install(policy(), 4).unwrap().1;
        let replay = store.install(policy(), 9).unwrap();
        let mut changed = policy();
        changed.restricted_action_ids = vec!["docs.publish".to_string()];
        let second = store.install(changed, 10).unwrap().1;

        assert!(!replay.0);
        assert_eq!(replay.1.installed_at_seq, 4);
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(
            store
                .resolve(&first.policy.policy_id, &first.content_hash)
                .unwrap(),
            Some(first)
        );
    }

    #[test]
    fn task_action_revalidation_rejects_tampered_request_lineage() {
        let body = policy();
        let binding =
            AuthorityPolicyBinding::new(body.clone(), body.content_hash().unwrap()).unwrap();
        let decision = AuthorityDecision {
            policy_id: body.policy_id.clone(),
            policy_content_hash: binding.content_hash.clone(),
            principal_id: body.principal_id.clone(),
            subject: body.subject.clone(),
            requested_action_ids: vec!["docs.inspect".to_string()],
            authorized_action_ids: vec!["docs.publish".to_string()],
        };

        assert_eq!(
            revalidate_action_authority(&binding, &decision, "docs.publish").unwrap_err(),
            AuthorityDenial::NotRequested("docs.publish".to_string())
        );
    }
}
