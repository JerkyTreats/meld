# Agent Management CLI Specification

## Overview

This document specifies the CLI commands for managing agents in the improved agent management system. These commands enable users to create, view, validate, and manage their custom agents stored in XDG directories.

## Command Suite

The agent management commands are organized under `merkle agent`:

```
merkle agent <subcommand> [options]
```


## List Agents

Display all available agents from XDG directory.

### Requirements

**Syntax**

```
merkle agent list [options]
```

**Options**
- `--format <text|json>`: Output format (default: text)
- `--role <Reader|Writer|Synthesis>`: Filter by role

**Behavior**
1. Load agents from XDG directory (`$XDG_CONFIG_HOME/merkle/agents/`)
2. Apply filters if specified
3. Display agent list with key information

**Output (Text Format)**
```
Available Agents:
  code-analyzer    Writer    ~/prompts/code-analysis.md
  docs-writer      Writer    ~/prompts/docs.md
  local-dev        Writer    ~/prompts/dev.md

Total: 3 agents

Note: Agents are provider-agnostic. Providers are selected at runtime.
```

**Output (JSON Format)**
```json
{
  "agents": [
    {
      "agent_id": "code-analyzer",
      "role": "Writer",
      "system_prompt_path": "~/prompts/code-analysis.md"
    }
  ],
  "total": 3
}
```

### Implementation

- Add `Agent` subcommand to `Commands` enum in `src/tooling/cli.rs`
- Add `List` variant with `format` and `role` options
- Implement agent loading from XDG directory in `src/agent.rs` (AgentRegistry)
- Add output formatting logic (text/JSON) in command handler
- Update `src/tooling/cli.rs` command dispatch logic

### Tests

- Unit tests in `src/agent.rs` for agent loading and filtering
- Integration test in `tests/integration/agent_authorization.rs` or new `tests/integration/agent_cli.rs`
  - Test listing all agents
  - Test filtering by role
  - Test text and JSON output formats
  - Test empty agent list handling

## Show Agent

Display detailed information about a specific agent.

### Requirements

**Syntax**

```
merkle agent show <agent_id> [options]
```

**Options**
- `--format <text|json>`: Output format (default: text)
- `--include-prompt`: Include prompt file content in output

**Behavior**
1. Search for agent in XDG directory
2. Load and display agent configuration
3. Optionally load and display prompt file content

**Output (Text Format)**
```
Agent: code-analyzer
Role: Writer
Prompt File: ~/prompts/code-analysis.md

Metadata:
  user_prompt_file: "Analyze the code at {path}..."
  user_prompt_directory: "Analyze the directory at {path}..."
  specialization: rust
  focus: performance

Prompt Content (--include-prompt):
# Code Analysis Assistant
...
```

**Output (JSON Format)**
```json
{
  "agent_id": "code-analyzer",
  "role": "Writer",
  "system_prompt_path": "~/prompts/code-analysis.md",
  "metadata": {
    "user_prompt_file": "Analyze the code at {path}...",
    "user_prompt_directory": "Analyze the directory at {path}...",
    "specialization": "rust",
    "focus": "performance"
  },
  "prompt_content": "..."
}
```

**Note**: Agents are provider-agnostic. No provider information in agent config.

### Implementation

- Add `Show` variant to `Agent` subcommand in `src/tooling/cli.rs`
- Add `agent_id` argument and `format`, `include_prompt` options
- Implement agent lookup in `src/agent.rs` (AgentRegistry::get)
- Add prompt file loading logic in `src/agent.rs`
- Add output formatting (text/JSON) in command handler

### Tests

- Unit tests in `src/agent.rs` for agent lookup and prompt loading
- Integration test in `tests/integration/agent_cli.rs`
  - Test showing existing agent
  - Test showing agent with --include-prompt
  - Test text and JSON output formats
  - Test error handling for non-existent agent

## Validate Agent

Validate agent configuration and prompt file.

### Requirements

**Syntax**

```
merkle agent validate <agent_id> [options]
```

**Options**
- `--verbose`: Show detailed validation results

**Behavior**
1. Load agent configuration
2. Validate required fields
3. Check prompt file exists and is readable
4. Validate prompt file is valid UTF-8
5. Check user prompt templates in metadata (if Writer/Synthesis)
6. Report all validation errors

**Output**
```
Validating agent: code-analyzer

✓ Agent ID matches filename
✓ Role is valid (Writer)
✓ Prompt file exists: ~/prompts/code-analysis.md
✓ Prompt file is readable
✓ Prompt file is valid UTF-8
✓ User prompt templates present in metadata

Validation passed: 6/6 checks
```

**Error Output**
```
Validating agent: invalid-agent

✗ Agent ID doesn't match filename
✗ Prompt file not found: ~/prompts/missing.md
✗ Missing user_prompt_file in metadata (required for Writer role)

Validation failed: 3 errors found
```

### Implementation

- Add `Validate` variant to `Agent` subcommand in `src/tooling/cli.rs`
- Add `agent_id` argument and `verbose` option
- Implement validation logic in `src/agent.rs` (AgentRegistry or new validation module)
- Add validation checks:
  - Agent ID matches filename
  - Role is valid enum value
  - Prompt file exists and is readable
  - Prompt file is valid UTF-8
  - User prompt templates in metadata (for Writer/Synthesis)

### Tests

- Unit tests in `src/agent.rs` for validation logic
- Integration test in `tests/integration/agent_cli.rs`
  - Test validation of valid agent
  - Test validation with missing prompt file
  - Test validation with invalid role
  - Test validation with missing metadata templates
  - Test verbose output format

## Create Agent

Interactively create a new agent configuration.

### Requirements

**Syntax**

```
merkle agent create <agent_id> [options]
```

**Options**
- `--role <Reader|Writer|Synthesis>`: Set role (required)
- `--prompt-path <path>`: Path to prompt file (required for Writer/Synthesis)
- `--interactive`: Interactive mode (default)
- `--non-interactive`: Non-interactive mode (use flags)

**Behavior (Interactive Mode)**
1. Prompt for agent role
2. If Writer/Synthesis, prompt for prompt file path
5. Create agent config file in XDG directory
6. Validate configuration
7. Display created agent

**Behavior (Non-Interactive Mode)**
1. Use provided flags
2. Validate required fields
3. Create agent config file
4. Display created agent

**Example (Interactive)**
```
$ merkle agent create my-agent

Role (Reader/Writer/Synthesis): Writer
Prompt file path: ~/prompts/my-prompt.md

Creating agent configuration...
✓ Agent created: ~/.config/merkle/agents/my-agent.toml

Note: Provider will be selected at runtime when using this agent.
```

Note: "Agent created:" follows pathing logic described in this document. 

### Implementation

- Add `Create` variant to `Agent` subcommand in `src/tooling/cli.rs`
- Add `agent_id` argument and options for role, prompt-path, interactive/non-interactive
- Implement interactive prompts using `dialoguer` or similar crate
- Add agent config file creation in `src/agent.rs` (AgentRegistry or new module)
- Use `src/tooling/editor.rs` patterns for interactive input if needed
- Implement XDG directory creation if needed
- Add validation after creation

### Tests

- Unit tests in `src/agent.rs` for agent creation logic
- Integration test in `tests/integration/agent_cli.rs`
  - Test non-interactive creation with all flags
  - Test creation of Reader agent
  - Test creation of Writer agent with prompt path
  - Test creation of Synthesis agent with prompt path
  - Test error handling for missing required fields
  - Test error handling for invalid prompt path

## Edit Agent

Edit an existing agent configuration.

### Requirements

**Syntax**

```
merkle agent edit <agent_id> [options]
```

**Options**
- `--prompt-path <path>`: Update prompt file path
- `--role <Reader|Writer|Synthesis>`: Update role
- `--editor <editor>`: Use specific editor (default: $EDITOR)

**Behavior**
1. Load existing agent configuration
2. Open in editor (or use flags for specific fields)
3. Validate updated configuration
4. Save changes
5. Display updated agent

**Note**: If using editor, system will:
- Create temporary file with current config
- Open in user's default editor
- Validate and save on exit
- Clean up temporary file

### Implementation

- Add `Edit` variant to `Agent` subcommand in `src/tooling/cli.rs`
- Add `agent_id` argument and options for prompt-path, role, editor
- Implement flag-based editing (direct field updates)
- Use `src/tooling/editor.rs` for editor-based editing
- Add agent config update logic in `src/agent.rs`
- Add validation after update

### Tests

- Unit tests in `src/agent.rs` for agent update logic
- Integration test in `tests/integration/agent_cli.rs`
  - Test editing with --prompt-path flag
  - Test editing with --role flag
  - Test editing with editor (mock editor)
  - Test error handling for non-existent agent
  - Test validation after edit

## Remove Agent

Remove an agent configuration (XDG agents only).

### Requirements

**Syntax**

```
merkle agent remove <agent_id> [options]
```

**Options**
- `--force`: Skip confirmation prompt

**Behavior**
1. Verify agent exists in XDG directory
2. Confirm removal (unless --force)
3. Remove agent config file
4. Display confirmation

**Output**
```
Removed agent: code-analyzer
Configuration file deleted: ~/.config/merkle/agents/code-analyzer.toml
```

**Error Handling**
- If agent not found: Error message
- If agent in use: Warning (but allow removal)

### Implementation

- Add `Remove` variant to `Agent` subcommand in `src/tooling/cli.rs`
- Add `agent_id` argument and `force` option
- Implement confirmation prompt (unless --force)
- Add agent config file deletion logic
- Check if agent is in use (optional warning)

### Tests

- Unit tests in `src/agent.rs` for agent removal logic
- Integration test in `tests/integration/agent_cli.rs`
  - Test removal with confirmation
  - Test removal with --force flag
  - Test error handling for non-existent agent
  - Test removal of agent config file

## Common Options

All commands support:
- `--config <path>`: Override config file location
- `--log-level <level>`: Set log level
- `--help`: Show command help

## Error Handling

### Agent Not Found
```
Error: Agent 'nonexistent' not found

Available agents:
  - code-analyzer
  - docs-writer
  - local-dev

Use 'merkle agent list' to see all agents.
```

### Prompt File Not Found
```
Error: Prompt file not found: ~/prompts/missing.md

Please ensure the file exists and is readable.
You can update the path with: merkle agent edit <agent_id> --prompt-path <new-path>
```

### Invalid Configuration
```
Error: Invalid agent configuration

Issues found:
  - Agent ID 'mismatch' doesn't match filename 'code-analyzer.toml'
  - Role 'Invalid' is not valid (must be Reader, Writer, or Synthesis)
  - Prompt file not found: ~/prompts/missing.md
  - Missing user_prompt_file in metadata (required for Writer role)

Fix these issues and try again.
```

## Implementation Notes

### Agent Discovery

1. **XDG Agents**: Load from `$XDG_CONFIG_HOME/merkle/agents/*.toml`

### Path Resolution

- Absolute paths used as-is
- Tilde (`~/`) expanded to `$HOME/`
- Relative paths resolved relative to `$XDG_CONFIG_HOME/merkle/`

### Validation

- Agent ID must match filename (without `.toml`)
- Role must be valid enum value
- Prompt file must exist (for Writer/Synthesis)
- Prompt file must be readable UTF-8
- User prompt templates must be in metadata (for Writer/Synthesis)

## Examples

### List All Agents
```bash
merkle agent list
```

### List Only Writer Agents
```bash
merkle agent list --role Writer
```

### Show Agent Details
```bash
merkle agent show code-analyzer --include-prompt
```

### Validate Agent
```bash
merkle agent validate code-analyzer
```

### Create New Agent
```bash
merkle agent create my-agent \
  --role Writer \
  --prompt-path ~/prompts/my-prompt.md
```

**Note**: Provider is selected at runtime when using the agent:
```bash
merkle context generate --path src/lib.rs \
  --agent my-agent \
  --provider openai-gpt4
```

### Edit Agent Prompt Path
```bash
merkle agent edit code-analyzer --prompt-path ~/prompts/new-prompt.md
```

### Remove Agent
```bash
merkle agent remove old-agent
```

## Related Documentation

- [Provider-Agent Separation](../provider/provider_agent_separation.md) - Separation design
- [Agent Management Requirements](agent_management_requirements.md) - Overall requirements
- [Provider Management Requirements](../provider/provider_management_requirements.md) - Provider configuration
- [Context Management README](../README.md) - How agents are used

---

[← Back to Context Management](../README.md)

