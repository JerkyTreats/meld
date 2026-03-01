# meld

Deterministic context management for AI agents. Meld tracks your codebase state using Merkle trees and stores AI-generated context frames linked to each file and directory.

## Why meld?

AI agents need context about your code. Traditional approaches require expensive full-codebase scans or semantic search. Meld provides:

- **Instant invalidation** — Hash comparison detects changes in O(1)
- **Fast lookups** — Retrieve context for any file without scanning
- **Immutable history** — Context frames are append-only and verifiable
- **Multi-agent support** — Multiple agents can read/write context independently

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Initialize meld in your project
meld init

# Scan the filesystem to build the tree
meld scan

# Check workspace status
meld status

# Generate context for the codebase
meld context generate
```

## Core Commands

### Workspace

```bash
meld scan                    # Build/rebuild the Merkle tree
meld status                  # Show workspace, agent, and provider status
meld watch                   # Watch for changes (daemon mode)
meld workspace validate      # Validate workspace integrity
```

### Context

```bash
meld context generate              # Generate context for all files
meld context generate ./src        # Generate for specific path
meld context get <node-id>         # Retrieve context for a node
meld context regenerate            # Force regenerate (--force --no-recursive)
```

### Agents

Agents are LLM-powered workers that generate context frames.

```bash
meld agent list              # List configured agents
meld agent create            # Create a new agent interactively
meld agent show <id>         # Show agent details
meld agent validate <id>     # Validate agent configuration
```

### Providers

Providers are LLM backends (OpenAI, Anthropic, Ollama, etc.).

```bash
meld provider list           # List configured providers
meld provider create         # Create a new provider interactively
meld provider test <name>    # Test provider connectivity
```

## Configuration

Meld uses XDG directories:

| Purpose | Location |
|---------|----------|
| Config | `~/.config/meld/` |
| Agents | `~/.config/meld/agents/*.toml` |
| Providers | `~/.config/meld/providers/*.toml` |
| Prompts | `~/.config/meld/prompts/*.md` |
| Data | `~/.local/share/meld/workspaces/<hash>/` |
| Logs | Platform state directory, e.g. `$XDG_STATE_HOME/meld/` on Linux |

### Logging

Logging is on by default and writes to a file under the platform state directory (e.g. `$XDG_STATE_HOME/meld/.../meld.log` on Linux). Use `--quiet` to disable logging, or `--log-file <path>` / `MERKLE_LOG_FILE` to set the log file path. Configure level, format, and output in `[logging]` in your config file.

### Workspace config

Create `.meld/config.toml` in your project root:

```toml
[storage]
store_path = ".meld/store"
frames_path = ".meld/frames"

[logging]
enabled = true
level = "info"
format = "text"
output = "file"
```

## How It Works

### Merkle Tree

Meld builds a Merkle tree of your filesystem. Each file and directory gets a deterministic `NodeID` based on its content and path. When files change, only affected hashes update — enabling instant change detection.

### Context Frames

Context frames are immutable blobs of AI-generated information attached to nodes. Each frame has:

- **FrameID** — Content-addressed hash
- **Basis** — The NodeID it describes
- **Content** — The actual context (summaries, analysis, etc.)

Frames are append-only. New context creates new frames; history is preserved.

### Agents & Providers

- **Agent** — Defines the prompt and role (Reader or Writer)
- **Provider** — The LLM backend that executes the prompt

Writer agents generate context frames. Reader agents can query context but not write.

## Architecture

```
Filesystem
    ↓
Merkle Tree (deterministic hashing)
    ↓
NodeRecord Store (fast lookups)
    ↓
Context Frames (AI-generated, append-only)
    ↓
Frame Heads (latest frame per node)
    ↓
Context Views (bounded retrieval)
```

## Development

```bash
# Run tests
cargo test

# Build release
cargo build --release

# Run with verbose logging
meld --verbose scan
```

## License

MIT OR Apache-2.0
