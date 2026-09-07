//! Installed Docs judgment instructions and explicitly selected guard operators.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::error::ApiError;
use crate::provider::{ChatMessage, MessageRole};

/// Exact semantic resources admitted with a Docs policy revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsSemanticTheory {
    pub schema_version: u32,
    pub source_extraction: String,
    pub readme_judgment: String,
    pub correspondence: String,
    pub drafting: String,
    pub revision: String,
    pub claim_guards: Vec<DocsClaimGuard>,
}

/// Named evaluator contracts. An empty selection delegates semantic entailment
/// to the configured judge; it never disables identity or citation integrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsClaimGuard {
    LiteralPresenceV1,
    CodeLineDirectPresenceV1,
    ClauseTermCoverageV1,
}

#[derive(Clone, Copy)]
pub(crate) enum DocsJudgmentOperation {
    SourceExtraction,
    ReadmeJudgment,
    Correspondence,
    Drafting,
    Revision,
}

pub(crate) struct DocsGeneration {
    pub request: crate::context::generation::contracts::GenerationOrchestrationRequest,
    pub messages: Vec<ChatMessage>,
}

impl DocsJudgmentOperation {
    fn frame_type(self) -> &'static str {
        match self {
            Self::SourceExtraction => "docs-source-claims",
            Self::ReadmeJudgment => "docs-claim-validation",
            Self::Correspondence => "docs-claim-correspondence",
            Self::Drafting => "docs-readme",
            Self::Revision => "docs-readme-revision",
        }
    }
}

impl DocsSemanticTheory {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.schema_version != 1 {
            return Err(invalid("unsupported Docs semantic theory schema"));
        }
        if [
            &self.source_extraction,
            &self.readme_judgment,
            &self.correspondence,
            &self.drafting,
            &self.revision,
        ]
        .iter()
        .any(|instruction| instruction.trim().is_empty())
        {
            return Err(invalid(
                "Docs semantic theory requires every judgment instruction",
            ));
        }
        if self.claim_guards.iter().collect::<BTreeSet<_>>().len() != self.claim_guards.len() {
            return Err(invalid("Docs semantic theory repeats a guard operator"));
        }
        Ok(())
    }

    /// Fingerprint the actual bounded messages and selected bindings, so source
    /// or instruction changes cannot reuse a request identity for different input.
    pub(crate) fn generation(
        &self,
        config: &super::capability::DocsCapabilityConfig,
        policy_identity: &str,
        operation: DocsJudgmentOperation,
        input: serde_json::Value,
        retry_count: usize,
        batch_index: usize,
    ) -> Result<DocsGeneration, ApiError> {
        let messages = self.messages(operation, input)?;
        let bytes = serde_json::to_vec(&(
            policy_identity,
            &config.subject_id,
            &config.agent_id,
            &config.target_root,
            &config.provider,
            operation.frame_type(),
            &messages,
            retry_count,
            batch_index,
        ))
        .map_err(|error| invalid(&error.to_string()))?;
        let digest = blake3::hash(&bytes);
        let mut request_bytes = [0; 8];
        request_bytes.copy_from_slice(&digest.as_bytes()[..8]);
        Ok(DocsGeneration {
            request: crate::context::generation::contracts::GenerationOrchestrationRequest {
                request_id: u64::from_le_bytes(request_bytes),
                node_id: *digest.as_bytes(),
                agent_id: config.agent_id.clone(),
                provider: config.provider.clone(),
                frame_type: operation.frame_type().into(),
                retry_count,
                force: true,
            },
            messages,
        })
    }

    pub(crate) fn messages(
        &self,
        operation: DocsJudgmentOperation,
        input: serde_json::Value,
    ) -> Result<Vec<ChatMessage>, ApiError> {
        self.validate()?;
        let instruction = match operation {
            DocsJudgmentOperation::SourceExtraction => &self.source_extraction,
            DocsJudgmentOperation::ReadmeJudgment => &self.readme_judgment,
            DocsJudgmentOperation::Correspondence => &self.correspondence,
            DocsJudgmentOperation::Drafting => &self.drafting,
            DocsJudgmentOperation::Revision => &self.revision,
        };
        Ok(vec![
            ChatMessage {
                role: MessageRole::System,
                content: instruction.clone(),
            },
            ChatMessage {
                role: MessageRole::User,
                content: serde_json::to_string(&input)
                    .map_err(|error| invalid(&error.to_string()))?,
            },
        ])
    }
}

fn invalid(message: &str) -> ApiError {
    ApiError::ConfigError(message.into())
}

#[cfg(test)]
mod tests;
