# Branch Feature Implementation Plan

Date: 2026-04-15
Status: active
Scope: phased implementation of canonical branches, dormant branch workflows, and federated workspace reads

## Overview

Objective:

- implement the full branch feature through the branch substrate, including canonical branch naming, dormant branch workflows, and federated workspace reads

Outcome:

- `branches` becomes the canonical runtime and CLI surface
- `roots` remains as a compatibility alias
- dormant workspace branches can be discovered, attached, migrated, and inspected
- federated reads can query many branch local traversal stores through one branch query facade

## Checkpoint 1

Goal:

- make `branches` the canonical module and CLI vocabulary while preserving `roots` compatibility

Tasks:

- add canonical `src/branches.rs`
- convert `src/roots.rs` into a compatibility facade
- add `branches` CLI command with `roots` alias compatibility
- switch internal routing and help surfaces to branch naming

Exit criteria:

- `meld branches status` works
- `meld roots status` still works through the alias path
- internal code can import `crate::branches`

Verification gates:

- parse parity for `branches` and `roots`
- command name stability
- existing roots status integration tests still pass

Status:

- completed

Verification evidence:

- Gate A pass via `cargo test parses_roots_status_command_alias`
- Gate A pass via `cargo test parses_branches_status_command`
- Gate A pass via `cargo test branch_command_names_are_stable`
- Gate B pass via `cargo test roots_runtime --tests`
- Gate A pass via `cargo test test_build_logging_config_default --bin meld`
- Gate E pass via staged diff review before commit

## Checkpoint 2

Goal:

- implement dormant branch workflows and the branch storage metadata needed for federation

Tasks:

- record branch store path in metadata
- add `branches discover`
- add `branches attach`
- add `branches migrate`
- update status output to show branch aware metadata

Exit criteria:

- dormant branches can be registered without becoming active workspaces
- migration can catch up registered dormant branches
- branch catalog contains enough metadata to reopen traversal stores

Verification gates:

- discovery excludes temp residue
- attach registers one explicit workspace path
- migrate updates status and replay metadata
- storage compatibility tests remain green

Status:

- completed

Verification evidence:

- Gate A pass via `cargo test parses_branches_attach_command`
- Gate B pass via `cargo test roots_runtime --tests`
- Gate C pass via `cargo test root_catalog_converts_to_branch_catalog`
- Gate C pass via `cargo test branch_catalog_saves_as_root_compatible_file`
- Gate C pass via `cargo test root_catalog_loads_into_branch_catalog`
- Gate C pass via `cargo test root_manifest_converts_to_branch_manifest`
- Gate C pass via `cargo test branch_manifest_saves_as_root_compatible_file`
- Gate C pass via `cargo test root_manifest_loads_into_branch_manifest`
- Gate E pass via staged diff review before commit

## Checkpoint 3

Goal:

- implement federated workspace reads above many branch local traversal stores

Tasks:

- add canonical branch query facade
- open traversal stores from branch catalog metadata
- support branch scope selection
- merge neighbors and walks across many branches
- expose branch graph status through the query facade

Exit criteria:

- federated single branch queries match local traversal behavior
- multi branch queries merge deterministically
- unhealthy branches do not poison healthy reads

Verification gates:

- federated query parity for one branch
- deterministic multi branch merge behavior
- branch failure isolation

Status:

- completed

Verification evidence:

- Gate A pass via `cargo test parses_branches_graph_neighbors_command`
- Gate A pass via `cargo test branch_command_names_are_stable`
- Gate B pass via `cargo test roots_runtime --tests`
- Gate D pass via `cargo test branches_query --tests`
- Gate A pass via `cargo test test_build_logging_config_default --bin meld`
- Gate E pass via staged diff review before commit

## Verification Strategy

Gate A:

- CLI parse and help parity for `branches` and `roots`

Gate B:

- integration tests for active and dormant branch workflows

Gate C:

- compatibility tests for manifest and catalog read and write paths

Gate D:

- federated query tests across one and many branch stores

Gate E:

- manual staged diff review before every commit

## Related Documents

- [Branch Federation Substrate](branch_federation_substrate.md)
- [Branch Lift Plan](branch_lift_plan.md)
- [Roots Retrofit Plan](roots_retrofit_plan.md)

## Exception List

- legacy root named on disk file names remain readable throughout this plan
- `roots` stays as a compatibility alias
- non-filesystem branch kinds remain out of scope for this implementation pass
