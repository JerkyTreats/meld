# Phase 2 Configuration Specification

## Overview

This document specifies the Configuration System component for Phase 2. It defines a runtime-driven configuration system that enables dynamic agent behavior and model provider management for existing features without requiring code changes. The configuration system follows Rust idiomatic best practices, using hierarchical configuration with environment variable overrides and runtime validation.

**Note**: This specification covers configuration for existing features only. Future enhancements (such as feature flags, retry logic, logging configuration, etc.) will be added when those features are implemented.

## Design Principles

### 1. Runtime-Driven Configuration
- **Provider and agent configuration**: Model provider settings and agent definitions driven from configuration
- **Dynamic updates**: Configuration can be reloaded at runtime (where safe to do so)
- **Environment-aware**: Different configurations for development, testing, and production
- **Validation on load**: Configuration validated at load time with clear error messages

### 2. Hierarchical Configuration
- **Layered precedence**: Default values → file config → environment variables → runtime overrides
- **Separation of concerns**: Non-sensitive config in version-controlled files, secrets in separate files or environment
- **Composition**: Configuration can be composed from multiple sources (base config + environment-specific overrides)

### 3. Type Safety and Validation
- **Strong typing**: All configuration structures are strongly typed with serde serialization
- **Schema validation**: Configuration validated against expected schema on load
- **Clear errors**: Validation errors provide actionable feedback

### 4. Agent-Centric Design
- **Per-agent configuration**: Each agent has its own configuration object
- **System prompts in config**: Agent system prompts defined in configuration, enabling runtime-driven behavior
- **Provider assignment**: Model providers assigned to agents via configuration
- **Capability configuration**: Agent capabilities and roles configurable per agent

## Configuration Structure

### Root Configuration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleConfig {
    /// Workspace root path (defaults to current directory)
    pub workspace_root: Option<PathBuf>,

    /// Model provider configurations
    pub providers: HashMap<String, ProviderConfig>,

    /// Agent definitions
    pub agents: HashMap<String, AgentConfig>,

    /// System-wide settings
    pub system: SystemConfig,
}
```

### Model Provider Configuration

Each model provider is defined with a unique name and configuration:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type (OpenAI, Anthropic, Ollama, LocalCustom)
    pub provider_type: ProviderType,

    /// Model identifier (e.g., "gpt-4", "claude-3-opus", "llama2")
    pub model: String,

    /// API key (optional, can be loaded from environment)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Base URL or endpoint (provider-specific)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Default completion options for this provider
    #[serde(default)]
    pub default_options: CompletionOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "local")]
    LocalCustom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    /// Temperature (0.0-2.0, default: 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p sampling (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Stop sequences
    #[serde(default)]
    pub stop: Vec<String>,
}

```

### Agent Configuration

Each agent has a unique configuration object that defines its behavior:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent identifier
    pub agent_id: String,

    /// Agent role (Reader, Writer, Synthesis)
    pub role: AgentRole,

    /// System prompt for this agent
    /// This is the primary behavior-defining prompt that guides agent actions when using LLM providers.
    /// The system prompt is used as the System message role when making provider API calls.
    /// If not provided, a default system prompt will be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Model provider to use (references a provider from providers map)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,

    /// Override completion options for this agent (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_options: Option<CompletionOptions>,

    /// Agent-specific metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}
```

### System Configuration

System-wide settings that affect all operations:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Default workspace root (if not specified)
    #[serde(default = "default_workspace_root")]
    pub default_workspace_root: PathBuf,

    /// Storage paths
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to node record store (relative to workspace root)
    #[serde(default = "default_store_path")]
    pub store_path: PathBuf,

    /// Path to frame storage (relative to workspace root)
    #[serde(default = "default_frames_path")]
    pub frames_path: PathBuf,
}
```

## Configuration Loading

### Configuration Sources

Configuration is loaded from multiple sources in order of precedence (highest to lowest):

1. **Runtime overrides**: Programmatic configuration updates
2. **Environment variables**: `MERKLE_*` prefixed variables
3. **Environment-specific files**: `config/production.toml`, `config/development.toml`
4. **Base configuration file**: `config/config.toml` or `.merkle/config.toml`
5. **Default values**: Hardcoded defaults in struct definitions

### Configuration File Format

Configuration files use TOML format for readability and ease of editing:

```toml
# config/config.toml

[system]
default_workspace_root = "."
[system.storage]
store_path = ".merkle/store"
frames_path = ".merkle/frames"

[providers.openai-gpt4]
provider_type = "openai"
model = "gpt-4"
# api_key loaded from OPENAI_API_KEY environment variable
endpoint = null
[providers.openai-gpt4.default_options]
temperature = 0.7
max_tokens = 2000

[providers.local-ollama]
provider_type = "ollama"
model = "llama2"
endpoint = "http://localhost:11434"
[providers.local-ollama.default_options]
temperature = 0.8
max_tokens = 1500

[agents.code-analyzer]
agent_id = "code-analyzer"
role = "Writer"
system_prompt = """
You are a code analysis assistant. Your role is to analyze code files and generate
comprehensive analysis frames that describe the code's structure, purpose, and key
characteristics. Focus on accuracy and clarity.
"""
provider_name = "openai-gpt4"

[agents.documentation-generator]
agent_id = "doc-generator"
role = "Writer"
system_prompt = """
You are a documentation generation assistant. Generate clear, concise documentation
frames that explain code functionality, APIs, and usage patterns.
"""
provider_name = "local-ollama"

[agents.synthesis-agent]
agent_id = "synthesis-agent"
role = "Synthesis"
system_prompt = """
You are a context synthesis assistant. Your role is to combine context frames from
child nodes into coherent branch-level summaries. Maintain accuracy and preserve
key information from child contexts.
"""
provider_name = "openai-gpt4"
```

### Environment Variable Overrides

Environment variables follow the pattern `MERKLE_<SECTION>_<KEY>` with nested keys using double underscores:

```bash
# Override system settings
export MERKLE_SYSTEM_STORAGE_STORE_PATH=.merkle/custom_store

# Override provider settings
export MERKLE_PROVIDERS_OPENAI_GPT4_API_KEY=sk-...
export MERKLE_PROVIDERS_OPENAI_GPT4_DEFAULT_OPTIONS_TEMPERATURE=0.9

# Override agent settings
export MERKLE_AGENTS_CODE_ANALYZER_PROVIDER_NAME=local-ollama
export MERKLE_AGENTS_CODE_ANALYZER_SYSTEM_PROMPT="You are a code analysis assistant."
```

### Configuration Loading Implementation

```rust
use config::{Config, ConfigError, Environment, File};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from files and environment
    pub fn load(workspace_root: &PathBuf) -> Result<MerkleConfig, ConfigError> {
        let config_dir = workspace_root.join("config");
        let merkle_config_dir = workspace_root.join(".merkle");

        let mut builder = Config::builder()
            // Load base config
            .add_source(File::with_name(
                config_dir.join("config.toml").to_str().unwrap_or("config/config.toml")
            ).required(false))
            // Load .merkle/config.toml if it exists
            .add_source(File::with_name(
                merkle_config_dir.join("config.toml").to_str().unwrap_or(".merkle/config.toml")
            ).required(false))
            // Load environment-specific config
            .add_source(File::with_name(
                format!("config/{}.toml", std::env::var("MERKLE_ENV").unwrap_or_else(|_| "development".to_string()))
            ).required(false))
            // Override with environment variables
            .add_source(Environment::with_prefix("MERKLE").separator("__"));

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: &PathBuf) -> Result<MerkleConfig, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name(path.to_str().unwrap()))
            .add_source(Environment::with_prefix("MERKLE").separator("__"))
            .build()?;
        config.try_deserialize()
    }

    /// Create default configuration
    pub fn default() -> MerkleConfig {
        MerkleConfig {
            workspace_root: None,
            providers: HashMap::new(),
            agents: HashMap::new(),
            system: SystemConfig::default(),
        }
    }
}
```

## Configuration Validation

### Validation Rules

Configuration is validated on load to ensure:

1. **Provider validation**:
   - Provider names are unique
   - Required fields are present (model, provider_type)
   - API keys present for cloud providers (or available in environment)
   - Endpoints are valid URLs (if specified)
   - Completion options are within valid ranges

2. **Agent validation**:
   - Agent IDs are unique
   - Provider references exist in providers map
   - Roles are valid (Reader, Writer, Synthesis)
   - System prompts are non-empty if provided

3. **System validation**:
   - Storage paths are valid

### Validation Implementation

```rust
impl MerkleConfig {
    /// Validate the entire configuration
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Validate providers
        for (name, provider) in &self.providers {
            if let Err(e) = provider.validate() {
                errors.push(ValidationError::Provider(name.clone(), e));
            }
        }

        // Validate agents
        for (name, agent) in &self.agents {
            if let Err(e) = agent.validate(&self.providers) {
                errors.push(ValidationError::Agent(name.clone(), e));
            }
        }

        // Validate system config
        if let Err(e) = self.system.validate() {
            errors.push(ValidationError::System(e));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug)]
pub enum ValidationError {
    Provider(String, String),
    Agent(String, String),
    System(String),
}
```

## Runtime Configuration Updates

### Dynamic Configuration Reloading

Some configuration can be updated at runtime without restarting:

- **Provider settings**: Provider endpoints and completion options
- **Agent provider assignments**: Agents can switch providers

**Not reloadable** (require restart):
- Storage paths
- Workspace root
- Agent definitions (new agents require restart)

### Configuration Update API

```rust
pub struct ConfigManager {
    config: Arc<RwLock<MerkleConfig>>,
}

impl ConfigManager {
    /// Reload configuration from files
    pub fn reload(&self, workspace_root: &PathBuf) -> Result<(), ConfigError> {
        let new_config = ConfigLoader::load(workspace_root)?;
        new_config.validate()?;
        *self.config.write() = new_config;
        Ok(())
    }

    /// Update a specific agent configuration
    pub fn update_agent(&self, agent_id: &str, updates: AgentConfigUpdate) -> Result<(), ConfigError> {
        let mut config = self.config.write();
        if let Some(agent) = config.agents.get_mut(agent_id) {
            updates.apply(agent)?;
            Ok(())
        } else {
            Err(ConfigError::NotFound(format!("Agent not found: {}", agent_id)))
        }
    }

    /// Get current configuration (read-only)
    pub fn get(&self) -> MerkleConfig {
        self.config.read().clone()
    }
}
```

## Integration with Existing Components

### Agent Registry Integration

Agent configurations are loaded into the AgentRegistry:

```rust
impl AgentRegistry {
    /// Load agents from configuration
    pub fn load_from_config(&mut self, config: &MerkleConfig) -> Result<(), ApiError> {
        for (_, agent_config) in &config.agents {
            let mut identity = AgentIdentity::new(
                agent_config.agent_id.clone(),
                agent_config.role,
            );

            // Set provider if configured
            if let Some(provider_name) = &agent_config.provider_name {
                let provider_config = config.providers.get(provider_name)
                    .ok_or_else(|| ApiError::ProviderNotConfigured(provider_name.clone()))?;

                identity.provider = Some(provider_config.to_model_provider()?);
            }

            // Store system prompt in metadata if provided
            if let Some(system_prompt) = &agent_config.system_prompt {
                identity.metadata.insert("system_prompt".to_string(), system_prompt.clone());
            }

            // Copy metadata from config
            for (key, value) in &agent_config.metadata {
                identity.metadata.insert(key.clone(), value.clone());
            }

            self.register(identity);
        }
        Ok(())
    }
}
```

### Provider Factory Integration

Provider configurations are used to create provider clients:

```rust
impl ProviderFactory {
    /// Create a provider client from configuration
    pub fn create_from_config(
        config: &ProviderConfig,
    ) -> Result<Box<dyn ModelProviderClient>, ApiError> {
        // Load API key from config or environment
        let api_key = config.api_key.clone()
            .or_else(|| std::env::var(format!("{}_API_KEY", config.provider_type.to_env_var())).ok());

        match config.provider_type {
            ProviderType::OpenAI => {
                let api_key = api_key.ok_or_else(|| {
                    ApiError::ProviderNotConfigured("OpenAI API key required".to_string())
                })?;
                Ok(Box::new(OpenAIClient::new(
                    config.model.clone(),
                    api_key,
                    config.endpoint.clone(),
                )?))
            }
            // ... other provider types
        }
    }
}
```

### System Prompt Usage

When an agent with a configured system prompt makes LLM provider calls, the system prompt is automatically included as the System message role:

```rust
// Example: Using agent's system prompt when generating frames
async fn generate_frame_with_agent_prompt(
    agent: &AgentIdentity,
    user_prompt: &str,
    context: &NodeContext,
) -> Result<CompletionResponse, ApiError> {
    let provider = agent.provider.as_ref()
        .ok_or_else(|| ApiError::ProviderNotConfigured(agent.agent_id.clone()))?;

    let client = ProviderFactory::create_from_config(provider)?;

    let mut messages = Vec::new();

    // Use system prompt from agent config if available
    let system_prompt = agent.metadata.get("system_prompt")
        .cloned()
        .unwrap_or_else(|| "You are a helpful assistant.".to_string());

    messages.push(ChatMessage {
        role: MessageRole::System,
        content: system_prompt,
    });

    // Add user prompt with context
    messages.push(ChatMessage {
        role: MessageRole::User,
        content: format!("Context:\n{}\n\nTask: {}", context_to_string(context), user_prompt),
    });

    client.complete(messages, CompletionOptions::default()).await
}
```

**System Prompt Behavior**:
- If `system_prompt` is provided in agent config, it is used for all LLM calls made by that agent
- If `system_prompt` is not provided, a default system prompt is used
- System prompts are stored in agent metadata when loaded from configuration
- System prompts can be overridden at runtime by updating agent metadata

## Configuration Examples

### Example 1: Multi-Provider Setup

```toml
[providers.openai-primary]
provider_type = "openai"
model = "gpt-4"
[providers.openai-primary.default_options]
temperature = 0.7

[providers.anthropic-backup]
provider_type = "anthropic"
model = "claude-3-opus"
[providers.anthropic-backup.default_options]
temperature = 0.8

[providers.local-dev]
provider_type = "ollama"
model = "llama2"
endpoint = "http://localhost:11434"
```

### Example 2: Agent with Custom System Prompt

```toml
[agents.rust-specialist]
agent_id = "rust-specialist"
role = "Writer"
system_prompt = """
You are a Rust programming expert. Your task is to analyze Rust code files
and generate detailed analysis frames. Focus on:
- Memory safety and ownership patterns
- Performance characteristics
- Error handling strategies
- Idiomatic Rust patterns
- Potential improvements or issues
"""
provider_name = "openai-primary"
[agents.rust-specialist.completion_options]
temperature = 0.5
max_tokens = 2000
```

### Example 3: Environment-Specific Configuration

```toml
# config/development.toml
[providers.local-dev]
provider_type = "ollama"
model = "llama2"

[agents.code-analyzer]
provider_name = "local-dev"  # Use local provider in dev
```

```toml
# config/production.toml
[providers.openai-primary]
provider_type = "openai"
model = "gpt-4"

[agents.code-analyzer]
provider_name = "openai-primary"  # Use cloud provider in prod
```

## Testing

### Test Criteria
- Configuration loads correctly from TOML files
- Environment variable overrides work correctly
- Configuration validation catches invalid values
- Agent configurations are correctly loaded into registry
- Provider configurations create valid provider clients
- Runtime configuration updates work for reloadable settings
- Default values are applied when fields are missing
- Configuration errors provide clear, actionable messages

### Test Configuration

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_minimal_config() {
        let config = ConfigLoader::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_provider_config() {
        let mut config = ConfigLoader::default();
        config.providers.insert("test".to_string(), ProviderConfig {
            provider_type: ProviderType::OpenAI,
            model: "gpt-4".to_string(),
            api_key: Some("test-key".to_string()),
            ..Default::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_agent_with_missing_provider() {
        let mut config = ConfigLoader::default();
        config.agents.insert("test-agent".to_string(), AgentConfig {
            agent_id: "test-agent".to_string(),
            role: AgentRole::Writer,
            provider_name: Some("nonexistent".to_string()),
            ..Default::default()
        });
        assert!(config.validate().is_err());
    }
}
```

## Dependencies

### Required Crates
- `config`: Hierarchical configuration loading with environment variable support
- `serde`: Serialization/deserialization for configuration structures
- `toml`: TOML file parsing

### Optional Crates
- `config-rs`: Alternative configuration library (if not using `config`)
- `figment`: Another configuration library option

## Future Enhancements

### Potential Additions
- **Configuration schema validation**: JSON Schema for configuration validation
- **Configuration templates**: Pre-built configuration templates for common setups
- **Configuration migration**: Automatic migration of old configuration formats
- **Configuration diffing**: Show differences between configuration versions
- **Configuration encryption**: Encrypt sensitive fields in configuration files
- **Remote configuration**: Load configuration from remote sources (HTTP, S3, etc.)
- **Configuration versioning**: Track configuration versions and changes

---

[← Back to Phase 2 Spec](phase2_spec.md) | [Component Specifications](phase2_components.md)
