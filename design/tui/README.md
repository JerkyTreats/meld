# TUI design

Design for `meld tui`, the interactive terminal interface planned for the current `meld` codebase.

## Status

- The current codebase already has the key engine hooks the TUI needs: `RunContext` for command execution, `ProgressRuntime` for session lifecycle, and `ProgressStore` for durable session and event history.
- The current codebase does not yet have a `tui` feature, TUI dependencies, or a `tui` CLI subcommand.
- These docs replace the older framing with the current `meld` names, module boundaries, and telemetry model.

## Documents

- `tui_spec.md` — Product spec and architecture for `meld tui`, including launch flow, views, command execution, data flow, and implementation phases.
- `tui_session_state_spec.md` — UI local session state for loaded agent, loaded provider, workspace status, overlays, HUD behavior, and default resolution.
- `tree_browser_spec.md` — Reusable tree browser for filesystem and meld node navigation, aligned to the current `NodeRecordStore` and head index model.

## Design rules

- The TUI is a thin adapter over existing domains.
- The TUI does not own business logic for scan, generation, workspace mutation, or provider calls.
- The TUI executes commands through `RunContext` and reads history through telemetry storage.
- The TUI may keep UI local state such as focus, selected session, loaded agent, and loaded provider.
- New TUI code should follow the modern Rust module layout and avoid `mod.rs`.

## Current code anchors

- CLI route and command execution live in `src/cli/route.rs`.
- Durable telemetry storage lives in `src/telemetry/sessions/service.rs` and `src/telemetry/sinks/store.rs`.
- Workspace summary and coverage data live in `src/workspace/section.rs`.
- Context retrieval APIs live in `src/api.rs`.

## Suggested implementation order

- Start with dashboard, session view, and session history.
- Add command bar execution on top of `RunContext`.
- Add content browser with the reusable tree browser.
- Add optional replay and richer markdown rendering after the first end to end flow works.
