//! Prompt context contracts for artifact refs and lineage payload.

use crate::context::belief_context::BELIEF_CONTEXT_BUNDLE_ARTIFACT_TYPE_ID;
use serde::{Deserialize, Serialize};

pub const MAX_PROMPT_ARTIFACT_BYTES: usize = 256 * 1024;
pub const MAX_CONTEXT_ARTIFACT_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptContextArtifactKind {
    SystemPrompt,
    UserPromptTemplate,
    RenderedPrompt,
    ContextPayload,
    /// Canonical belief context bundle that conditioned a rendered prompt.
    BeliefContextBundle,
}

impl PromptContextArtifactKind {
    pub fn max_bytes(self) -> usize {
        match self {
            PromptContextArtifactKind::SystemPrompt => MAX_PROMPT_ARTIFACT_BYTES,
            PromptContextArtifactKind::UserPromptTemplate => MAX_PROMPT_ARTIFACT_BYTES,
            PromptContextArtifactKind::RenderedPrompt => MAX_PROMPT_ARTIFACT_BYTES,
            PromptContextArtifactKind::ContextPayload => MAX_CONTEXT_ARTIFACT_BYTES,
            PromptContextArtifactKind::BeliefContextBundle => MAX_CONTEXT_ARTIFACT_BYTES,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PromptContextArtifactKind::SystemPrompt => "system_prompt",
            PromptContextArtifactKind::UserPromptTemplate => "user_prompt_template",
            PromptContextArtifactKind::RenderedPrompt => "rendered_prompt",
            PromptContextArtifactKind::ContextPayload => "context_payload",
            PromptContextArtifactKind::BeliefContextBundle => {
                BELIEF_CONTEXT_BUNDLE_ARTIFACT_TYPE_ID
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptContextArtifactRef {
    pub artifact_id: String,
    pub digest: String,
    pub byte_len: usize,
    pub kind: PromptContextArtifactKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptContextLineageContract {
    pub prompt_link_id: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub system_prompt: PromptContextArtifactRef,
    pub user_prompt_template: PromptContextArtifactRef,
    pub rendered_prompt: PromptContextArtifactRef,
    pub context_payload: PromptContextArtifactRef,
    /// Digest-addressed belief context bundle, present only for
    /// `belief_context`-conditioned generations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belief_context_bundle: Option<PromptContextArtifactRef>,
}
