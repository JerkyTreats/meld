# Phase 2 Model Provider Specification

## Overview

This document specifies the Model Provider Abstraction component for Phase 2. It defines how agents interact with multiple LLM providers (OpenAI, Anthropic, local models) through a unified interface while maintaining provider-agnostic agent identity.

## Design Principles

### 1. Unified API Format
- **OpenAI-compatible format**: All providers (including local) use OpenAI API format
- **Consistent interface**: Single trait/interface for all providers
- **Provider-agnostic code**: Agent code doesn't need to know which provider is used

### 2. Provider Flexibility
- **Multiple providers supported**: OpenAI, Anthropic, Ollama, custom local servers
- **Easy provider switching**: Agents can change providers without code changes
- **Local-first support**: Full support for local models via Ollama or custom endpoints

### 3. Agent Identity Preservation
- **Provider-independent identity**: Agent identity separate from provider choice
- **Provider metadata**: Provider info stored in frame metadata (not in FrameID)
- **Attribution**: Frame metadata includes provider and model used

### 4. Determinism Considerations
- **FrameID determinism**: FrameID based on inputs (prompt, context, agent_id), not outputs
- **Content may vary**: LLM outputs are inherently non-deterministic (acceptable)
- **Basis tracking**: Frame basis includes prompt/context hash, not response content

## Supported Providers

### OpenAI
- **Models**: GPT-3.5, GPT-4, GPT-4 Turbo, and other OpenAI models
- **API**: OpenAI REST API
- **Configuration**: Requires API key, optional custom endpoint

### Anthropic
- **Models**: Claude 3 Opus, Sonnet, Haiku, and other Claude models
- **API**: Anthropic REST API (mapped to OpenAI-compatible format)
- **Configuration**: Requires API key

### Ollama (Local)
- **Models**: Any model available in Ollama (Llama, Mistral, etc.)
- **API**: Ollama REST API (mapped to OpenAI-compatible format)
- **Configuration**: Model name, optional custom base URL (default: http://localhost:11434)

### Custom Local
- **Models**: Any model served via OpenAI-compatible API
- **API**: OpenAI-compatible REST API
- **Configuration**: Custom endpoint URL, optional API key
- **Use cases**: LM Studio, vLLM, text-generation-webui, etc.

## Data Structures

### ModelProvider Enum
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelProvider {
    OpenAI {
        model: String,
        api_key: String,
        base_url: Option<String>,  // For custom endpoints (e.g., Azure OpenAI)
    },
    Anthropic {
        model: String,
        api_key: String,
    },
    Ollama {
        model: String,
        base_url: Option<String>,  // Default: http://localhost:11434
    },
    LocalCustom {
        model: String,
        endpoint: String,  // Full endpoint URL (e.g., http://localhost:8080/v1)
        api_key: Option<String>,
    },
}
```

### ModelProviderClient Trait
```rust
#[async_trait]
pub trait ModelProviderClient: Send + Sync {
    /// Generate a completion from a list of messages
    async fn complete(
        &self,
        messages: Vec<ChatMessage>,
        options: CompletionOptions,
    ) -> Result<CompletionResponse, ApiError>;

    /// Generate a streaming completion
    async fn stream(
        &self,
        messages: Vec<ChatMessage>,
        options: CompletionOptions,
    ) -> Result<CompletionStream, ApiError>;

    /// Get the provider name
    fn provider_name(&self) -> &str;

    /// Get the model name
    fn model_name(&self) -> &str;
}
```

### Chat Message Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub temperature: Option<f32>,      // 0.0-2.0, default: 1.0
    pub max_tokens: Option<u32>,       // Maximum tokens to generate
    pub top_p: Option<f32>,           // Nucleus sampling
    pub frequency_penalty: Option<f32>, // -2.0 to 2.0
    pub presence_penalty: Option<f32>,  // -2.0 to 2.0
    pub stop: Option<Vec<String>>,      // Stop sequences
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub type CompletionStream = Box<dyn Stream<Item = Result<String, ApiError>> + Send>;
```

## Integration with Agent Model

### Extended AgentIdentity
```rust
pub struct AgentIdentity {
    pub agent_id: String,
    pub role: AgentRole,
    pub capabilities: Vec<Capability>,
    pub provider: Option<ModelProvider>,  // Optional: agents can be provider-agnostic
}
```

### Provider Attribution in Frames
When an agent uses a provider to generate frame content, the provider information is stored in frame metadata:

```rust
frame.metadata.insert("provider".to_string(), provider_name);
frame.metadata.insert("model".to_string(), model_name);
frame.metadata.insert("provider_type".to_string(), "openai".to_string()); // or "anthropic", "ollama", "local"
```

**Note**: Provider info is NOT included in FrameID computation. FrameID is based on:
- Basis (NodeID or FrameID)
- Agent ID
- Frame type
- Content (the actual LLM response)

This means:
- Same prompt + same agent + same context → same FrameID (deterministic)
- Different LLM responses → different FrameIDs (expected, non-deterministic outputs)

## Implementation Strategy

### 1. Use Existing Crate
Leverage the `genai` crate (or similar) for unified provider abstraction:
- Provides unified API across providers
- Handles provider-specific API differences
- Supports streaming
- Well-maintained and tested

### 2. Wrap in Trait
Define `ModelProviderClient` trait that wraps the underlying crate:
- Provides clean abstraction
- Allows for future provider additions
- Enables testing with mock providers

### 3. Provider Factory
```rust
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create_client(
        provider: &ModelProvider,
    ) -> Result<Box<dyn ModelProviderClient>, ApiError> {
        match provider {
            ModelProvider::OpenAI { model, api_key, base_url } => {
                // Create OpenAI client using genai
                Ok(Box::new(OpenAIClient::new(model, api_key, base_url)?))
            }
            ModelProvider::Anthropic { model, api_key } => {
                // Create Anthropic client using genai
                Ok(Box::new(AnthropicClient::new(model, api_key)?))
            }
            ModelProvider::Ollama { model, base_url } => {
                // Create Ollama client using genai
                Ok(Box::new(OllamaClient::new(model, base_url)?))
            }
            ModelProvider::LocalCustom { model, endpoint, api_key } => {
                // Create custom local client using genai with custom endpoint
                Ok(Box::new(CustomLocalClient::new(model, endpoint, api_key)?))
            }
        }
    }
}
```

## Error Handling

### Provider-Specific Errors
```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    // ... existing variants ...

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("Provider request failed: {0}")]
    ProviderRequestFailed(String),

    #[error("Provider authentication failed: {0}")]
    ProviderAuthFailed(String),

    #[error("Provider rate limit exceeded: {0}")]
    ProviderRateLimit(String),

    #[error("Provider model not found: {0}")]
    ProviderModelNotFound(String),
}
```

### Error Mapping
Map provider-specific errors to `ApiError`:
- Network errors → `ProviderRequestFailed`
- Authentication errors → `ProviderAuthFailed`
- Rate limits → `ProviderRateLimit`
- Invalid model → `ProviderModelNotFound`
- Other errors → `ProviderError` with details

## Configuration

### Agent Configuration
Agents can specify provider configuration:
```rust
let agent = AgentIdentity::new(
    "agent-1".to_string(),
    AgentRole::Writer,
);

// Set provider
agent.provider = Some(ModelProvider::Ollama {
    model: "llama2".to_string(),
    base_url: Some("http://localhost:11434".to_string()),
});
```

### Environment-Based Configuration
Provider credentials can be loaded from environment variables:
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `OLLAMA_BASE_URL` (optional, defaults to http://localhost:11434)

### Configuration Validation
Validate provider configuration on agent creation:
- Check API keys are present (for cloud providers)
- Verify endpoints are reachable (for local providers)
- Validate model names (if possible)

## Usage Examples

### Example 1: Agent with OpenAI Provider
```rust
let agent = AgentIdentity::new(
    "openai-agent".to_string(),
    AgentRole::Writer,
);

agent.provider = Some(ModelProvider::OpenAI {
    model: "gpt-4".to_string(),
    api_key: std::env::var("OPENAI_API_KEY")?,
    base_url: None,
});

// Agent can now generate frames using OpenAI
```

### Example 2: Agent with Local Ollama Provider
```rust
let agent = AgentIdentity::new(
    "local-agent".to_string(),
    AgentRole::Writer,
);

agent.provider = Some(ModelProvider::Ollama {
    model: "llama2".to_string(),
    base_url: None,  // Uses default http://localhost:11434
});

// Agent can now generate frames using local Ollama
```

### Example 3: Agent with Custom Local Server
```rust
let agent = AgentIdentity::new(
    "custom-local-agent".to_string(),
    AgentRole::Writer,
);

agent.provider = Some(ModelProvider::LocalCustom {
    model: "custom-model".to_string(),
    endpoint: "http://localhost:8080/v1".to_string(),
    api_key: None,  // May not be needed for local
});

// Agent can now generate frames using custom local server
```

### Example 4: Generating a Frame with Provider
```rust
async fn generate_analysis_frame(
    agent: &AgentIdentity,
    node_id: NodeID,
    context: &NodeContext,
) -> Result<FrameID, ApiError> {
    // Verify agent has provider configured
    let provider = agent.provider.as_ref()
        .ok_or_else(|| ApiError::ProviderNotConfigured(agent.agent_id.clone()))?;

    // Create provider client
    let client = ProviderFactory::create_client(provider)?;

    // Build prompt from context
    let messages = vec![
        ChatMessage {
            role: MessageRole::System,
            content: "You are a code analysis assistant.".to_string(),
        },
        ChatMessage {
            role: MessageRole::User,
            content: format!("Analyze this code: {}", context_to_string(context)),
        },
    ];

    // Generate completion
    let response = client.complete(
        messages,
        CompletionOptions {
            temperature: Some(0.7),
            max_tokens: Some(1000),
            ..Default::default()
        },
    ).await?;

    // Create frame with generated content
    let frame = Frame::new(
        Basis::Node(node_id),
        response.content.into_bytes(),
        "analysis".to_string(),
        agent.agent_id.clone(),
        {
            let mut metadata = HashMap::new();
            metadata.insert("provider".to_string(), client.provider_name().to_string());
            metadata.insert("model".to_string(), client.model_name().to_string());
            metadata
        },
    )?;

    // Store frame
    put_frame(node_id, frame, agent.agent_id.clone()).await
}
```

## Testing

### Test Criteria
- Multiple providers can be used simultaneously
- Provider errors handled gracefully
- Agent identity preserved regardless of provider
- Provider switching works correctly
- Local providers work with OpenAI-compatible format
- Streaming support works for supported providers
- Provider configuration validated on agent creation
- Frame attribution includes provider metadata

### Mock Provider for Testing
```rust
pub struct MockProvider {
    responses: Vec<String>,
    current: usize,
}

impl ModelProviderClient for MockProvider {
    async fn complete(&self, _messages: Vec<ChatMessage>, _options: CompletionOptions)
        -> Result<CompletionResponse, ApiError>
    {
        // Return mock response
        Ok(CompletionResponse {
            content: self.responses[self.current].clone(),
            model: "mock-model".to_string(),
            usage: TokenUsage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
            finish_reason: Some("stop".to_string()),
        })
    }

    // ... other methods ...
}
```

## Dependencies

### Required Crates
- `genai` (or similar): Unified provider abstraction
- `async-trait`: For async trait support
- `serde`: For serialization/deserialization
- `tokio`: For async runtime

### Optional Crates
- `reqwest`: For HTTP client (if genai doesn't include)
- `futures`: For streaming support

## Future Enhancements

### Potential Additions
- **Provider pooling**: Reuse connections for performance
- **Response caching**: Cache provider responses (with invalidation)
- **Retry logic**: Automatic retries for transient failures
- **Rate limiting**: Built-in rate limiting per provider
- **Cost tracking**: Track token usage and costs per provider
- **Provider health checks**: Monitor provider availability

---

[← Back to Phase 2 Spec](phase2_spec.md) | [Component Specifications](phase2_components.md)
