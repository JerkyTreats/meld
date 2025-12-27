# Frame Generation Queue Specification

## Overview

This specification defines the batch queue system for automatically generating context frames using LLM providers. The system enables agents to generate meaningful context for all nodes in the workspace, handling large-scale operations (hundreds to thousands of nodes) efficiently through batching, rate limiting, and concurrent processing.

## Problem Statement

### Current State

- Agents can have LLM providers configured
- `generate_frame()` method exists but is not called automatically
- `ensure_agent_frame()` only creates metadata-only frames
- No batching or queueing for large-scale generation
- Watch mode doesn't trigger LLM-based generation

### Requirements

1. **Automatic Generation**: Each agent should generate frames for all nodes automatically
2. **Scalability**: Handle hundreds/thousands of nodes × multiple agents efficiently
3. **Batching**: Process generation requests in batches to avoid overwhelming systems
4. **Rate Limiting**: Respect provider rate limits per agent
5. **Non-Blocking**: Don't block watch mode or initial scan
6. **Error Handling**: Gracefully handle failures without stopping the system
7. **Priority**: Prioritize new/changed files over existing ones

## Goals

### Primary Goals

1. **Automatic Frame Generation**: Agents with providers automatically generate context frames
2. **Batch Processing**: Efficiently process large numbers of generation requests
3. **Concurrent Processing**: Process multiple requests concurrently while respecting limits
4. **Rate Limiting**: Prevent overwhelming LLM providers
5. **Non-Blocking Operations**: Background processing that doesn't block main workflows

### Secondary Goals

1. **Progress Tracking**: Log progress and completion status
2. **Retry Logic**: Automatic retries for transient failures
3. **Priority Queue**: Prioritize important requests (new files, user requests)
4. **Resource Management**: Control memory and CPU usage
5. **Observability**: Metrics and logging for monitoring

## Architecture

### Component Overview

```text
┌─────────────────────────────────────────────────────────────┐
│              Frame Generation Queue System                   │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────┐  │
│  │ Request      │─────▶│ Queue        │─────▶│ Batch    │  │
│  │ Enqueuer     │      │ Manager      │      │ Processor │  │
│  └──────────────┘      └──────────────┘      └──────────┘  │
│                                                               │
│                              ▼                               │
│                    ┌──────────────────┐                      │
│                    │  Worker Pool     │                      │
│                    │  (Per Agent)     │                      │
│                    └──────────────────┘                      │
│                              ▼                               │
│                    ┌──────────────────┐                      │
│                    │  Agent Adapter    │                      │
│                    │  (generate_frame)│                      │
│                    └──────────────────┘                      │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. GenerationRequest

Represents a single frame generation request.

```rust
pub struct GenerationRequest {
    /// NodeID to generate frame for
    pub node_id: NodeID,
    /// Agent ID that will generate the frame
    pub agent_id: String,
    /// Frame type to generate
    pub frame_type: String,
    /// Priority level (higher = more important)
    pub priority: Priority,
    /// Number of retry attempts made
    pub retry_count: usize,
    /// Timestamp when request was created
    pub created_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,      // Existing files during initial scan
    Normal = 1,   // Default priority
    High = 2,     // New files in watch mode
    Urgent = 3,   // User-initiated requests
}
```

#### 2. FrameGenerationQueue

Thread-safe queue for managing generation requests.

```rust
pub struct FrameGenerationQueue {
    /// Pending requests (priority-sorted)
    queue: Arc<Mutex<Vec<GenerationRequest>>>,
    /// Active worker tasks
    workers: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Configuration
    config: GenerationConfig,
    /// API for frame operations
    api: Arc<ContextApi>,
    /// Adapter for LLM generation
    adapter: Arc<ContextApiAdapter>,
    /// Rate limiters per agent
    rate_limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}
```

#### 3. GenerationConfig

Configuration for the generation queue.

```rust
pub struct GenerationConfig {
    /// Maximum concurrent generations per agent
    pub max_concurrent_per_agent: usize,
    /// Batch size for processing requests
    pub batch_size: usize,
    /// Maximum retry attempts per request
    pub max_retry_attempts: usize,
    /// Delay between retries (milliseconds)
    pub retry_delay_ms: u64,
    /// Rate limit: minimum delay between requests per agent (milliseconds)
    pub rate_limit_ms: Option<u64>,
    /// Maximum queue size (prevents memory exhaustion)
    pub max_queue_size: usize,
    /// Number of worker tasks per agent
    pub workers_per_agent: usize,
}
```

#### 4. BatchProcessor

Processes batches of generation requests.

```rust
pub struct BatchProcessor {
    queue: Arc<FrameGenerationQueue>,
    config: GenerationConfig,
}

impl BatchProcessor {
    /// Process a batch of requests grouped by agent
    async fn process_batch(&self, batch: Vec<GenerationRequest>) -> Result<BatchResult, ApiError>;

    /// Group requests by agent for efficient processing
    fn group_by_agent(&self, requests: Vec<GenerationRequest>) -> HashMap<String, Vec<GenerationRequest>>;
}
```

## API Design

### Queue Operations

```rust
impl FrameGenerationQueue {
    /// Create a new generation queue
    pub fn new(
        api: Arc<ContextApi>,
        adapter: Arc<ContextApiAdapter>,
        config: GenerationConfig,
    ) -> Self;

    /// Enqueue a generation request
    pub fn enqueue(
        &self,
        node_id: NodeID,
        agent_id: String,
        frame_type: Option<String>,
        priority: Priority,
    ) -> Result<(), ApiError>;

    /// Enqueue multiple requests (batch enqueue)
    pub fn enqueue_batch(
        &self,
        requests: Vec<(NodeID, String, Option<String>, Priority)>,
    ) -> Result<(), ApiError>;

    /// Start background workers
    pub fn start(&self) -> Result<(), ApiError>;

    /// Stop background workers (graceful shutdown)
    pub fn stop(&self) -> Result<(), ApiError>;

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats;

    /// Wait for queue to drain (all requests processed)
    pub async fn wait_for_completion(&self, timeout: Option<Duration>) -> Result<(), ApiError>;
}
```

### Integration with ensure_agent_frame

```rust
impl ContextApi {
    /// Ensure agent frame exists, queueing generation if agent has provider
    pub fn ensure_agent_frame(
        &self,
        node_id: NodeID,
        agent_id: String,
        frame_type: Option<String>,
        generation_queue: Option<Arc<FrameGenerationQueue>>,
    ) -> Result<Option<FrameID>, ApiError> {
        // Check if frame already exists
        if self.has_agent_frame(&node_id, &agent_id)? {
            return Ok(None);
        }

        // Get agent
        let agent = {
            let registry = self.agent_registry.read();
            registry.get_or_error(&agent_id)?.clone()
        };

        // If agent has provider and queue is available, queue generation
        if let (Some(provider), Some(queue)) = (agent.provider.as_ref(), generation_queue) {
            // Queue generation request
            let priority = Priority::Normal; // Could be configurable
            queue.enqueue(node_id, agent_id, frame_type, priority)?;
            return Ok(None); // Frame will be created asynchronously
        }

        // Fallback: create metadata frame
        // ... existing implementation ...
    }
}
```

## Workflow

### Initial Scan Workflow

```text
1. Build Merkle tree from filesystem
2. Populate NodeRecord store
3. For each node × each agent:
   a. Check if agent has provider configured
   b. Check if agent has system_prompt configured
   c. Check if agent has appropriate user_prompt template (user_prompt_file or user_prompt_directory)
   d. If all yes: Enqueue generation request (Priority::Low)
   e. If provider but missing prompts: Log error, skip (do not enqueue)
   f. If no provider: Create metadata frame immediately
4. Start generation queue workers
5. Process queue in background
6. Log progress as batches complete
7. Log any skipped requests due to missing prompts
```

### Watch Mode Workflow

```text
1. File change detected
2. Update tree for affected paths
3. For each affected node × each agent:
   a. Check if frame exists
   b. If not and agent has provider, system_prompt, and appropriate user_prompt: Enqueue (Priority::High)
   c. If not and agent has provider but missing prompts: Log error, skip
   d. If not and no provider: Create metadata frame
4. Queue processes requests in background
5. Watch mode continues without blocking
```

### Generation Request Processing

```text
1. Worker picks up request from queue
2. Check rate limiter for agent
3. Wait if rate limit exceeded
4. Validate agent has required prompts configured:
   a. Check for system_prompt in metadata
   b. Check for user_prompt_file or user_prompt_directory (based on node type)
   c. If any missing: Log error, skip request, continue
   d. If all present: Proceed to generation
5. Call adapter.generate_frame():
   a. Get node context
   b. Build prompts from agent config (see Prompt Generation)
   c. Replace placeholders in user prompt template
   d. Call LLM provider with system and user prompts
   e. Create frame with generated content
   f. Store frame via put_frame()
6. On success: Log completion
7. On failure: Retry if attempts < max_retries and error is retryable
8. On permanent failure (including missing prompts): Log error, skip, continue
```

## Prompt Generation

### Agent-Configured Prompts

All prompts must be configured in the agent's metadata. No hardcoded prompts are used.

#### Required Metadata Keys

For agents with providers configured, the following metadata keys are required:

- `system_prompt`: Defines the agent's role and behavior (required)
- `user_prompt_file`: Template for generating user prompts for file nodes (required)
- `user_prompt_directory`: Template for generating user prompts for directory nodes (required)

#### Prompt Template Format

User prompt templates support placeholders that are replaced at generation time:

- `{path}`: The file or directory path
- `{node_type}`: "File" or "Directory"
- `{file_size}`: File size in bytes (for file nodes only)

#### File Nodes

```rust
fn generate_file_prompt(
    node_record: &NodeRecord,
    agent: &AgentIdentity,
) -> Result<(String, String), ApiError> {
    // Require system prompt from agent config
    let system_prompt = agent.metadata
        .get("system_prompt")
        .ok_or_else(|| ApiError::ConfigurationError(format!(
            "Agent '{}' has provider configured but no system_prompt in metadata. \
             System prompts are required for LLM-based frame generation.",
            agent.agent_id
        )))?;

    // Require user prompt template from agent config
    let user_prompt_template = agent.metadata
        .get("user_prompt_file")
        .ok_or_else(|| ApiError::ConfigurationError(format!(
            "Agent '{}' has provider configured but no user_prompt_file in metadata. \
             User prompt templates are required for LLM-based frame generation.",
            agent.agent_id
        )))?;

    // Replace placeholders in template
    let user_prompt = user_prompt_template
        .replace("{path}", &node_record.path.display().to_string())
        .replace("{node_type}", "File");

    // For file nodes, add file size if available
    if let crate::store::NodeType::File { size, .. } = &node_record.node_type {
        let user_prompt = user_prompt.replace("{file_size}", &size.to_string());
        Ok((system_prompt.clone(), user_prompt))
    } else {
        Ok((system_prompt.clone(), user_prompt))
    }
}
```

#### Directory Nodes

```rust
fn generate_directory_prompt(
    node_record: &NodeRecord,
    agent: &AgentIdentity,
) -> Result<(String, String), ApiError> {
    // Require system prompt from agent config
    let system_prompt = agent.metadata
        .get("system_prompt")
        .ok_or_else(|| ApiError::ConfigurationError(format!(
            "Agent '{}' has provider configured but no system_prompt in metadata. \
             System prompts are required for LLM-based frame generation.",
            agent.agent_id
        )))?;

    // Require user prompt template from agent config
    let user_prompt_template = agent.metadata
        .get("user_prompt_directory")
        .ok_or_else(|| ApiError::ConfigurationError(format!(
            "Agent '{}' has provider configured but no user_prompt_directory in metadata. \
             User prompt templates are required for LLM-based frame generation.",
            agent.agent_id
        )))?;

    // Replace placeholders in template
    let user_prompt = user_prompt_template
        .replace("{path}", &node_record.path.display().to_string())
        .replace("{node_type}", "Directory");

    Ok((system_prompt.clone(), user_prompt))
}
```

### Context Building

When generating prompts, the system:

1. Retrieves prompts from agent metadata (system_prompt, user_prompt_file/user_prompt_directory)
2. Replaces placeholders in user prompt templates with actual values:
   - `{path}` → node path
   - `{node_type}` → "File" or "Directory"
   - `{file_size}` → file size in bytes (file nodes only)
3. Optionally includes additional context in the user prompt:
   - File content (for file nodes, if size < threshold and configured)
   - Existing context frames (if any and configured)
   - Parent directory context (if available and configured)

**Note**: Additional context inclusion is optional and should be configured per agent via metadata flags if desired.

## Configuration

### WatchConfig Extensions

```rust
pub struct WatchConfig {
    // ... existing fields ...

    /// Enable automatic LLM-based frame generation
    pub auto_generate_frames: bool,

    /// Generation queue configuration
    pub generation_config: GenerationConfig,
}
```

### GenerationConfig Defaults

```rust
impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_per_agent: 3,
            batch_size: 50,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            rate_limit_ms: Some(100), // 100ms between requests per agent
            max_queue_size: 10000,
            workers_per_agent: 2,
        }
    }
}
```

### Configuration File Example

```toml
[watch]
auto_generate_frames = true

[watch.generation]
max_concurrent_per_agent = 3
batch_size = 50
max_retry_attempts = 3
retry_delay_ms = 1000
rate_limit_ms = 100
max_queue_size = 10000
workers_per_agent = 2
```

## Error Handling

### Error Types

1. **Transient Errors**: Network failures, rate limits, timeouts
   - Retry with exponential backoff
   - Max retry attempts configured

2. **Permanent Errors**: Invalid node, agent not found, provider error, missing prompts
   - Log error
   - Skip request
   - Continue processing

3. **Missing Prompts**: Agent has provider but missing required prompts (system_prompt, user_prompt_file, or user_prompt_directory)
   - Log error (not fatal) with specific missing prompt(s)
   - Skip generation request
   - Continue processing other requests
   - Do not create fallback metadata frame (user should fix configuration)

4. **Queue Full**: Too many pending requests
   - Log warning
   - Optionally block or drop low-priority requests

### Retry Strategy

```rust
// Helper function to validate prompts
fn validate_agent_prompts(agent: &AgentIdentity, node_record: &NodeRecord) -> Vec<String> {
    let mut missing = Vec::new();

    if !agent.metadata.contains_key("system_prompt") {
        missing.push("system_prompt".to_string());
    }

    match node_record.node_type {
        crate::store::NodeType::File { .. } => {
            if !agent.metadata.contains_key("user_prompt_file") {
                missing.push("user_prompt_file".to_string());
            }
        }
        crate::store::NodeType::Directory => {
            if !agent.metadata.contains_key("user_prompt_directory") {
                missing.push("user_prompt_directory".to_string());
            }
        }
    }

    missing
}

async fn process_with_retry(
    request: &GenerationRequest,
    adapter: &ContextApiAdapter,
    agent: &AgentIdentity,
    node_record: &NodeRecord,
) -> Result<FrameID, ApiError> {
    // Validate all required prompts before attempting generation
    let missing_prompts = validate_agent_prompts(agent, node_record);
    if !missing_prompts.is_empty() {
        error!(
            agent_id = %agent.agent_id,
            node_id = %hex::encode(request.node_id),
            missing = ?missing_prompts,
            "Agent has provider configured but missing required prompts. Skipping generation."
        );
        return Err(ApiError::ConfigurationError(format!(
            "Agent '{}' missing required prompts: {}",
            agent.agent_id,
            missing_prompts.join(", ")
        )));
    }

    let mut last_error = None;

    for attempt in 0..config.max_retry_attempts {
        match adapter.generate_frame(...).await {
            Ok(frame_id) => return Ok(frame_id),
            Err(e) => {
                last_error = Some(e);
                // Don't retry configuration errors (like missing system_prompt)
                if !is_retryable_error(&e) {
                    return Err(e);
                }
                if is_transient_error(&e) && attempt < config.max_retry_attempts - 1 {
                    let delay = Duration::from_millis(
                        config.retry_delay_ms * (1 << attempt) // Exponential backoff
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| ApiError::GenerationFailed("Unknown error".to_string())))
}

fn is_retryable_error(error: &ApiError) -> bool {
    match error {
        ApiError::ConfigurationError(_) => false, // Don't retry config errors
        ApiError::ProviderNotConfigured(_) => false,
        _ => true,
    }
}
```

## Rate Limiting

### Per-Agent Rate Limiting

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::time::{Duration, Instant};

struct AgentRateLimiter {
    semaphore: Arc<Semaphore>,
    last_request: Arc<RwLock<HashMap<String, Instant>>>,
    min_delay: Option<Duration>,
}

impl AgentRateLimiter {
    async fn acquire(&self, agent_id: &str) -> Result<(), ApiError> {
        // Acquire semaphore (concurrency limit)
        let _permit = self.semaphore.acquire().await
            .map_err(|_| ApiError::RateLimitExceeded)?;

        // Check rate limit delay
        if let Some(min_delay) = self.min_delay {
            let mut last = self.last_request.write().await;
            if let Some(last_time) = last.get(agent_id) {
                let elapsed = last_time.elapsed();
                if elapsed < min_delay {
                    tokio::time::sleep(min_delay - elapsed).await;
                }
            }
            last.insert(agent_id.to_string(), Instant::now());
        }

        Ok(())
    }
}
```

## Performance Considerations

### Scalability

- **Queue Size**: Bounded to prevent memory exhaustion
- **Concurrency**: Configurable per-agent limits
- **Batching**: Process requests in batches for efficiency
- **Priority**: Process high-priority requests first

### Resource Management

- **Memory**: Bounded queue, limit concurrent operations
- **CPU**: Worker pool size limits
- **Network**: Rate limiting prevents overwhelming providers
- **Disk**: Frames are stored incrementally

### Optimization Strategies

1. **Batch Grouping**: Group by agent to minimize context switching
2. **Priority Queue**: Process urgent requests first
3. **Lazy Loading**: Only load node content when needed
4. **Caching**: Cache agent/provider clients
5. **Parallel Processing**: Multiple workers per agent

## Observability

### Logging

```rust
// Queue operations
info!(
    queue_size = queue.len(),
    pending = stats.pending,
    processing = stats.processing,
    completed = stats.completed,
    failed = stats.failed,
    "Generation queue status"
);

// Individual requests
debug!(
    node_id = %hex::encode(request.node_id),
    agent_id = %request.agent_id,
    attempt = request.retry_count + 1,
    "Processing generation request"
);

// Missing prompts (error, but non-fatal)
error!(
    agent_id = %agent_id,
    node_id = %hex::encode(node_id),
    missing_prompts = ?missing_prompts,
    "Agent has provider configured but missing required prompts in metadata. Skipping generation. \
     Please configure system_prompt and user_prompt_file/user_prompt_directory for this agent."
);

// Completion
info!(
    node_id = %hex::encode(node_id),
    agent_id = %agent_id,
    frame_id = %hex::encode(frame_id),
    duration_ms = duration.as_millis(),
    "Frame generation completed"
);
```

### Metrics (Future)

- Queue size over time
- Processing rate (requests/second)
- Success/failure rates
- Average processing time
- Rate limit hits
- Retry counts

## Implementation Plan

### Phase 1: Core Queue Infrastructure

1. Create `FrameGenerationQueue` struct
2. Implement basic queue operations (enqueue, dequeue)
3. Add priority queue support
4. Add thread-safe operations

### Phase 2: Worker Pool

1. Implement worker tasks
2. Add per-agent concurrency limits
3. Add rate limiting
4. Add retry logic

### Phase 3: Integration

1. Modify `ensure_agent_frame()` to use queue
2. Integrate with watch mode
3. Integrate with initial scan
4. Add configuration support

### Phase 4: Error Handling & Observability

1. Add comprehensive error handling
2. Add logging
3. Add queue statistics
4. Add graceful shutdown

### Phase 5: Optimization

1. Add batching optimizations
2. Add priority queue improvements
3. Add caching
4. Performance tuning

## Testing Strategy

### Unit Tests

1. Queue operations (enqueue, dequeue, priority)
2. Rate limiting logic
3. Retry logic
4. Error handling

### Integration Tests

1. End-to-end generation workflow
2. Multiple agents processing simultaneously
3. Rate limiting behavior
4. Queue overflow handling

### Performance Tests

1. Large-scale generation (1000+ nodes)
2. Multiple agents (5+ agents)
3. Rate limit compliance
4. Memory usage under load

## Security Considerations

### Provider API Keys

- Store securely (environment variables, config files with restricted permissions)
- Never log API keys
- Rotate keys regularly

### Provider Rate Limiting

- Respect provider rate limits
- Implement backoff strategies
- Monitor for abuse

### Resource Limits

- Bound queue size to prevent DoS
- Limit concurrent operations
- Timeout long-running requests

## Future Enhancements

### Phase 2: Advanced Features

1. **Adaptive Rate Limiting**: Adjust based on provider responses
2. **Smart Batching**: Group related nodes for better context
3. **Incremental Generation**: Only generate for changed nodes
4. **User Prompts**: Allow custom prompts per node/agent

### Phase 3: Optimization

1. **Parallel Provider Calls**: Use multiple providers simultaneously
2. **Caching**: Cache similar node contexts
3. **Streaming**: Stream generation results
4. **Compression**: Compress prompts for large files

## References

- [Watch Mode Specification](watch_mode_spec.md) - Context for watch mode integration
- [Phase 2 Architecture](phase2_architecture.md) - Overall system architecture
- [Agent Adapter](src/tooling/adapter.rs) - LLM generation interface
- [Agent Configuration](phase2_configuration.md) - Agent setup and configuration
