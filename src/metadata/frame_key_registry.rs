//! Frame metadata key registry and policy descriptors.

use crate::context::frame_metadata_keys as context_keys;
use crate::metadata::owned_frame_metadata_keys as owned_keys;
use crate::provider::frame_metadata_keys as provider_keys;

pub use crate::metadata::frame_key_descriptor::{
    FrameMetadataHashImpact, FrameMetadataKeyDescriptor, FrameMetadataMutabilityClass,
    FrameMetadataRedactionPolicy, FrameMetadataRetentionPolicy, FrameMetadataSchemaType,
    FrameMetadataVisibilityPolicy, FrameMetadataWritePolicy,
};

pub use context_keys::{
    FORBIDDEN_KEY_CONTEXT, FORBIDDEN_KEY_RAW_CONTEXT, FORBIDDEN_KEY_RAW_PROMPT, KEY_AGENT_ID,
    KEY_DELETED, KEY_PROMPT,
};
pub use owned_keys::{KEY_CONTEXT_DIGEST, KEY_PROMPT_DIGEST, KEY_PROMPT_LINK_ID};
pub use provider_keys::{KEY_MODEL, KEY_PROVIDER, KEY_PROVIDER_TYPE};

const FRAME_METADATA_KEY_REGISTRY: &[FrameMetadataKeyDescriptor] = &[
    context_keys::DESCRIPTOR_AGENT_ID,
    provider_keys::DESCRIPTOR_PROVIDER,
    provider_keys::DESCRIPTOR_MODEL,
    provider_keys::DESCRIPTOR_PROVIDER_TYPE,
    context_keys::DESCRIPTOR_PROMPT,
    context_keys::DESCRIPTOR_DELETED,
    owned_keys::DESCRIPTOR_PROMPT_DIGEST,
    owned_keys::DESCRIPTOR_CONTEXT_DIGEST,
    owned_keys::DESCRIPTOR_PROMPT_LINK_ID,
    context_keys::DESCRIPTOR_CONTEXT,
    context_keys::DESCRIPTOR_RAW_PROMPT,
    context_keys::DESCRIPTOR_RAW_CONTEXT,
];

pub fn frame_metadata_key_descriptors() -> &'static [FrameMetadataKeyDescriptor] {
    FRAME_METADATA_KEY_REGISTRY
}

pub fn frame_metadata_key_descriptor(key: &str) -> Option<&'static FrameMetadataKeyDescriptor> {
    FRAME_METADATA_KEY_REGISTRY
        .iter()
        .find(|descriptor| descriptor.key == key)
}

pub fn is_key_visible_by_default(key: &str) -> bool {
    frame_metadata_key_descriptor(key).is_some_and(|descriptor| {
        descriptor.visibility_policy == FrameMetadataVisibilityPolicy::VisibleByDefault
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_contains_expected_key_set() {
        let expected = HashSet::from([
            KEY_AGENT_ID,
            KEY_PROVIDER,
            KEY_MODEL,
            KEY_PROVIDER_TYPE,
            KEY_PROMPT,
            KEY_DELETED,
            KEY_PROMPT_DIGEST,
            KEY_CONTEXT_DIGEST,
            KEY_PROMPT_LINK_ID,
            FORBIDDEN_KEY_CONTEXT,
            FORBIDDEN_KEY_RAW_PROMPT,
            FORBIDDEN_KEY_RAW_CONTEXT,
        ]);

        let actual = frame_metadata_key_descriptors()
            .iter()
            .map(|descriptor| descriptor.key)
            .collect::<HashSet<_>>();

        assert_eq!(actual, expected);
    }

    #[test]
    fn registry_has_no_duplicate_keys() {
        let mut seen = HashSet::new();
        for descriptor in frame_metadata_key_descriptors() {
            assert!(
                seen.insert(descriptor.key),
                "duplicate key {}",
                descriptor.key
            );
        }
    }

    #[test]
    fn descriptors_define_complete_contract_fields() {
        for descriptor in frame_metadata_key_descriptors() {
            assert!(!descriptor.owner_domain.is_empty());
            assert!(descriptor.max_bytes > 0);
            match descriptor.schema_type {
                FrameMetadataSchemaType::Utf8String => {}
            }
            assert_eq!(
                descriptor.visibility_policy,
                match descriptor.redaction_policy {
                    FrameMetadataRedactionPolicy::VisibleByDefault => {
                        FrameMetadataVisibilityPolicy::VisibleByDefault
                    }
                    FrameMetadataRedactionPolicy::HiddenByDefault
                    | FrameMetadataRedactionPolicy::PrivilegedOnly
                    | FrameMetadataRedactionPolicy::NeverReturn => {
                        FrameMetadataVisibilityPolicy::HiddenByDefault
                    }
                }
            );
        }
    }

    #[test]
    fn forbidden_keys_are_hidden_and_never_return() {
        for key in [
            KEY_PROMPT,
            FORBIDDEN_KEY_CONTEXT,
            FORBIDDEN_KEY_RAW_PROMPT,
            FORBIDDEN_KEY_RAW_CONTEXT,
        ] {
            let descriptor = frame_metadata_key_descriptor(key).expect("descriptor exists");
            assert_eq!(descriptor.write_policy, FrameMetadataWritePolicy::Forbidden);
            assert_eq!(
                descriptor.visibility_policy,
                FrameMetadataVisibilityPolicy::HiddenByDefault
            );
            assert_eq!(
                descriptor.redaction_policy,
                FrameMetadataRedactionPolicy::NeverReturn
            );
            assert_eq!(
                descriptor.mutability_class,
                FrameMetadataMutabilityClass::Ephemeral
            );
            assert_eq!(
                descriptor.retention_policy,
                FrameMetadataRetentionPolicy::RuntimeOnly
            );
        }
    }

    #[test]
    fn visible_by_default_keys_match_contract() {
        let visible = frame_metadata_key_descriptors()
            .iter()
            .filter(|descriptor| {
                descriptor.visibility_policy == FrameMetadataVisibilityPolicy::VisibleByDefault
            })
            .map(|descriptor| descriptor.key)
            .collect::<HashSet<_>>();

        let expected = HashSet::from([
            KEY_PROVIDER,
            KEY_MODEL,
            KEY_PROVIDER_TYPE,
            KEY_PROMPT_DIGEST,
            KEY_CONTEXT_DIGEST,
            KEY_PROMPT_LINK_ID,
        ]);

        assert_eq!(visible, expected);
    }

    #[test]
    fn descriptor_lookup_is_stable_for_known_and_unknown_keys() {
        assert!(frame_metadata_key_descriptor(KEY_AGENT_ID).is_some());
        assert!(frame_metadata_key_descriptor(KEY_PROMPT_DIGEST).is_some());
        assert!(frame_metadata_key_descriptor("unknown_key").is_none());
    }
}
