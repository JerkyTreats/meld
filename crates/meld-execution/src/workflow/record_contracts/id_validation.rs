use crate::error::ApiError;

const HEX64_LEN: usize = 64;

/// Validates that an identifier is non-empty and has the required prefix.
pub fn validate_prefixed_id(
    record_type: &str,
    field_name: &str,
    value: &str,
    prefix: &str,
) -> Result<(), ApiError> {
    if value.is_empty() {
        return Err(contract_error(
            record_type,
            format!("{field_name} must not be empty"),
        ));
    }
    if !value.starts_with(prefix) {
        return Err(contract_error(
            record_type,
            format!("{field_name} must start with {prefix}"),
        ));
    }
    if value.len() <= prefix.len() {
        return Err(contract_error(
            record_type,
            format!("{field_name} must contain a suffix after {prefix}"),
        ));
    }

    Ok(())
}

/// Validates a lowercase 64-character hex identifier.
pub fn validate_hex64(record_type: &str, field_name: &str, value: &str) -> Result<(), ApiError> {
    if !is_hex64(value) {
        return Err(contract_error(
            record_type,
            format!("{field_name} must be 64 char lowercase hex"),
        ));
    }
    Ok(())
}

/// Validates a positive millisecond timestamp.
pub fn validate_timestamp_ms(
    record_type: &str,
    field_name: &str,
    value: u64,
) -> Result<(), ApiError> {
    if value == 0 {
        return Err(contract_error(
            record_type,
            format!("{field_name} must be a positive millisecond timestamp"),
        ));
    }
    Ok(())
}

fn contract_error(record_type: &str, reason: String) -> ApiError {
    ApiError::ConfigError(format!(
        "Workflow record contract '{}' invalid: {}",
        record_type, reason
    ))
}

fn is_hex64(value: &str) -> bool {
    value.len() == HEX64_LEN
        && value
            .as_bytes()
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
