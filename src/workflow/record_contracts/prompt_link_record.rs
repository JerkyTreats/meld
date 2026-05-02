use crate::metadata::prompt_link_contract::PromptLinkContractV1;

pub use meld_execution::workflow::record_contracts::prompt_link_record::{
    PromptLinkRecordInputV1, PromptLinkRecordV1,
};

pub fn prompt_link_record_from_contract_v1(
    contract: &PromptLinkContractV1,
    input: &PromptLinkRecordInputV1,
) -> PromptLinkRecordV1 {
    meld_execution::workflow::record_contracts::prompt_link_record::prompt_link_record_from_contract_v1(
        &meld_execution::PromptLinkContractView {
            prompt_link_id: contract.prompt_link_id.clone(),
            prompt_digest: contract.prompt_digest.clone(),
            context_digest: contract.context_digest.clone(),
            system_prompt_artifact_id: contract.system_prompt_artifact_id.clone(),
            user_prompt_template_artifact_id: contract.user_prompt_template_artifact_id.clone(),
            rendered_prompt_artifact_id: contract.rendered_prompt_artifact_id.clone(),
            context_artifact_id: contract.context_artifact_id.clone(),
        },
        input,
    )
}

pub fn validate_prompt_link_record_v1(
    record: &PromptLinkRecordV1,
) -> Result<(), crate::error::ApiError> {
    validate_schema_version(record.schema_version)?;
    validate_prefixed_id("prompt_link_id", &record.prompt_link_id, "prompt-link-")?;
    validate_prefixed_id("thread_id", &record.thread_id, "thread-")?;
    validate_prefixed_id("turn_id", &record.turn_id, "turn-")?;
    validate_timestamp_ms("created_at_ms", record.created_at_ms)?;
    validate_prompt_link_record_references(record)?;
    Ok(())
}

pub fn validate_prompt_link_record_references(
    record: &PromptLinkRecordV1,
) -> Result<(), crate::error::ApiError> {
    validate_hex64("node_id", &record.node_id).map_err(map_reference_error)?;
    validate_hex64("frame_id", &record.frame_id).map_err(map_reference_error)?;
    validate_hex64(
        "system_prompt_artifact_id",
        &record.system_prompt_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        "user_prompt_template_artifact_id",
        &record.user_prompt_template_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        "rendered_prompt_artifact_id",
        &record.rendered_prompt_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64("context_artifact_id", &record.context_artifact_id)
        .map_err(map_reference_error)?;
    Ok(())
}

fn validate_schema_version(schema_version: u32) -> Result<(), crate::error::ApiError> {
    if schema_version != meld_execution::workflow::WORKFLOW_RECORD_SCHEMA_VERSION_V1 {
        return Err(crate::error::ApiError::WorkflowRecordContractInvalid {
            record_type: "prompt_link".to_string(),
            reason: format!(
                "schema_version must be {}, got {}",
                meld_execution::workflow::WORKFLOW_RECORD_SCHEMA_VERSION_V1,
                schema_version
            ),
        });
    }
    Ok(())
}

fn validate_prefixed_id(
    field_name: &str,
    value: &str,
    prefix: &str,
) -> Result<(), crate::error::ApiError> {
    if value.is_empty() {
        return Err(contract_error(format!("{field_name} must not be empty")));
    }
    if !value.starts_with(prefix) {
        return Err(contract_error(format!(
            "{field_name} must start with {prefix}"
        )));
    }
    if value.len() <= prefix.len() {
        return Err(contract_error(format!(
            "{field_name} must contain a suffix after {prefix}"
        )));
    }
    Ok(())
}

fn validate_timestamp_ms(field_name: &str, value: u64) -> Result<(), crate::error::ApiError> {
    if value == 0 {
        return Err(contract_error(format!(
            "{field_name} must be a positive millisecond timestamp"
        )));
    }
    Ok(())
}

fn validate_hex64(field_name: &str, value: &str) -> Result<(), crate::error::ApiError> {
    let valid = value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
    if !valid {
        return Err(contract_error(format!(
            "{field_name} must be 64 char lowercase hex"
        )));
    }
    Ok(())
}

fn contract_error(reason: String) -> crate::error::ApiError {
    crate::error::ApiError::WorkflowRecordContractInvalid {
        record_type: "prompt_link".to_string(),
        reason,
    }
}

fn map_reference_error(err: crate::error::ApiError) -> crate::error::ApiError {
    match err {
        crate::error::ApiError::WorkflowRecordContractInvalid { reason, .. } => {
            crate::error::ApiError::WorkflowRecordReferenceInvalid {
                record_type: "prompt_link".to_string(),
                reason,
            }
        }
        other => other,
    }
}
