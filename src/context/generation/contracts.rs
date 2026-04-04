use crate::metadata::frame_types::FrameMetadata;
use crate::metadata::frame_write_contract::GeneratedFrameMetadataInput;
use crate::provider::ChatMessage;
use crate::provider::ProviderExecutionBinding;
use crate::types::NodeID;
use serde::{Deserialize, Serialize};

pub type GeneratedMetadataBuilder =
    dyn Fn(&GeneratedFrameMetadataInput) -> FrameMetadata + Send + Sync;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOrchestrationRequest {
    pub request_id: u64,
    pub node_id: NodeID,
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
