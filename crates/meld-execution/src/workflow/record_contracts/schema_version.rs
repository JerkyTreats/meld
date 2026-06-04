use crate::error::ApiError;

/// Current schema version for workflow records.
pub const WORKFLOW_RECORD_SCHEMA_VERSION_V1: u32 = 1;

/// Validates that a workflow record uses the supported schema version.
pub fn validate_schema_version(record_type: &str, schema_version: u32) -> Result<(), ApiError> {
    if schema_version != WORKFLOW_RECORD_SCHEMA_VERSION_V1 {
        return Err(ApiError::ConfigError(format!(
            "Workflow record contract '{}' requires schema_version {} but got {}",
            record_type, WORKFLOW_RECORD_SCHEMA_VERSION_V1, schema_version
        )));
    }

    Ok(())
}
