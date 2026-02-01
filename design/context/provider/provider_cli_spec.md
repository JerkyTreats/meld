# Provider Management CLI Specification

## Overview

This document specifies the CLI commands for managing LLM provider configurations. Providers handle API endpoints, authentication, model selection, and default completion parameters, independent from agents.

## Command Suite

The provider management commands are organized under `merkle provider`:

```
merkle provider <subcommand> [options]
```

## Commands

### 1) List Providers

Display all available providers from XDG directory.

**Syntax**

```
merkle provider list [options]
```

**Options**
- `--format <text|json>`: Output format (default: text)
- `--type <openai|anthropic|ollama|local>`: Filter by provider type

**Behavior**
1. Load providers from XDG directory (`$XDG_CONFIG_HOME/merkle/providers/`)
2. Apply filters if specified
3. Display provider list with key information

**Output (Text Format)**
```
Available Providers:
  openai-gpt4         openai    gpt-4              https://api.openai.com/v1
  openai-gpt35       openai    gpt-3.5-turbo      https://api.openai.com/v1
  anthropic-claude   anthropic claude-3-opus      (default endpoint)
  local-ollama       ollama    llama2             http://localhost:11434

Total: 4 providers
```

**Output (JSON Format)**
```json
{
  "providers": [
    {
      "provider_name": "openai-gpt4",
      "provider_type": "openai",
      "model": "gpt-4",
      "endpoint": "https://api.openai.com/v1"
    }
  ],
  "total": 4
}
```

### 2) Show Provider

Display detailed information about a specific provider.

**Syntax**

```
merkle provider show <provider_name> [options]
```

**Options**
- `--format <text|json>`: Output format (default: text)
- `--include-credentials`: Show API key status (not actual key)

**Behavior**
1. Search for provider in XDG directory
2. Load and display provider configuration
3. Optionally show API key status

**Output (Text Format)**
```
Provider: openai-gpt4
Type: openai
Model: gpt-4
Endpoint: https://api.openai.com/v1
API Key: Set (from environment)  # or "Set (from config)" or "Not set"

Default Completion Options:
  temperature: 0.7
  max_tokens: 2000
  top_p: 0.9
```

**Output (JSON Format)**
```json
{
  "provider_name": "openai-gpt4",
  "provider_type": "openai",
  "model": "gpt-4",
  "endpoint": "https://api.openai.com/v1",
  "api_key_status": "set_from_env",
  "default_options": {
    "temperature": 0.7,
    "max_tokens": 2000,
    "top_p": 0.9
  }
}
```

### 3) Validate Provider

Validate provider configuration and test connectivity.

**Syntax**

```
merkle provider validate <provider_name> [options]
```

**Options**
- `--test-connectivity`: Test provider API connectivity
- `--check-model`: Verify model is available
- `--verbose`: Show detailed validation results

**Behavior**
1. Load provider configuration
2. Validate required fields
3. Check API key availability (config or env)
4. Validate endpoint URL format
5. Optionally test API connectivity
6. Optionally verify model availability
7. Report all validation errors

**Output**
```
Validating provider: openai-gpt4

✓ Provider name matches filename
✓ Provider type is valid (openai)
✓ Model is not empty
✓ API key available (from environment)
✓ Endpoint URL is valid
✓ API connectivity: OK
✓ Model 'gpt-4' is available

Validation passed: 7/7 checks
```

**Error Output**
```
Validating provider: invalid-provider

✗ Provider name doesn't match filename
✗ API key not found (set OPENAI_API_KEY or add to config)
✗ Endpoint URL invalid: not-a-url
✗ API connectivity failed: Connection refused

Validation failed: 4 errors found
```

### 4) Test Provider

Test provider connectivity and model availability.

**Syntax**

```
merkle provider test <provider_name> [options]
```

**Options**
- `--model <model_name>`: Test specific model (overrides config)
- `--timeout <seconds>`: Connection timeout (default: 10)

**Behavior**
1. Load provider configuration
2. Create provider client
3. Test API connectivity
4. List available models
5. Verify configured model is available
6. Optionally run test completion

**Output**
```
Testing provider: openai-gpt4

✓ Provider client created
✓ API connectivity: OK (200ms)
✓ Model 'gpt-4' is available
✓ Test completion: OK

Provider is working correctly.
```

### 5) Create Provider

Interactively create a new provider configuration.

**Syntax**

```
merkle provider create <provider_name> [options]
```

**Options**
- `--type <openai|anthropic|ollama|local>`: Set provider type (required)
- `--model <model_name>`: Set model (required)
- `--endpoint <url>`: Set endpoint URL
- `--api-key <key>`: Set API key (or use env var)
- `--interactive`: Interactive mode (default)
- `--non-interactive`: Non-interactive mode (use flags)

**Behavior (Interactive Mode)**
1. Prompt for provider type
2. Prompt for model name
3. Prompt for endpoint (if applicable)
4. Prompt for API key (or suggest env var)
5. Prompt for default completion options
6. Create provider config file in XDG directory
7. Validate configuration
8. Display created provider

**Behavior (Non-Interactive Mode)**
1. Use provided flags
2. Validate required fields
3. Create provider config file
4. Display created provider

**Example (Interactive)**
```
$ merkle provider create my-openai

Provider type (openai/anthropic/ollama/local): openai
Model name: gpt-4
Endpoint URL (optional, default: https://api.openai.com/v1): 
API key (optional, will use OPENAI_API_KEY env var if not set): 
Default temperature (0.0-2.0, default: 1.0): 0.7
Default max tokens (optional): 2000

Creating provider configuration...
✓ Provider created: ~/.config/merkle/providers/my-openai.toml
```

### 6) Edit Provider

Edit an existing provider configuration.

**Syntax**

```
merkle provider edit <provider_name> [options]
```

**Options**
- `--model <model_name>`: Update model
- `--endpoint <url>`: Update endpoint
- `--api-key <key>`: Update API key
- `--editor <editor>`: Use specific editor (default: $EDITOR)

**Behavior**
1. Load existing provider configuration
2. Open in editor (or use flags for specific fields)
3. Validate updated configuration
4. Save changes
5. Display updated provider

**Note**: If using editor, system will:
- Create temporary file with current config
- Open in user's default editor
- Validate and save on exit
- Clean up temporary file

### 7) Remove Provider

Remove a provider configuration (XDG providers only).

**Syntax**

```
merkle provider remove <provider_name> [options]
```

**Options**
- `--force`: Skip confirmation prompt

**Behavior**
1. Verify provider exists in XDG directory
2. Check if provider is in use (warn if so)
3. Confirm removal (unless --force)
4. Remove provider config file
5. Display confirmation

**Output**
```
Removed provider: openai-gpt4
Configuration file deleted: ~/.config/merkle/providers/openai-gpt4.toml
```

**Warning (if in use)**
```
Warning: Provider 'openai-gpt4' may be in use by agents.
Are you sure you want to remove it? (y/N)
```

## Common Options

All commands support:
- `--config <path>`: Override config file location
- `--log-level <level>`: Set log level
- `--help`: Show command help

## Error Handling

### Provider Not Found
```
Error: Provider 'nonexistent' not found

Available providers:
  - openai-gpt4
  - anthropic-claude
  - local-ollama

Use 'merkle provider list' to see all providers.
```

### API Key Not Found
```
Error: API key not found for provider 'openai-gpt4'

Set the API key using one of:
  1. Environment variable: export OPENAI_API_KEY=sk-...
  2. Config file: merkle provider edit openai-gpt4 --api-key sk-...
```

### Invalid Configuration
```
Error: Invalid provider configuration

Issues found:
  - Provider name 'mismatch' doesn't match filename 'openai-gpt4.toml'
  - Provider type 'invalid' is not valid (must be openai, anthropic, ollama, or local)
  - Model name cannot be empty
  - Endpoint URL invalid: not-a-url

Fix these issues and try again.
```

### Connectivity Failure
```
Error: Provider connectivity test failed

Provider: openai-gpt4
Endpoint: https://api.openai.com/v1
Error: Connection refused

Check:
  - Network connectivity
  - Endpoint URL is correct
  - API key is valid
  - Firewall/proxy settings
```

## Implementation Notes

### Provider Discovery

1. **XDG Providers**: Load from `$XDG_CONFIG_HOME/merkle/providers/*.toml`

### API Key Resolution

1. API key in provider config file
2. Environment variable (provider-specific)
3. Error if required and not found

### Validation

- Provider name must match filename (without `.toml`)
- Provider type must be valid enum value
- Model must not be empty
- Endpoint must be valid URL (if provided)
- API key must be available (for cloud providers)
- Completion options must be in valid ranges

## Examples

### List All Providers
```bash
merkle provider list
```

### List Only OpenAI Providers
```bash
merkle provider list --type openai
```

### Show Provider Details
```bash
merkle provider show openai-gpt4 --include-credentials
```

### Validate Provider
```bash
merkle provider validate openai-gpt4 --test-connectivity
```

### Test Provider
```bash
merkle provider test openai-gpt4
```

### Create New Provider
```bash
merkle provider create my-openai \
  --type openai \
  --model gpt-4 \
  --endpoint https://api.openai.com/v1
```

### Edit Provider Model
```bash
merkle provider edit openai-gpt4 --model gpt-4-turbo
```

### Remove Provider
```bash
merkle provider remove old-provider
```

## Related Documentation

- [Provider-Agent Separation](provider_agent_separation.md) - Separation design
- [Provider Management Requirements](provider_management_requirements.md) - Overall requirements
- [Phase 2 Model Providers](../workflow/phase2_model_providers.md) - Provider implementation

---

[← Back to Context Management](../README.md)

