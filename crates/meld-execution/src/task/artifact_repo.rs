//! Task-scoped artifact repository behavior.
//!
//! The task domain owns artifact meaning. Root runtime code may choose the
//! concrete sled database, but records stay scoped by repo id and are exposed
//! through the same snapshot shape used by in-memory task execution.

mod codec;
pub mod error;
mod keys;
mod records;
mod sled;

use crate::error::ApiError;
use crate::task::contracts::{
    ArtifactLinkRecord, ArtifactLinkRelation, ArtifactRecord, ArtifactRepoRecord,
};
pub use error::TaskArtifactRepoError;
use sled::SledArtifactRepo;

/// Task-scoped artifact repository with optional durable backing.
///
/// The in-memory form remains useful for focused task tests. `open_sled` uses
/// the same public API while persisting records for host reopen and task network
/// handoff.
#[derive(Debug, Clone)]
pub struct TaskArtifactRepo {
    record: ArtifactRepoRecord,
    durable: Option<SledArtifactRepo>,
}

impl TaskArtifactRepo {
    /// Creates an empty task-scoped artifact repository.
    pub fn new(repo_id: impl Into<String>) -> Self {
        Self {
            record: ArtifactRepoRecord {
                repo_id: repo_id.into(),
                artifacts: Vec::new(),
                artifact_links: Vec::new(),
            },
            durable: None,
        }
    }

    /// Opens a sled-backed task artifact repository within a caller owned database.
    ///
    /// `repo_id` scopes records in shared sled trees so product assembly can
    /// place many task repos under one task artifact store.
    pub fn open_sled(
        db: ::sled::Db,
        repo_id: impl Into<String>,
    ) -> Result<Self, TaskArtifactRepoError> {
        let durable = SledArtifactRepo::open(db, repo_id.into())?;
        Ok(Self {
            record: durable.record().clone(),
            durable: Some(durable),
        })
    }

    /// Returns the current artifact repo record snapshot.
    pub fn record(&self) -> &ArtifactRepoRecord {
        &self.record
    }

    /// Flushes durable writes when this repository has a persistent backing store.
    pub fn flush(&self) -> Result<(), TaskArtifactRepoError> {
        if let Some(durable) = &self.durable {
            durable.flush()?;
        }
        Ok(())
    }

    /// Appends one artifact to the repository.
    pub fn append_artifact(&mut self, artifact: ArtifactRecord) -> Result<(), ApiError> {
        if self
            .record
            .artifacts
            .iter()
            .any(|existing| existing.artifact_id == artifact.artifact_id)
        {
            return Err(ApiError::ConfigError(format!(
                "Artifact repo '{}' already contains artifact '{}'",
                self.record.repo_id, artifact.artifact_id
            )));
        }
        if let Some(durable) = &mut self.durable {
            // Persist before changing the public snapshot so callers never see
            // artifact state that failed to reach the durable store.
            durable
                .append_artifact(&artifact)
                .map_err(|error| ApiError::ConfigError(error.to_string()))?;
        }
        self.record.artifacts.push(artifact);
        Ok(())
    }

    /// Appends one durable relation between existing artifacts.
    pub fn append_link(&mut self, link: ArtifactLinkRecord) -> Result<(), ApiError> {
        self.ensure_artifact_exists(&link.from_artifact_id)?;
        self.ensure_artifact_exists(&link.to_artifact_id)?;
        if let Some(durable) = &mut self.durable {
            // The durable backend rechecks endpoints in the same transaction as
            // the link write. The in-memory checks keep both modes aligned.
            durable
                .append_link(&link)
                .map_err(|error| ApiError::ConfigError(error.to_string()))?;
        }
        self.record.artifact_links.push(link);
        Ok(())
    }

    /// Records that one artifact supersedes another.
    pub fn mark_superseded(
        &mut self,
        prior_artifact_id: &str,
        replacement_artifact_id: &str,
        detail: impl Into<String>,
    ) -> Result<(), ApiError> {
        self.append_link(ArtifactLinkRecord {
            from_artifact_id: prior_artifact_id.to_string(),
            to_artifact_id: replacement_artifact_id.to_string(),
            relation: ArtifactLinkRelation::Supersedes,
            detail: detail.into(),
        })
    }

    /// Returns one artifact by id.
    pub fn get_artifact(&self, artifact_id: &str) -> Option<&ArtifactRecord> {
        self.record
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
    }

    /// Returns all artifacts emitted by one capability instance.
    pub fn artifacts_for_capability_instance(
        &self,
        capability_instance_id: &str,
    ) -> Vec<&ArtifactRecord> {
        self.record
            .artifacts
            .iter()
            .filter(|artifact| artifact.producer.capability_instance_id == capability_instance_id)
            .collect()
    }

    /// Returns all artifacts emitted for one output slot.
    pub fn artifacts_for_output_slot(
        &self,
        capability_instance_id: &str,
        output_slot_id: &str,
    ) -> Vec<&ArtifactRecord> {
        self.record
            .artifacts
            .iter()
            .filter(|artifact| {
                artifact.producer.capability_instance_id == capability_instance_id
                    && artifact.producer.output_slot_id.as_deref() == Some(output_slot_id)
            })
            .collect()
    }

    fn ensure_artifact_exists(&self, artifact_id: &str) -> Result<(), ApiError> {
        if self.get_artifact(artifact_id).is_none() {
            return Err(ApiError::ConfigError(format!(
                "Artifact repo '{}' does not contain artifact '{}'",
                self.record.repo_id, artifact_id
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::contracts::ArtifactProducerRef;
    use serde_json::json;

    fn artifact(artifact_id: &str, output_slot_id: &str) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.to_string(),
            artifact_type_id: "readme_summary".to_string(),
            schema_version: 1,
            content: json!({ "summary": artifact_id }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: "capinst_ctx_finalize".to_string(),
                invocation_id: Some("invk_1".to_string()),
                output_slot_id: Some(output_slot_id.to_string()),
            },
        }
    }

    #[test]
    fn append_and_lookup_artifact() {
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        let artifact = artifact("artifact_1", "readme_summary");

        repo.append_artifact(artifact.clone()).unwrap();

        assert_eq!(repo.get_artifact("artifact_1"), Some(&artifact));
    }

    #[test]
    fn append_rejects_duplicate_artifact_id() {
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        repo.append_artifact(artifact("artifact_1", "readme_summary"))
            .unwrap();

        let error = repo
            .append_artifact(artifact("artifact_1", "readme_summary"))
            .unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("already contains artifact"));
    }

    #[test]
    fn mark_superseded_records_explicit_link() {
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        repo.append_artifact(artifact("artifact_1", "readme_summary"))
            .unwrap();
        repo.append_artifact(artifact("artifact_2", "readme_summary"))
            .unwrap();

        repo.mark_superseded("artifact_1", "artifact_2", "retry replacement")
            .unwrap();

        assert_eq!(repo.record().artifact_links.len(), 1);
        assert_eq!(
            repo.record().artifact_links[0].relation,
            ArtifactLinkRelation::Supersedes
        );
    }

    #[test]
    fn append_link_rejects_missing_artifact_ids() {
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        repo.append_artifact(artifact("artifact_1", "readme_summary"))
            .unwrap();

        let error = repo
            .mark_superseded("artifact_1", "missing_artifact", "retry replacement")
            .unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("missing_artifact"));
        assert!(repo.record().artifact_links.is_empty());
    }

    #[test]
    fn artifacts_for_capability_instance_filters_by_instance() {
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        repo.append_artifact(artifact("artifact_1", "readme_summary"))
            .unwrap();
        repo.append_artifact(artifact("artifact_2", "other_slot"))
            .unwrap();
        let mut other_capability = artifact("artifact_3", "readme_summary");
        other_capability.producer.capability_instance_id = "capinst_other".to_string();
        repo.append_artifact(other_capability).unwrap();

        let artifact_ids = repo
            .artifacts_for_capability_instance("capinst_ctx_finalize")
            .into_iter()
            .map(|artifact| artifact.artifact_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(artifact_ids, vec!["artifact_1", "artifact_2"]);
    }

    #[test]
    fn artifacts_for_output_slot_requires_capability_and_slot_match() {
        let mut repo = TaskArtifactRepo::new("repo_docs_writer");
        repo.append_artifact(artifact("artifact_1", "readme_summary"))
            .unwrap();
        repo.append_artifact(artifact("artifact_2", "other_slot"))
            .unwrap();
        let mut other_capability = artifact("artifact_3", "readme_summary");
        other_capability.producer.capability_instance_id = "capinst_other".to_string();
        repo.append_artifact(other_capability).unwrap();

        let artifact_ids = repo
            .artifacts_for_output_slot("capinst_ctx_finalize", "readme_summary")
            .into_iter()
            .map(|artifact| artifact.artifact_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(artifact_ids, vec!["artifact_1"]);
    }
}
