# Context & Agent Management Refactor Implementation Plan

## Overview

This document outlines the phased implementation plan for refactoring the context and agent management system. The refactor introduces provider-agent separation, XDG-based configuration storage, and new CLI commands for managing agents, providers, and context operations.

The implementation follows a logical progression: first decoupling providers from agents, then moving to XDG-based storage, and finally implementing the new CLI commands that build on these foundations.

---

## Development Phases

### Phase 1 — Provider-Agent Separation

**Goal**: Decouple provider configuration from agent configuration, enabling runtime provider selection.

| Task | Status |
|------|--------|
| Design ProviderRegistry structure | Pending |
| Implement ProviderConfig type | Pending |
| Remove provider_name from AgentIdentity | Pending |
| Remove completion_options from agent config | Pending |
| Update AgentIdentity to be provider-agnostic | Pending |
| Implement ProviderRegistry::load_from_xdg() | Pending |
| Implement ProviderRegistry::create_client() | Pending |
| Update FrameGenerationQueue to accept provider at runtime | Pending |
| Update ContextApiAdapter to pass provider to queue | Pending |
| Add provider metadata to frame generation | Pending |
| Update completion options resolution (provider defaults + agent preferences) | Pending |
| Provider-agent separation tests | Pending |

**Exit Criteria:**
- ✅ ProviderRegistry implemented and independent from AgentRegistry
- ✅ AgentIdentity no longer contains provider references
- ✅ Frame generation accepts provider_name as runtime parameter
- ✅ Completion options resolved from provider defaults (not agent config)
- ✅ Frame metadata includes provider information for attribution
- ✅ All tests pass with new architecture

**Key Changes:**
- `AgentIdentity` struct: Remove `provider` field
- `ProviderRegistry`: New registry for provider configurations
- `FrameGenerationQueue`: Accept `provider_name` parameter in `enqueue()` and `enqueue_and_wait()`
- `ContextApiAdapter`: Pass provider_name when generating frames
- Frame metadata: Include `provider`, `model`, `provider_type` fields

**Dependencies:**
- None (foundational change)

---

### Phase 2 — XDG Configuration System

**Goal**: Move agent and provider configurations to XDG directories, supporting markdown-based prompts.

| Task | Status |
|------|--------|
| Implement XDG directory resolution utilities | Pending |
| Create ProviderRegistry::load_from_xdg() implementation | Pending |
| Create AgentRegistry::load_from_xdg() implementation | Pending |
| Implement prompt file path resolution (absolute, tilde, relative) | Pending |
| Implement markdown prompt file loading | Pending |
| Update agent config schema (system_prompt_path instead of system_prompt) | Pending |
| Implement provider config schema (XDG TOML format) | Pending |
| Implement prompt file validation (exists, readable, UTF-8) | Pending |
| Implement prompt content caching with modification time checks | Pending |
| Add configuration validation for agents and providers | Pending |
| XDG configuration loading tests | Pending |

**Exit Criteria:**
- ✅ Agents load from `$XDG_CONFIG_HOME/merkle/agents/*.toml`
- ✅ Providers load from `$XDG_CONFIG_HOME/merkle/providers/*.toml`
- ✅ Agent configs reference markdown prompt files via `system_prompt_path`
- ✅ Prompt files can be anywhere (absolute, tilde, or relative paths)
- ✅ Prompt files loaded and validated on agent load
- ✅ Clear error messages for missing/invalid configs

**Key Changes:**
- New directory structure: `$XDG_CONFIG_HOME/merkle/agents/` and `$XDG_CONFIG_HOME/merkle/providers/`
- Agent config format: `system_prompt_path` field instead of inline `system_prompt`
- Provider config format: Separate TOML files per provider
- Path resolution: Support absolute, tilde (`~/`), and relative paths

**Dependencies:**
- Phase 1 (Provider-Agent Separation) - Registry structures must support XDG loading

---

### Phase 3 — Agent Management CLI Commands

**Goal**: Implement CLI commands for managing agents stored in XDG directories.

| Task | Status |
|------|--------|
| Implement `merkle agent list` command | Pending |
| Implement `merkle agent show <agent_id>` command | Pending |
| Implement `merkle agent validate <agent_id>` command | Pending |
| Implement `merkle agent create <agent_id>` command (interactive) | Pending |
| Implement `merkle agent edit <agent_id>` command | Pending |
| Implement `merkle agent remove <agent_id>` command | Pending |
| Add agent filtering (by role, by source) | Pending |
| Add output formatting (text, JSON) | Pending |
| Implement prompt file content display (--include-prompt) | Pending |
| Implement agent validation logic (config + prompt file checks) | Pending |
| Add editor integration for `agent edit` | Pending |
| Agent CLI tests | Pending |

**Exit Criteria:**
- ✅ `merkle agent list` shows all agents from XDG directory
- ✅ `merkle agent show` displays agent details with optional prompt content
- ✅ `merkle agent validate` checks config and prompt file validity
- ✅ `merkle agent create` creates new agent configs interactively
- ✅ `merkle agent edit` allows editing agent configs
- ✅ `merkle agent remove` removes XDG agents (with confirmation)
- ✅ All commands support text and JSON output formats
- ✅ Clear error messages for missing/invalid agents

**Key Commands:**
- `merkle agent list [--format text|json] [--role Reader|Writer|Synthesis]`
- `merkle agent show <agent_id> [--format text|json] [--include-prompt]`
- `merkle agent validate <agent_id> [--verbose]`
- `merkle agent create <agent_id> [--role <role>] [--prompt-path <path>] [--interactive|--non-interactive]`
- `merkle agent edit <agent_id> [--prompt-path <path>] [--role <role>] [--editor <editor>]`
- `merkle agent remove <agent_id> [--force]`

**Dependencies:**
- Phase 2 (XDG Configuration System) - Agents must load from XDG directories

---

### Phase 4 — Provider Management CLI Commands

**Goal**: Implement CLI commands for managing providers stored in XDG directories.

| Task | Status |
|------|--------|
| Implement `merkle provider list` command | Pending |
| Implement `merkle provider show <provider_name>` command | Pending |
| Implement `merkle provider validate <provider_name>` command | Pending |
| Implement `merkle provider test <provider_name>` command | Pending |
| Implement `merkle provider create <provider_name>` command (interactive) | Pending |
| Implement `merkle provider edit <provider_name>` command | Pending |
| Implement `merkle provider remove <provider_name>` command | Pending |
| Add provider filtering (by type, by source) | Pending |
| Add output formatting (text, JSON) | Pending |
| Implement API key status display (without exposing keys) | Pending |
| Implement provider validation logic (config + connectivity checks) | Pending |
| Implement provider connectivity testing | Pending |
| Add editor integration for `provider edit` | Pending |
| Provider CLI tests | Pending |

**Exit Criteria:**
- ✅ `merkle provider list` shows all providers from XDG directory
- ✅ `merkle provider show` displays provider details with API key status
- ✅ `merkle provider validate` checks config validity and optionally tests connectivity
- ✅ `merkle provider test` tests provider connectivity and model availability
- ✅ `merkle provider create` creates new provider configs interactively
- ✅ `merkle provider edit` allows editing provider configs
- ✅ `merkle provider remove` removes XDG providers (with confirmation)
- ✅ All commands support text and JSON output formats
- ✅ Clear error messages for missing/invalid providers

**Key Commands:**
- `merkle provider list [--format text|json] [--type openai|anthropic|ollama|local]`
- `merkle provider show <provider_name> [--format text|json] [--include-credentials]`
- `merkle provider validate <provider_name> [--test-connectivity] [--check-model] [--verbose]`
- `merkle provider test <provider_name> [--model <model>] [--timeout <seconds>]`
- `merkle provider create <provider_name> [--type <type>] [--model <model>] [--endpoint <url>] [--api-key <key>] [--interactive|--non-interactive]`
- `merkle provider edit <provider_name> [--model <model>] [--endpoint <url>] [--api-key <key>] [--editor <editor>]`
- `merkle provider remove <provider_name> [--force]`

**Dependencies:**
- Phase 2 (XDG Configuration System) - Providers must load from XDG directories
- Phase 1 (Provider-Agent Separation) - ProviderRegistry must be implemented

---

### Phase 5 — Context Commands with New Architecture

**Goal**: Implement and update context commands to use the new provider-agent separation and XDG configuration.

| Task | Status |
|------|--------|
| Implement `merkle context generate` command | Pending |
| Implement `merkle context get` command | Pending |
| Add `--provider` flag to context generate | Pending |
| Update path resolution (canonicalize, lookup NodeID) | Pending |
| Update agent resolution (default to single Writer agent or require --agent) | Pending |
| Update frame type resolution (default to context-<agent_id>) | Pending |
| Implement head frame existence check (--force flag) | Pending |
| Implement sync/async mode (--sync, --async flags) | Pending |
| Add frame filtering (--agent, --frame-type) to context get | Pending |
| Add output formatting (--format text|json, --combine, --separator) | Pending |
| Add metadata display (--include-metadata) | Pending |
| Add deleted frame handling (--include-deleted) | Pending |
| Update error messages with helpful suggestions | Pending |
| Context CLI tests | Pending |

**Exit Criteria:**
- ✅ `merkle context generate` creates frames using agent + provider (runtime binding)
- ✅ `merkle context generate` supports `--provider` flag for runtime provider selection
- ✅ `merkle context get` retrieves and displays frames with filtering and formatting
- ✅ Path resolution works correctly (canonicalize, NodeID lookup)
- ✅ Agent resolution works (default or explicit via --agent)
- ✅ Frame type defaults to `context-<agent_id>` when not specified
- ✅ Head frame checks prevent duplicate generation (unless --force)
- ✅ Sync and async modes work correctly
- ✅ All filtering, formatting, and output options work
- ✅ Clear error messages with remediation suggestions

**Key Commands:**
- `merkle context generate --path <path>|--node <node_id> [--agent <agent_id>] [--provider <provider_name>] [--frame-type <type>] [--force] [--sync|--async]`
- `merkle context get --path <path>|--node <node_id> [--agent <agent_id>] [--frame-type <type>] [--max-frames <n>] [--ordering recency|deterministic] [--combine] [--separator <text>] [--format text|json] [--include-metadata] [--include-deleted]`

**Key Changes:**
- Context generate: Requires `--provider` flag (or uses default from config)
- Context generate: Agent and provider bound at runtime, not configuration time
- Context get: Rich filtering and formatting options
- Error messages: Include suggestions (e.g., "Run `merkle scan` to update tree")

**Dependencies:**
- Phase 1 (Provider-Agent Separation) - Runtime provider selection required
- Phase 2 (XDG Configuration System) - Agents and providers loaded from XDG
- Phase 3 (Agent Management CLI) - Agent discovery and validation
- Phase 4 (Provider Management CLI) - Provider discovery and validation

---

## Implementation Order Summary

1. **Phase 1: Provider-Agent Separation** (Foundation)
   - Decouples providers from agents
   - Enables runtime provider selection
   - No external dependencies

2. **Phase 2: XDG Configuration System** (Storage)
   - Moves configs to XDG directories
   - Enables markdown prompts
   - Depends on Phase 1 (registry structures)

3. **Phase 3: Agent Management CLI** (Agent Tooling)
   - CLI for managing agents
   - Depends on Phase 2 (XDG loading)

4. **Phase 4: Provider Management CLI** (Provider Tooling)
   - CLI for managing providers
   - Depends on Phase 2 (XDG loading) and Phase 1 (ProviderRegistry)

5. **Phase 5: Context Commands** (User-Facing Commands)
   - Main user-facing commands
   - Depends on all previous phases

---

## Testing Strategy

### Unit Tests
- Registry loading and validation
- Path resolution (absolute, tilde, relative)
- Prompt file loading and caching
- Configuration validation

### Integration Tests
- End-to-end CLI command execution
- XDG directory structure creation and loading
- Provider-agent runtime binding
- Frame generation with new architecture

### CLI Tests
- All command variations and flags
- Error handling and error messages
- Output formatting (text and JSON)
- Interactive command flows

---

## Success Criteria

The refactor is complete when:

1. ✅ Providers and agents are completely separated
2. ✅ Agents and providers stored in XDG directories
3. ✅ Agents use markdown prompt files
4. ✅ All CLI commands implemented and tested
5. ✅ Clear error messages and user guidance
6. ✅ Documentation updated
7. ✅ All existing tests pass
8. ✅ New tests cover all functionality

---

## Related Documentation

- **[README.md](README.md)** - Context management overview
- **[provider/provider_agent_separation.md](provider/provider_agent_separation.md)** - Separation design
- **[provider/provider_management_requirements.md](provider/provider_management_requirements.md)** - Provider requirements
- **[agents/agent_management_requirements.md](agents/agent_management_requirements.md)** - Agent requirements
- **[context_generate_command.md](context_generate_command.md)** - Context generate spec
- **[context_get_command.md](context_get_command.md)** - Context get spec
- **[agents/agent_cli_spec.md](agents/agent_cli_spec.md)** - Agent CLI spec
- **[provider/provider_cli_spec.md](provider/provider_cli_spec.md)** - Provider CLI spec

---

[← Back to Context Management](README.md)

