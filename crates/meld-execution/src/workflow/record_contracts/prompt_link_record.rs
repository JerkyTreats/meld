use crate::error::ApiError;
use crate::generation::PromptLinkContractView;
use crate::workflow::record_contracts::id_validation::{
    validate_hex64, validate_prefixed_id, validate_timestamp_ms,
};
use crate::workflow::record_contracts::schema_version::{
    validate_schema_version, WORKFLOW_RECORD_SCHEMA_VERSION_V1,
};
use serde::{Deserialize, Serialize};

const RECORD_TYPE: &str = "prompt_link";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptLinkRecordV1 {
    pub schema_version: u32,
    pub prompt_link_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub node_id: String,
    pub frame_id: String,
    pub system_prompt_artifact_id: String,
    pub user_prompt_template_artifact_id: String,
    pub rendered_prompt_artifact_id: String,
    pub context_artifact_id: String,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptLinkRecordInputV1 {
    pub thread_id: String,
    pub turn_id: String,
    pub node_id: String,
    pub frame_id: String,
    pub created_at_ms: u64,
}

pub fn prompt_link_record_from_contract_v1(
    contract: &PromptLinkContractView,
    input: &PromptLinkRecordInputV1,
) -> PromptLinkRecordV1 {
    PromptLinkRecordV1 {
        schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
        prompt_link_id: contract.prompt_link_id.clone(),
        thread_id: input.thread_id.clone(),
        turn_id: input.turn_id.clone(),
        node_id: input.node_id.clone(),
        frame_id: input.frame_id.clone(),
        system_prompt_artifact_id: contract.system_prompt_artifact_id.clone(),
        user_prompt_template_artifact_id: contract.user_prompt_template_artifact_id.clone(),
        rendered_prompt_artifact_id: contract.rendered_prompt_artifact_id.clone(),
        context_artifact_id: contract.context_artifact_id.clone(),
        created_at_ms: input.created_at_ms,
    }
}

pub fn validate_prompt_link_record_v1(record: &PromptLinkRecordV1) -> Result<(), ApiError> {
    validate_schema_version(RECORD_TYPE, record.schema_version)?;
    validate_prefixed_id(
        RECORD_TYPE,
        "prompt_link_id",
        &record.prompt_link_id,
        "prompt-link-",
    )?;
    validate_prefixed_id(RECORD_TYPE, "thread_id", &record.thread_id, "thread-")?;
    validate_prefixed_id(RECORD_TYPE, "turn_id", &record.turn_id, "turn-")?;
    validate_timestamp_ms(RECORD_TYPE, "created_at_ms", record.created_at_ms)?;
    validate_prompt_link_record_references(record)?;
    Ok(())
}

pub fn validate_prompt_link_record_references(record: &PromptLinkRecordV1) -> Result<(), ApiError> {
    validate_hex64(RECORD_TYPE, "node_id", &record.node_id).map_err(map_reference_error)?;
    validate_hex64(RECORD_TYPE, "frame_id", &record.frame_id).map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "system_prompt_artifact_id",
        &record.system_prompt_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "user_prompt_template_artifact_id",
        &record.user_prompt_template_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "rendered_prompt_artifact_id",
        &record.rendered_prompt_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "context_artifact_id",
        &record.context_artifact_id,
    )
    .map_err(map_reference_error)?;
    Ok(())
}

fn map_reference_error(err: ApiError) -> ApiError {
    ApiError::ConfigError(format!(
        "Workflow record references for '{}' invalid: {}",
        RECORD_TYPE, err
    ))
}
