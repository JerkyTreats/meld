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
    use crate::task::{
        ArtifactProducerRef, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    };
    use serde_json::json;

    fn artifact(
        artifact_type_id: &str,
        schema_version: u32,
        content: serde_json::Value,
    ) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: "artifact_1".to_string(),
            artifact_type_id: artifact_type_id.to_string(),
            schema_version,
            content,
            producer: ArtifactProducerRef {
                task_id: "task_1".to_string(),
                capability_instance_id: "cap_1".to_string(),
                invocation_id: Some("inv_1".to_string()),
                output_slot_id: Some("slot_1".to_string()),
            },
        }
    }

    #[test]
    fn ignores_non_expansion_artifacts() {
        let artifact = artifact("other", 1, json!({}));

        assert!(parse_task_expansion_request_artifact(&artifact)
            .unwrap()
            .is_none());
    }

    #[test]
    fn parses_valid_expansion_request_artifact() {
        let artifact = artifact(
            TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID,
            TASK_EXPANSION_SCHEMA_VERSION,
            json!({
                "expansion_id": "expansion_1",
                "expansion_kind": "discover_children",
                "content": { "node_id": "node_root" }
            }),
        );

        let request = parse_task_expansion_request_artifact(&artifact)
            .unwrap()
            .unwrap();

        assert_eq!(request.expansion_id, "expansion_1");
        assert_eq!(request.expansion_kind, "discover_children");
        assert_eq!(request.content["node_id"], "node_root");
    }

    #[test]
    fn rejects_unsupported_expansion_schema_version() {
        let artifact = artifact(
            TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID,
            TASK_EXPANSION_SCHEMA_VERSION + 1,
            json!({
                "expansion_id": "expansion_1",
                "expansion_kind": "discover_children",
                "content": {}
            }),
        );

        let error = parse_task_expansion_request_artifact(&artifact).unwrap_err();

        assert!(error.to_string().contains("unsupported schema version"));
    }

    #[test]
    fn rejects_malformed_expansion_content() {
        let artifact = artifact(
            TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID,
            TASK_EXPANSION_SCHEMA_VERSION,
            json!({
                "expansion_id": "expansion_1",
                "content": {}
            }),
        );

        let error = parse_task_expansion_request_artifact(&artifact).unwrap_err();

        assert!(error.to_string().contains("Failed to decode"));
    }
}
