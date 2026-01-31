# Agent Management System Requirements

## Overview

This document specifies requirements for the agent management system supporting user-customized agents with markdown-based prompts stored in XDG directories.

## Goals

1. **User-Level Agent Management**: Agents stored in XDG config directory, accessible across all workspaces
2. **Markdown Prompts**: System prompts stored as separate markdown files for better editing and formatting
3. **Flexible Prompt Location**: Prompt files can be anywhere on the system, fully user-managed
4. **Improved Discoverability**: Easy to list, view, and manage user-defined agents
5. **Better Tooling**: CLI commands for agent management

## Requirements

### R1: XDG-Based Agent Storage

**Requirement**: Agent configurations must be stored in XDG config directory.

**Specification**:
- **Location**: `$XDG_CONFIG_HOME/merkle/agents/` (defaults to `~/.config/merkle/agents/`)
- **Format**: One agent per file: `$XDG_CONFIG_HOME/merkle/agents/<agent_id>.toml`
- **Structure**: TOML files containing agent metadata, role, and prompt file path (provider-agnostic)

**Rationale**:
- Follows XDG Base Directory Specification
- User-level configuration (not workspace-specific)
- Agents available across all workspaces
- Standard location for user configuration

**Directory Structure**:
```
$XDG_CONFIG_HOME/merkle/
├── config.toml              # System configuration (existing)
├── agents/                   # Agent configurations (provider-agnostic)
│   ├── code-analyzer.toml
│   ├── docs-writer.toml
│   └── local-dev.toml
└── providers/                # Provider configurations (separate)
    ├── openai-gpt4.toml
    └── local-ollama.toml
```

### R2: Markdown Prompt Files

**Requirement**: System prompts must be stored as separate markdown files.

**Specification**:
- **Format**: Markdown files (`.md` extension)
- **Location**: User-managed, can be anywhere on the filesystem
- **Reference**: Agent config contains absolute or relative path to prompt file
- **Content**: Full markdown support (headings, lists, code blocks, etc.)

**Rationale**:
- Better editing experience (syntax highlighting, formatting)
- Version control friendly
- Reusable across multiple agents
- Supports complex, formatted prompts

**Example Prompt File** (`~/prompts/code-analysis.md`):
```markdown
# Code Analysis Assistant

You are a code analysis assistant specialized in understanding and documenting codebases.

## Your Role

Your primary responsibilities include:
- Analyzing code structure and patterns
- Identifying key components and relationships
- Generating comprehensive documentation
- Providing clear, accurate code summaries

## Guidelines

- Focus on accuracy and clarity
- Maintain technical precision
- Use appropriate terminology
- Structure information logically
```

### R3: Agent Configuration Format

**Requirement**: Agent configuration files must reference prompt files by path.

**Specification**:
- **File Format**: TOML
- **Location**: `$XDG_CONFIG_HOME/merkle/agents/<agent_id>.toml`
- **Required Fields**:
  - `agent_id`: Unique identifier (must match filename)
  - `role`: AgentRole (Reader, Writer, Synthesis)
  - `system_prompt_path`: Path to markdown prompt file (absolute or relative to config dir)

**Example Agent Config** (`~/.config/merkle/agents/code-analyzer.toml`):
```toml
agent_id = "code-analyzer"
role = "Writer"

# Path to markdown prompt file (absolute or relative to ~/.config/merkle/)
system_prompt_path = "~/prompts/code-analysis.md"
```


### R4: Prompt Path Resolution

**Requirement**: System must resolve prompt file paths correctly.

**Specification**:
- **Absolute Paths**: Used as-is
- **Relative Paths**: Resolved relative to `$XDG_CONFIG_HOME/merkle/`
- **Tilde Expansion**: `~/` expanded to `$HOME/`
- **Validation**: Verify file exists and is readable at load time
- **Error Handling**: Clear error messages if prompt file not found

**Path Resolution Priority**:
1. Absolute path (if starts with `/`)
2. Tilde expansion (if starts with `~/`)
3. Relative to `$XDG_CONFIG_HOME/merkle/`
4. Relative to `$pwd` (if starts wtih `./`)

**Examples**:
- `system_prompt_path = "/home/user/prompts/code.md"` → absolute
- `system_prompt_path = "~/prompts/code.md"` → `$HOME/prompts/code.md`
- `system_prompt_path = "prompts/code.md"` → `$XDG_CONFIG_HOME/merkle/prompts/code.md`
- `system_prompt_path = "./prompts/code.md"` → `$pwd/prompts/code.md`

### R5: Agent Discovery and Management

**Requirement**: CLI commands for agent management.

**Specification**:
- **List Agents**: `merkle agent list` - Show all available agents
- **Show Agent**: `merkle agent show <agent_id>` - Display agent details
- **Validate Agent**: `merkle agent validate <agent_id>` - Check configuration
- **Create Agent**: `merkle agent create <agent_id>` - Interactive agent creation
- **Edit Agent**: `merkle agent edit <agent_id>` - Edit agent configuration

**Output Formats**:
- Text format (default, human-readable)
- JSON format (`--format json`) for scripting

**Agent List Output**:
```
Available Agents:
  code-analyzer    Writer    ~/prompts/code-analysis.md
  docs-writer      Writer    ~/prompts/docs.md
  local-dev        Writer    ~/prompts/dev.md
```

**Note**: Provider information is not shown in agent list since agents are provider-agnostic. Providers are selected at runtime.

### R6: Prompt File Management

**Requirement**: System must handle prompt file lifecycle.

**Specification**:
- **Validation**: Verify prompt file exists when agent is loaded
- **Caching**: Cache prompt content after first read (with file modification time check)
- **Reload**: Support reloading prompts without restart
- **Error Handling**: Graceful degradation if prompt file missing
- **Template Support**: Consider prompt templates with variable substitution (future)

**File Monitoring** (Optional, Future):
- Watch prompt files for changes
- Auto-reload on modification
- Notify users of prompt updates

### R7: Agent Registry Integration

**Requirement**: AgentRegistry must load agents from XDG directory.

**Specification**:
- **Loading**: `AgentRegistry::load_from_xdg()` method
- **Error Handling**: Continue loading other agents if one fails

**Loading Sequence**:
1. Load XDG agents (`$XDG_CONFIG_HOME/merkle/agents/*.toml`)
2. Validate all agents
3. Report errors but continue

### R8: Configuration Validation

**Requirement**: Validate agent configurations and prompt files.

**Specification**:
- **Agent Config Validation**:
  - Required fields present
  - `agent_id` matches filename
  - `role` is valid enum value
  - `system_prompt_path` is valid path
  - Prompt file exists and is readable
- **Prompt File Validation**:
  - File exists and is readable
  - File is valid UTF-8
  - File is not empty
  - Optional: Validate markdown syntax

**Validation Errors**:
- Clear error messages with file locations
- Line numbers for TOML parsing errors
- Path information for missing files

### R9: Documentation and Examples

**Requirement**: Provide documentation and example agents.

**Specification**:
- **Documentation**: User guide for creating and managing agents
- **Examples**: Example agent configurations and prompt files
- **Templates**: Starter templates for common agent types

**Example Locations**:
- `$XDG_CONFIG_HOME/merkle/agents/examples/` - Example agent configs
- `$XDG_CONFIG_HOME/merkle/agents/prompts/` - Example prompt files (optional)

## Non-Requirements

### Out of Scope

1. **Prompt Templates**: Variable substitution in prompts (future enhancement)
2. **Prompt Versioning**: Git integration or version control (user-managed)
3. **Agent Sharing**: Distribution mechanism for sharing agents (user-managed)
4. **GUI Tools**: Visual agent editor (CLI only)
5. **Prompt Validation**: Markdown syntax validation (optional, not required)
6. **Agent Permissions**: Fine-grained access control (future)

## Implementation Considerations

### File System Structure

```
$XDG_CONFIG_HOME/merkle/
├── config.toml                    # System configuration
├── agents/                         # Agent configurations (provider-agnostic)
│   ├── code-analyzer.toml
│   ├── docs-writer.toml
│   └── examples/                  # Optional examples
│       └── basic-writer.toml
└── providers/                      # Provider configurations (separate)
    ├── openai-gpt4.toml
    └── local-ollama.toml

User-managed prompt files:
~/prompts/
├── code-analysis.md
├── documentation.md
└── development.md
```

### Agent Config Schema

```toml
# Required
agent_id = "string"                    # Must match filename
role = "Reader" | "Writer" | "Synthesis"
system_prompt_path = "string"          # Path to .md file

# Optional
metadata = { ... }                     # Key-value pairs (including user prompt templates)
```

**Note**: Agents are provider-agnostic. No `provider_name` or `completion_options` fields. See [Provider-Agent Separation](provider_agent_separation.md).

### Error Scenarios

1. **Missing Prompt File**: Error with path and suggestion to create file
2. **Invalid Agent ID**: Error if agent_id doesn't match filename
3. **Invalid Role**: Error with valid options
4. **Invalid Prompt Path**: Error if prompt file path cannot be resolved

## Success Criteria

1. ✅ Users can create agents in XDG directory
2. ✅ Agents use markdown prompt files
3. ✅ Prompt files can be anywhere on filesystem
4. ✅ Agents available across all workspaces
5. ✅ CLI commands for agent management
6. ✅ Clear error messages for configuration issues

## Related Documentation

- [Provider-Agent Separation](../provider/provider_agent_separation.md) - Separation of provider and agent concerns
- [Provider Management Requirements](../provider/provider_management_requirements.md) - Provider configuration
- [Context Management README](../README.md) - Context commands that use agents
- [Phase 2 Configuration](../workflow/phase2_configuration.md) - Current configuration system
- [Phase 2 Model Providers](../workflow/phase2_model_providers.md) - Provider implementation details

---

## Appendix: Example Agent Configurations

### Example 1: Basic Writer Agent

**File**: `~/.config/merkle/agents/docs-writer.toml`
```toml
agent_id = "docs-writer"
role = "Writer"
system_prompt_path = "~/prompts/documentation.md"

[metadata]
user_prompt_file = "Generate documentation for the file at {path}..."
user_prompt_directory = "Generate documentation for the directory at {path}..."
```

**Prompt File**: `~/prompts/documentation.md`
```markdown
# Documentation Generator

You are a documentation generation assistant. Your role is to create clear, 
comprehensive documentation for code and APIs.

Focus on:
- Clarity and readability
- Complete API coverage
- Usage examples
- Best practices
```

**Usage**: Provider selected at runtime
```bash
merkle context generate --path src/lib.rs \
  --agent docs-writer \
  --provider openai-gpt35
```

### Example 2: Advanced Agent with Metadata

**File**: `~/.config/merkle/agents/code-analyzer.toml`
```toml
agent_id = "code-analyzer"
role = "Writer"
system_prompt_path = "/home/user/prompts/code-analysis.md"

[metadata]
user_prompt_file = "Analyze the code at {path}. Focus on {focus}..."
user_prompt_directory = "Analyze the directory structure at {path}..."
specialization = "rust"
focus = "performance"
```

**Note**: No `provider_name` or `completion_options`. Agent is provider-agnostic.

### Example 3: Reader Agent

**File**: `~/.config/merkle/agents/reader.toml`
```toml
agent_id = "reader"
role = "Reader"
# Reader agents don't need prompts or providers
# They only read context frames
```

---

[← Back to Phase 2 Spec](../workflow/phase2_spec.md)

