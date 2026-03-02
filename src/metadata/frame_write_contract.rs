//! Shared frame metadata write boundary.

use crate::error::ApiError;
use crate::metadata::frame_types::FrameMetadata;

pub const KEY_AGENT_ID: &str = "agent_id";
pub const KEY_PROVIDER: &str = "provider";
pub const KEY_MODEL: &str = "model";
pub const KEY_PROVIDER_TYPE: &str = "provider_type";
pub const KEY_PROMPT: &str = "prompt";
pub const KEY_DELETED: &str = "deleted";
pub const KEY_PROMPT_DIGEST: &str = "prompt_digest";
pub const KEY_CONTEXT_DIGEST: &str = "context_digest";
pub const KEY_PROMPT_LINK_ID: &str = "prompt_link_id";

pub const FORBIDDEN_KEY_CONTEXT: &str = "context";
pub const FORBIDDEN_KEY_RAW_PROMPT: &str = "raw_prompt";
pub const FORBIDDEN_KEY_RAW_CONTEXT: &str = "raw_context";

pub const METADATA_PER_KEY_MAX_BYTES: usize = 16 * 1024;
pub const METADATA_TOTAL_MAX_BYTES: usize = 64 * 1024;

const ALLOWED_KEYS: &[&str] = &[
    KEY_AGENT_ID,
    KEY_PROVIDER,
    KEY_MODEL,
    KEY_PROVIDER_TYPE,
    KEY_PROMPT,
    KEY_DELETED,
    KEY_PROMPT_DIGEST,
    KEY_CONTEXT_DIGEST,
    KEY_PROMPT_LINK_ID,
];

const FORBIDDEN_KEYS: &[&str] = &[
    FORBIDDEN_KEY_CONTEXT,
    FORBIDDEN_KEY_RAW_PROMPT,
    FORBIDDEN_KEY_RAW_CONTEXT,
];

/// Build frame metadata for generation queue writes.
pub fn build_generated_metadata(
    agent_id: &str,
    provider: &str,
    model: &str,
    provider_type: &str,
    prompt: &str,
) -> FrameMetadata {
    let mut metadata = FrameMetadata::new();
    metadata.insert(KEY_AGENT_ID.to_string(), agent_id.to_string());
    metadata.insert(KEY_PROVIDER.to_string(), provider.to_string());
    metadata.insert(KEY_MODEL.to_string(), model.to_string());
    metadata.insert(KEY_PROVIDER_TYPE.to_string(), provider_type.to_string());
    metadata.insert(KEY_PROMPT.to_string(), prompt.to_string());
    metadata
}

/// Validate frame metadata at the shared write boundary.
pub fn validate_frame_metadata(metadata: &FrameMetadata, agent_id: &str) -> Result<(), ApiError> {
    let mut total_bytes = 0usize;
    for (key, value) in metadata {
        if FORBIDDEN_KEYS.contains(&key.as_str()) {
            return Err(ApiError::FrameMetadataForbiddenKey { key: key.clone() });
        }

        if !ALLOWED_KEYS.contains(&key.as_str()) {
            return Err(ApiError::FrameMetadataUnknownKey { key: key.clone() });
        }

        let entry_bytes = key.len() + value.len();
        if entry_bytes > METADATA_PER_KEY_MAX_BYTES {
            return Err(ApiError::FrameMetadataPerKeyBudgetExceeded {
                key: key.clone(),
                actual_bytes: entry_bytes,
                max_bytes: METADATA_PER_KEY_MAX_BYTES,
            });
        }

        total_bytes += entry_bytes;
        if total_bytes > METADATA_TOTAL_MAX_BYTES {
            return Err(ApiError::FrameMetadataTotalBudgetExceeded {
                actual_bytes: total_bytes,
                max_bytes: METADATA_TOTAL_MAX_BYTES,
            });
        }
    }

    let frame_agent_id =
        metadata
            .get(KEY_AGENT_ID)
            .ok_or_else(|| ApiError::InvalidFrame(format!(
                "Frame missing {} in metadata",
                KEY_AGENT_ID
            )))?;
    if frame_agent_id != agent_id {
        return Err(ApiError::InvalidFrame(format!(
            "Frame metadata {} '{}' does not match provided agent_id '{}'",
            KEY_AGENT_ID, frame_agent_id, agent_id
        )));
    }

    Ok(())
}
