# TUI session state specification

## 1. Purpose

The TUI keeps a small amount of UI local session state to make interactive use faster.

This state is not part of the `meld` engine. It exists only inside the TUI layer.

Examples:

- loaded agent
- loaded provider
- selected session
- current view
- workspace scanned status
- current content filters

## 2. Principles

- UI local state does not replace command arguments
- UI local state only provides defaults and remembered selections
- business logic still lives in existing domains
- state should start in memory only

## 3. State model

The first version can use a single shared state struct.

```rust
pub struct TuiState {
    pub workspace_root: PathBuf,
    pub workspace_scanned: bool,
    pub active_view: ViewId,
    pub selected_session_id: Option<String>,
    pub loaded_agent_id: Option<String>,
    pub loaded_provider_name: Option<String>,
    pub preferred_frame_type: Option<String>,
    pub output_overlay: Option<OutputOverlayState>,
    pub command_bar: CommandBarState,
}
```

The exact types may vary, but the boundaries should stay the same.

## 4. Loaded agent and provider

Loaded agent and loaded provider are convenience defaults.

They influence:

- default values for `context generate`
- default filters in content view
- labels shown in the HUD

They do not:

- mutate stored agent or provider config
- bypass command validation
- change existing command semantics

Resolution rules:

- if the user explicitly sets `--agent` or `--provider` in a command, that explicit value wins
- otherwise the TUI uses loaded values when present
- if no loaded value exists, the TUI leaves the field unset and the existing command route resolves defaults as usual

## 5. Workspace scanned status

The TUI needs a lightweight view of whether the workspace has been scanned.

This should be derived from the existing workspace status path rather than tracked as a second source of truth.

When the workspace is not scanned, the TUI should:

- show a clear banner or init view
- offer a one key path to run `scan`
- block content view actions that require tree data

## 6. HUD

The HUD is a persistent status strip that shows the current interactive context.

Suggested fields:

- current view name
- selected session id, when present
- loaded agent, when present
- loaded provider, when present
- workspace status such as scanned or not scanned

The HUD is informational. It should not be the only place a critical warning appears.

## 7. Picker overlays

The first overlays should be:

- agent picker
- provider picker
- help overlay

Agent picker behavior:

- list agents from the current registry
- allow filter by typing
- set loaded agent on confirm

Provider picker behavior:

- list providers from the current registry
- allow filter by typing
- set loaded provider on confirm

These overlays read from the current registries through the same context already used by the CLI.

## 8. Command defaults

When the user triggers a common action from a key binding, the TUI may prefill a command.

Examples:

```text
:context generate src --agent writer
:context get src/lib.rs --agent writer
```

Prefill rules:

- use loaded agent if present
- use loaded provider if present and the target command accepts it
- use the currently selected node path if the action comes from content view

The command bar still shows the final text before execution so the user can edit it.

## 9. Persistence

Phase 1 should keep this state in memory only.

Reasons:

- lower implementation cost
- no early config migration burden
- fewer surprises during first rollout

Optional later work may persist a few non critical values in user scoped config, such as last loaded agent or last selected view.

That persistence should live outside the workspace and should not become required for TUI startup.

## 10. File layout

Suggested files:

- `src/tui/state.rs` for shared state types
- `src/tui/app.rs` for lifecycle and state transitions
- `src/tui/overlay.rs` for overlay traits and shared behavior
- `src/tui/overlay/agent.rs` for agent picker
- `src/tui/overlay/provider.rs` for provider picker

## 11. Required tests

- explicit command args win over loaded defaults
- loaded agent prefill works for generate commands
- loaded provider prefill works when the command accepts a provider
- not scanned workspaces show the init banner
- picker confirm updates state
- picker cancel leaves state unchanged

## 12. Open choices

- whether replay mode should preserve a distinct selected session separate from live follow mode
- whether the HUD should show frame type directly or derive it from loaded agent
- whether a later release should persist a small user local session state snapshot
