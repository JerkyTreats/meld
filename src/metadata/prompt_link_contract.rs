//! Prompt link contract for generation lineage payload.

use crate::error::ApiError;
use crate::prompt_context::contracts::PromptContextLineageContract;
use serde::{Deserialize, Serialize};

const PROMPT_LINK_PREFIX_BYTES: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptLinkContractV1 {
    pub prompt_link_id: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub system_prompt_artifact_id: String,
    pub user_prompt_template_artifact_id: String,
    pub rendered_prompt_artifact_id: String,
    pub context_artifact_id: String,
}

impl PromptLinkContractV1 {
    pub fn from_lineage(lineage: &PromptContextLineageContract) -> Self {
        Self {
            prompt_link_id: lineage.prompt_link_id.clone(),
            prompt_digest: lineage.prompt_digest.clone(),
            context_digest: lineage.context_digest.clone(),
            system_prompt_artifact_id: lineage.system_prompt.artifact_id.clone(),
            user_prompt_template_artifact_id: lineage.user_prompt_template.artifact_id.clone(),
            rendered_prompt_artifact_id: lineage.rendered_prompt.artifact_id.clone(),
            context_artifact_id: lineage.context_payload.artifact_id.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), ApiError> {
        if !self.prompt_link_id.starts_with("prompt-link-") {
            return Err(ApiError::PromptLinkContractInvalid {
                reason: "prompt_link_id must start with prompt-link-".to_string(),
            });
        }

        for (name, value) in [
            ("prompt_digest", self.prompt_digest.as_str()),
            ("context_digest", self.context_digest.as_str()),
            (
                "system_prompt_artifact_id",
                self.system_prompt_artifact_id.as_str(),
            ),
            (
                "user_prompt_template_artifact_id",
                self.user_prompt_template_artifact_id.as_str(),
            ),
            (
                "rendered_prompt_artifact_id",
                self.rendered_prompt_artifact_id.as_str(),
            ),
            ("context_artifact_id", self.context_artifact_id.as_str()),
        ] {
            if !is_blake3_hex(value) {
                return Err(ApiError::PromptLinkContractInvalid {
                    reason: format!("{} must be 64 char lowercase hex", name),
                });
            }
        }

        Ok(())
    }
}

pub fn build_prompt_link_id(prompt_digest: &str) -> String {
    let prompt_link_suffix_len = PROMPT_LINK_PREFIX_BYTES.min(prompt_digest.len());
    format!("prompt-link-{}", &prompt_digest[..prompt_link_suffix_len])
}

fn is_blake3_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_context::contracts::{PromptContextArtifactKind, PromptContextArtifactRef};

    #[test]
    fn validates_contract() {
        let digest = "a".repeat(64);
        let contract = PromptLinkContractV1 {
            prompt_link_id: build_prompt_link_id(&digest),
            prompt_digest: digest.clone(),
            context_digest: digest.clone(),
            system_prompt_artifact_id: digest.clone(),
            user_prompt_template_artifact_id: digest.clone(),
            rendered_prompt_artifact_id: digest.clone(),
            context_artifact_id: digest,
        };
        contract.validate().unwrap();
    }

    #[test]
    fn from_lineage_maps_ids_and_digests() {
        let digest = "b".repeat(64);
        let lineage = PromptContextLineageContract {
            prompt_link_id: "prompt-link-bbbbbbbbbbbbbbbb".to_string(),
            prompt_digest: digest.clone(),
            context_digest: digest.clone(),
            system_prompt: PromptContextArtifactRef {
                artifact_id: digest.clone(),
                digest: digest.clone(),
                byte_len: 1,
                kind: PromptContextArtifactKind::SystemPrompt,
            },
            user_prompt_template: PromptContextArtifactRef {
                artifact_id: digest.clone(),
                digest: digest.clone(),
                byte_len: 1,
                kind: PromptContextArtifactKind::UserPromptTemplate,
            },
            rendered_prompt: PromptContextArtifactRef {
                artifact_id: digest.clone(),
                digest: digest.clone(),
                byte_len: 1,
                kind: PromptContextArtifactKind::RenderedPrompt,
            },
            context_payload: PromptContextArtifactRef {
                artifact_id: digest.clone(),
                digest: digest.clone(),
                byte_len: 1,
                kind: PromptContextArtifactKind::ContextPayload,
            },
        };

        let contract = PromptLinkContractV1::from_lineage(&lineage);
        assert_eq!(contract.prompt_digest, lineage.prompt_digest);
        assert_eq!(contract.context_digest, lineage.context_digest);
        assert_eq!(
            contract.context_artifact_id,
            lineage.context_payload.artifact_id
        );
    }
}
