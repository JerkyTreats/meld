//! Durable exact executable capability contract revisions.

use meld_lang::CapabilityRef;
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use thiserror::Error;

use super::CapabilityTypeContract;

const TREE_REVISIONS: &str = "capability_contract_registry_revisions";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Exact reference to one executable capability contract revision.
pub struct CapabilityContractRevisionRef {
    /// Stable capability type and version selector.
    pub selector: CapabilityRef,
    /// Exact identity derived from the full contract body.
    pub content_identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One installed executable capability contract revision.
pub struct CapabilityContractRevision {
    /// Intact execution-owned contract body.
    pub contract: CapabilityTypeContract,
    /// Exact identity derived from the contract body.
    pub content_identity: String,
    /// Sequence observed on first installation.
    pub installed_at_seq: u64,
}

impl CapabilityContractRevision {
    /// Return the exact reference for this revision.
    pub fn revision_ref(&self) -> CapabilityContractRevisionRef {
        CapabilityContractRevisionRef {
            selector: CapabilityRef {
                capability_type_id: self.contract.capability_type_id.clone(),
                capability_version: self.contract.capability_version,
            },
            content_identity: self.content_identity.clone(),
        }
    }
}

#[derive(Debug, Error)]
/// Failure to install or resolve an executable contract revision.
pub enum CapabilityContractRevisionStoreError {
    /// The supplied contract failed owner validation.
    #[error("capability contract is invalid: {0}")]
    Invalid(String),
    /// Different content attempted to reuse a type and version selector.
    #[error("capability contract version drift for '{type_id}' version {version}")]
    VersionDrift {
        /// Stable capability type identity.
        type_id: String,
        /// Reused capability version.
        version: u32,
    },
    /// Stored content no longer matches its exact identity.
    #[error("capability contract revision content identity is corrupt")]
    Corrupt,
    /// Physical persistence or serialization failed.
    #[error("capability contract storage failed: {0}")]
    Storage(String),
}

#[derive(Clone)]
/// Append-only store for exact executable contract revisions.
pub struct CapabilityContractRegistryStore {
    db: Db,
    revisions: Tree,
}

impl CapabilityContractRegistryStore {
    /// Open the execution-owned tree in a shared theory database.
    pub fn new(db: Db) -> Result<Self, CapabilityContractRevisionStoreError> {
        Ok(Self {
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage)?,
            db,
        })
    }

    /// Install or reuse one selector, rejecting semantic version drift.
    pub fn install(
        &self,
        contract: CapabilityTypeContract,
        installed_at_seq: u64,
    ) -> Result<(bool, CapabilityContractRevision), CapabilityContractRevisionStoreError> {
        contract
            .validate()
            .map_err(|error| CapabilityContractRevisionStoreError::Invalid(error.to_string()))?;
        let selector = CapabilityRef {
            capability_type_id: contract.capability_type_id.clone(),
            capability_version: contract.capability_version,
        };
        if let Some(existing) = self.resolve_selector(&selector)? {
            if existing.contract != contract {
                return Err(CapabilityContractRevisionStoreError::VersionDrift {
                    type_id: selector.capability_type_id,
                    version: selector.capability_version,
                });
            }
            return Ok((false, existing));
        }
        let revision = CapabilityContractRevision {
            content_identity: contract.content_identity(),
            contract,
            installed_at_seq,
        };
        self.revisions
            .insert(selector_key(&selector)?, encode(&revision)?)
            .map_err(to_storage)?;
        self.db.flush().map_err(to_storage)?;
        Ok((true, revision))
    }

    /// Resolve and integrity-check one exact revision.
    pub fn resolve(
        &self,
        reference: &CapabilityContractRevisionRef,
    ) -> Result<Option<CapabilityContractRevision>, CapabilityContractRevisionStoreError> {
        let Some(revision) = self.resolve_selector(&reference.selector)? else {
            return Ok(None);
        };
        if revision.content_identity != reference.content_identity {
            return Ok(None);
        }
        Ok(Some(revision))
    }

    fn resolve_selector(
        &self,
        selector: &CapabilityRef,
    ) -> Result<Option<CapabilityContractRevision>, CapabilityContractRevisionStoreError> {
        let Some(raw) = self
            .revisions
            .get(selector_key(selector)?)
            .map_err(to_storage)?
        else {
            return Ok(None);
        };
        let revision: CapabilityContractRevision = decode(&raw)?;
        if revision.contract.capability_type_id != selector.capability_type_id
            || revision.contract.capability_version != selector.capability_version
            || revision.contract.content_identity() != revision.content_identity
        {
            return Err(CapabilityContractRevisionStoreError::Corrupt);
        }
        Ok(Some(revision))
    }
}

fn selector_key(selector: &CapabilityRef) -> Result<Vec<u8>, CapabilityContractRevisionStoreError> {
    encode(&(
        selector.capability_type_id.as_str(),
        selector.capability_version,
    ))
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CapabilityContractRevisionStoreError> {
    serde_json::to_vec(value)
        .map_err(|error| CapabilityContractRevisionStoreError::Storage(error.to_string()))
}

fn decode<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
) -> Result<T, CapabilityContractRevisionStoreError> {
    serde_json::from_slice(bytes)
        .map_err(|error| CapabilityContractRevisionStoreError::Storage(error.to_string()))
}

fn to_storage(error: sled::Error) -> CapabilityContractRevisionStoreError {
    CapabilityContractRevisionStoreError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{ExecutionClass, ExecutionContract, ScopeContract};

    fn contract(domain: &str) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: "docs.read".into(),
            capability_version: 1,
            owning_domain: domain.into(),
            scope_contract: ScopeContract {
                scope_kind: "workspace".into(),
                scope_ref_kind: "path".into(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![],
            output_contract: vec![],
            effect_contract: vec![],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "complete".into(),
                retry_class: "never".into(),
                cancellation_supported: false,
            },
        }
    }

    #[test]
    fn rejects_changed_content_under_the_same_selector() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = CapabilityContractRegistryStore::new(db).unwrap();
        store.install(contract("docs"), 1).unwrap();
        let error = store.install(contract("other"), 2).unwrap_err();
        assert!(matches!(
            error,
            CapabilityContractRevisionStoreError::VersionDrift { .. }
        ));
    }
}
