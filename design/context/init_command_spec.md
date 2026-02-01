# Initialization Command Specification

## Overview

This document specifies the `merkle init` command, which initializes the default agent configurations and prompt files required for the Merkle context system to function. The command sets up the XDG configuration structure with default agents that users can immediately use or customize.

## Goals

1. **First-Time Setup**: Enable users to get started with Merkle without manual configuration
2. **Default Agents**: Provide a complete set of default agents covering all roles (Reader, Writer, Synthesis)
3. **Built-in Prompts**: Embed default agent prompts in the binary for reliable initialization
4. **Idempotent Operation**: Safe to run multiple times without duplicating configuration
5. **User Customization**: Initialize agents that users can immediately customize

## Command Structure

### Basic Command

```
merkle init [options]
```

### Options

- `--force`: Force re-initialization, overwriting existing default agents
- `--list`: List what would be initialized without actually creating files
- `--prompts-dir <path>`: Override default prompts directory location (advanced)

### Command Behavior

1. **Check Existing Configuration**: Verify if default agents already exist
2. **Validate XDG Directories**: Ensure required XDG directories exist (create if needed)
3. **Initialize Prompts**: Copy built-in prompt files to `$XDG_CONFIG_HOME/merkle/prompts/`
4. **Initialize Agents**: Create agent TOML files in `$XDG_CONFIG_HOME/merkle/agents/`
5. **Validate Initialization**: Verify all agents and prompts are valid
6. **Report Results**: Display summary of what was created/updated

## Default Agents

The initialization command creates the following default agents:

### 1. Reader Agent

**Agent ID**: `reader`  
**Role**: `Reader`  
**Purpose**: Read-only agent for querying context frames without generation capabilities

**Configuration**:
```toml
agent_id = "reader"
role = "Reader"
# Reader agents don't require system prompts or providers
```

**Prompt File**: None (Reader agents don't generate frames)

### 2. Writer Agent (Code Analysis)

**Agent ID**: `code-analyzer`  
**Role**: `Writer`  
**Purpose**: Analyze code files and generate comprehensive analysis frames

**Configuration**:
```toml
agent_id = "code-analyzer"
role = "Writer"
system_prompt_path = "prompts/code-analyzer.md"

[metadata]
user_prompt_file = "Analyze the code file at {path}. Provide a comprehensive analysis including:\n- Code structure and organization\n- Key functions and their purposes\n- Dependencies and relationships\n- Notable patterns or conventions\n- Potential issues or improvements"
user_prompt_directory = "Analyze the directory structure at {path}. Provide an overview including:\n- Directory purpose and organization\n- Key files and their roles\n- Module relationships\n- Overall architecture patterns"
```

**Prompt File**: `prompts/code-analyzer.md` (built into binary)

### 3. Writer Agent (Documentation)

**Agent ID**: `docs-writer`  
**Role**: `Writer`  
**Purpose**: Generate clear, comprehensive documentation for code and APIs

**Configuration**:
```toml
agent_id = "docs-writer"
role = "Writer"
system_prompt_path = "prompts/docs-writer.md"

[metadata]
user_prompt_file = "Generate comprehensive documentation for the code file at {path}. Include:\n- Purpose and overview\n- API documentation\n- Usage examples\n- Important notes and warnings\n- Related components"
user_prompt_directory = "Generate documentation for the directory at {path}. Include:\n- Directory purpose and structure\n- Module overview\n- Key components and their roles\n- Usage guidelines"
```

**Prompt File**: `prompts/docs-writer.md` (built into binary)

### 4. Synthesis Agent

**Agent ID**: `synthesis-agent`  
**Role**: `Synthesis`  
**Purpose**: Combine context frames from child nodes into coherent branch-level summaries

**Configuration**:
```toml
agent_id = "synthesis-agent"
role = "Synthesis"
system_prompt_path = "prompts/synthesis-agent.md"

[metadata]
user_prompt_directory = "Synthesize context frames from child nodes in the directory at {path}. Combine the information into a coherent summary that:\n- Preserves key information from child contexts\n- Identifies common themes and patterns\n- Highlights important relationships\n- Maintains accuracy and completeness"
```

**Prompt File**: `prompts/synthesis-agent.md` (built into binary)

## Prompt File Structure

### Build-Time Storage

Default agent prompts are stored in the source repository at:

```
prompts/
├── code-analyzer.md
├── docs-writer.md
└── synthesis-agent.md
```

These files are embedded into the binary at build time using Rust's `include_str!()` macro or similar embedding mechanism.

### Runtime Storage

During initialization, prompt files are copied to:

```
$XDG_CONFIG_HOME/merkle/prompts/
├── code-analyzer.md
├── docs-writer.md
└── synthesis-agent.md
```

**Path Resolution**: Agent configurations use relative paths (`prompts/<name>.md`) which resolve relative to `$XDG_CONFIG_HOME/merkle/` per the path resolution rules in [Agent Management Requirements](agents/agent_management_requirements.md).

### Prompt File Format

All prompt files are Markdown documents with the following structure:

```markdown
# Agent Name

Brief description of the agent's role and purpose.

## Your Role

Detailed description of what the agent does and its responsibilities.

## Guidelines

Specific guidelines for the agent's behavior:
- Guideline 1
- Guideline 2
- Guideline 3

## Output Format

Expected output format and structure (if applicable).

## Examples

Example scenarios or use cases (optional).
```

## Implementation Details

### Build-Time Embedding

Prompts are embedded into the binary using Rust's `include_str!()` macro:

```rust
// src/init.rs
pub const DEFAULT_PROMPTS: &[(&str, &str)] = &[
    ("code-analyzer.md", include_str!("../prompts/code-analyzer.md")),
    ("docs-writer.md", include_str!("../prompts/docs-writer.md")),
    ("synthesis-agent.md", include_str!("../prompts/synthesis-agent.md")),
];
```

### Initialization Logic

1. **Directory Creation**:
   - Ensure `$XDG_CONFIG_HOME/merkle/` exists
   - Ensure `$XDG_CONFIG_HOME/merkle/agents/` exists
   - Ensure `$XDG_CONFIG_HOME/merkle/prompts/` exists
   - Ensure `$XDG_CONFIG_HOME/merkle/providers/` exists (for future use)

2. **Prompt File Initialization**:
   - For each built-in prompt:
     - Check if file exists in `$XDG_CONFIG_HOME/merkle/prompts/`
     - If `--force` or file doesn't exist, write prompt content
     - If file exists and `--force` not set, skip (preserve user customizations)

3. **Agent Configuration Initialization**:
   - For each default agent:
     - Check if agent config exists in `$XDG_CONFIG_HOME/merkle/agents/`
     - If `--force` or config doesn't exist, create agent TOML
     - If config exists and `--force` not set, skip (preserve user customizations)
     - Validate agent configuration after creation

4. **Validation**:
   - Load all initialized agents using `AgentRegistry::load_from_xdg()`
   - Validate each agent using `AgentRegistry::validate_agent()`
   - Report any validation errors

### Idempotency

The command is idempotent by default:

- **Existing Prompts**: If a prompt file exists, it is **not** overwritten (preserves user customizations)
- **Existing Agents**: If an agent config exists, it is **not** overwritten (preserves user customizations)
- **Force Mode**: `--force` flag overwrites existing files

**Rationale**: Users may customize default agents and prompts. The init command should not destroy their customizations unless explicitly requested.

### Error Handling

**Missing XDG Config Home**:
```
Error: Could not determine XDG config home directory (HOME not set)

Please set the HOME environment variable or XDG_CONFIG_HOME.
```

**Directory Creation Failure**:
```
Error: Failed to create directory: ~/.config/merkle/prompts

Permission denied. Please check directory permissions.
```

**Agent Validation Failure**:
```
Warning: Agent 'code-analyzer' failed validation after initialization

Issues:
  - Prompt file not found: prompts/code-analyzer.md

Please check the initialization output and fix any issues.
```

## Output Format

### Success Output

```
Initializing Merkle configuration...

Created prompts directory: ~/.config/merkle/prompts/
  ✓ code-analyzer.md
  ✓ docs-writer.md
  ✓ synthesis-agent.md

Created agents directory: ~/.config/merkle/agents/
  ✓ reader.toml (Reader)
  ✓ code-analyzer.toml (Writer)
  ✓ docs-writer.toml (Writer)
  ✓ synthesis-agent.toml (Synthesis)

Validation:
  ✓ All 4 agents validated successfully

Initialization complete! You can now use:
  - merkle agent list          # List all agents
  - merkle agent show <id>     # View agent details
  - merkle context generate    # Generate context frames
```

### Idempotent Run (No Changes)

```
Initializing Merkle configuration...

All default agents already exist. Use --force to re-initialize.

Existing agents:
  - reader (Reader)
  - code-analyzer (Writer)
  - docs-writer (Writer)
  - synthesis-agent (Synthesis)

Run 'merkle init --force' to overwrite existing configurations.
```

### List Mode Output

```
Initialization Preview:

Would create prompts:
  - prompts/code-analyzer.md
  - prompts/docs-writer.md
  - prompts/synthesis-agent.md

Would create agents:
  - reader.toml (Reader)
  - code-analyzer.toml (Writer)
  - docs-writer.toml (Writer)
  - synthesis-agent.toml (Synthesis)

Run 'merkle init' to perform initialization.
```

### Force Mode Output

```
Initializing Merkle configuration (--force mode)...

Overwriting existing configurations...

Created prompts directory: ~/.config/merkle/prompts/
  ✓ code-analyzer.md (overwritten)
  ✓ docs-writer.md (overwritten)
  ✓ synthesis-agent.md (overwritten)

Created agents directory: ~/.config/merkle/agents/
  ✓ reader.toml (overwritten)
  ✓ code-analyzer.toml (overwritten)
  ✓ docs-writer.toml (overwritten)
  ✓ synthesis-agent.toml (overwritten)

Validation:
  ✓ All 4 agents validated successfully

Initialization complete!
```

## Additional Initialization Items

Based on the context management system requirements, the following items should also be initialized:

### 1. XDG Directory Structure

Ensure all required XDG directories exist:
- `$XDG_CONFIG_HOME/merkle/` - Base config directory
- `$XDG_CONFIG_HOME/merkle/agents/` - Agent configurations
- `$XDG_CONFIG_HOME/merkle/providers/` - Provider configurations (for Phase 4)
- `$XDG_CONFIG_HOME/merkle/prompts/` - Default prompt files

**Note**: These directories are auto-created by existing utilities (`agents_dir()`, `providers_dir()`), but init should verify they exist.

### 2. Example Configuration (Optional)

Consider creating an example `config.toml` in `$XDG_CONFIG_HOME/merkle/` if it doesn't exist:

```toml
# Merkle Configuration
# This file contains system-level configuration for Merkle.

[system]
default_workspace_root = "."

[system.storage]
store_path = ".merkle/store"
frames_path = ".merkle/frames"
```

**Rationale**: Provides a starting point for system configuration, though this is optional since workspace-specific configs take precedence.

### 3. README or Documentation (Optional)

Consider creating a `README.md` in `$XDG_CONFIG_HOME/merkle/` with:
- Overview of the directory structure
- Links to documentation
- Quick start guide

**Rationale**: Helps users understand the configuration structure.

## Implementation Location

### Command Registration

Add to `src/tooling/cli.rs`:

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands ...
    /// Initialize default agents and prompts
    Init {
        /// Force re-initialization (overwrite existing)
        #[arg(long)]
        force: bool,
        
        /// List what would be initialized without creating files
        #[arg(long)]
        list: bool,
    },
}
```

### Implementation Module

Create `src/init.rs` with:

- `InitCommand` struct for initialization logic
- `initialize_default_agents()` function
- `initialize_prompts()` function
- `validate_initialization()` function
- Built-in prompt constants

### Integration

- Add `init` module to `src/lib.rs`
- Wire up command handler in `src/tooling/cli.rs`
- Use existing `AgentRegistry` and XDG utilities

## Testing Requirements

### Unit Tests (`src/init.rs`)

1. **Prompt Embedding Tests**:
   - `test_prompts_embedded()` - Verify prompts are embedded in binary
   - `test_prompt_content_valid()` - Verify prompt content is valid UTF-8

2. **Initialization Logic Tests**:
   - `test_initialize_prompts_creates_files()` - Prompts written correctly
   - `test_initialize_agents_creates_configs()` - Agent configs created correctly
   - `test_initialize_idempotent()` - Idempotent behavior (no overwrite)
   - `test_initialize_force_overwrites()` - Force mode overwrites existing

3. **Validation Tests**:
   - `test_validate_initialized_agents()` - All agents validate successfully
   - `test_validation_reports_errors()` - Validation errors reported correctly

### Integration Tests (`tests/integration/init_command.rs`)

1. **Command Execution Tests**:
   - `test_init_creates_default_agents()` - Default agents created
   - `test_init_creates_prompts()` - Prompt files created
   - `test_init_idempotent()` - Safe to run multiple times
   - `test_init_force_overwrites()` - Force mode works
   - `test_init_list_mode()` - List mode shows preview

2. **Directory Creation Tests**:
   - `test_init_creates_xdg_directories()` - All directories created
   - `test_init_handles_existing_directories()` - Existing directories handled

3. **Validation Tests**:
   - `test_init_validates_all_agents()` - All agents pass validation
   - `test_init_reports_validation_errors()` - Errors reported correctly

4. **Error Handling Tests**:
   - `test_init_handles_missing_xdg_home()` - Missing XDG_CONFIG_HOME handled
   - `test_init_handles_permission_errors()` - Permission errors handled gracefully

## Related Documentation

- [Agent Management Requirements](agents/agent_management_requirements.md) - Agent configuration requirements
- [Agent CLI Specification](agents/agent_cli_spec.md) - Agent management commands
- [Provider Management Requirements](provider/provider_management_requirements.md) - Provider configuration (Phase 4)
- [Context Generate Command](context_generate_command.md) - Using agents for context generation
- [PLAN.md](PLAN.md) - Overall implementation plan

## Future Enhancements

### Phase 4 Integration

When Phase 4 (Provider Management CLI) is implemented, consider:

1. **Default Provider Initialization**: Optionally initialize a default provider (e.g., local-ollama)
2. **Provider-Agent Pairing**: Suggest provider configurations for default agents

### Advanced Features

1. **Template System**: Allow users to specify custom agent templates
2. **Migration Support**: Migrate agents from old config.toml format
3. **Backup Creation**: Create backup of existing configs before force initialization
4. **Custom Prompt Sources**: Allow initialization from custom prompt directories

---

[← Back to Context Management](README.md)

