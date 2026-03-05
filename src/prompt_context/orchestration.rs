//! Orchestration helpers for prompt context lineage persistence.

use crate::error::ApiError;
use crate::metadata::frame_write_contract::GeneratedFrameMetadataInput;
use crate::metadata::prompt_link_contract::build_prompt_link_id;
use crate::metadata::prompt_link_contract::PromptLinkContractV1;
use crate::prompt_context::contracts::{PromptContextArtifactKind, PromptContextLineageContract};
use crate::prompt_context::storage::PromptContextArtifactStorage;

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
    pub prompt_link_contract: PromptLinkContractV1,
    pub metadata_input: GeneratedFrameMetadataInput,
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

pub fn prepare_generated_lineage(
    storage: &PromptContextArtifactStorage,
    input: &PromptContextLineageInput,
    agent_id: &str,
    provider: &str,
    model: &str,
    provider_type: &str,
) -> Result<PreparedPromptContextLineage, ApiError> {
    let lineage = persist_prompt_context_lineage(storage, input)?;
    let prompt_link_contract = PromptLinkContractV1::from_lineage(&lineage);
    prompt_link_contract.validate()?;

    let metadata_input = GeneratedFrameMetadataInput {
        agent_id: agent_id.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        provider_type: provider_type.to_string(),
        prompt_digest: prompt_link_contract.prompt_digest.clone(),
        context_digest: prompt_link_contract.context_digest.clone(),
        prompt_link_id: prompt_link_contract.prompt_link_id.clone(),
    };

    Ok(PreparedPromptContextLineage {
        lineage,
        prompt_link_contract,
        metadata_input,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn prepare_generated_lineage_builds_valid_metadata_input() {
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
            "writer",
            "provider",
            "model",
            "local",
        )
        .unwrap();

        assert_eq!(prepared.metadata_input.agent_id, "writer");
        assert_eq!(prepared.metadata_input.provider, "provider");
        assert_eq!(prepared.metadata_input.model, "model");
        assert_eq!(prepared.metadata_input.provider_type, "local");
        assert_eq!(
            prepared.metadata_input.prompt_digest,
            prepared.lineage.prompt_digest
        );
        assert_eq!(
            prepared.metadata_input.context_digest,
            prepared.lineage.context_digest
        );
        prepared.prompt_link_contract.validate().unwrap();
    }
}
