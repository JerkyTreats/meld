//! Shared frame metadata key descriptor contract types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMetadataWritePolicy {
    Allowed,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMetadataVisibilityPolicy {
    VisibleByDefault,
    HiddenByDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMetadataMutabilityClass {
    Identity,
    Attested,
    Annotation,
    Ephemeral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMetadataHashImpact {
    HashCritical,
    NonHashCritical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMetadataRetentionPolicy {
    Persistent,
    WorkflowScoped,
    RuntimeOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMetadataRedactionPolicy {
    VisibleByDefault,
    HiddenByDefault,
    PrivilegedOnly,
    NeverReturn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMetadataSchemaType {
    Utf8String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameMetadataKeyDescriptor {
    pub key: &'static str,
    pub owner_domain: &'static str,
    pub schema_type: FrameMetadataSchemaType,
    pub mutability_class: FrameMetadataMutabilityClass,
    pub hash_impact: FrameMetadataHashImpact,
    pub max_bytes: usize,
    pub retention_policy: FrameMetadataRetentionPolicy,
    pub redaction_policy: FrameMetadataRedactionPolicy,
    pub write_policy: FrameMetadataWritePolicy,
    pub visibility_policy: FrameMetadataVisibilityPolicy,
}

pub const DESCRIPTOR_DEFAULT_MAX_BYTES: usize = 16 * 1024;
