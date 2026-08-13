//! Durable exact revisions of belief-owned outcome mappings.

use std::io;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::belief::{
    ConfiguredOutcomeMappingSet, OutcomeMappingSetConfig, TheoryInstallDisposition,
    TheoryRevisionRef,
};
use crate::error::StorageError;

const REGISTRY_ID: &str = "outcome_mapping";
const TREE_REVISIONS: &str = "outcome_mapping_registry_revisions";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One installed outcome-mapping revision.
pub struct OutcomeMappingRevision {
    /// Stable mapping-set identity.
    pub mapping_id: String,
    /// Canonical mapping-set content hash.
    pub content_hash: String,
    /// Intact mapping-set body.
    pub config: OutcomeMappingSetConfig,
    /// Sequence observed on first installation.
    pub installed_at_seq: u64,
}

impl OutcomeMappingRevision {
    /// Return the exact world-model reference.
    pub fn revision_ref(&self) -> TheoryRevisionRef {
        TheoryRevisionRef {
            registry: REGISTRY_ID.to_string(),
            id: self.mapping_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }
}

#[derive(Clone)]
/// Append-only store for exact outcome-mapping revisions.
pub struct OutcomeMappingRegistryStore {
    db: Db,
    revisions: Tree,
}

impl OutcomeMappingRegistryStore {
    /// Open the belief-owned tree in the world-model database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage_io)?,
            db,
        })
    }

    /// Install or reuse one exact mapping-set revision.
    pub fn install(
        &self,
        config: OutcomeMappingSetConfig,
        installed_at_seq: u64,
    ) -> Result<(TheoryInstallDisposition, OutcomeMappingRevision), StorageError> {
        ConfiguredOutcomeMappingSet::new(config.clone())?;
        let mapping_id = config.mapping_id.clone();
        let content_hash = hash_body(&config)?;
        if let Some(existing) = self.resolve(&mapping_id, &content_hash)? {
            return Ok((TheoryInstallDisposition::Unchanged, existing));
        }
        let revision = OutcomeMappingRevision {
            mapping_id: mapping_id.clone(),
            content_hash,
            config,
            installed_at_seq,
        };
        self.revisions
            .insert(
                revision_key(&mapping_id, &revision.content_hash)?,
                encode(&revision)?,
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok((TheoryInstallDisposition::Installed, revision))
    }

    /// Resolve and integrity-check one historical mapping-set revision.
    pub fn resolve(
        &self,
        mapping_id: &str,
        content_hash: &str,
    ) -> Result<Option<OutcomeMappingRevision>, StorageError> {
        let Some(raw) = self
            .revisions
            .get(revision_key(mapping_id, content_hash)?)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision: OutcomeMappingRevision = decode(&raw)?;
        ConfiguredOutcomeMappingSet::new(revision.config.clone())?;
        if revision.mapping_id != mapping_id
            || revision.content_hash != content_hash
            || hash_body(&revision.config)? != revision.content_hash
        {
            return Err(StorageError::InvalidPath(
                "outcome mapping revision content identity is corrupt".to_string(),
            ));
        }
        Ok(Some(revision))
    }
}

fn hash_body<T: Serialize>(body: &T) -> Result<String, StorageError> {
    Ok(blake3::hash(&encode(body)?).to_hex().to_string())
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

    fn mapping() -> OutcomeMappingSetConfig {
        serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/outcome_interpretation.docs_freshness.json"
        ))
        .unwrap()
    }

    #[test]
    fn exact_mapping_revisions_are_idempotent_and_historical() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = OutcomeMappingRegistryStore::new(db).unwrap();
        let (_, first) = store.install(mapping(), 3).unwrap();
        let (same, replay) = store.install(mapping(), 8).unwrap();
        let mut changed = mapping();
        changed.rules[0].source_kind = "world_model_unobserved_scope_v2".to_string();
        let (_, second) = store.install(changed, 9).unwrap();

        assert_eq!(same, TheoryInstallDisposition::Unchanged);
        assert_eq!(replay.installed_at_seq, 3);
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(
            store
                .resolve(&first.mapping_id, &first.content_hash)
                .unwrap(),
            Some(first)
        );
    }
}
