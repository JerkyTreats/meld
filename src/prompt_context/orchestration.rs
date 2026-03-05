//! Orchestration helpers for prompt context lineage persistence.

use crate::error::ApiError;
use crate::metadata::prompt_link_contract::build_prompt_link_id;
use crate::prompt_context::contracts::{PromptContextArtifactKind, PromptContextLineageContract};
use crate::prompt_context::storage::PromptContextArtifactStorage;

#[derive(Debug, Clone)]
pub struct PromptContextLineageInput {
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub rendered_prompt: String,
    pub context_payload: String,
}

pub fn persist_prompt_context_lineage(
    storage: &PromptContextArtifactStorage,
    input: &PromptContextLineageInput,
) -> Result<PromptContextLineageContract, ApiError> {
    let system_prompt = storage.write_utf8(
        PromptContextArtifactKind::SystemPrompt,
        &input.system_prompt,
    )?;
    let user_prompt_template = storage.write_utf8(
        PromptContextArtifactKind::UserPromptTemplate,
        &input.user_prompt_template,
    )?;
    let rendered_prompt = storage.write_utf8(
        PromptContextArtifactKind::RenderedPrompt,
        &input.rendered_prompt,
    )?;
    let context_payload = storage.write_utf8(
        PromptContextArtifactKind::ContextPayload,
        &input.context_payload,
    )?;

    Ok(PromptContextLineageContract {
        prompt_link_id: build_prompt_link_id(&rendered_prompt.digest),
        prompt_digest: rendered_prompt.digest.clone(),
        context_digest: context_payload.digest.clone(),
        system_prompt,
        user_prompt_template,
        rendered_prompt,
        context_payload,
    })
}
