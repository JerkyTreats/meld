//! Durable task artifact repository record shapes.

use crate::task::contracts::{ArtifactLinkRecord, ArtifactRecord};
use serde::{Deserialize, Serialize};

pub(super) const TASK_ARTIFACT_RECORD_SCHEMA_VERSION: u32 = 1;

pub(super) const TREE_ARTIFACT_RECORDS: &str = "task_artifact_records";
pub(super) const TREE_ARTIFACT_LINKS: &str = "task_artifact_links";
pub(super) const TREE_REPO_META: &str = "task_artifact_repo_meta";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredArtifactRecord {
    pub(super) record_schema_version: u32,
    pub(super) repo_id: String,
    pub(super) artifact: ArtifactRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct StoredArtifactLinkRecord {
    pub(super) record_schema_version: u32,
    pub(super) repo_id: String,
    pub(super) sequence: u64,
    pub(super) link: ArtifactLinkRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StoredArtifactRepoMeta {
    pub(super) record_schema_version: u32,
    pub(super) repo_id: String,
    pub(super) next_link_sequence: u64,
}

impl StoredArtifactRecord {
    pub(super) fn new(repo_id: &str, artifact: ArtifactRecord) -> Self {
        Self {
            record_schema_version: TASK_ARTIFACT_RECORD_SCHEMA_VERSION,
            repo_id: repo_id.to_string(),
            artifact,
        }
    }
}

impl StoredArtifactLinkRecord {
    pub(super) fn new(repo_id: &str, sequence: u64, link: ArtifactLinkRecord) -> Self {
        Self {
            record_schema_version: TASK_ARTIFACT_RECORD_SCHEMA_VERSION,
            repo_id: repo_id.to_string(),
            sequence,
            link,
        }
    }
}

impl StoredArtifactRepoMeta {
    pub(super) fn new(repo_id: &str, next_link_sequence: u64) -> Self {
        Self {
            record_schema_version: TASK_ARTIFACT_RECORD_SCHEMA_VERSION,
            repo_id: repo_id.to_string(),
            next_link_sequence,
        }
    }
}
