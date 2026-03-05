//! Shared frame metadata write boundary.

use crate::error::ApiError;
use crate::metadata::frame_key_registry::{
    frame_metadata_key_descriptor, frame_metadata_key_descriptors, FrameMetadataMutabilityClass,
    FrameMetadataWritePolicy, KEY_AGENT_ID, KEY_CONTEXT_DIGEST, KEY_MODEL, KEY_PROMPT_DIGEST,
    KEY_PROMPT_LINK_ID, KEY_PROVIDER, KEY_PROVIDER_TYPE,
};
use crate::metadata::frame_types::FrameMetadata;

pub const METADATA_PER_KEY_MAX_BYTES: usize = 16 * 1024;
pub const METADATA_TOTAL_MAX_BYTES: usize = 64 * 1024;
const PROMPT_LINK_PREFIX_BYTES: usize = 16;
const REQUIRED_FRAME_METADATA_KEYS: [&str; 7] = [
    KEY_AGENT_ID,
    KEY_PROVIDER,
    KEY_MODEL,
    KEY_PROVIDER_TYPE,
    KEY_PROMPT_DIGEST,
    KEY_CONTEXT_DIGEST,
    KEY_PROMPT_LINK_ID,
];

pub struct FrameMetadataValidationInput<'a> {
    pub metadata: &'a FrameMetadata,
    pub actor_agent_id: &'a str,
    pub previous_metadata: Option<&'a FrameMetadata>,
}

/// Build frame metadata for generation queue writes.
pub fn build_generated_metadata(
    agent_id: &str,
    provider: &str,
    model: &str,
    provider_type: &str,
    prompt: &str,
    context_payload: &str,
) -> FrameMetadata {
    let mut metadata = FrameMetadata::new();
    metadata.insert(KEY_AGENT_ID.to_string(), agent_id.to_string());
    metadata.insert(KEY_PROVIDER.to_string(), provider.to_string());
    metadata.insert(KEY_MODEL.to_string(), model.to_string());
    metadata.insert(KEY_PROVIDER_TYPE.to_string(), provider_type.to_string());

    let prompt_digest = blake3::hash(prompt.as_bytes()).to_hex().to_string();
    let context_digest = blake3::hash(context_payload.as_bytes())
        .to_hex()
        .to_string();
    let prompt_link_suffix_len = PROMPT_LINK_PREFIX_BYTES.min(prompt_digest.len());
    let prompt_link_id = format!("prompt-link-{}", &prompt_digest[..prompt_link_suffix_len]);
    metadata.insert(KEY_PROMPT_DIGEST.to_string(), prompt_digest);
    metadata.insert(KEY_CONTEXT_DIGEST.to_string(), context_digest);
    metadata.insert(KEY_PROMPT_LINK_ID.to_string(), prompt_link_id);

    metadata
}

/// Validate frame metadata at the shared write boundary.
pub fn validate_frame_metadata(input: FrameMetadataValidationInput<'_>) -> Result<(), ApiError> {
    let metadata = input.metadata;

    validate_known_keys(metadata)?;
    validate_write_policy(metadata)?;
    validate_required_keys(metadata)?;
    validate_agent_identity(metadata, input.actor_agent_id)?;
    validate_size_budget(metadata)?;

    if let Some(previous_metadata) = input.previous_metadata {
        validate_mutability_transition(metadata, previous_metadata)?;
    }

    Ok(())
}

fn validate_known_keys(metadata: &FrameMetadata) -> Result<(), ApiError> {
    for key in sorted_keys(metadata) {
        if frame_metadata_key_descriptor(key).is_none() {
            return Err(ApiError::FrameMetadataUnknownKey { key: key.clone() });
        }
    }
    Ok(())
}

fn validate_write_policy(metadata: &FrameMetadata) -> Result<(), ApiError> {
    for key in sorted_keys(metadata) {
        let descriptor = frame_metadata_key_descriptor(key).expect("known key");
        if descriptor.write_policy == FrameMetadataWritePolicy::Forbidden {
            return Err(ApiError::FrameMetadataForbiddenKey { key: key.clone() });
        }
    }
    Ok(())
}

fn validate_required_keys(metadata: &FrameMetadata) -> Result<(), ApiError> {
    for key in REQUIRED_FRAME_METADATA_KEYS {
        if !metadata.contains_key(key) {
            return Err(ApiError::FrameMetadataMissingRequiredKey {
                key: key.to_string(),
            });
        }
    }
    Ok(())
}

fn validate_agent_identity(metadata: &FrameMetadata, actor_agent_id: &str) -> Result<(), ApiError> {
    let frame_agent_id = metadata
        .get(KEY_AGENT_ID)
        .expect("required key must be present before agent validation");

    if frame_agent_id != actor_agent_id {
        return Err(ApiError::InvalidFrame(format!(
            "Frame metadata {} '{}' does not match provided agent_id '{}'",
            KEY_AGENT_ID, frame_agent_id, actor_agent_id
        )));
    }

    Ok(())
}

fn validate_size_budget(metadata: &FrameMetadata) -> Result<(), ApiError> {
    let mut total_bytes = 0usize;
    for key in sorted_keys(metadata) {
        let value = metadata.get(key).expect("key from sorted metadata set");
        let descriptor = frame_metadata_key_descriptor(key).expect("known key");

        let entry_bytes = key.len() + value.len();
        if entry_bytes > descriptor.max_bytes {
            return Err(ApiError::FrameMetadataPerKeyBudgetExceeded {
                key: key.clone(),
                actual_bytes: entry_bytes,
                max_bytes: descriptor.max_bytes,
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

    Ok(())
}

fn validate_mutability_transition(
    metadata: &FrameMetadata,
    previous_metadata: &FrameMetadata,
) -> Result<(), ApiError> {
    let mut immutable_keys = frame_metadata_key_descriptors()
        .iter()
        .filter(|descriptor| descriptor.write_policy == FrameMetadataWritePolicy::Allowed)
        .filter(|descriptor| {
            matches!(
                descriptor.mutability_class,
                FrameMetadataMutabilityClass::Identity | FrameMetadataMutabilityClass::Attested
            )
        })
        .collect::<Vec<_>>();
    immutable_keys.sort_by_key(|descriptor| descriptor.key);

    for descriptor in immutable_keys {
        let previous = previous_metadata.get(descriptor.key);
        if let Some(previous_value) = previous {
            let current = metadata.get(descriptor.key);
            if current != Some(previous_value) {
                return Err(ApiError::FrameMetadataMutabilityViolation {
                    key: descriptor.key.to_string(),
                    class: descriptor.mutability_class,
                });
            }
        }
    }

    Ok(())
}

fn sorted_keys(metadata: &FrameMetadata) -> Vec<&String> {
    let mut keys = metadata.keys().collect::<Vec<_>>();
    keys.sort_unstable_by(|left, right| left.as_str().cmp(right.as_str()));
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn required_metadata(agent_id: &str) -> FrameMetadata {
        let mut metadata = FrameMetadata::new();
        metadata.insert(KEY_AGENT_ID.to_string(), agent_id.to_string());
        metadata.insert(KEY_PROVIDER.to_string(), "provider-a".to_string());
        metadata.insert(KEY_MODEL.to_string(), "model-a".to_string());
        metadata.insert(KEY_PROVIDER_TYPE.to_string(), "local".to_string());
        metadata.insert(KEY_PROMPT_DIGEST.to_string(), "prompt-a".to_string());
        metadata.insert(KEY_CONTEXT_DIGEST.to_string(), "context-a".to_string());
        metadata.insert(KEY_PROMPT_LINK_ID.to_string(), "prompt-link-a".to_string());
        metadata
    }

    #[test]
    fn build_generated_metadata_includes_context_digest() {
        let metadata = build_generated_metadata(
            "writer-a",
            "provider-a",
            "model-a",
            "local",
            "task prompt",
            "prompt context",
        );
        assert!(metadata.contains_key(KEY_PROMPT_DIGEST));
        assert!(metadata.contains_key(KEY_CONTEXT_DIGEST));
        assert!(metadata.contains_key(KEY_PROMPT_LINK_ID));
    }

    #[test]
    fn validate_rejects_missing_required_keys() {
        let mut metadata = required_metadata("writer-a");
        metadata.remove(KEY_CONTEXT_DIGEST);
        let result = validate_frame_metadata(FrameMetadataValidationInput {
            metadata: &metadata,
            actor_agent_id: "writer-a",
            previous_metadata: None,
        });
        assert!(matches!(
            result,
            Err(ApiError::FrameMetadataMissingRequiredKey { .. })
        ));
    }

    #[test]
    fn validate_rejects_attested_mutability_changes() {
        let previous_metadata = required_metadata("writer-a");
        let mut metadata = previous_metadata.clone();
        metadata.insert(KEY_MODEL.to_string(), "model-b".to_string());
        let result = validate_frame_metadata(FrameMetadataValidationInput {
            metadata: &metadata,
            actor_agent_id: "writer-a",
            previous_metadata: Some(&previous_metadata),
        });
        assert!(matches!(
            result,
            Err(ApiError::FrameMetadataMutabilityViolation { .. })
        ));
    }

    #[test]
    fn forbidden_error_precedes_missing_required_keys() {
        let mut metadata = FrameMetadata::new();
        metadata.insert("raw_prompt".to_string(), "raw".to_string());
        let result = validate_frame_metadata(FrameMetadataValidationInput {
            metadata: &metadata,
            actor_agent_id: "writer-a",
            previous_metadata: None,
        });
        assert!(matches!(
            result,
            Err(ApiError::FrameMetadataForbiddenKey { .. })
        ));
    }
}
