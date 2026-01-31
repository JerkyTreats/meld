# Provider Management Requirements

## Overview

This document specifies requirements for managing LLM provider configurations independently from agents. Providers handle API endpoints, authentication, model selection, and default completion parameters.

## Goals

1. **Independent Management**: Providers managed separately from agents
2. **XDG-Based Storage**: Providers stored in XDG config directory
3. **Runtime Binding**: Providers selected at runtime, not configuration time
4. **Reusability**: Same provider can be used by multiple agents
5. **Flexibility**: Easy to add, update, and switch providers

## Requirements

### R1: XDG-Based Provider Storage

**Requirement**: Provider configurations must be stored in XDG config directory.

**Specification**:
- **Location**: `$XDG_CONFIG_HOME/merkle/providers/` (defaults to `~/.config/merkle/providers/`)
- **Format**: One provider per file: `$XDG_CONFIG_HOME/merkle/providers/<provider_name>.toml`
- **Structure**: TOML files containing provider type, model, credentials, and options

**Rationale**:
- Follows XDG Base Directory Specification
- User-level configuration (not workspace-specific)
- Providers available across all workspaces
- Standard location for user configuration

**Directory Structure**:
```
$XDG_CONFIG_HOME/merkle/
├── config.toml              # System configuration (existing)
├── agents/                   # Agent configurations
│   ├── code-analyzer.toml
│   └── docs-writer.toml
└── providers/                # Provider configurations
    ├── openai-gpt4.toml
    ├── anthropic-claude.toml
    └── local-ollama.toml
```

### R2: Provider Configuration Format

**Requirement**: Provider configuration files must define all provider-specific settings.

**Specification**:
- **File Format**: TOML
- **Location**: `$XDG_CONFIG_HOME/merkle/providers/<provider_name>.toml`
- **Required Fields**:
  - `provider_name`: Unique identifier (must match filename)
  - `provider_type`: Provider type enum (openai, anthropic, ollama, local)
  - `model`: Model identifier
- **Optional Fields**:
  - `api_key`: API key (can be from environment)
  - `endpoint`: Custom endpoint URL
  - `default_options`: Default completion options

**Example Provider Config** (`~/.config/merkle/providers/openai-gpt4.toml`):
```toml
provider_name = "openai-gpt4"
provider_type = "openai"
model = "gpt-4"

# API key can be in file or environment variable
api_key = null  # Uses OPENAI_API_KEY env var

# Optional custom endpoint (e.g., Azure OpenAI)
endpoint = null  # Default: https://api.openai.com/v1

# Default completion options
[default_options]
temperature = 0.7
max_tokens = 2000
top_p = 0.9
frequency_penalty = null
presence_penalty = null
stop = null
```

### R3: Provider Type Support

**Requirement**: System must support all provider types with appropriate configuration.

**Specification**:

#### OpenAI Provider
```toml
provider_name = "openai-gpt4"
provider_type = "openai"
model = "gpt-4"
api_key = null  # From OPENAI_API_KEY env var
endpoint = null  # Optional custom endpoint
```

#### Anthropic Provider
```toml
provider_name = "anthropic-claude"
provider_type = "anthropic"
model = "claude-3-opus-20240229"
api_key = null  # From ANTHROPIC_API_KEY env var
```

#### Ollama Provider (Local)
```toml
provider_name = "local-ollama"
provider_type = "ollama"
model = "llama2"
endpoint = "http://localhost:11434"  # Optional, defaults to localhost:11434
```

#### Custom Local Provider
```toml
provider_name = "local-custom"
provider_type = "local"
model = "custom-model"
endpoint = "http://localhost:8080/v1"  # Required
api_key = null  # Optional
```

### R4: API Key Resolution

**Requirement**: System must resolve API keys from config or environment variables.

**Specification**:
- **Priority Order**:
  1. API key in provider config file
  2. Environment variable (provider-specific)
  3. Error if required and not found
- **Environment Variables**:
  - OpenAI: `OPENAI_API_KEY`
  - Anthropic: `ANTHROPIC_API_KEY`
  - Ollama: No API key required
  - Local: Optional, provider-specific

**Error Handling**:
- Clear error if API key required but not found
- Suggest setting environment variable
- Provide provider-specific guidance

### R5: Provider Registry

**Requirement**: Provider registry must manage provider configurations independently.

**Specification**:
- **Location**: Separate from agent registry
- **Operations**:
  - Load providers from XDG directory
  - Validate provider configurations
  - Create provider clients on demand
  - Cache provider configurations
- **API**:
  - `load_from_xdg()` - Load all providers
  - `get(provider_name)` - Get provider config
  - `list_all()` - List all providers
  - `create_client(provider_name)` - Create provider client

**Provider Registry Structure**:
```rust
pub struct ProviderRegistry {
    providers: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    pub fn load_from_xdg(&mut self) -> Result<(), ApiError>;
    pub fn get(&self, provider_name: &str) -> Option<&ProviderConfig>;
    pub fn list_all(&self) -> Vec<&ProviderConfig>;
    pub fn create_client(&self, provider_name: &str) -> Result<Box<dyn ModelProviderClient>, ApiError>;
}
```

### R6: Provider Validation

**Requirement**: Provider configurations must be validated on load.

**Specification**:
- **Required Field Validation**:
  - `provider_name` must match filename
  - `provider_type` must be valid enum value
  - `model` must not be empty
- **Type-Specific Validation**:
  - OpenAI/Anthropic: API key must be available (config or env)
  - Ollama: Endpoint must be valid URL (if provided)
  - Local: Endpoint must be valid URL (required)
- **Options Validation**:
  - Temperature: 0.0-2.0
  - Max tokens: Positive integer
  - Other options: Type-appropriate ranges

**Validation Errors**:
- Clear error messages with file locations
- Line numbers for TOML parsing errors
- Suggestions for fixing issues

### R7: Provider Discovery and Management

**Requirement**: CLI commands for provider management.

**Specification**:
- **List Providers**: `merkle provider list` - Show all available providers
- **Show Provider**: `merkle provider show <provider_name>` - Display provider details
- **Validate Provider**: `merkle provider validate <provider_name>` - Check configuration
- **Test Provider**: `merkle provider test <provider_name>` - Test connectivity
- **Create Provider**: `merkle provider create <provider_name>` - Interactive creation
- **Edit Provider**: `merkle provider edit <provider_name>` - Edit configuration

**Output Formats**:
- Text format (default, human-readable)
- JSON format (`--format json`) for scripting

### R8: Default Completion Options

**Requirement**: Providers must define default completion options.

**Specification**:
- **Default Options**: Each provider has default completion options
- **Override Priority**:
  1. Command-line flags (highest)
  2. Agent preferences (if specified)
  3. Provider defaults (lowest)
- **Options**:
  - `temperature`: 0.0-2.0
  - `max_tokens`: Positive integer
  - `top_p`: 0.0-1.0
  - `frequency_penalty`: -2.0 to 2.0
  - `presence_penalty`: -2.0 to 2.0
  - `stop`: Array of strings

**Example**:
```toml
[default_options]
temperature = 0.7
max_tokens = 2000
top_p = 0.9
```

### R9: Provider Client Creation

**Requirement**: Provider registry must create provider clients on demand.

**Specification**:
- **Lazy Creation**: Clients created when needed, not on load
- **Caching**: Provider configs cached, clients created per request
- **Error Handling**: Clear errors if provider not found or invalid
- **Type Conversion**: Convert `ProviderConfig` → `ModelProvider` enum → client

**Flow**:
1. Look up provider in registry
2. Validate provider config
3. Resolve API key (config or env)
4. Convert to `ModelProvider` enum
5. Create provider-specific client
6. Return client trait object

## Non-Requirements

### Out of Scope

1. **Provider Pooling**: Connection pooling (future enhancement)
2. **Provider Health Checks**: Availability monitoring (future)
3. **Cost Tracking**: Token usage and cost tracking (future)
4. **Rate Limiting**: Built-in rate limiting (handled by queue system)
5. **Provider Sharing**: Distribution mechanism (user-managed)

## Implementation Considerations

### File System Structure

```
$XDG_CONFIG_HOME/merkle/providers/
├── openai-gpt4.toml
├── openai-gpt35.toml
├── anthropic-claude.toml
├── local-ollama.toml
└── local-custom.toml
```

### Provider Config Schema

```toml
# Required
provider_name = "string"                    # Must match filename
provider_type = "openai" | "anthropic" | "ollama" | "local"
model = "string"                            # Model identifier

# Optional
api_key = "string" | null                   # Or from env var
endpoint = "string" | null                  # Custom endpoint

# Default completion options
[default_options]
temperature = 0.7
max_tokens = 2000
top_p = 0.9
frequency_penalty = null
presence_penalty = null
stop = null
```

### Error Scenarios

1. **Missing Provider**: Error with suggestion to list available providers
2. **Invalid Provider Type**: Error with valid options
3. **Missing API Key**: Error with environment variable guidance
4. **Invalid Endpoint**: Error with URL format requirements
5. **Invalid Options**: Error with valid ranges

## Success Criteria

1. ✅ Providers stored in XDG directory
2. ✅ Providers independent from agents
3. ✅ Providers can be reused by multiple agents
4. ✅ CLI commands for provider management
5. ✅ Clear error messages for configuration issues

## Related Documentation

- [Provider-Agent Separation](provider_agent_separation.md) - Separation design
- [Agent Management Requirements](../agents/agent_management_requirements.md) - Agent configuration
- [Phase 2 Model Providers](../workflow/phase2_model_providers.md) - Provider implementation

---

[← Back to Agent Management](../agents/README.md)

