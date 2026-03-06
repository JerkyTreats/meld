//! Canonical prompt link record contract for workflow consumers.

use crate::error::ApiError;
use crate::metadata::prompt_link_contract::PromptLinkContractV1;
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
    contract: &PromptLinkContractV1,
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
    match err {
        ApiError::WorkflowRecordContractInvalid { reason, .. } => {
            ApiError::WorkflowRecordReferenceInvalid {
                record_type: RECORD_TYPE.to_string(),
                reason,
            }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_record() -> PromptLinkRecordV1 {
        let digest = "a".repeat(64);
        PromptLinkRecordV1 {
            schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
            prompt_link_id: "prompt-link-aaaaaaaaaaaaaaaa".to_string(),
            thread_id: "thread-a".to_string(),
            turn_id: "turn-1".to_string(),
            node_id: digest.clone(),
            frame_id: digest.clone(),
            system_prompt_artifact_id: digest.clone(),
            user_prompt_template_artifact_id: digest.clone(),
            rendered_prompt_artifact_id: digest.clone(),
            context_artifact_id: digest,
            created_at_ms: 1,
        }
    }

    #[test]
    fn validate_accepts_valid_record() {
        validate_prompt_link_record_v1(&valid_record()).unwrap();
    }

    #[test]
    fn validate_rejects_invalid_prefix() {
        let mut record = valid_record();
        record.thread_id = "bad".to_string();
        let err = validate_prompt_link_record_v1(&record).unwrap_err();
        assert!(matches!(
            err,
            ApiError::WorkflowRecordContractInvalid { .. }
        ));
    }

    #[test]
    fn validate_rejects_invalid_artifact_reference() {
        let mut record = valid_record();
        record.context_artifact_id = "deadbeef".to_string();
        let err = validate_prompt_link_record_v1(&record).unwrap_err();
        assert!(matches!(
            err,
            ApiError::WorkflowRecordReferenceInvalid { .. }
        ));
    }

    #[test]
    fn from_contract_maps_fields() {
        let digest = "b".repeat(64);
        let contract = PromptLinkContractV1 {
            prompt_link_id: "prompt-link-bbbbbbbbbbbbbbbb".to_string(),
            prompt_digest: digest.clone(),
            context_digest: digest.clone(),
            system_prompt_artifact_id: digest.clone(),
            user_prompt_template_artifact_id: digest.clone(),
            rendered_prompt_artifact_id: digest.clone(),
            context_artifact_id: digest.clone(),
        };
        let input = PromptLinkRecordInputV1 {
            thread_id: "thread-b".to_string(),
            turn_id: "turn-2".to_string(),
            node_id: digest.clone(),
            frame_id: digest.clone(),
            created_at_ms: 2,
        };

        let record = prompt_link_record_from_contract_v1(&contract, &input);
        assert_eq!(record.prompt_link_id, contract.prompt_link_id);
        assert_eq!(record.thread_id, input.thread_id);
        assert_eq!(record.turn_id, input.turn_id);
        assert_eq!(record.context_artifact_id, contract.context_artifact_id);
    }
}
