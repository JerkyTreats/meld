# Agent Management CLI Specification

## Overview

This document specifies the CLI commands for managing agents in the improved agent management system. These commands enable users to create, view, validate, and manage their custom agents stored in XDG directories.

## Command Suite

The agent management commands are organized under `merkle agent`:

```
merkle agent <subcommand> [options]
```

## Commands

### 1) List Agents

Display all available agents from XDG directory.

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

### 2) Show Agent

Display detailed information about a specific agent.

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

### 3) Validate Agent

Validate agent configuration and prompt file.

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

### 4) Create Agent

Interactively create a new agent configuration.

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
3. Prompt for user prompt templates (optional, can add to metadata)
4. Prompt for additional metadata (optional)
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
User prompt template for files (optional): Analyze the code at {path}...
User prompt template for directories (optional): Analyze the directory at {path}...

Creating agent configuration...
✓ Agent created: ~/.config/merkle/agents/my-agent.toml

Note: Provider will be selected at runtime when using this agent.
```

### 5) Edit Agent

Edit an existing agent configuration.

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

### 6) Remove Agent

Remove an agent configuration (XDG agents only).

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

