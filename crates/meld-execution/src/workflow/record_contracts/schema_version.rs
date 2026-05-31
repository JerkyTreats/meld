use crate::error::ApiError;

/// Constant used by the workflow record schema version v1 execution contract.
pub const WORKFLOW_RECORD_SCHEMA_VERSION_V1: u32 = 1;

/// Execution helper for validate schema version.
pub fn validate_schema_version(record_type: &str, schema_version: u32) -> Result<(), ApiError> {
    if schema_version != WORKFLOW_RECORD_SCHEMA_VERSION_V1 {
        return Err(ApiError::ConfigError(format!(
            "Workflow record contract '{}' requires schema_version {} but got {}",
            record_type, WORKFLOW_RECORD_SCHEMA_VERSION_V1, schema_version
        )));
    }

    Ok(())
}
