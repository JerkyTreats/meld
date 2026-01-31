# Provider-Agent Separation Specification

## Overview

This document specifies the separation of concerns between **Providers** (LLM API configuration) and **Agents** (prompt engineering and output expectations). This separation enables agents to be provider-agnostic and allows providers to be managed independently.

## Design Principles

### 1. Separation of Concerns

**Providers** are responsible for:
- API endpoint configuration
- Authentication and credentials
- Model selection
- Network settings
- Default completion parameters

**Agents** are responsible for:
- Prompt structure and format
- System and user prompt templates
- Output expectations and parsing
- Role and behavior definition
- Response processing logic

### 2. Runtime Binding

- Agents and providers are **bound at runtime**, not at configuration time
- Same agent can use different providers
- Provider selection happens when generating frames, not when defining agents
- Enables provider switching without agent reconfiguration

### 3. Provider-Agnostic Agents

- Agents define **how** to use LLMs (prompts, output format)
- Agents do **not** define **where** to call (API endpoints, models)
- Agent configuration is independent of provider configuration
- Agents can work with any compatible provider

## Current State vs. Desired State

### Current State (Coupled)

**Agent Configuration**:
```toml
[agents.code-analyzer]
agent_id = "code-analyzer"
role = "Writer"
system_prompt = "..."
provider_name = "openai-gpt4"  # ← Coupling point
[agents.code-analyzer.completion_options]  # ← Provider concern
temperature = 0.5
max_tokens = 2000
```

**Problems**:
- Agent must know provider name
- Completion options mixed with agent config
- Provider changes require agent reconfiguration
- Cannot reuse agent with different providers

### Desired State (Separated)

**Agent Configuration** (Provider-Agnostic):
```toml
# ~/.config/merkle/agents/code-analyzer.toml
agent_id = "code-analyzer"
role = "Writer"
system_prompt_path = "~/prompts/code-analysis.md"
# No provider_name!
# No completion_options!
```

**Provider Configuration** (Separate):
```toml
# ~/.config/merkle/providers/openai-gpt4.toml
provider_name = "openai-gpt4"
provider_type = "openai"
model = "gpt-4"
api_key = "sk-..."  # or from env
endpoint = null
[default_options]
temperature = 0.7
max_tokens = 2000
```

**Runtime Binding**:
```bash
# Provider specified at invocation time
merkle context generate --path src/lib.rs \
  --agent code-analyzer \
  --provider openai-gpt4
```

## Provider Management

### Provider Configuration Structure

**Location**: `$XDG_CONFIG_HOME/merkle/providers/` (defaults to `~/.config/merkle/providers/`)

**Format**: One provider per file: `$XDG_CONFIG_HOME/merkle/providers/<provider_name>.toml`

**Schema**:
```toml
# Provider name (must match filename)
provider_name = "openai-gpt4"

# Provider type
provider_type = "openai"  # openai | anthropic | ollama | local

# Model identifier
model = "gpt-4"

# API key (optional, can be from environment)
api_key = "sk-..."  # or null to use env var

# Endpoint (optional, provider-specific)
endpoint = null  # or custom URL

# Default completion options
[default_options]
temperature = 0.7
max_tokens = 2000
top_p = 0.9
frequency_penalty = null
presence_penalty = null
stop = null
```

### Provider Types

#### OpenAI
```toml
provider_name = "openai-gpt4"
provider_type = "openai"
model = "gpt-4"
api_key = null  # Uses OPENAI_API_KEY env var
endpoint = null  # Default: https://api.openai.com/v1
```

#### Anthropic
```toml
provider_name = "anthropic-claude"
provider_type = "anthropic"
model = "claude-3-opus-20240229"
api_key = null  # Uses ANTHROPIC_API_KEY env var
```

#### Ollama (Local)
```toml
provider_name = "local-ollama"
provider_type = "ollama"
model = "llama2"
endpoint = "http://localhost:11434"  # Optional, defaults to localhost:11434
```

#### Custom Local
```toml
provider_name = "local-custom"
provider_type = "local"
model = "custom-model"
endpoint = "http://localhost:8080/v1"
api_key = null  # Optional
```

### Provider Registry

**Purpose**: Manage provider configurations independently from agents

**Location**: Separate from agent registry
- Provider registry: `$XDG_CONFIG_HOME/merkle/providers/`
- Agent registry: `$XDG_CONFIG_HOME/merkle/agents/`

**Operations**:
- Load providers from XDG directory
- Validate provider configurations
- Create provider clients on demand
- Cache provider configurations

## Agent Management (Updated)

### Agent Configuration Structure

**Location**: `$XDG_CONFIG_HOME/merkle/agents/` (as specified in agent_management_requirements.md)

**Format**: One agent per file: `$XDG_CONFIG_HOME/merkle/agents/<agent_id>.toml`

**Schema** (Provider-Agnostic):
```toml
# Agent ID (must match filename)
agent_id = "code-analyzer"

# Agent role
role = "Writer"  # Reader | Writer | Synthesis

# System prompt file (markdown)
system_prompt_path = "~/prompts/code-analysis.md"

# User prompt templates (in metadata)
[metadata]
user_prompt_file = "Analyze the code at {path}..."
user_prompt_directory = "Analyze the directory at {path}..."

# Agent-specific metadata (no provider references!)
[metadata]
specialization = "rust"
focus = "performance"
```

**Removed Fields**:
- ❌ `provider_name` - No longer in agent config
- ❌ `completion_options` - Provider concern, not agent concern

## Runtime Provider Selection

### Command-Line Interface

**Context Generation**:
```bash
# Provider specified at invocation
merkle context generate --path src/lib.rs \
  --agent code-analyzer \
  --provider openai-gpt4

# Different provider, same agent
merkle context generate --path src/lib.rs \
  --agent code-analyzer \
  --provider local-ollama
```

**Provider Selection Priority**:
1. `--provider` flag (explicit)
2. Default provider from environment/config
3. Error if no provider specified and no default

### API Interface

**Frame Generation**:
```rust
// Provider specified at call time
api.generate_frame(
    node_id,
    agent_id: "code-analyzer",
    provider_name: "openai-gpt4",  // ← Runtime binding
    frame_type: "analysis",
)?;
```

**Provider Resolution**:
1. Look up provider in provider registry
2. Create provider client
3. Use agent's prompts with provider's API
4. Combine agent completion options with provider defaults

## Completion Options Resolution

### Priority Order

When generating frames, completion options are resolved as:

1. **Provider Defaults** (from provider config)
2. **Agent Overrides** (if agent specifies completion preferences)
3. **Command-Line Overrides** (if specified via flags)

**Note**: Agents may specify completion preferences in metadata (e.g., `preferred_temperature = "0.5"`), but these are suggestions, not requirements. The actual provider's defaults take precedence unless overridden.

### Agent Completion Preferences

Agents can express preferences in metadata (optional):
```toml
[metadata]
preferred_temperature = "0.5"
preferred_max_tokens = "2000"
```

These are **preferences**, not requirements:
- Used as hints when no explicit options provided
- Can be overridden by provider defaults
- Can be overridden by command-line flags

## Provider Registry API

### Provider Registry Structure

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    /// Load providers from XDG directory
    pub fn load_from_xdg(&mut self) -> Result<(), ApiError>;
    
    /// Get provider configuration
    pub fn get(&self, provider_name: &str) -> Option<&ProviderConfig>;
    
    /// List all providers
    pub fn list_all(&self) -> Vec<&ProviderConfig>;
    
    /// Create provider client
    pub fn create_client(&self, provider_name: &str) -> Result<Box<dyn ModelProviderClient>, ApiError>;
}
```

### Provider Configuration

```rust
pub struct ProviderConfig {
    pub provider_name: String,
    pub provider_type: ProviderType,
    pub model: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub default_options: CompletionOptions,
}
```

## Agent Registry (Updated)

### Agent Registry Structure

```rust
pub struct AgentRegistry {
    agents: HashMap<String, AgentIdentity>,
}

impl AgentRegistry {
    /// Load agents from XDG directory
    pub fn load_from_xdg(&mut self) -> Result<(), ApiError>;
    
    /// Get agent (no provider embedded)
    pub fn get(&self, agent_id: &str) -> Option<&AgentIdentity>;
    
    /// List all agents
    pub fn list_all(&self) -> Vec<&AgentIdentity>;
}
```

### Agent Identity (Updated)

```rust
pub struct AgentIdentity {
    pub agent_id: String,
    pub role: AgentRole,
    pub capabilities: Vec<Capability>,
    // ❌ REMOVED: pub provider: Option<ModelProvider>,
    pub metadata: HashMap<String, String>,
}
```

## Frame Generation Flow (Updated)

### Updated Flow

1. **Resolve Agent**: Look up agent in agent registry
2. **Resolve Provider**: Look up provider in provider registry (from runtime parameter)
3. **Load Prompts**: Load system prompt from agent's prompt file
4. **Create Provider Client**: Create client from provider config
5. **Build Messages**: Use agent's prompts to build chat messages
6. **Generate**: Call provider with messages and options
7. **Process Response**: Agent processes response (if needed)
8. **Create Frame**: Store frame with agent_id and provider metadata

### Provider Attribution

Frame metadata includes provider information:
```rust
frame.metadata.insert("provider".to_string(), provider_name);
frame.metadata.insert("model".to_string(), model_name);
frame.metadata.insert("provider_type".to_string(), provider_type);
```

**Note**: Provider info is for attribution only, not part of agent identity.

## Benefits

### 1. Flexibility

- Same agent works with multiple providers
- Easy provider switching
- Test agents with different providers

### 2. Separation of Concerns

- Agents focus on prompts and behavior
- Providers focus on API configuration
- Clear boundaries between concerns

### 3. Reusability

- Agents can be shared without provider dependencies
- Providers can be shared without agent dependencies
- Mix and match agents and providers

### 4. Maintainability

- Agent changes don't affect provider config
- Provider changes don't affect agent config
- Easier to manage and update

## Examples

### Example 1: Same Agent, Different Providers

**Agent Config** (`code-analyzer.toml`):
```toml
agent_id = "code-analyzer"
role = "Writer"
system_prompt_path = "~/prompts/code-analysis.md"
```

**Usage**:
```bash
# Use with OpenAI
merkle context generate --path src/lib.rs \
  --agent code-analyzer \
  --provider openai-gpt4

# Use with local Ollama
merkle context generate --path src/lib.rs \
  --agent code-analyzer \
  --provider local-ollama
```

### Example 2: Provider-Specific Defaults

**Provider Config** (`openai-gpt4.toml`):
```toml
provider_name = "openai-gpt4"
provider_type = "openai"
model = "gpt-4"
[default_options]
temperature = 0.7
max_tokens = 2000
```

**Agent Config** (`code-analyzer.toml`):
```toml
agent_id = "code-analyzer"
role = "Writer"
system_prompt_path = "~/prompts/code-analysis.md"
[metadata]
preferred_temperature = "0.5"  # Preference, not requirement
```

**Resolution**:
- Provider default: `temperature = 0.7`
- Agent preference: `temperature = 0.5`
- Final: `temperature = 0.7` (provider default wins unless overridden)

### Example 3: Provider-Agnostic Agent Sharing

**Agent** (`docs-writer.toml`):
```toml
agent_id = "docs-writer"
role = "Writer"
system_prompt_path = "~/prompts/documentation.md"
```

**Shared Usage**:
- User A uses with `openai-gpt4`
- User B uses with `anthropic-claude`
- User C uses with `local-ollama`
- Same agent, different providers

## Related Documentation

- [Agent Management Requirements](../agents/agent_management_requirements.md) - Agent configuration
- [Provider Management Requirements](provider_management_requirements.md) - Provider configuration
- [Context Management README](../README.md) - Context commands

---

[← Back to Agent Management](../agents/README.md)

