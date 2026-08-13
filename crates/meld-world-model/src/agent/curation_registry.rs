//! Durable exact revisions of Agent-owned curation rules.

use std::io;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use super::AgentCurationRuleConfig;
use crate::belief::{TheoryInstallDisposition, TheoryRevisionRef};
use crate::error::StorageError;

const REGISTRY_ID: &str = "agent_curation_rule";
const TREE_REVISIONS: &str = "agent_curation_rule_registry_revisions";

/// One installed curation-rule revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCurationRuleRevision {
    /// Stable rule identity.
    pub rule_id: String,
    /// Canonical rule content hash.
    pub content_hash: String,
    /// Intact rule body.
    pub rule: AgentCurationRuleConfig,
    /// Sequence observed on first installation.
    pub installed_at_seq: u64,
}

impl AgentCurationRuleRevision {
    /// Return the exact world-model reference.
    pub fn revision_ref(&self) -> TheoryRevisionRef {
        TheoryRevisionRef {
            registry: REGISTRY_ID.to_string(),
            id: self.rule_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }
}

/// Append-only store for curation-rule revisions.
#[derive(Clone)]
pub struct AgentCurationRuleRegistryStore {
    db: Db,
    revisions: Tree,
}

impl AgentCurationRuleRegistryStore {
    /// Open the Agent-owned tree in the world-model database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage_io)?,
            db,
        })
    }

    /// Install or reuse one exact revision.
    pub fn install(
        &self,
        rule_id: &str,
        rule: AgentCurationRuleConfig,
        installed_at_seq: u64,
    ) -> Result<(TheoryInstallDisposition, AgentCurationRuleRevision), StorageError> {
        require_non_empty("curation rule id", rule_id)?;
        rule.validate()?;
        let content_hash = hash_body(&rule)?;
        if let Some(existing) = self.resolve(rule_id, &content_hash)? {
            return Ok((TheoryInstallDisposition::Unchanged, existing));
        }
        let revision = AgentCurationRuleRevision {
            rule_id: rule_id.to_string(),
            content_hash,
            rule,
            installed_at_seq,
        };
        self.revisions
            .insert(
                revision_key(rule_id, &revision.content_hash)?,
                encode(&revision)?,
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok((TheoryInstallDisposition::Installed, revision))
    }

    /// Resolve and integrity-check one historical revision.
    pub fn resolve(
        &self,
        rule_id: &str,
        content_hash: &str,
    ) -> Result<Option<AgentCurationRuleRevision>, StorageError> {
        let Some(raw) = self
            .revisions
            .get(revision_key(rule_id, content_hash)?)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision: AgentCurationRuleRevision = decode(&raw)?;
        revision.rule.validate()?;
        if revision.rule_id != rule_id
            || revision.content_hash != content_hash
            || hash_body(&revision.rule)? != revision.content_hash
        {
            return Err(StorageError::InvalidPath(
                "curation rule revision content identity is corrupt".to_string(),
            ));
        }
        Ok(Some(revision))
    }
}

fn hash_body(rule: &AgentCurationRuleConfig) -> Result<String, StorageError> {
    Ok(blake3::hash(&encode(rule)?).to_hex().to_string())
}

fn revision_key(id: &str, hash: &str) -> Result<Vec<u8>, StorageError> {
    encode(&(id, hash))
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(value).map_err(to_storage_data)
}

fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, StorageError> {
    serde_json::from_slice(bytes).map_err(to_storage_data)
}

fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidPath(format!(
            "{label} must not be empty"
        )));
    }
    Ok(())
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(error.to_string()))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(
        io::ErrorKind::InvalidData,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(threshold: f64) -> AgentCurationRuleConfig {
        AgentCurationRuleConfig {
            dimension_id: "freshness".into(),
            threshold,
            priority_urgency: 1,
            desired_summary: "fresh docs".into(),
            source_kind: "belief".into(),
        }
    }

    #[test]
    fn exact_revisions_are_append_only_and_preserve_first_sequence() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = AgentCurationRuleRegistryStore::new(db).unwrap();
        let (_, first) = store.install("strict", rule(0.8), 4).unwrap();
        let (same, replay) = store.install("strict", rule(0.8), 9).unwrap();
        let (_, second) = store.install("strict", rule(0.9), 10).unwrap();
        assert_eq!(same, TheoryInstallDisposition::Unchanged);
        assert_eq!(replay.installed_at_seq, 4);
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(
            store.resolve("strict", &first.content_hash).unwrap(),
            Some(first)
        );
    }
}
