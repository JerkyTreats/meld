# Workspace Lifecycle Services Spec

Date: 2026-02-16

## Objective

Define a focused extraction for workspace lifecycle so command handlers stop owning lifecycle orchestration and become thin routes.

Related ownership spec: [God Module Detangling Spec](../god_module_detangling_spec.md).
Related watch runtime spec: [Workspace Watch Runtime Spec](workspace_watch_runtime_spec.md).

## Scope

This spec covers workspace lifecycle orchestration for the workspace command family.

- `workspace validate`
- `workspace delete`
- `workspace restore`
- `workspace compact`
- `workspace list-deleted`
- workspace status assembly and reporting ownership
- workspace watch runtime and editor bridge ownership
- shared workspace root and ignore policy used by these commands

## Out Of Scope

This spec does not redesign business behavior.

- No change to tombstone policy semantics
- No change to head index data model
- No change to frame storage format
- No change to CLI parse or help surface

## Current Mix Of Concerns

`src/tooling/cli.rs` currently mixes route and lifecycle orchestration.

- Shell concern that should remain: parse `WorkspaceCommands` and route to one service call
- Orchestration concern to move: `run_workspace_validate`
- Orchestration concern to move: `run_workspace_delete`
- Orchestration concern to move: `run_workspace_restore`
- Orchestration concern to move: `run_workspace_compact`
- Orchestration concern to move: `run_workspace_list_deleted`
- Orchestration concern to move: root and ignore policy coordination for consistency checks

## Target Ownership

### Workspace lifecycle service owns

- workspace lifecycle orchestration and policy
- subtree traversal policy for delete restore compact
- store and head index consistency checks
- ignore list side effects tied to delete and restore
- operation response models for text and json adapters
- workspace status assembly for workspace section data
- workspace watch runtime orchestration and event batching
- workspace editor bridge contracts for watch consumers

### CLI shell owns

- parse and route for `WorkspaceCommands`
- output envelope selection for text and json
- translation from service errors to CLI error surface

### Repositories and lower domains own

- node store persistence
- head index persistence
- frame storage purge mechanics
- ignore list read and write primitives

## Orchestration Concerns To Move

The list below tracks each orchestration concern, the target home, and current home status.

### Validate orchestration

- Current shell area: `run_workspace_validate`
- Target home: workspace lifecycle application service
- Home status: missing dedicated service

### Delete orchestration

- Current shell area: `run_workspace_delete`
- Target home: workspace lifecycle application service
- Home status: missing dedicated service

### Restore orchestration

- Current shell area: `run_workspace_restore`
- Target home: workspace lifecycle application service
- Home status: missing dedicated service

### Compact orchestration

- Current shell area: `run_workspace_compact`
- Target home: workspace lifecycle application service
- Home status: missing dedicated service

### List deleted orchestration

- Current shell area: `run_workspace_list_deleted`
- Target home: workspace lifecycle application service
- Home status: missing dedicated service

### Root and ignore policy coordination

- Current shell area: validation and mutation handlers in `src/tooling/cli.rs`
- Target home: workspace lifecycle application service
- Home status: partial, lower level primitives exist in `ignore`, `tree`, `store`, and `workspace_status`

### Workspace status assembly

- Current area: `src/workspace_status.rs`
- Target home: `src/workspace/status_service.rs`
- Home status: partial, workspace section logic exists but ownership is not in workspace package

### Workspace watch runtime and editor bridge

- Current area: `src/tooling/watch.rs` and `src/tooling/editor.rs`
- Target home: `src/workspace/watch/runtime.rs`, `src/workspace/watch/events.rs`, `src/workspace/watch/editor_bridge.rs`
- Home status: partial, runtime logic exists but is not in workspace package

## Proposed Workspace Shape

Create one workspace package with explicit service and runtime owners.

- module: `src/workspace/lifecycle_service.rs`
- module: `src/workspace/status_service.rs`
- module: `src/workspace/watch/mod.rs`
- module: `src/workspace/watch/runtime.rs`
- module: `src/workspace/watch/events.rs`
- module: `src/workspace/watch/editor_bridge.rs`
- facade: `WorkspaceLifecycleService`
- request and response types per operation

## Request And Response Contracts

### Validate

- request fields: workspace root, output mode
- response fields: validity, errors, warnings, consistency metrics

### Delete

- request fields: path or node id, dry run, no ignore
- response fields: deleted node count, deleted head count, ignore list update result

### Restore

- request fields: path or node id, dry run
- response fields: restored node count, restored head count, ignore list update result

### Compact

- request fields: ttl days, all, keep frames, dry run
- response fields: purged node count, purged head count, purged frame count

### List deleted

- request fields: older than days filter
- response fields: deleted node summaries and age metadata

## Migration Plan

1. Add characterization tests for current workspace command behavior in text and json.
2. Introduce `WorkspaceLifecycleService` behind current CLI handlers with no behavior change.
3. Move validate orchestration and consistency checks into the service.
4. Move delete restore compact list deleted orchestration into the service.
5. Move workspace status assembly from `src/workspace_status.rs` into `src/workspace/status_service.rs`.
6. Move watch runtime and editor bridge from `src/tooling/watch.rs` and `src/tooling/editor.rs` into `src/workspace/watch`.
7. Keep CLI handlers as route and output adapters only.
8. Remove `run_workspace_*` lifecycle methods from `src/tooling/cli.rs`.

## Test Plan

### Behavior parity coverage

- validate parity for errors warnings and metrics
- delete parity for dry run and ignore list behavior
- restore parity for dry run and ignore list behavior
- compact parity for ttl all keep frames and dry run
- list deleted parity for filter behavior and ordering

### Boundary coverage

- shell route tests confirm one service call per workspace command
- guard tests confirm no direct store mutation from CLI workspace routes
- error mapping tests for not found invalid input and persistence failures

### Data integrity coverage

- consistency tests for store and head index after delete restore compact
- tests for ignore list side effects after delete and restore
- tests for deterministic output fields in json mode

## Acceptance Criteria

- workspace lifecycle orchestration is owned by `WorkspaceLifecycleService`
- workspace status assembly is owned by `src/workspace/status_service.rs`
- workspace watch runtime and editor bridge are owned by `src/workspace/watch`
- no workspace lifecycle business logic remains in `src/tooling/cli.rs`
- `src/tooling/cli.rs` keeps parse route and output responsibilities only for workspace commands
- command behavior matches existing semantics for text and json
- characterization suite passes for validate delete restore compact and list deleted

## Risks And Mitigation

- Risk: subtle drift in ignore list side effects
- Mitigation: characterization tests before migration and contract tests after migration

- Risk: integrity regressions across store and head index updates
- Mitigation: operation level consistency tests and deterministic result assertions

- Risk: route layer bypasses service boundary during future edits
- Mitigation: route guard tests and ownership rules in this spec

## Deliverables

- `src/workspace/lifecycle_service.rs` with request and response models
- `src/workspace/status_service.rs` for workspace status assembly ownership
- `src/workspace/watch` runtime events and editor bridge modules
- CLI route wiring that delegates workspace lifecycle commands to the service
- characterization and parity tests for workspace lifecycle commands
- migration report listing moved methods and boundary checks
