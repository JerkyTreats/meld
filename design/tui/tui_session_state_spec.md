# TUI session state specification

## 1. Purpose

The TUI maintains in-memory session state for the currently "loaded" agent, provider, and workspace. This state streamlines interactive workflows -- the user selects an agent and provider once, and all subsequent operations (content browsing, generation, review) use them by default. The core merkle library is unchanged; it has no concept of loaded agent/provider state. This is purely a TUI concern.

## 2. Session state

The TUI holds a `TuiSessionState` struct in memory:

- active_agent: Option of AgentIdentity -- the currently loaded agent
- active_provider: Option of ProviderConfig -- the currently loaded provider
- workspace_root: PathBuf -- the current working directory (may not be initialized)
- workspace_initialized: bool -- whether `merkle scan` has been run for this root

This state is not persisted across TUI restarts. Each launch starts with nothing loaded. The TUI reads the agent and provider registries from the existing XDG config paths to populate selection lists.

## 3. HUD (Heads-Up Display)

A persistent bar displayed at the top of every view. Compact single line.

**Layout when loaded:**

```
Agent: code-reader | Provider: anthropic-claude | Workspace: ~/projects/myapp (scanned, 342 nodes)
```

**Layout when partially loaded:**

```
Agent: code-reader | Provider: press `p` to load | Workspace: ~/projects/myapp (scanned, 342 nodes)
```

**Layout when nothing loaded:**

```
Agent: press `a` to load | Provider: press `p` to load | Workspace: press `w` to change
```

**Layout when uninitialized workspace:**

```
Agent: -- | Provider: -- | Workspace: ~/projects/myapp (not scanned, press `s` to scan)
```

**HUD rules:**
- Agent and provider show as dimmed placeholder text with hotkey hint when unloaded
- Workspace always shows the path. Parenthetical shows scan status and node count when initialized, or "not scanned" when uninitialized
- When an agent is loaded, the HUD shows its `agent_id`. When a provider is loaded, shows `provider_name`
- The HUD occupies one terminal row. Truncate the workspace path with ellipsis if the line exceeds terminal width

## 4. Agent loading

**Trigger:** Press `a` from any view.

**Behavior:**
1. Open an overlay list of all agents from `AgentRegistry::list_all()`
2. Each row shows: agent_id, role (Reader/Writer/Synthesis), capabilities summary
3. Navigate with j/k or arrow keys. Press Enter to select. Press Escape to cancel.
4. On selection, set `active_agent` in session state. HUD updates immediately.
5. If the content browser is open, refresh the displayed content to show frames for the newly loaded agent.

**Filter:** The list shows all agents, not just Writers. The user may want to browse context from a Reader agent. Generation commands will validate that the loaded agent has the Writer or Synthesis role before proceeding.

**Quick switch:** If an agent is already loaded, pressing `a` opens the list with the current agent highlighted. Selecting a different agent replaces the loaded one.

## 5. Provider loading

**Trigger:** Press `p` from any view.

**Behavior:**
1. Open an overlay list of all providers from `ProviderRegistry::list_all()`
2. Each row shows: provider_name, provider_type (OpenAI/Anthropic/Ollama/LocalCustom), model
3. Navigate with j/k or arrow keys. Press Enter to select. Press Escape to cancel.
4. On selection, set `active_provider` in session state. HUD updates immediately.

**Validation:** Provider health (is the endpoint reachable, is the API key valid) is not checked on load. Errors surface when a generation command actually calls the provider. The TUI does not add startup latency for provider validation.

## 6. Content browser integration

The content browser (tui_spec.md section 4.3) changes behavior based on loaded agent:

**Agent loaded:**
- Tree navigator shows context coverage for the loaded agent's frame type (`context-{agent_id}`)
- Selecting a node displays the head frame for that agent's frame type
- Frame metadata header shows the loaded agent's ID
- Nodes without context for this agent are dimmed

**No agent loaded:**
- Selecting a node opens a frame picker: a list of all frame types that have heads for this node
- Each row shows: frame_type, agent_id, generation timestamp
- Selecting a frame type displays that frame's content
- Tree navigator shows aggregate coverage (any frame type, any agent)

**Switching agents while browsing:** If the user presses `a` and loads a different agent, the content viewer refreshes to show the new agent's frame for the currently selected node. If the new agent has no frame for that node, show a placeholder ("No context for agent X. Press `g` to generate.").

## 7. Generation integration

When a generate command is issued from the command bar or via the `g`/`f` hotkeys in the content browser:

**Agent resolution:**
1. If `active_agent` is loaded, use it. Skip the `--agent` flag.
2. If no agent loaded, check how many Writer agents exist:
   - One Writer: auto-select it (matches CLI behavior) and set it as `active_agent`
   - Multiple Writers: open the agent picker overlay. After selection, set as `active_agent` and proceed.
   - Zero Writers: show error in output panel

**Provider resolution:**
1. If `active_provider` is loaded, use it. Skip the `--provider` flag.
2. If no provider loaded, open the provider picker overlay. After selection, set as `active_provider` and proceed.

**Command bar override:** If the user explicitly types `:context generate ./src --agent foo --provider bar`, those values override the loaded state for that command only. The session state is not changed.

**Role validation:** If the loaded agent does not have Writer or Synthesis role, show an error: "Agent X is a Reader. Generation requires a Writer or Synthesis agent. Press `a` to load a different agent."

## 8. Uninitialized workspace

When the TUI launches, it checks workspace status via the same logic as `build_workspace_status()` -- compute root hash, check if root exists in store.

**If not initialized (scan has not run):**

- All four views (dashboard, session, content browser, history) are replaced by a single **initialization view**
- The initialization view shows:
  - Current directory path (large, prominent)
  - Message: "This workspace has not been scanned. Scan to initialize."
  - Action: Press `s` to run `merkle scan` immediately
  - Action: Press `w` to change the working directory first

**After scan completes:**
- The TUI transitions to the dashboard view
- Workspace status updates in the HUD
- All views become available

**Re-entering uninitialized state:** This should not happen during normal use (scanning is additive). However, if the store is deleted externally, the TUI detects it on the next workspace status check and returns to the initialization view.

## 9. Workspace directory selection

**Trigger:** Press `w` from any view (or from the initialization view).

**Behavior:** Opens a directory browser overlay for selecting a new workspace root.

**Directory browser design:**

- **Tree panel:** Collapsible directory tree starting from the filesystem root or user home. Directories only, no files. Expand/collapse with Enter or right/left arrow.
- **Path input:** An editable text field at the top showing the current selection as an absolute path. The user can type directly to jump to a known path. Tab completion for path segments.
- **Confirm:** Press Enter when the path input is focused, or press Enter on a highlighted directory in the tree.
- **Cancel:** Escape returns to the previous view without changing the workspace.

**After selection:**
1. Set `workspace_root` to the new path
2. Run workspace status check (is it scanned?)
3. If scanned: update HUD, refresh all views for the new workspace
4. If not scanned: transition to the initialization view
5. Clear `active_agent` and `active_provider` -- they may not be relevant to the new workspace

**Design nuance -- directory browser UX:** Making directory selection feel responsive requires careful handling:
- The tree should lazy-load children (only read directory entries when a node is expanded, not on startup)
- Symlinks should be followed but cycles guarded against (visited set)
- Permission errors (unreadable directories) should show the directory dimmed, not crash
- The path input and tree should stay synchronized: typing a valid path scrolls the tree to it, selecting in the tree updates the path input
- Very deep trees should have a max visible depth or a "go to parent" shortcut
- **This component is complex enough to warrant its own implementation spike. Consider whether an existing crate (e.g. `tui-file-dialog` or similar) meets the need before building from scratch.**

## 10. Key bindings (additions to tui_spec.md)

### Global (all views)
- `a` -- open agent picker overlay
- `p` -- open provider picker overlay
- `w` -- open workspace directory browser

### Initialization view
- `s` -- run `merkle scan`
- `w` -- open workspace directory browser

### Overlay (agent picker, provider picker, directory browser)
- `j`/`k` or Up/Down -- navigate list
- Enter -- select
- Escape -- cancel
- `/` -- filter/search within list

## 11. Implementation location

### New files (all behind `#[cfg(feature = "tui")]`)

- `src/tui/session_state.rs` -- `TuiSessionState` struct, agent/provider load/unload, workspace status tracking
- `src/tui/hud.rs` -- HUD widget rendering, layout logic, truncation
- `src/tui/overlays/agent_picker.rs` -- agent list overlay, filtering, selection
- `src/tui/overlays/provider_picker.rs` -- provider list overlay, filtering, selection
- `src/tui/overlays/directory_browser.rs` -- directory tree + path input overlay
- `src/tui/overlays/mod.rs` -- shared overlay trait, focus management
- `src/tui/views/init.rs` -- initialization view (unscanned workspace)

### Updated files

- `src/tui/app.rs` -- hold `TuiSessionState`, wire HUD into layout, handle global hotkeys for a/p/w, manage initialization view gating
- `src/tui/views/dashboard.rs` -- read workspace status from session state, display loaded agent/provider in summary
- `src/tui/views/content.rs` -- use `active_agent` for frame type selection, fall back to frame picker when no agent loaded, refresh on agent switch
- `src/tui/views/mod.rs` -- add HUD to shared layout, add initialization view variant
- `src/tui/command_bar.rs` -- inject loaded agent/provider into command execution when not explicitly overridden
- `src/tui/keybindings.rs` -- add a/p/w/s bindings

### Engine code (no changes)

The core library (`AgentRegistry`, `ProviderRegistry`, `build_workspace_status`, generation logic) is not modified. The TUI calls existing public APIs:
- `AgentRegistry::load_from_xdg()` and `list_all()` to populate the picker
- `ProviderRegistry::load_from_xdg()` and `list_all()` to populate the picker
- `build_workspace_status()` to check initialization
- Generation functions receive agent_id and provider_name as parameters, same as CLI

## 12. Required tests

### Unit tests

**TuiSessionState:**
- Initial state has no agent, no provider, workspace_initialized false
- Loading an agent updates active_agent and fires a state change notification
- Loading a provider updates active_provider and fires a state change notification
- Changing workspace clears active_agent and active_provider
- Loading a new agent replaces the previous one (not additive)

**HUD rendering:**
- With agent + provider + initialized workspace: renders full info line
- With no agent: renders placeholder with hotkey hint
- With no provider: renders placeholder with hotkey hint
- With uninitialized workspace: renders "not scanned" with hotkey hint
- Long workspace path is truncated with ellipsis when terminal width is narrow

**Agent picker:**
- Lists all agents from registry (Reader, Writer, Synthesis)
- Filter by typing narrows the list (e.g. typing "code" shows only agents with "code" in ID)
- Selecting an agent returns its AgentIdentity
- Cancelling returns None
- Currently loaded agent is highlighted in the list

**Provider picker:**
- Lists all providers from registry
- Filter by typing narrows the list
- Selecting a provider returns its ProviderConfig
- Cancelling returns None

**Generation with session state:**
- With agent and provider loaded: generate command uses them, does not prompt
- With agent loaded but no provider: provider picker opens before generation proceeds
- With no agent loaded and one Writer in registry: auto-selects and loads it
- With no agent loaded and multiple Writers: agent picker opens
- Explicit --agent/--provider in command bar overrides loaded state without changing it

**Content browser with session state:**
- Agent loaded: selecting a node shows that agent's frame
- Agent loaded, node has no frame for that agent: shows placeholder with generate hint
- No agent loaded: selecting a node shows frame picker (all available frame types)
- Switching agents refreshes the content viewer for the currently selected node

### Integration tests (headless, no terminal)

**Workspace initialization flow:**
- Launch TUI against unscanned directory: initialization view is active, other views are gated
- Run scan from initialization view: workspace transitions to initialized, dashboard becomes active
- Workspace status check detects scanned workspace: initialization view is skipped

**Directory browser:**
- Opening browser shows current directory expanded
- Navigating to a child directory and confirming changes workspace_root
- Typing a valid path in the input jumps the tree to that location
- Selecting an unscanned directory triggers initialization view
- Cancelling preserves the original workspace

## 13. Design nuances requiring further work

### Directory browser complexity
The directory browser overlay (section 9) is fully specified in `tree_browser_spec.md`. It uses a generic `TreeDataProvider` trait backed by `tui-tree-widget` for rendering, with `FileSystemProvider` for workspace selection and `MerkleTreeProvider` for the content browser tree. See that spec for crate evaluation, architecture, lazy loading, symlink/cycle handling, and implementation details.

### Agent/provider config reload
If the user adds a new agent or provider config file while the TUI is running, the picker lists will be stale. Options:
- Refresh registries on each picker open (simple, slight latency on large configs)
- Watch the XDG config directories for changes (reactive, more complex)
- Add a manual refresh hotkey in the picker

Recommendation: refresh on each picker open. Agent/provider configs are small; the overhead is negligible.

### Frame type mapping
The loaded agent implies a frame type of `context-{agent_id}`. But agents may produce frames with custom frame types. The current spec assumes the default mapping. If custom frame types become common, the content browser would need an additional selector or the session state would need to track frame_type separately from agent.

### Multi-workspace
The current design supports one workspace at a time. If users want to compare context across workspaces, they would need to switch via `w`. A split-view or tab-per-workspace model is out of scope but noted for future consideration.

## 14. Related docs

- **tui_spec.md** -- The parent TUI spec. This document extends sections 4.3 (content browser), 5 (command bar), and 6 (key bindings).
- **tree_browser_spec.md** -- The reusable tree browser component used for workspace directory selection and content browser navigation.
- **design/context/context_generate_by_path_spec.md** -- The generate command that the TUI invokes with loaded agent/provider.
- **design/context/generation_orchestrator_spec.md** -- The orchestrator that processes generation requests submitted by the TUI.
