//! Metadata domain owned and hosted frame metadata key declarations.

use crate::metadata::frame_key_descriptor::{
    FrameMetadataHashImpact, FrameMetadataKeyDescriptor, FrameMetadataMutabilityClass,
    FrameMetadataRedactionPolicy, FrameMetadataRetentionPolicy, FrameMetadataSchemaType,
    FrameMetadataVisibilityPolicy, FrameMetadataWritePolicy, DESCRIPTOR_DEFAULT_MAX_BYTES,
};

pub const KEY_PROMPT_DIGEST: &str = "prompt_digest";
pub const KEY_CONTEXT_DIGEST: &str = "context_digest";
pub const KEY_PROMPT_LINK_ID: &str = "prompt_link_id";

pub const DESCRIPTOR_PROMPT_DIGEST: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_PROMPT_DIGEST,
    owner_domain: "metadata",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Attested,
    hash_impact: FrameMetadataHashImpact::HashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::Persistent,
    redaction_policy: FrameMetadataRedactionPolicy::VisibleByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
};

pub const DESCRIPTOR_CONTEXT_DIGEST: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_CONTEXT_DIGEST,
    owner_domain: "metadata",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Attested,
    hash_impact: FrameMetadataHashImpact::HashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::Persistent,
    redaction_policy: FrameMetadataRedactionPolicy::VisibleByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
};

// Workflow domain declarations are hosted here until src/workflow is introduced.
pub const DESCRIPTOR_PROMPT_LINK_ID: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_PROMPT_LINK_ID,
    owner_domain: "workflow",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Attested,
    hash_impact: FrameMetadataHashImpact::HashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::Persistent,
    redaction_policy: FrameMetadataRedactionPolicy::VisibleByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
};
