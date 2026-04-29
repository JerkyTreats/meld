use crate::task::contracts::{ArtifactProducerRef, ArtifactRecord};
use crate::task::expansion::{
    TaskExpansionRequest, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
};

pub fn parse_task_expansion_request_artifact(
    artifact: &ArtifactRecord,
) -> Result<Option<TaskExpansionRequest>, serde_json::Error> {
    if artifact.artifact_type_id != TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID
        || artifact.schema_version != TASK_EXPANSION_SCHEMA_VERSION
    {
        return Ok(None);
    }

    serde_json::from_value::<TaskExpansionRequest>(artifact.content.clone()).map(Some)
}

pub fn build_task_expansion_request_artifact(
    artifact_id: impl Into<String>,
    producer: ArtifactProducerRef,
    request: &TaskExpansionRequest,
) -> Result<ArtifactRecord, serde_json::Error> {
    Ok(ArtifactRecord {
        artifact_id: artifact_id.into(),
        artifact_type_id: TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID.to_string(),
        schema_version: TASK_EXPANSION_SCHEMA_VERSION,
        content: serde_json::to_value(request)?,
        producer,
    })
}
