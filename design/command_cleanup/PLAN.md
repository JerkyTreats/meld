# Command Cleanup and Status Implementation Plan

## Overview

This document outlines the phased implementation plan for the command cleanup restructure and status command suite. It covers: display stack and workspace status; agent and provider status; unified merkle status; and detailed workspace command specs (scan, validate, watch, node delete with tombstone semantics). All specs live in design/command_cleanup/; design/context/ is reference only for agent/provider concepts.

---

## Development phases

| Phase | Goal | Dependencies | Completion |
|-------|------|--------------|------------|
| 1 | Display stack and workspace status | None | Done |
| 2 | Agent status and provider status | Phase 1 | Done |
| 3 | Unified merkle status | Phase 1, Phase 2 | Done |
| 4 | Workspace command specs and scan / validate / watch | Phase 1–3 if status touches workspace | Done |
| 5 | Node delete (tombstone-based) | Node store, head index, ignore_list_spec | Done |
| 6 | Remove deprecated commands | Phase 2; Phases 1–5 complete | Not started |

---

### Phase 1 — Display stack and workspace status

| Field | Value |
|-------|--------|
| Goal | Introduce a consistent display stack (comfy-table, styling crate) and implement workspace status output per workspace_status_requirements.md. |
| Dependencies | None |
| Docs | workspace_status_requirements.md |
| Completion | Done |

| Task | Completion |
|------|------------|
| Add comfy-table and one styling crate (owo-colors or colored) to Cargo.toml; pin versions. | Done |
| Implement workspace status section: tree (scanned/not scanned, root hash, node count), optional breakdown, context coverage per agent, top paths by node count (max 5). | Done |
| Use comfy-table for all tables; use styling crate for section headings and optional status colors; respect NO_COLOR and TTY. | Done |
| Ensure merkle workspace status (or equivalent) produces this section. | Done |

| Exit criterion | Completion |
|----------------|------------|
| comfy-table and styling crate in use for workspace status output. | Done |
| Workspace status shows tree state, node count, optional breakdown, and per-agent context coverage when scanned. | Done |
| Output matches workspace_status_requirements.md; text and JSON formats. | Done |

| Key change | Completion |
|------------|------------|
| New dependencies: comfy-table, owo-colors or colored. | Done |
| Status handler for workspace section; shared formatting helpers. | Done |

---

### Phase 2 — Agent status and provider status

| Field | Value |
|-------|--------|
| Goal | Implement merkle agent status and merkle provider status per their specs; reuse display stack. |
| Dependencies | Phase 1 (display stack) |
| Docs | agent_status_spec.md, provider_status_spec.md, provider_validate_spec.md |
| Completion | Done |

| Task | Completion |
|------|------------|
| Remove top-level validate-providers; provider validation is merkle provider validate per provider_validate_spec.md. | Done |
| Implement merkle agent status: table (Agent, Role, Valid, Prompt); data from AgentRegistry::list_all() and validate_agent(); text and JSON. | Done |
| Implement merkle provider status: table (Provider, Type, Model, optional Connectivity); optional --test-connectivity; text and JSON. | Done |
| Reuse comfy-table and styling crate for headings and tables. | Done |

| Exit criterion | Completion |
|----------------|------------|
| merkle agent status and merkle provider status produce output matching agent_status_spec.md and provider_status_spec.md. | Done |
| Empty lists do not fail; validation and connectivity reused from existing code. | Done |

| Key change | Completion |
|------------|------------|
| New CLI variants: Agent::Status, Provider::Status. | Done |
| Handlers that build status from registries and validation/connectivity logic. | Done |

---

### Phase 3 — Unified merkle status

| Field | Value |
|-------|--------|
| Goal | Implement top-level merkle status that concatenates workspace, agents, and providers sections; support section filters. |
| Dependencies | Phase 1, Phase 2 |
| Docs | status_command_spec.md |
| Completion | Done |

| Task | Completion |
|------|------------|
| Implement merkle status with optional --workspace-only, --agents-only, --providers-only; default: all three sections. | Done |
| Pass --breakdown and --test-connectivity through to workspace and provider sections. | Done |
| Prefer single status module producing all sections; top-level status and subcommands call same logic. | Done |
| Ensure merkle workspace status = workspace section only (alias or same logic as merkle status --workspace-only). | Done |

| Exit criterion | Completion |
|----------------|------------|
| merkle status outputs all three sections by default; section filters work. | Done |
| Output of each section matches the dedicated subcommands (merkle workspace status, merkle agent status, merkle provider status). | Done |

| Key change | Completion |
|------------|------------|
| Top-level Status command wires to unified status handler; section selection by flags. | Done |
| Shared status module or coordinated handlers for workspace, agent, provider. | Done |

---

### Phase 4 — Workspace command specs and scan / validate / watch

| Field | Value |
|-------|--------|
| Goal | Implement or align scan, validate, and watch with detailed specs; ensure CLI and behavior match specs. |
| Dependencies | Existing tree and store; Phase 1–3 only if status touches workspace |
| Docs | ignore_list_spec.md, scan_command_spec.md, validate_command_spec.md, watch_command_spec.md |
| Completion | Done |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Implement or refine ignore list per ignore_list_spec.md: .gitignore and XDG_DATA_HOME/merkle/workspace_path/ignore_list; scan reads both; merkle workspace ignore path lists or adds path; workspace delete (tombstone) appends path unless --no-ignore; workspace restore removes path when present. | Done |
| 2 | Implement or refine merkle scan per scan_command_spec.md: load ignore list and .gitignore, pass to Walker; args, TreeBuilder, populate store, guards, output. Scan creates/refreshes the workspace tree. | Done |
| 3 | Implement or refine merkle workspace validate per validate_command_spec.md: store, head index, basis index consistency; errors/warnings; text and JSON. | Done |
| 4 | Implement or refine merkle watch per watch_command_spec.md: options (debounce, batch), daemon, file watcher, tree update, optional context regeneration; use same ignore sources as scan (no --ignore flag). | Done |
| 5 | Add or update tests for each command as specified in the specs. | Done |

| Exit criterion | Completion |
|----------------|------------|
| scan, validate, and watch behavior and CLI match scan_command_spec.md, validate_command_spec.md, watch_command_spec.md. | Done |
| Required guards and output formats implemented; tests added. | Done |

| Key change | Completion |
|------------|------------|
| CLI and handlers in src/tooling/cli.rs and related modules; tree builder, store, head index, watch daemon as specified. | Done |

Phase 4 notes: Ignore module in src/ignore.rs; merkle workspace ignore in WorkspaceCommands; scan, validate, and watch use shared ignore loading. TreeBuilder accepts optional WalkerConfig. NodeRecordStore::flush added; scan flushes after populate. Workspace validate has --format text or json and uses full node count from store. Watch no longer accepts --ignore; daemon loads ignore from .gitignore and ignore_list. Integration tests in tests/integration/workspace_commands.rs.

---

### Phase 5 — Node delete (tombstone-based)

| Field | Value |
|-------|--------|
| Goal | Implement node deletion using tombstone semantics per node_deletion_spec.md and node_deletion_and_append_only.md. Tombstoning preserves append-only semantics while enabling logical deletion, recovery, and eventual compaction. |
| Dependencies | Node store and head index implementation |
| Docs | node_deletion_spec.md, node_deletion_and_append_only.md, ignore_list_spec.md |
| Completion | Done |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Add `tombstoned_at: Option<u64>` field to NodeRecord. Update serialization. Treat missing field as None on load (backward compatible). | Done |
| 2 | Add NodeRecordStore methods: `tombstone`, `restore`, `purge`, `list_tombstoned`. Update `find_by_path` to skip tombstoned nodes. Add `list_active`. | Done |
| 3 | Update HeadIndex storage format to include tombstone state per entry. Add `tombstone_heads_for_node`, `restore_heads_for_node`, `purge_tombstoned`, `get_active_head`. Update persistence format with version field. | Done |
| 4 | Add FrameStorage `purge` method for compaction (removes frame blob by FrameID). | Done |
| 5 | Add ContextApi methods: `tombstone_node` (cascade tombstone), `restore_node` (cascade restore), `compact` (purge old tombstones and optionally frames). | Done |
| 6 | Add CLI `merkle workspace delete <path>` and `--node <id>` with tombstone semantics; options --dry-run, --no-ignore; cascade always on. Unless --no-ignore, append path to ignore list per ignore_list_spec.md. | Done |
| 7 | Add CLI `merkle workspace restore <path>` and `--node <id>` to clear tombstone markers; remove path from ignore list when present per ignore_list_spec.md. | Done |
| 8 | Add CLI `merkle workspace compact` with --ttl (default 90 days), --all, --keep-frames, --dry-run options. | Done |
| 9 | Add CLI `merkle workspace list-deleted` with --older-than filter and --format json support. | Done |
| 10 | Integration tests: delete, restore, compact, list-deleted; cascade behavior; idempotency; ignore list append on delete and remove on restore per ignore_list_spec.md. | Done |
| 11 | Unit tests: NodeRecordStore tombstone/restore/purge; HeadIndex tombstone methods; FrameStorage purge. | Done |

| Exit criterion | Completion |
|----------------|------------|
| `merkle workspace delete` tombstones node and descendants; queries exclude tombstoned entries. | Done |
| `merkle workspace restore` clears tombstone markers and restores nodes to active state. | Done |
| `merkle workspace compact` purges old tombstones; --keep-frames preserves frame blobs. | Done |
| `merkle workspace list-deleted` shows tombstoned nodes with timestamps. | Done |
| Frame blobs preserved on delete; only purged on explicit compact (unless --keep-frames). | Done |
| Backward compatible: existing data loads without migration. | Done |
| Ignore list: delete appends path unless --no-ignore; restore removes path when present per ignore_list_spec.md. | Done |

| Key change | Completion |
|------------|------------|
| NodeRecord gains tombstoned_at field; HeadIndex entries gain tombstone state. | Done |
| Store and index APIs for tombstone/restore/purge; new CLI commands. | Done |
| Append-only semantics preserved; data recoverable until explicit compaction. | Done |
| ignore_list_spec.md: delete updates list, restore removes from list. | Done |

Phase 5 notes: NodeRecord and SledNodeRecordStore in src/store; HeadIndex with HeadEntry and versioned persistence in src/heads.rs; FrameStorage::purge in src/frame/storage.rs; ContextApi tombstone_node/restore_node/compact and result types in src/api.rs; ignore::remove_from_ignore_list in src/ignore.rs; workspace_status uses list_active; CLI delete/restore/compact/list-deleted in src/tooling/cli.rs. Unit tests in store, heads, frame/storage; integration tests in tests/integration/node_deletion.rs.

---

### Phase 6 — Remove deprecated commands

| Field | Value |
|-------|--------|
| Goal | Remove deprecated CLI commands once all replacements are in place. See command_list.md for the full remove vs keep list. |
| Dependencies | Phase 2 (so merkle provider validate exists); Phases 1–5 complete |
| Docs | command_list.md |
| Completion | Not started |

| Task | Completion |
|------|------------|
| Remove top-level validate-providers; replacement is merkle provider validate (Phase 2). | Not started |
| Remove low-level, node-ID-based commands: get-node, get-text, put-frame, list-frames, get-head. Path-based flows use merkle context get --path, merkle context generate, etc. | Not started |
| Remove CLI variants, dispatch branches, and any handlers used only by the removed commands. Update help and docs to drop references. | Not started |
| Optional: one release prior to removal, emit a deprecation warning when a removed command is invoked so scripts can migrate. | Not started |

| Exit criterion | Completion |
|----------------|------------|
| merkle validate-providers and the five low-level commands are no longer available. | Not started |
| Help and documentation do not reference the removed commands. | Not started |

| Key change | Completion |
|------------|------------|
| CLI: remove command variants and their dispatch; prune help text. No new user-facing behavior; removal only. | Not started |

---

## Implementation order summary

| Order | Phase | Summary |
|-------|-------|---------|
| 1 | Phase 1: Display stack and workspace status | Foundation for all status output. |
| 2 | Phase 2: Agent status and provider status | Dedicated status subcommands. |
| 3 | Phase 3: Unified merkle status | Single entry point and section filters. |
| 4 | Phase 4: Scan, validate, watch specs | Workspace command behavior and tests. |
| 5 | Phase 5: Node delete (tombstone-based) | Tombstone/restore/compact APIs and CLI. |
| 6 | Phase 6: Remove deprecated commands | Drop validate-providers and low-level get-node, get-text, put-frame, list-frames, get-head. |

---

## Related documentation

**Spec docs with test requirements:** Each command spec has a "Tests required" section. [agent_status_spec.md](agent_status_spec.md), [provider_status_spec.md](provider_status_spec.md), [provider_validate_spec.md](provider_validate_spec.md), [status_command_spec.md](status_command_spec.md), [ignore_list_spec.md](ignore_list_spec.md), [scan_command_spec.md](scan_command_spec.md), [validate_command_spec.md](validate_command_spec.md), [watch_command_spec.md](watch_command_spec.md), [node_deletion_spec.md](node_deletion_spec.md). The delete spec includes unit, integration, consistency, and edge-case tests.

- [README.md](README.md) — Overview and command list.
- [command_list.md](command_list.md) — Remove vs keep commands.
- [workspace_status_requirements.md](workspace_status_requirements.md) — Workspace status and display stack.
- [status_command_spec.md](status_command_spec.md) — Unified status.
- [agent_status_spec.md](agent_status_spec.md), [provider_status_spec.md](provider_status_spec.md) — Agent and provider status.
- [provider_validate_spec.md](provider_validate_spec.md) — merkle provider validate; replaces top-level validate-providers.
- [ignore_list_spec.md](ignore_list_spec.md) — Ignore list and .gitignore; XDG location; scan, workspace ignore, workspace delete (tombstone), and workspace restore behavior.
- [scan_command_spec.md](scan_command_spec.md), [validate_command_spec.md](validate_command_spec.md), [watch_command_spec.md](watch_command_spec.md) — Workspace commands (merkle scan, merkle workspace validate, merkle watch).
- [node_deletion_spec.md](node_deletion_spec.md), [node_deletion_and_append_only.md](node_deletion_and_append_only.md) — Tombstone-based deletion: merkle workspace delete/restore/compact and append-only policy.
