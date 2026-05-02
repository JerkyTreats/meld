use crate::error::ApiError;

pub const WORKFLOW_RECORD_SCHEMA_VERSION_V1: u32 = 1;

pub fn validate_schema_version(record_type: &str, schema_version: u32) -> Result<(), ApiError> {
    if schema_version != WORKFLOW_RECORD_SCHEMA_VERSION_V1 {
        return Err(ApiError::ConfigError(format!(
            "Workflow record contract '{}' requires schema_version {} but got {}",
            record_type, WORKFLOW_RECORD_SCHEMA_VERSION_V1, schema_version
        )));
    }

    Ok(())
}
