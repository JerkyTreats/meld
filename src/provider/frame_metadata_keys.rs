//! Provider owned frame metadata key declarations.

use crate::metadata::frame_key_descriptor::{
    FrameMetadataHashImpact, FrameMetadataKeyDescriptor, FrameMetadataMutabilityClass,
    FrameMetadataRedactionPolicy, FrameMetadataRetentionPolicy, FrameMetadataSchemaType,
    FrameMetadataVisibilityPolicy, FrameMetadataWritePolicy, DESCRIPTOR_DEFAULT_MAX_BYTES,
};

pub const KEY_PROVIDER: &str = "provider";
pub const KEY_MODEL: &str = "model";
pub const KEY_PROVIDER_TYPE: &str = "provider_type";

pub const DESCRIPTOR_PROVIDER: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_PROVIDER,
    owner_domain: "provider",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Attested,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::Persistent,
    redaction_policy: FrameMetadataRedactionPolicy::VisibleByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
};

pub const DESCRIPTOR_MODEL: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_MODEL,
    owner_domain: "provider",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Attested,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::Persistent,
    redaction_policy: FrameMetadataRedactionPolicy::VisibleByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
};

pub const DESCRIPTOR_PROVIDER_TYPE: FrameMetadataKeyDescriptor = FrameMetadataKeyDescriptor {
    key: KEY_PROVIDER_TYPE,
    owner_domain: "provider",
    schema_type: FrameMetadataSchemaType::Utf8String,
    mutability_class: FrameMetadataMutabilityClass::Attested,
    hash_impact: FrameMetadataHashImpact::NonHashCritical,
    max_bytes: DESCRIPTOR_DEFAULT_MAX_BYTES,
    retention_policy: FrameMetadataRetentionPolicy::Persistent,
    redaction_policy: FrameMetadataRedactionPolicy::VisibleByDefault,
    write_policy: FrameMetadataWritePolicy::Allowed,
    visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
};
