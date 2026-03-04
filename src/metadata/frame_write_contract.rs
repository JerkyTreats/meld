//! Shared frame metadata write boundary.

use crate::error::ApiError;
use crate::metadata::frame_key_registry::{
    frame_metadata_key_descriptor, FrameMetadataWritePolicy, KEY_AGENT_ID, KEY_MODEL,
    KEY_PROMPT_DIGEST, KEY_PROMPT_LINK_ID, KEY_PROVIDER, KEY_PROVIDER_TYPE,
};
use crate::metadata::frame_types::FrameMetadata;

pub const METADATA_PER_KEY_MAX_BYTES: usize = 16 * 1024;
pub const METADATA_TOTAL_MAX_BYTES: usize = 64 * 1024;

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

    let prompt_digest = blake3::hash(prompt.as_bytes()).to_hex().to_string();
    let prompt_link_suffix_len = 16usize.min(prompt_digest.len());
    let prompt_link_id = format!("prompt-link-{}", &prompt_digest[..prompt_link_suffix_len]);
    metadata.insert(KEY_PROMPT_DIGEST.to_string(), prompt_digest);
    metadata.insert(KEY_PROMPT_LINK_ID.to_string(), prompt_link_id);

    metadata
}

/// Validate frame metadata at the shared write boundary.
pub fn validate_frame_metadata(metadata: &FrameMetadata, agent_id: &str) -> Result<(), ApiError> {
    let mut total_bytes = 0usize;
    for (key, value) in metadata {
        let Some(descriptor) = frame_metadata_key_descriptor(key) else {
            return Err(ApiError::FrameMetadataUnknownKey { key: key.clone() });
        };

        if descriptor.write_policy == FrameMetadataWritePolicy::Forbidden {
            return Err(ApiError::FrameMetadataForbiddenKey { key: key.clone() });
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
