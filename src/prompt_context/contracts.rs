//! Prompt context contracts for artifact refs and lineage payload.

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
}

impl PromptContextArtifactKind {
    pub fn max_bytes(self) -> usize {
        match self {
            PromptContextArtifactKind::SystemPrompt => MAX_PROMPT_ARTIFACT_BYTES,
            PromptContextArtifactKind::UserPromptTemplate => MAX_PROMPT_ARTIFACT_BYTES,
            PromptContextArtifactKind::RenderedPrompt => MAX_PROMPT_ARTIFACT_BYTES,
            PromptContextArtifactKind::ContextPayload => MAX_CONTEXT_ARTIFACT_BYTES,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PromptContextArtifactKind::SystemPrompt => "system_prompt",
            PromptContextArtifactKind::UserPromptTemplate => "user_prompt_template",
            PromptContextArtifactKind::RenderedPrompt => "rendered_prompt",
            PromptContextArtifactKind::ContextPayload => "context_payload",
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
}
