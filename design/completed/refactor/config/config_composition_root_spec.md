# Config Composition Root Spec

Date: 2026-02-17

## Objective

Define a domain first config model where `config` is a composition root and domain specific config policy is owned by each domain.

Related ownership specs:
- [God Module Detangling Spec](../god_module_detangling_spec.md)
- [Provider Diagnostics Connectivity Spec](../provider/provider_diagnostics_connectivity.md)
- [Agent Provider Config Management Commands Spec](../agent/agent_provider_config_management_commands.md)

## Scope

This spec covers config ownership and boundary policy.

- source loading from workspace file, global file, and environment
- precedence and merge policy
- runtime config composition for startup and command execution
- workspace and storage path resolution policy
- delegation to provider and agent config contracts

## Out Of Scope

This spec does not redesign domain behavior.

- no provider transport changes
- no agent identity model changes
- no CLI parse and help changes

## Domain Principle

Domain modules own domain config.

- provider domain owns provider config schema, validation, repository policy, and migrations
- agent domain owns agent config schema, validation, repository policy, and migrations
- config domain owns source loading, precedence, merge, and composition only

## Current Mix Of Concerns

Current config behavior is split across `src/config.rs`, `src/provider.rs`, and `src/agent.rs`.

- `src/config.rs` owns provider and agent config schema and validation
- `src/provider.rs` and `src/agent.rs` own domain repository paths and file writes
- command routes coordinate save and reload policy in shell handlers

This split weakens ownership and duplicates policy.

## Target Ownership

### Config domain owns

- source adapters for workspace file, global file, and environment
- precedence and merge policy
- runtime composition and caching policy
- workspace storage path resolution

### Provider domain owns

- provider config schema and defaults
- provider config validation policy
- provider config repository contract and XDG implementation

### Agent domain owns

- agent config schema and defaults
- agent prompt config validation policy
- agent config repository contract and XDG implementation

### CLI shell owns

- parse and route only
- output mapping for text and json

## Concerns To Move

### Provider schema and validation

- current area: `src/config.rs` provider config and provider type definitions
- target home: `src/provider/domain/config.rs` and `src/provider/domain/validation.rs`

### Agent schema and validation

- current area: `src/config.rs` agent config definitions and validation
- target home: `src/agent/domain/config.rs` and `src/agent/domain/validation.rs`

### Provider config repository operations

- current area: `src/provider.rs` load save delete and path helpers
- target home: `src/provider/ports/repository.rs` and `src/provider/infra/repository/xdg.rs`

### Agent config repository operations

- current area: `src/agent.rs` load save delete and path helpers
- target home: `src/agent/ports/repository.rs` and `src/agent/infra/repository/xdg.rs`

### Source loading and merge policy

- current area: `ConfigLoader` in `src/config.rs`
- target home: `src/config/sources/*` and `src/config/composition/service.rs`

### Workspace storage path resolution

- current area: storage path resolver in `src/config.rs`
- target home: `src/config/workspace/storage_paths.rs`

## Proposed Config Domain Shape

- `src/config/mod.rs`
- `src/config/facade.rs`
- `src/config/sources/mod.rs`
- `src/config/sources/workspace_file.rs`
- `src/config/sources/global_file.rs`
- `src/config/sources/environment.rs`
- `src/config/composition/mod.rs`
- `src/config/composition/service.rs`
- `src/config/composition/merge_policy.rs`
- `src/config/workspace/mod.rs`
- `src/config/workspace/storage_paths.rs`
- `src/config/paths/mod.rs`
- `src/config/paths/xdg_root.rs`

## Composition Contracts

### Config composition request

- workspace root
- explicit config file override when provided
- environment snapshot

### Config composition response

- system config and logging config
- provider config map using provider domain types
- agent config map using agent domain types
- resolved storage paths for workspace operations

### Error contract

- source read and parse errors with source id
- merge conflicts with deterministic conflict reporting
- domain validation failures returned from provider and agent contracts

## Migration Plan

1. add characterization tests for current source precedence and merge behavior
2. move provider config schema and validation into provider domain modules
3. move agent config schema and validation into agent domain modules
4. move provider and agent repository operations behind domain ports and XDG adapters
5. slim `ConfigLoader` into config composition service that delegates domain parsing and validation
6. update CLI and startup paths to consume composition service only
7. remove duplicate config policy from shell and registry code paths

## Test Plan

### Behavior parity coverage

- precedence parity for workspace file, global file, and environment
- parity for missing file behavior and fallback defaults
- parity for provider and agent validation results

### Boundary coverage

- guard tests confirm config domain does not own provider or agent policy rules
- guard tests confirm provider and agent domains do not own cross domain merge policy
- route tests confirm shell commands call domain services through config composition only

### Contract coverage

- deterministic merge output field checks
- deterministic validation error envelope checks
- storage path resolution parity checks

## Acceptance Criteria

- config domain is composition root only
- provider and agent domains own their config schema and repository policy
- shell routes do not perform config persistence or merge policy logic
- source precedence and merge behavior remain compatible
- characterization and boundary tests pass

## Risks And Mitigation

- risk: temporary duplication during staged migration
- mitigation: compatibility wrappers with short removal windows

- risk: precedence drift between old and new loaders
- mitigation: characterization matrix across source combinations

- risk: boundary regression over time
- mitigation: ownership guard tests and review checklist updates

## Deliverables

- config composition modules under `src/config`
- provider and agent config ownership modules under each domain
- compatibility wrappers for staged migration
- parity and boundary test suites for config loading and validation
