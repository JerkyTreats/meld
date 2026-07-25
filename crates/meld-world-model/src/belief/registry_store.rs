//! Sled-backed belief-family registry.
//!
//! Owner: world model belief domain. First store-backed implementor of the
//! frozen [`BeliefFamilyRegistry`] contract. Revisions are append-only records
//! keyed by family identity and [`ConfigSnapshot`] content hash; the current
//! head is an index over those records, mirroring belief revision storage.
//! Reinstalling the current content hash is a no-op, changed content moves the
//! head to a new revision, and every revision ever installed stays resolvable
//! so durable lineage references remain answerable.

use std::io;
use std::sync::Arc;

use sled::{Db, Tree};

use crate::belief::config::BeliefConfigLoader;
use crate::belief::contracts::BeliefFamilyConfig;
use crate::belief::registry::{
    BeliefFamilyRegistry, BeliefFamilyRevision, TheoryInstallDisposition,
};
use crate::error::StorageError;

const TREE_FAMILY_REVISIONS: &str = "belief_family_registry_revisions";
const TREE_FAMILY_CURRENT: &str = "belief_family_registry_current";

/// Durable registry of belief-family theory revisions.
#[derive(Clone)]
pub struct BeliefFamilyRegistryStore {
    db: Db,
    revisions: Tree,
    current: Tree,
}

impl BeliefFamilyRegistryStore {
    /// Open the registry trees against a shared sled database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            revisions: db.open_tree(TREE_FAMILY_REVISIONS).map_err(to_storage_io)?,
            current: db.open_tree(TREE_FAMILY_CURRENT).map_err(to_storage_io)?,
            db,
        })
    }

    /// Open the registry behind an `Arc` for runtime assembly.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Current revisions of every installed family in family-id order.
    ///
    /// The current tree is keyed by family id, so iteration order is already
    /// deterministic and bounded by the number of installed families.
    pub fn installed_families(&self) -> Result<Vec<BeliefFamilyRevision>, StorageError> {
        let mut out = Vec::new();
        for item in self.current.iter() {
            let (family_id, hash) = item.map_err(to_storage_io)?;
            let family_id = String::from_utf8(family_id.to_vec()).map_err(to_storage_utf8)?;
            let hash = String::from_utf8(hash.to_vec()).map_err(to_storage_utf8)?;
            let revision = self.read_revision(&family_id, &hash)?.ok_or_else(|| {
                StorageError::InvalidPath(format!(
                    "current head for family '{family_id}' cites missing revision '{hash}'"
                ))
            })?;
            out.push(revision);
        }
        Ok(out)
    }

    fn current_hash(&self, family_id: &str) -> Result<Option<String>, StorageError> {
        let Some(raw) = self
            .current
            .get(family_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(
            String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?,
        ))
    }

    fn read_revision(
        &self,
        family_id: &str,
        content_hash: &str,
    ) -> Result<Option<BeliefFamilyRevision>, StorageError> {
        let Some(raw) = self
            .revisions
            .get(revision_key(family_id, content_hash).as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
    }
}

impl BeliefFamilyRegistry for BeliefFamilyRegistryStore {
    fn install(
        &mut self,
        config: BeliefFamilyConfig,
        installed_at_seq: u64,
    ) -> Result<(TheoryInstallDisposition, BeliefFamilyRevision), StorageError> {
        // Validation and hashing reuse the ConfigSnapshot path so the registry
        // revision hash equals the config snapshot hash cited by belief
        // revisions and freshness checks.
        let snapshot = BeliefConfigLoader::snapshot(config)?;
        let family_id = snapshot.config.family_id.clone();
        if self.current_hash(&family_id)?.as_deref() == Some(snapshot.hash.as_str()) {
            // Self-heal a head that cites a missing revision record: the
            // caller supplied identical content, so rewriting the record
            // restores resolvability without changing installed identity.
            let revision = match self.read_revision(&family_id, &snapshot.hash)? {
                Some(existing) => existing,
                None => {
                    let revision = BeliefFamilyRevision {
                        family_id: family_id.clone(),
                        content_hash: snapshot.hash.clone(),
                        config: snapshot.config,
                        installed_at_seq,
                    };
                    self.revisions
                        .insert(
                            revision_key(&family_id, &revision.content_hash).as_bytes(),
                            serde_json::to_vec(&revision).map_err(to_storage_data)?,
                        )
                        .map_err(to_storage_io)?;
                    self.db.flush().map_err(to_storage_io)?;
                    revision
                }
            };
            return Ok((TheoryInstallDisposition::Unchanged, revision));
        }
        // A previously installed hash keeps its original record so lineage
        // references stay pinned to the first installation sequence; only the
        // current head moves.
        let revision = match self.read_revision(&family_id, &snapshot.hash)? {
            Some(existing) => existing,
            None => {
                let revision = BeliefFamilyRevision {
                    family_id: family_id.clone(),
                    content_hash: snapshot.hash.clone(),
                    config: snapshot.config,
                    installed_at_seq,
                };
                self.revisions
                    .insert(
                        revision_key(&family_id, &revision.content_hash).as_bytes(),
                        serde_json::to_vec(&revision).map_err(to_storage_data)?,
                    )
                    .map_err(to_storage_io)?;
                revision
            }
        };
        self.current
            .insert(family_id.as_bytes(), revision.content_hash.as_bytes())
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok((TheoryInstallDisposition::Installed, revision))
    }

    fn current(&self, family_id: &str) -> Result<Option<BeliefFamilyRevision>, StorageError> {
        let Some(hash) = self.current_hash(family_id)? else {
            return Ok(None);
        };
        self.read_revision(family_id, &hash)
    }

    fn resolve(
        &self,
        family_id: &str,
        content_hash: &str,
    ) -> Result<Option<BeliefFamilyRevision>, StorageError> {
        self.read_revision(family_id, content_hash)
    }
}

fn revision_key(family_id: &str, content_hash: &str) -> String {
    format!("{family_id}::{content_hash}")
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn to_storage_utf8(err: std::string::FromUtf8Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}
