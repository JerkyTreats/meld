# Workspace Watch Runtime Spec

Date: 2026-02-17

## Objective

Define extraction for workspace watch runtime and editor bridge so watch behavior is owned by workspace domain modules.

Related ownership specs:
- [Workspace Lifecycle Services Spec](workspace_lifecycle_services.md)
- [Workspace Migration Guide](workspace_migration_guide.md)
- [CLI Migration Plan](../cli/cli_migration_plan.md)
- [Telemetry Migration Plan](../telemetry/telemetry_migration_plan.md)
- [Tooling Diffusion Map](../cli/tooling_diffusion_map.md)
- [Telemetry Event Engine Spec](../telemetry/telemetry_event_engine_spec.md)
- [Context Domain Structure Spec](../context/context_domain_structure.md)

## Scope

This spec covers workspace watch runtime behavior.

- watch command runtime lifecycle
- file change intake and normalization
- debounce and batch policy
- ignore and path filter policy
- editor bridge contracts for watch consumers
- queue submit hooks for optional context generation
- telemetry emission hooks for watch sessions

## Out Of Scope

This spec does not redesign business behavior.

- no change to CLI command flags
- no change to context generation semantics
- no change to provider transport behavior

## Current Mix Of Concerns

Watch behavior is split across shell and legacy tooling paths.

- `src/tooling/watch.rs` owns runtime and event batching
- `src/tooling/editor.rs` owns editor bridge behavior
- `src/tooling/cli.rs` owns watch route and runtime setup

This split blurs workspace ownership for watch behavior.

## Target Ownership

### Workspace watch owns

- runtime start stop lifecycle
- change intake normalization and filtering
- debounce and batch policy
- editor bridge contracts
- integration hooks for queue submit and telemetry emit

### CLI shell owns

- parse and route for watch command
- output envelope selection
- error translation for shell output

### Other domains own

- context generation queue processing behavior
- telemetry event schema and sink routing

## Concerns To Move

### Watch runtime lifecycle

- current area: `src/tooling/watch.rs`
- target home: `src/workspace/watch/runtime.rs`
- home status: missing dedicated workspace module owner

### Event model and batching policy

- current area: `ChangeEvent` and `EventBatcher` in `src/tooling/watch.rs`
- target home: `src/workspace/watch/events.rs`
- home status: missing dedicated workspace module owner

### Editor bridge behavior

- current area: `src/tooling/editor.rs`
- target home: `src/workspace/watch/editor_bridge.rs`
- home status: missing dedicated workspace module owner

### Watch route setup

- current area: `src/tooling/cli.rs` watch command path
- target home: remain in CLI route layer with delegation to workspace watch runtime
- home status: mixed shell and runtime setup logic

## Proposed Module Shape

- `src/workspace/watch/mod.rs`
- `src/workspace/watch/runtime.rs`
- `src/workspace/watch/events.rs`
- `src/workspace/watch/editor_bridge.rs`

Compatibility wrappers during migration:

- `src/tooling/watch.rs`
- `src/tooling/editor.rs`

## Runtime Contracts

### Watch runtime request

- workspace root
- debounce and batch windows
- ignore patterns
- optional queue integration flags
- optional telemetry session id

### Watch runtime response

- startup result
- runtime health summary
- shutdown summary

### Event contract

- normalized created modified removed and renamed events
- deterministic batch ordering for reproducible processing

## Migration Plan

1. add characterization tests for current watch behavior and event batching
2. introduce workspace watch modules with compatibility wrappers
3. move event normalization and batching into `src/workspace/watch/events.rs`
4. move runtime lifecycle into `src/workspace/watch/runtime.rs`
5. move editor bridge behavior into `src/workspace/watch/editor_bridge.rs`
6. keep CLI watch route as parse route and delegation only
7. remove legacy watch and editor wrappers after call site migration

## Test Plan

### Behavior parity coverage

- parity for debounce and batch windows
- parity for ignore filter behavior
- parity for create modify remove rename handling
- parity for startup and shutdown behavior

### Boundary coverage

- route tests confirm CLI delegates to workspace watch runtime
- guard tests confirm CLI does not own event batching policy
- guard tests confirm workspace watch does not own telemetry sink internals

### Contract coverage

- deterministic event ordering checks
- editor bridge callback contract checks
- queue submit integration contract checks

## Acceptance Criteria

- watch runtime ownership is under `src/workspace/watch`
- editor bridge ownership is under `src/workspace/watch/editor_bridge.rs`
- CLI watch route delegates to workspace watch runtime
- compatibility wrappers preserve behavior until migration completes
- characterization and boundary suites pass

## Risks And Mitigation

- risk: event drift during module relocation
- mitigation: characterization suite for event matrices and batch behavior

- risk: runtime lifecycle regressions
- mitigation: startup and shutdown contract tests

- risk: shell logic regains runtime policy
- mitigation: route guard tests and ownership checks

## Deliverables

- workspace watch runtime modules under `src/workspace/watch`
- compatibility wrappers for legacy watch and editor modules
- characterization and boundary tests for watch runtime behavior
- migration report with moved watch concerns and wrappers removed
