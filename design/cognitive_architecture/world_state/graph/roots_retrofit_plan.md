# Roots Retrofit Plan

Date: 2026-04-15
Status: active
Scope: phased implementation path from current roots migration support to branch ready workspace federation substrate

## Overview

Objective:

- retrofit the current `roots` implementation into a branch ready substrate without breaking active workspace behavior

Outcome:

- current workspace registration remains stable
- root metadata becomes branch aware internally
- runtime gains the seams needed for later federated read support
- each checkpoint lands with verification evidence and one atomic commit

## Development Phases

### Checkpoint 1

Goal:

- stabilize and commit the existing roots registration and migration bookkeeping slice as the baseline for the retrofit

Tasks:

- verify the current `roots` implementation against tests and CLI behavior
- confirm documentation links for branch substrate and root first slice framing
- commit only the baseline files for roots registration, ledger, catalog, and status

Exit criteria:

- active workspace startup writes manifest, ledger, and catalog
- `meld roots status` reports registered workspaces
- baseline integration tests pass

Key seams:

- `src/roots/`
- `src/cli/`
- `src/bin/meld.rs`
- `tests/integration/roots_runtime.rs`

Status:

- completed

Verification evidence:

- Gate A pass via `cargo test roots_runtime --tests`
- Gate B pass via `cargo test parses_roots_status_command`
- Gate B pass via `cargo test roots_command_names_are_stable`
- Gate B pass via `cargo test test_build_logging_config_default --bin meld`
- Gate D pass via scoped diff review before staging

### Checkpoint 2

Goal:

- generalize metadata contracts from root only records toward branch ready records while preserving current storage compatibility

Tasks:

- add internal branch aware contract types alongside root compatibility types
- add conversion paths between root records and branch aware records
- preserve current on disk file names and legacy readers
- update runtime internals to operate on branch aware metadata while keeping `roots` external surfaces stable

Exit criteria:

- runtime behavior remains unchanged for active workspaces
- manifest and catalog readers accept current files
- new internal tests cover round trip compatibility

Key seams:

- `src/roots/contracts.rs`
- `src/roots/manifest.rs`
- `src/roots/catalog.rs`
- `src/roots/runtime.rs`

Status:

- completed

Verification evidence:

- Gate A pass via `cargo test roots_runtime --tests`
- Gate B pass via `cargo test parses_roots_status_command`
- Gate B pass via `cargo test roots_command_names_are_stable`
- Gate C pass via `cargo test root_manifest_converts_to_branch_manifest`
- Gate C pass via `cargo test root_catalog_converts_to_branch_catalog`
- Gate C pass via `cargo test branch_manifest_saves_as_root_compatible_file`
- Gate C pass via `cargo test root_manifest_loads_into_branch_manifest`
- Gate C pass via `cargo test branch_catalog_saves_as_root_compatible_file`
- Gate C pass via `cargo test root_catalog_loads_into_branch_catalog`
- Gate D pass via scoped diff review before staging

### Checkpoint 3

Goal:

- introduce branch style runtime seams inside the existing roots domain

Tasks:

- add branch oriented resolution and handle types behind the current roots facade
- separate identity resolution from storage path finalization
- isolate workspace specific assumptions behind a dedicated adapter path

Exit criteria:

- active startup resolves through the new internal seam
- no CLI or user facing behavior regresses
- characterization tests cover identity stability and relocation safe behavior where supported

Key seams:

- `src/roots/locator.rs`
- `src/roots/runtime.rs`
- `src/cli/route.rs`

Status:

- completed

Verification evidence:

- Gate A pass via `cargo test roots_runtime --tests`
- Gate C pass via `cargo test active_root_resolution_matches_active_branch_resolution`
- Gate C pass via `cargo test resolved_root_converts_to_branch_handle`
- Gate C pass via `cargo test resolved_branch_converts_to_root`
- Gate D pass via scoped diff review before staging

### Checkpoint 4

Goal:

- prepare the roots domain for later rename into `branches` without taking the rename yet

Tasks:

- add compatibility wrappers and naming boundaries that isolate root legacy terms
- ensure operator surfaces can later alias `roots` to `branches`
- update docs and code comments to make `workspace_fs` clearly the first branch kind

Exit criteria:

- the remaining root named surface is intentionally compatibility scoped
- the internal model is ready for a mechanical domain rename later

Key seams:

- `src/roots.rs`
- `src/roots/tooling.rs`
- `design/cognitive_architecture/world_state/graph/branch_federation_substrate.md`

Status:

- completed

Verification evidence:

- Gate A pass via `cargo test roots_runtime --tests`
- Gate B pass via `cargo test parses_roots_status_command`
- Gate B pass via `cargo test roots_command_names_are_stable`
- Gate D pass via scoped diff review before staging

## Verification Strategy

Gate A:

- targeted integration tests for roots startup and status

Gate B:

- targeted CLI parsing and help coverage for roots command surfaces

Gate C:

- compatibility tests for manifest and catalog read and write paths

Gate D:

- manual diff review before each commit to ensure unrelated user changes stay out of scope

## Implementation Order Summary

1. checkpoint 1 baseline verification and commit
2. checkpoint 2 metadata generalization with compatibility tests
3. checkpoint 3 runtime seam introduction
4. checkpoint 4 compatibility cleanup and rename preparation

## Related Documentation

- [Branch Federation Substrate](branch_federation_substrate.md)
- [Root Federation Runtime](root_federation_runtime.md)
- [Root Migration Architecture](root_migration_architecture.md)
- [Root Migration First Slice](root_migration_first_slice.md)
- [Graph](README.md)

## Exception List

- on disk file names remain root named during this retrofit
- CLI keeps `roots` terminology during this retrofit
- no federated multi branch reads land in this plan
- no destructive migration of existing workspace data is allowed

## Phase Notes

Checkpoint 1 notes:

- baseline already exists in the worktree and must be verified before further retrofit
- committed baseline should include the roots runtime slice, command surface, branch substrate docs, and this workflow plan

Checkpoint 2 notes:

- branch awareness should first appear as internal types and conversions
- current on disk root file names and CLI terms remain unchanged after this checkpoint

Checkpoint 3 notes:

- runtime seam work should remain additive and should not force the external rename
- `RunContext` now resolves and tracks an internal branch handle while root named compatibility methods remain available

Checkpoint 4 notes:

- rename preparation is complete when future `branches` extraction becomes low risk
- internal consumers now depend on branch neutral aliases and helpers while `roots` remains the compatibility operator surface
