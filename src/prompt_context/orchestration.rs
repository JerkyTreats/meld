//! Orchestration helpers for prompt context lineage persistence.

use crate::error::ApiError;
use crate::execution::PromptArtifactReadPort;
use crate::metadata::prompt_link_contract::build_prompt_link_id;
use crate::prompt_context::contracts::{PromptContextArtifactKind, PromptContextLineageContract};

#[derive(Debug, Clone)]
pub struct PromptContextLineageInput {
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub rendered_prompt: String,
    pub context_payload: String,
}

#[derive(Debug, Clone)]
pub struct PreparedPromptContextLineage {
    pub lineage: PromptContextLineageContract,
}

pub fn persist_prompt_context_lineage(
    storage: &(impl PromptArtifactReadPort + ?Sized),
    input: &PromptContextLineageInput,
) -> Result<PromptContextLineageContract, ApiError> {
    let system_prompt = storage.write_prompt_artifact_utf8(
        PromptContextArtifactKind::SystemPrompt,
        &input.system_prompt,
    )?;
    let user_prompt_template = storage.write_prompt_artifact_utf8(
        PromptContextArtifactKind::UserPromptTemplate,
        &input.user_prompt_template,
    )?;
    let rendered_prompt = storage.write_prompt_artifact_utf8(
        PromptContextArtifactKind::RenderedPrompt,
        &input.rendered_prompt,
    )?;
    let context_payload = storage.write_prompt_artifact_utf8(
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

pub fn prepare_generated_lineage(
    storage: &(impl PromptArtifactReadPort + ?Sized),
    input: &PromptContextLineageInput,
) -> Result<PreparedPromptContextLineage, ApiError> {
    let lineage = persist_prompt_context_lineage(storage, input)?;
    Ok(PreparedPromptContextLineage { lineage })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_context::PromptContextArtifactStorage;
    use tempfile::TempDir;

    #[test]
    fn prepare_generated_lineage_carries_canonical_lineage_only() {
        let temp = TempDir::new().unwrap();
        let storage = PromptContextArtifactStorage::new(temp.path()).unwrap();

        let prepared = prepare_generated_lineage(
            &storage,
            &PromptContextLineageInput {
                system_prompt: "system".to_string(),
                user_prompt_template: "template".to_string(),
                rendered_prompt: "rendered".to_string(),
                context_payload: "context".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            prepared.lineage.prompt_digest,
            prepared.lineage.rendered_prompt.digest
        );
        assert_eq!(
            prepared.lineage.context_digest,
            prepared.lineage.context_payload.digest
        );
        assert!(prepared.lineage.prompt_link_id.starts_with("prompt-link-"));
    }
}
