//! Task-scoped artifact repository behavior.

use crate::error::ApiError;
use crate::task::contracts::{
    ArtifactLinkRecord, ArtifactLinkRelation, ArtifactRecord, ArtifactRepoRecord,
};

/// In-memory artifact repository with append and lookup semantics.
#[derive(Debug, Clone)]
pub struct TaskArtifactRepo {
    record: ArtifactRepoRecord,
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
        }
    }

    /// Returns the current durable artifact repo record snapshot.
    pub fn record(&self) -> &ArtifactRepoRecord {
        &self.record
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
        self.record.artifacts.push(artifact);
        Ok(())
    }

    /// Appends one durable relation between existing artifacts.
    pub fn append_link(&mut self, link: ArtifactLinkRecord) -> Result<(), ApiError> {
        self.ensure_artifact_exists(&link.from_artifact_id)?;
        self.ensure_artifact_exists(&link.to_artifact_id)?;
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
}
