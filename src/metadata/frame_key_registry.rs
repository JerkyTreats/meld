//! Frame metadata key registry and policy descriptors.

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
pub struct FrameMetadataKeyDescriptor {
    pub key: &'static str,
    pub write_policy: FrameMetadataWritePolicy,
    pub visibility_policy: FrameMetadataVisibilityPolicy,
}

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

const FRAME_METADATA_KEY_REGISTRY: &[FrameMetadataKeyDescriptor] = &[
    FrameMetadataKeyDescriptor {
        key: KEY_AGENT_ID,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: KEY_PROVIDER,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: KEY_MODEL,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: KEY_PROVIDER_TYPE,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
    },
    // Raw prompt payload is forbidden during metadata contract readiness hardening.
    FrameMetadataKeyDescriptor {
        key: KEY_PROMPT,
        write_policy: FrameMetadataWritePolicy::Forbidden,
        visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: KEY_DELETED,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: KEY_PROMPT_DIGEST,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: KEY_CONTEXT_DIGEST,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: KEY_PROMPT_LINK_ID,
        write_policy: FrameMetadataWritePolicy::Allowed,
        visibility_policy: FrameMetadataVisibilityPolicy::VisibleByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: FORBIDDEN_KEY_CONTEXT,
        write_policy: FrameMetadataWritePolicy::Forbidden,
        visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: FORBIDDEN_KEY_RAW_PROMPT,
        write_policy: FrameMetadataWritePolicy::Forbidden,
        visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
    },
    FrameMetadataKeyDescriptor {
        key: FORBIDDEN_KEY_RAW_CONTEXT,
        write_policy: FrameMetadataWritePolicy::Forbidden,
        visibility_policy: FrameMetadataVisibilityPolicy::HiddenByDefault,
    },
];

pub fn frame_metadata_key_descriptor(key: &str) -> Option<&'static FrameMetadataKeyDescriptor> {
    FRAME_METADATA_KEY_REGISTRY.iter().find(|descriptor| descriptor.key == key)
}

pub fn is_key_visible_by_default(key: &str) -> bool {
    frame_metadata_key_descriptor(key).is_some_and(|descriptor| {
        descriptor.visibility_policy == FrameMetadataVisibilityPolicy::VisibleByDefault
    })
}

