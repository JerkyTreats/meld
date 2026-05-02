use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::execution::contracts::ProviderExecutionBinding;

pub type NodeId = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub additional_json: BTreeMap<String, Value>,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            temperature: Some(1.0),
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            additional_json: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOrchestrationRequest {
    pub request_id: u64,
    pub node_id: NodeId,
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
    pub frame_type: String,
    pub retry_count: usize,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAssemblyOutput {
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub rendered_prompt: String,
    pub context_payload: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFrameMetadataInput {
    pub agent_id: String,
    pub provider: String,
    pub model: String,
    pub provider_type: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub prompt_link_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLineageRequest {
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub rendered_prompt: String,
    pub context_payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLinkContractView {
    pub prompt_link_id: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub system_prompt_artifact_id: String,
    pub user_prompt_template_artifact_id: String,
    pub rendered_prompt_artifact_id: String,
    pub context_artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedPromptLineage {
    pub prompt_link_contract: PromptLinkContractView,
    pub metadata_input: GeneratedFrameMetadataInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviousMetadataSnapshotView {
    pub frame_id: Option<String>,
    pub prompt_digest: Option<String>,
    pub context_digest: Option<String>,
    pub prompt_link_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptContextLineageProgressEventData {
    pub node_id: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub prompt_link_id: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub system_prompt_artifact_id: String,
    pub user_prompt_template_artifact_id: String,
    pub rendered_prompt_artifact_id: String,
    pub context_artifact_id: String,
    pub lineage_failure_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadataValidationProgressEventData {
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub prompt_digest: String,
    pub context_digest: String,
    pub prompt_link_id: String,
    pub previous_frame_id: Option<String>,
    pub previous_prompt_digest: Option<String>,
    pub previous_context_digest: Option<String>,
    pub previous_prompt_link_id: Option<String>,
    pub workflow_id: Option<String>,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub turn_seq: Option<u32>,
    pub attempt: Option<usize>,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
    pub error: Option<String>,
}
