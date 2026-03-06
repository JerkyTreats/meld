//! Schema version contract for workflow owned records.

use crate::error::ApiError;

pub const WORKFLOW_RECORD_SCHEMA_VERSION_V1: u32 = 1;

pub fn validate_schema_version(record_type: &str, schema_version: u32) -> Result<(), ApiError> {
    if schema_version != WORKFLOW_RECORD_SCHEMA_VERSION_V1 {
        return Err(ApiError::WorkflowRecordContractInvalid {
            record_type: record_type.to_string(),
            reason: format!(
                "schema_version must be {}, got {}",
                WORKFLOW_RECORD_SCHEMA_VERSION_V1, schema_version
            ),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_v1() {
        validate_schema_version("thread_turn_gate", WORKFLOW_RECORD_SCHEMA_VERSION_V1).unwrap();
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let err = validate_schema_version("thread_turn_gate", 2).unwrap_err();
        assert!(matches!(
            err,
            ApiError::WorkflowRecordContractInvalid { .. }
        ));
    }
}
