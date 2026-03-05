use crate::metadata::frame_types::FrameMetadata;
use crate::provider::ChatMessage;
use crate::types::NodeID;

pub type GeneratedMetadataBuilder =
    dyn Fn(&str, &str, &str, &str, &str, &str) -> FrameMetadata + Send + Sync;

#[derive(Debug, Clone)]
pub struct GenerationOrchestrationRequest {
    pub request_id: u64,
    pub node_id: NodeID,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub retry_count: usize,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct PromptAssemblyOutput {
    pub user_prompt: String,
    pub context_payload: String,
    pub messages: Vec<ChatMessage>,
}
