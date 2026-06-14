//! Sled-backed task artifact repository storage.
//!
//! The store uses shared trees keyed by repo id so one product task artifact
//! database can hold many task-scoped repositories without cross task leakage.

use crate::task::artifact_repo::{
    codec::{decode_error, decode_optional, to_decode, to_storage},
    error::TaskArtifactRepoError,
    keys::{
        artifact_id_from_key, artifact_key, link_key, link_sequence_from_key, repo_meta_key,
        repo_prefix,
    },
    records::{
        StoredArtifactLinkRecord, StoredArtifactRecord, StoredArtifactRepoMeta,
        TASK_ARTIFACT_RECORD_SCHEMA_VERSION, TREE_ARTIFACT_LINKS, TREE_ARTIFACT_RECORDS,
        TREE_REPO_META,
    },
};
use crate::task::contracts::{ArtifactLinkRecord, ArtifactRecord, ArtifactRepoRecord};
use ::sled::{
    transaction::{ConflictableTransactionError, TransactionError, Transactional},
    Db, Tree,
};
use std::collections::BTreeSet;

/// Sled-backed artifact repository state loaded into the public repo snapshot.
///
/// `next_link_sequence` is metadata for append order only. Artifact and link
/// meaning remains in the public task contracts.
#[derive(Debug, Clone)]
pub(super) struct SledArtifactRepo {
    db: Db,
    artifacts: Tree,
    links: Tree,
    meta: Tree,
    repo_id: String,
    next_link_sequence: u64,
    record: ArtifactRepoRecord,
}

impl SledArtifactRepo {
    pub(super) fn open(db: Db, repo_id: String) -> Result<Self, TaskArtifactRepoError> {
        // Separate trees keep artifact, link, and repo metadata records easy to
        // validate independently while sharing one caller supplied database.
        let artifacts = db.open_tree(TREE_ARTIFACT_RECORDS).map_err(to_storage)?;
        let links = db.open_tree(TREE_ARTIFACT_LINKS).map_err(to_storage)?;
        let meta = db.open_tree(TREE_REPO_META).map_err(to_storage)?;
        let loaded_artifacts = load_artifacts(&artifacts, &repo_id)?;
        let artifact_ids = loaded_artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_str())
            .collect::<BTreeSet<_>>();
        let (loaded_links, next_sequence_from_links) = load_links(&links, &repo_id, &artifact_ids)?;
        let next_link_sequence =
            load_next_link_sequence(&meta, &repo_id, next_sequence_from_links)?;

        Ok(Self {
            db,
            artifacts,
            links,
            meta,
            repo_id: repo_id.clone(),
            next_link_sequence,
            record: ArtifactRepoRecord {
                repo_id,
                artifacts: loaded_artifacts,
                artifact_links: loaded_links,
            },
        })
    }

    pub(super) fn record(&self) -> &ArtifactRepoRecord {
        &self.record
    }

    pub(super) fn flush(&self) -> Result<(), TaskArtifactRepoError> {
        self.db.flush().map_err(to_storage)?;
        Ok(())
    }

    pub(super) fn append_artifact(
        &mut self,
        artifact: &ArtifactRecord,
    ) -> Result<(), TaskArtifactRepoError> {
        let key = artifact_key(&self.repo_id, &artifact.artifact_id);
        let value = serde_json::to_vec(&StoredArtifactRecord::new(&self.repo_id, artifact.clone()))
            .map_err(to_decode)?;

        // Check and insert inside the transaction so durable state remains
        // append only even if another handle races this write.
        self.artifacts
            .transaction(|artifacts| {
                if artifacts.get(key.clone())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        TaskArtifactRepoError::Invariant(format!(
                            "artifact repo '{}' already contains artifact '{}'",
                            self.repo_id, artifact.artifact_id
                        )),
                    ));
                }
                artifacts.insert(key.clone(), value.clone())?;
                Ok(())
            })
            .map_err(to_transaction)?;
        self.flush()?;
        self.record.artifacts.push(artifact.clone());
        Ok(())
    }

    pub(super) fn append_link(
        &mut self,
        link: &ArtifactLinkRecord,
    ) -> Result<(), TaskArtifactRepoError> {
        let sequence = self.next_link_sequence;
        let link_key = link_key(&self.repo_id, sequence);
        let link_value = serde_json::to_vec(&StoredArtifactLinkRecord::new(
            &self.repo_id,
            sequence,
            link.clone(),
        ))
        .map_err(to_decode)?;
        let meta_key = repo_meta_key(&self.repo_id);
        let meta_value =
            serde_json::to_vec(&StoredArtifactRepoMeta::new(&self.repo_id, sequence + 1))
                .map_err(to_decode)?;
        let from_key = artifact_key(&self.repo_id, &link.from_artifact_id);
        let to_key = artifact_key(&self.repo_id, &link.to_artifact_id);

        // Link sequence and endpoint validation commit together. Reopen derives
        // ordering from this sequence, not from local executor memory.
        (&self.artifacts, &self.links, &self.meta)
            .transaction(|(artifacts, links, meta)| {
                if artifacts.get(from_key.clone())?.is_none() {
                    return Err(ConflictableTransactionError::Abort(
                        TaskArtifactRepoError::Invariant(format!(
                            "artifact repo '{}' does not contain artifact '{}'",
                            self.repo_id, link.from_artifact_id
                        )),
                    ));
                }
                if artifacts.get(to_key.clone())?.is_none() {
                    return Err(ConflictableTransactionError::Abort(
                        TaskArtifactRepoError::Invariant(format!(
                            "artifact repo '{}' does not contain artifact '{}'",
                            self.repo_id, link.to_artifact_id
                        )),
                    ));
                }
                if links.get(link_key.clone())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        TaskArtifactRepoError::Invariant(format!(
                            "artifact repo '{}' already contains link sequence '{}'",
                            self.repo_id, sequence
                        )),
                    ));
                }

                links.insert(link_key.clone(), link_value.clone())?;
                meta.insert(meta_key.clone(), meta_value.clone())?;
                Ok(())
            })
            .map_err(to_transaction)?;
        self.next_link_sequence = sequence + 1;
        self.flush()?;
        self.record.artifact_links.push(link.clone());
        Ok(())
    }
}

fn load_artifacts(
    artifacts: &Tree,
    repo_id: &str,
) -> Result<Vec<ArtifactRecord>, TaskArtifactRepoError> {
    let mut loaded = Vec::new();
    for item in artifacts.scan_prefix(repo_prefix(repo_id)) {
        let (key, value) = item.map_err(to_storage)?;
        let artifact_id = artifact_id_from_key(repo_id, &key)?;
        let stored: StoredArtifactRecord = serde_json::from_slice(&value).map_err(to_decode)?;
        validate_schema(stored.record_schema_version)?;
        if stored.repo_id != repo_id {
            return Err(decode_error("stored artifact repo id mismatch"));
        }
        if stored.artifact.artifact_id != artifact_id {
            return Err(decode_error("stored artifact key mismatch"));
        }
        loaded.push(stored.artifact);
    }
    Ok(loaded)
}

fn load_links(
    links: &Tree,
    repo_id: &str,
    artifact_ids: &BTreeSet<&str>,
) -> Result<(Vec<ArtifactLinkRecord>, u64), TaskArtifactRepoError> {
    let mut loaded = Vec::new();
    let mut next_sequence = 0u64;
    for item in links.scan_prefix(repo_prefix(repo_id)) {
        let (key, value) = item.map_err(to_storage)?;
        let sequence = link_sequence_from_key(repo_id, &key)?;
        let stored: StoredArtifactLinkRecord = serde_json::from_slice(&value).map_err(to_decode)?;
        validate_schema(stored.record_schema_version)?;
        if stored.repo_id != repo_id {
            return Err(decode_error("stored artifact link repo id mismatch"));
        }
        if stored.sequence != sequence {
            return Err(decode_error("stored artifact link key mismatch"));
        }
        if !artifact_ids.contains(stored.link.from_artifact_id.as_str())
            || !artifact_ids.contains(stored.link.to_artifact_id.as_str())
        {
            return Err(decode_error("stored artifact link endpoint missing"));
        }
        next_sequence = sequence + 1;
        loaded.push(stored.link);
    }
    Ok((loaded, next_sequence))
}

fn load_next_link_sequence(
    meta: &Tree,
    repo_id: &str,
    next_sequence_from_links: u64,
) -> Result<u64, TaskArtifactRepoError> {
    let raw: Option<StoredArtifactRepoMeta> =
        decode_optional(meta.get(repo_meta_key(repo_id)).map_err(to_storage)?)?;
    let Some(stored) = raw else {
        if next_sequence_from_links > 0 {
            return Err(decode_error(
                "artifact repo metadata missing for stored links",
            ));
        }
        return Ok(0);
    };
    validate_schema(stored.record_schema_version)?;
    if stored.repo_id != repo_id {
        return Err(decode_error(
            "stored artifact repo metadata repo id mismatch",
        ));
    }
    // Metadata may be ahead of link records after manual repair, but it must
    // never lag the stored sequence.
    if next_sequence_from_links > 0 && stored.next_link_sequence < next_sequence_from_links {
        return Err(decode_error(
            "stored artifact repo metadata sequence is stale",
        ));
    }
    Ok(stored.next_link_sequence)
}

fn validate_schema(schema_version: u32) -> Result<(), TaskArtifactRepoError> {
    if schema_version != TASK_ARTIFACT_RECORD_SCHEMA_VERSION {
        return Err(decode_error(format!(
            "unsupported task artifact record schema version '{}'",
            schema_version
        )));
    }
    Ok(())
}

fn to_transaction(error: TransactionError<TaskArtifactRepoError>) -> TaskArtifactRepoError {
    match error {
        TransactionError::Abort(error) => error,
        TransactionError::Storage(error) => TaskArtifactRepoError::Storage(error.to_string()),
    }
}
