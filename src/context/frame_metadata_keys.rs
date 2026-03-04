//! Context owned frame metadata key declarations.

use crate::metadata::frame_key_descriptor::{
    FrameMetadataHashImpact, FrameMetadataKeyDescriptor, FrameMetadataMutabilityClass,
    FrameMetadataRedactionPolicy, FrameMetadataRetentionPolicy, FrameMetadataSchemaType,
    FrameMetadataVisibilityPolicy, FrameMetadataWritePolicy, DESCRIPTOR_DEFAULT_MAX_BYTES,
};

pub const KEY_AGENT_ID: &str = "agent_id";
pub const KEY_PROMPT: &str = "prompt";
pub const KEY_DELETED: &str = "deleted";
pub const FORBIDDEN_KEY_CONTEXT: &str = "context";
pub const FORBIDDEN_KEY_RAW_PROMPT: &str = "raw_prompt";
pub const FORBIDDEN_KEY_RAW_CONTEXT: &str = "raw_context";

pub const DESCRIPTOR_AGENT_ID: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_AGENT_ID,
    owner_domain: "context",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Identity,
    hash_impact: FrameMetadataHashImpact::HashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::Persistent,
    redaction_policy: FrameMetadataRedactionPolicy::HiddenByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
};

// Raw prompt payload is forbidden during metadata contract readiness hardening.
pub const DESCRIPTOR_PROMPT: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_PROMPT,
    owner_domain: "context",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Ephemeral,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::RuntimeOnly,
    redaction_policy: FrameMetadataRedactionPolicy::NeverReturn,
    write_policy: FrameMetadataWritePolicy::Forbidden,
    visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
};

pub const DESCRIPTOR_DELETED: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_DELETED,
    owner_domain: "context",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Annotation,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::WorkflowScoped,
    redaction_policy: FrameMetadataRedactionPolicy::HiddenByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
};

pub const DESCRIPTOR_CONTEXT: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: FORBIDDEN_KEY_CONTEXT,
    owner_domain: "context",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Ephemeral,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::RuntimeOnly,
    redaction_policy: FrameMetadataRedactionPolicy::NeverReturn,
    write_policy: FrameMetadataWritePolicy::Forbidden,
    visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
};

pub const DESCRIPTOR_RAW_PROMPT: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: FORBIDDEN_KEY_RAW_PROMPT,
    owner_domain: "context",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Ephemeral,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::RuntimeOnly,
    redaction_policy: FrameMetadataRedactionPolicy::NeverReturn,
    write_policy: FrameMetadataWritePolicy::Forbidden,
    visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
};

pub const DESCRIPTOR_RAW_CONTEXT: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: FORBIDDEN_KEY_RAW_CONTEXT,
    owner_domain: "context",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Ephemeral,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::RuntimeOnly,
    redaction_policy: FrameMetadataRedactionPolicy::NeverReturn,
    write_policy: FrameMetadataWritePolicy::Forbidden,
    visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
};
