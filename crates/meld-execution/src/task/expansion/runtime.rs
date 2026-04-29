//! Task expansion runtime helpers.

use crate::error::ApiError;
use crate::task::contracts::ArtifactRecord;
use crate::task::expansion::contracts::{
    TaskExpansionRequest, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
};

/// Returns the parsed request for a task expansion artifact.
pub fn parse_task_expansion_request_artifact(
    artifact: &ArtifactRecord,
) -> Result<Option<TaskExpansionRequest>, ApiError> {
    if artifact.artifact_type_id != TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID {
        return Ok(None);
    }
    if artifact.schema_version != TASK_EXPANSION_SCHEMA_VERSION {
        return Err(ApiError::ConfigError(format!(
            "Task expansion artifact '{}' has unsupported schema version '{}'",
            artifact.artifact_id, artifact.schema_version
        )));
    }

    serde_json::from_value(artifact.content.clone())
        .map(Some)
        .map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to decode task expansion artifact '{}': {}",
                artifact.artifact_id, err
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::contracts::ArtifactProducerRef;
    use serde_json::json;

    #[test]
    fn ignores_non_expansion_artifacts() {
        let artifact = ArtifactRecord {
            artifact_id: "artifact_1".to_string(),
            artifact_type_id: "other".to_string(),
            schema_version: 1,
            content: json!({}),
            producer: ArtifactProducerRef {
                task_id: "task_1".to_string(),
                capability_instance_id: "cap_1".to_string(),
                invocation_id: Some("inv_1".to_string()),
                output_slot_id: Some("slot_1".to_string()),
            },
        };

        assert!(parse_task_expansion_request_artifact(&artifact)
            .unwrap()
            .is_none());
    }
}
