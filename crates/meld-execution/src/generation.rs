//! Generation DTOs shared by workflow execution and provider adapters.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::execution::contracts::ProviderExecutionBinding;

/// Binary workspace node identifier used by execution DTOs.
pub type NodeId = [u8; 32];

/// Provider chat message role used by generation requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    /// System role message sent before user or assistant content.
    System,
    /// User role message sent to provider execution.
    User,
    /// Assistant role message returned or replayed for provider execution.
    Assistant,
}

/// Single provider chat message after prompt assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Chat role assigned to this provider message.
    pub role: MessageRole,
    /// Provider message content.
    pub content: String,
}

/// Provider completion options configured by generation execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    /// Optional provider sampling temperature override.
    pub temperature: Option<f32>,
    /// Optional provider output token ceiling.
    pub max_tokens: Option<u32>,
    /// Optional provider nucleus sampling override.
    pub top_p: Option<f32>,
    /// Optional provider frequency penalty override.
    pub frequency_penalty: Option<f32>,
    /// Optional provider presence penalty override.
    pub presence_penalty: Option<f32>,
    /// Optional provider stop sequences.
    pub stop: Option<Vec<String>>,
    /// Provider-specific JSON options not modeled by the shared contract.
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

/// Provider token usage reported with a completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt token count reported by the provider.
    pub prompt_tokens: u32,
    /// Completion token count reported by the provider.
    pub completion_tokens: u32,
    /// Total token count reported by the provider.
    pub total_tokens: u32,
}

/// Provider completion response returned to execution orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Provider completion content returned to orchestration.
    pub content: String,
    /// Provider model name used for completion.
    pub model: String,
    /// Token usage reported by the provider.
    pub usage: TokenUsage,
    /// Provider finish reason when one was reported.
    pub finish_reason: Option<String>,
    /// Provider-authored execution evidence, absent from historical responses.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub execution_metadata: BTreeMap<String, Value>,
}

/// Request to run one generation orchestration against a workspace node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOrchestrationRequest {
    /// Caller supplied request identifier for generation orchestration.
    pub request_id: u64,
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: NodeId,
    /// Agent identifier responsible for this execution request.
    pub agent_id: String,
    /// Provider binding selected for execution.
    pub provider: ProviderExecutionBinding,
    /// Context frame type produced or consumed by this execution path.
    pub frame_type: String,
    /// Number of retries already attempted for this request.
    pub retry_count: usize,
    /// True when execution should bypass cached or existing output.
    pub force: bool,
}

/// Fully assembled prompt payload passed to provider execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAssemblyOutput {
    /// System prompt text supplied to provider execution.
    pub system_prompt: String,
    /// User prompt template before workflow input rendering.
    pub user_prompt_template: String,
    /// Rendered user prompt text sent to provider execution.
    pub rendered_prompt: String,
    /// Context payload assembled for provider execution.
    pub context_payload: String,
    /// Ordered provider chat messages assembled for completion.
    pub messages: Vec<ChatMessage>,
}

/// Metadata input used when writing a generated context frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFrameMetadataInput {
    /// Agent identifier responsible for this execution request.
    pub agent_id: String,
    /// Provider binding selected for execution.
    pub provider: String,
    /// Provider model name used for completion.
    pub model: String,
    /// Provider implementation type used for generated metadata.
    pub provider_type: String,
    /// Digest of the rendered prompt content.
    pub prompt_digest: String,
    /// Digest of the context payload used for generation.
    pub context_digest: String,
    /// Prompt lineage identifier associated with generated frame metadata.
    pub prompt_link_id: String,
}

/// Prompt lineage payload that should be persisted for a generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLineageRequest {
    /// System prompt text supplied to provider execution.
    pub system_prompt: String,
    /// User prompt template before workflow input rendering.
    pub user_prompt_template: String,
    /// Rendered user prompt text sent to provider execution.
    pub rendered_prompt: String,
    /// Context payload assembled for provider execution.
    pub context_payload: String,
}

/// Stable view of prompt lineage artifact identifiers and content digests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLinkContractView {
    /// Prompt lineage identifier associated with generated frame metadata.
    pub prompt_link_id: String,
    /// Digest of the rendered prompt content.
    pub prompt_digest: String,
    /// Digest of the context payload used for generation.
    pub context_digest: String,
    /// Artifact identifier for the system prompt lineage record.
    pub system_prompt_artifact_id: String,
    /// Artifact identifier for the user prompt template lineage record.
    pub user_prompt_template_artifact_id: String,
    /// Artifact identifier for the rendered prompt lineage record.
    pub rendered_prompt_artifact_id: String,
    /// Artifact identifier for the context payload lineage record.
    pub context_artifact_id: String,
    /// Digest of the persisted belief context bundle artifact, when one
    /// conditioned this prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belief_bundle_digest: Option<String>,
}

/// Prompt lineage output prepared before generated frame metadata is built.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedPromptLineage {
    /// Prompt link view persisted for generated frame metadata.
    pub prompt_link_contract: PromptLinkContractView,
    /// Generated frame metadata input derived from prompt lineage.
    pub metadata_input: GeneratedFrameMetadataInput,
}

/// Previous frame metadata snapshot used to compare generation lineage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviousMetadataSnapshotView {
    /// Context frame identifier associated with this execution record.
    pub frame_id: Option<String>,
    /// Digest of the rendered prompt content.
    pub prompt_digest: Option<String>,
    /// Digest of the context payload used for generation.
    pub context_digest: Option<String>,
    /// Prompt lineage identifier associated with generated frame metadata.
    pub prompt_link_id: Option<String>,
}

/// Progress event payload for prompt context lineage persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptContextLineageProgressEventData {
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: String,
    /// Agent identifier responsible for this execution request.
    pub agent_id: String,
    /// Configured provider name reported through progress telemetry.
    pub provider_name: String,
    /// Context frame type produced or consumed by this execution path.
    pub frame_type: String,
    /// Prompt lineage identifier associated with generated frame metadata.
    pub prompt_link_id: String,
    /// Digest of the rendered prompt content.
    pub prompt_digest: String,
    /// Digest of the context payload used for generation.
    pub context_digest: String,
    /// Artifact identifier for the system prompt lineage record.
    pub system_prompt_artifact_id: String,
    /// Artifact identifier for the user prompt template lineage record.
    pub user_prompt_template_artifact_id: String,
    /// Artifact identifier for the rendered prompt lineage record.
    pub rendered_prompt_artifact_id: String,
    /// Artifact identifier for the context payload lineage record.
    pub context_artifact_id: String,
    /// Policy name used when prompt lineage persistence fails.
    pub lineage_failure_policy: String,
}

/// Progress event payload for generated frame metadata validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadataValidationProgressEventData {
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: String,
    /// Workspace path associated with this execution record.
    pub path: String,
    /// Agent identifier responsible for this execution request.
    pub agent_id: String,
    /// Configured provider name reported through progress telemetry.
    pub provider_name: String,
    /// Context frame type produced or consumed by this execution path.
    pub frame_type: String,
    /// Digest of the rendered prompt content.
    pub prompt_digest: String,
    /// Digest of the context payload used for generation.
    pub context_digest: String,
    /// Prompt lineage identifier associated with generated frame metadata.
    pub prompt_link_id: String,
    /// Previous frame identifier used for metadata comparison when available.
    pub previous_frame_id: Option<String>,
    /// Previous prompt digest used for metadata comparison when available.
    pub previous_prompt_digest: Option<String>,
    /// Previous context digest used for metadata comparison when available.
    pub previous_context_digest: Option<String>,
    /// Previous prompt lineage identifier when available.
    pub previous_prompt_link_id: Option<String>,
    /// Workflow profile identifier that owns this execution record.
    pub workflow_id: Option<String>,
    /// Workflow thread identifier within workflow runtime state.
    pub thread_id: Option<String>,
    /// Workflow turn identifier within the owning workflow profile or thread.
    pub turn_id: Option<String>,
    /// Workflow turn sequence number reported through telemetry.
    pub turn_seq: Option<u32>,
    /// Workflow or task attempt number reported through telemetry.
    pub attempt: Option<usize>,
    /// Planning run identifier associated with this execution record.
    pub plan_id: Option<String>,
    /// Traversal level index associated with this execution record.
    pub level_index: Option<usize>,
    /// Execution error text reported through telemetry when available.
    pub error: Option<String>,
}
