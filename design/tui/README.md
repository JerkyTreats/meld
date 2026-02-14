# TUI design

Design for `merkle tui`, the feature-gated interactive terminal interface.

## Documents

- **tui_spec.md** -- Full specification for the TUI: four views dashboard, session monitoring, content browser, session history, command bar for executing merkle commands, key bindings, event consumption from observability sled store, markdown rendering, and required tests. Feature-gated behind `--features tui` with ratatui and crossterm dependencies.

- **tui_session_state_spec.md** -- TUI-only session state: loaded agent, loaded provider, workspace initialization. HUD display, agent/provider picker overlays, directory browser, content browser integration, generation defaults, and uninitialized workspace flow.

- **tree_browser_spec.md** -- Reusable tree navigation component. Generic TreeDataProvider trait with FileSystemProvider and MerkleTreeProvider implementations. Tree panel + path input with tab completion and synchronization. Crate evaluation (tui-tree-widget recommended). Picker and browse modes.

## Related docs

- **design/observability/observability_spec.md** -- The event system and sled event store that the TUI consumes.
- **generation_orchestrator_spec.md** -- The orchestrator whose events drive the TUI's session view.
