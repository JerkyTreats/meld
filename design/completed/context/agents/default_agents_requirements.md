# Default Agents Requirements

## Overview

This document specifies the requirements for default agents that are initialized by the `merkle init` command. These agents provide a complete, working set of agents covering all roles (Reader, Writer, Synthesis) that users can immediately use or customize.

## Goals

1. **Complete Coverage**: Provide agents for all three roles (Reader, Writer, Synthesis)
2. **Immediate Usability**: Agents should work out-of-the-box with minimal configuration
3. **Clear Purpose**: Each agent has a well-defined, distinct purpose
4. **Customizable**: Users can easily modify or extend default agents
5. **Best Practices**: Default agents demonstrate best practices for agent configuration

## Required Default Agents

### 1. Reader Agent

**Agent ID**: `reader`  
**Role**: `Reader`  
**Purpose**: Read-only agent for querying and retrieving context frames

**Characteristics**:
- No system prompt required (Reader agents don't generate frames)
- No provider required (Reader agents don't call LLMs)
- Simplest agent configuration
- Used for context retrieval operations

**Use Cases**:
- Query existing context frames
- Retrieve historical context
- Read-only context operations

**Configuration**:
```toml
agent_id = "reader"
role = "Reader"
# No system_prompt_path (Reader agents don't generate frames)
# No metadata required
```

**Rationale**: Provides a minimal, working Reader agent that demonstrates the simplest agent configuration. Users can see how Reader agents differ from Writer/Synthesis agents.

---

### 2. Code Analyzer Agent

**Agent ID**: `code-analyzer`  
**Role**: `Writer`  
**Purpose**: Analyze code files and generate comprehensive analysis frames

**Characteristics**:
- Focuses on code structure, patterns, and relationships
- Provides detailed technical analysis
- Suitable for code review and understanding
- Works with any codebase

**Use Cases**:
- Analyze individual code files
- Understand code structure and organization
- Identify patterns and conventions
- Code review and documentation

**System Prompt Focus**:
- Code analysis and understanding
- Structure and organization
- Patterns and conventions
- Technical accuracy

**User Prompt Templates**:
- **File**: Analyze code file with focus on structure, functions, dependencies, patterns
- **Directory**: Analyze directory structure with focus on organization, module relationships, architecture

**Configuration**:
```toml
agent_id = "code-analyzer"
role = "Writer"
system_prompt_path = "prompts/code-analyzer.md"

[metadata]
user_prompt_file = "Analyze the code file at {path}. Provide a comprehensive analysis including:\n- Code structure and organization\n- Key functions and their purposes\n- Dependencies and relationships\n- Notable patterns or conventions\n- Potential issues or improvements"
user_prompt_directory = "Analyze the directory structure at {path}. Provide an overview including:\n- Directory purpose and organization\n- Key files and their roles\n- Module relationships\n- Overall architecture patterns"
```

**Rationale**: Code analysis is a fundamental use case for context generation. This agent provides a solid foundation that users can customize for specific languages or analysis styles.

---

### 3. Documentation Writer Agent

**Agent ID**: `docs-writer`  
**Role**: `Writer`  
**Purpose**: Generate clear, comprehensive documentation for code and APIs

**Characteristics**:
- Focuses on clarity and completeness
- Generates user-friendly documentation
- Includes examples and usage patterns
- Suitable for API documentation and guides

**Use Cases**:
- Generate API documentation
- Create code documentation
- Write usage guides
- Document patterns and conventions

**System Prompt Focus**:
- Documentation clarity and completeness
- User-friendly explanations
- Examples and usage patterns
- API coverage

**User Prompt Templates**:
- **File**: Generate documentation with purpose, API docs, examples, notes
- **Directory**: Generate directory documentation with structure, components, usage guidelines

**Configuration**:
```toml
agent_id = "docs-writer"
role = "Writer"
system_prompt_path = "prompts/docs-writer.md"

[metadata]
user_prompt_file = "Generate comprehensive documentation for the code file at {path}. Include:\n- Purpose and overview\n- API documentation\n- Usage examples\n- Important notes and warnings\n- Related components"
user_prompt_directory = "Generate documentation for the directory at {path}. Include:\n- Directory purpose and structure\n- Module overview\n- Key components and their roles\n- Usage guidelines"
```

**Rationale**: Documentation generation is a common use case. This agent demonstrates how to structure prompts for documentation-focused tasks and provides a template users can customize.

---

### 4. Synthesis Agent

**Agent ID**: `synthesis-agent`  
**Role**: `Synthesis`  
**Purpose**: Combine context frames from child nodes into coherent branch-level summaries

**Characteristics**:
- Specialized for directory/branch synthesis
- Combines multiple context frames
- Preserves key information from child contexts
- Identifies patterns and relationships

**Use Cases**:
- Synthesize directory-level context
- Combine multiple file contexts
- Generate branch summaries
- Create high-level overviews

**System Prompt Focus**:
- Information synthesis and combination
- Pattern identification
- Relationship mapping
- Summary generation

**User Prompt Templates**:
- **Directory**: Synthesize child contexts into coherent summary with themes, patterns, relationships

**Configuration**:
```toml
agent_id = "synthesis-agent"
role = "Synthesis"
system_prompt_path = "prompts/synthesis-agent.md"

[metadata]
user_prompt_directory = "Synthesize context frames from child nodes in the directory at {path}. Combine the information into a coherent summary that:\n- Preserves key information from child contexts\n- Identifies common themes and patterns\n- Highlights important relationships\n- Maintains accuracy and completeness"
```

**Rationale**: Synthesis is a specialized operation that requires different prompt structure than file-level analysis. This agent demonstrates Synthesis role usage and provides a working example.

---

## Prompt File Requirements

### Prompt File Location

All default agent prompts are stored in:

**Build-Time**: `prompts/` directory in source repository  
**Runtime**: `$XDG_CONFIG_HOME/merkle/prompts/` (after initialization)

### Prompt File Naming

Prompt files follow the naming convention: `<agent-id>.md`

Examples:
- `code-analyzer.md`
- `docs-writer.md`
- `synthesis-agent.md`

### Prompt File Structure

All prompt files should follow this structure:

```markdown
# Agent Name

Brief one-sentence description of the agent's purpose.

## Your Role

Detailed description of what the agent does, its responsibilities, and its primary function.

## Guidelines

Specific guidelines for the agent's behavior:
- Guideline 1: Description
- Guideline 2: Description
- Guideline 3: Description

## Output Format

Expected output format and structure (if applicable).

## Examples

Example scenarios or use cases (optional).
```

### Prompt File Content Requirements

1. **Clear Purpose**: Prompt should clearly define the agent's role
2. **Specific Guidelines**: Include specific behavioral guidelines
3. **Output Expectations**: Define expected output format
4. **Best Practices**: Follow prompt engineering best practices
5. **Language**: Use clear, professional language
6. **Length**: Prompts should be comprehensive but concise (typically 200-500 words)

## Agent Configuration Requirements

### Required Fields

All agent configurations must include:
- `agent_id`: Unique identifier matching filename
- `role`: Valid AgentRole enum value (Reader, Writer, Synthesis)

### Optional Fields

Writer and Synthesis agents should include:
- `system_prompt_path`: Path to markdown prompt file
- `metadata.user_prompt_file`: Template for file-level operations
- `metadata.user_prompt_directory`: Template for directory-level operations

### Path Resolution

- `system_prompt_path` uses relative paths (e.g., `prompts/code-analyzer.md`)
- Paths resolve relative to `$XDG_CONFIG_HOME/merkle/` per [Agent Management Requirements](agent_management_requirements.md)

## Validation Requirements

All default agents must:

1. **Pass Configuration Validation**:
   - Agent ID matches filename
   - Role is valid enum value
   - Required fields present

2. **Pass Prompt Validation** (Writer/Synthesis only):
   - Prompt file exists and is readable
   - Prompt file is valid UTF-8
   - Prompt file is not empty

3. **Pass Metadata Validation** (Writer/Synthesis only):
   - User prompt templates present in metadata
   - Templates include required placeholders (e.g., `{path}`)

## Customization Guidelines

Users should be able to:

1. **Modify Prompts**: Edit prompt files in `$XDG_CONFIG_HOME/merkle/prompts/`
2. **Modify Agents**: Edit agent configs in `$XDG_CONFIG_HOME/merkle/agents/`
3. **Create Variants**: Create new agents based on default agents
4. **Extend Functionality**: Add custom metadata and templates

**Important**: Default agents should serve as templates and examples, not rigid constraints.

## Non-Requirements

### Out of Scope

1. **Language-Specific Agents**: Default agents are language-agnostic
2. **Provider Configuration**: Agents are provider-agnostic (providers configured separately)
3. **Domain-Specific Agents**: Default agents are general-purpose
4. **Advanced Features**: Complex prompt templates or variable substitution (future)

## Success Criteria

Default agents meet requirements when:

1. ✅ All four agents (Reader, Code Analyzer, Docs Writer, Synthesis) are initialized
2. ✅ All agents pass validation after initialization
3. ✅ All prompt files are valid Markdown and readable
4. ✅ All agents can be used immediately with any provider
5. ✅ Users can customize agents without breaking functionality
6. ✅ Agent configurations demonstrate best practices

## Related Documentation

- [Initialization Command Specification](../init_command_spec.md) - How agents are initialized
- [Agent Management Requirements](agent_management_requirements.md) - Agent configuration requirements
- [Agent CLI Specification](agent_cli_spec.md) - Agent management commands
- [Provider-Agent Separation](../provider/provider_agent_separation.md) - Agent-provider relationship

---

[← Back to Context Management](../README.md)

