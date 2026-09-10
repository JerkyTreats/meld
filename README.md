# meld

Meld reconciles installed intentions against owner-observed knowledge. Agent judges what remains to be done, Strategy constructs the work, and Execution and Curation return separate operational and semantic evidence.

Docs and Security require separately selected [external owner packages](owners/README.md).

The native runtime supports [Startup](theory/startup/README.md), [README maintenance](theory/docs_freshness/README.md), [declared Security mitigation](theory/dependency_security_mitigation/README.md) and [declared code changes](theory/code_change/README.md). Each guide explains configuration, inert preparation and activation through `meld runtime run`. The Startup guide provides a small end-to-end example with no workspace or model provider.

Legacy Workflow execution is retired. Existing Workflow profiles remain inspectable; use the native product guides for executable reconciliation. The context commands below remain available for Merkle-tree and stored-frame tooling.

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
# Prepare bundled Startup without a workspace or provider
meld init

# Run the native flywheel; Ctrl-C requests drain
meld runtime run

# Inspect progress from another terminal, or retained evidence after stopping
meld runtime startup-account --agent-id startup-agent
```

`meld init --json` reports native preparation receipts. Repeating it preserves
prepared identities. Incompatible older configuration remains untouched; the
error explains how to create a separate configuration with `--config`.
Initialization no longer creates Reader/Writer profiles or prompt templates;
`init --force` and `init --list` are retired. The context/profile commands below
remain separate legacy tooling.

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

Canonical declarative design intent lives in [Cognitive Architecture](design/cognitive_architecture/README.md). That design directory defines the intended domains, cross domain contracts, event spine, world model, execution loop, and crate ownership. The overview below describes the current user-facing context flow.

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

Local Cargo builds and checks are routine. Publication is separate: the CI
release job requires a manual dispatch on `master` with `publish_release`
explicitly enabled. Ordinary pushes and verification dispatches do not authorize
release tags, artifacts or crates.io publication. Release dispatch still requires
explicit owner approval.


For external command feedback, use the [Meld Eval first-use probe](../meld-eval/README.md#probe-first-use) with an existing compiled binary. It records the artifact identity and public command results in isolated directories without building or opening domain stores.

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
