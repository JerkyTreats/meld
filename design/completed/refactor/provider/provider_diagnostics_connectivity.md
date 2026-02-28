# Provider Diagnostics Connectivity Spec

Date: 2026-02-17

## Objective

Define provider diagnostics extraction with a domain first structure where provider meaning and provider use cases live under `src/provider/`.

Related ownership spec: [God Module Detangling Spec](../god_module_detangling_spec.md).

## Scope

This spec covers provider diagnostics and the provider domain module layout that owns provider workflows.

- provider status connectivity checks
- provider validate connectivity and model checks
- provider test flow and summary shaping
- provider domain layering and module ownership
- shared timeout retry and error mapping policy

## Out Of Scope

This spec does not redesign provider behavior.

- no change to provider config schema
- no change to CLI parse and help surface
- no change to core telemetry transport primitives

## Domain First Principle

Provider domain concerns should live in `src/provider/` with layered internals.

- domain layer owns provider entities and domain rules
- ports layer owns contracts used by other domains
- application layer owns provider use case orchestration
- infrastructure layer owns client and repository implementations
- CLI shell stays in the cli boundary and routes to provider application services

## Target Ownership

### Provider domain owns

- provider entities and validation rules
- client and repository contracts
- diagnostics status validate and test orchestration
- provider config mutation orchestration
- generation facing provider use cases used by context generation paths

### CLI shell owns

- parse and route for provider commands
- output mode selection and envelope policy
- error translation to CLI surface

### Cross domain callers own

- call provider ports and application services only
- avoid direct client implementation access

## Concerns To Move

The list below maps each concern to its target module home.

### Diagnostics command orchestration

- current shell area: `handle_provider_status`
- current shell area: `handle_provider_validate`
- current shell area: `handle_provider_test`
- target home: `src/provider/application/diagnostics_service.rs`

### Config command orchestration

- current shell area: provider create edit remove handlers
- target home: `src/provider/application/command_service.rs`

### Connectivity and model check policy

- current shell area: duplicated checks across status validate and test handlers
- target home: `src/provider/application/diagnostics_service.rs`

### Provider interaction contract for callers

- current shell area: mixed direct registry and client usage
- target home: `src/provider/ports/client.rs`

### Provider config persistence contract

- current shell area: mixed static file load save delete paths
- target home: `src/provider/ports/repository.rs`

### Provider client implementations

- current mixed area: provider transport implementations in monolithic module
- target home: `src/provider/infra/clients/*`

### Generation path provider usage

- current mixed area: provider usage in generation and queue flows
- target home: `src/provider/application/generation_service.rs` and `src/provider/ports/client.rs`

## Proposed Domain Shape

Create a provider package with explicit layered ownership.

- `src/provider/mod.rs`
- `src/provider/domain/mod.rs`
- `src/provider/domain/model.rs`
- `src/provider/domain/validation.rs`
- `src/provider/ports/mod.rs`
- `src/provider/ports/client.rs`
- `src/provider/ports/repository.rs`
- `src/provider/application/mod.rs`
- `src/provider/application/diagnostics_service.rs`
- `src/provider/application/command_service.rs`
- `src/provider/application/generation_service.rs`
- `src/provider/infra/mod.rs`
- `src/provider/infra/clients/mod.rs`
- `src/provider/infra/clients/openai.rs`
- `src/provider/infra/clients/anthropic.rs`
- `src/provider/infra/clients/ollama.rs`
- `src/provider/infra/clients/local.rs`
- `src/provider/infra/repository/mod.rs`
- `src/provider/infra/repository/xdg.rs`

## Service Contracts

### Diagnostics service contract

- operations: `diagnose_status`, `diagnose_validate`, `diagnose_test`
- deterministic response fields for text and json adapters

### Command service contract

- operations: create edit remove validate status
- post mutation reload policy and consistency checks

### Generation service contract

- operation: resolve provider client and model policy for generation callers
- reusable for queue and orchestrator flows

### Ports contracts

- client port exposes completion and model listing capabilities
- repository port exposes load list save delete capabilities

## Migration Plan

1. add characterization tests for provider status validate test and config command behavior
2. introduce provider layered modules with compatibility wrappers in existing entry points
3. move diagnostics orchestration into `src/provider/application/diagnostics_service.rs`
4. move config command orchestration into `src/provider/application/command_service.rs`
5. move client implementations into `src/provider/infra/clients/*`
6. move repository implementation into `src/provider/infra/repository/xdg.rs`
7. move queue and generation provider calls behind provider ports and generation service
8. keep CLI handlers as parse route and output adapters only

## Test Plan

### Behavior parity coverage

- status parity with and without connectivity checks
- validate parity with connectivity and model checks
- test parity for success failure and timeout paths
- config command parity for create edit remove validate and status

### Boundary coverage

- route tests confirm one provider application service call per command route
- guard tests confirm CLI does not own provider diagnostics or persistence policy
- guard tests confirm generation paths use provider ports instead of client implementations

### Contract coverage

- deterministic json field tests for diagnostics and command outputs
- provider port contract tests with mock implementations
- per provider client integration tests for completion and model listing

## Acceptance Criteria

- provider domain concerns live under `src/provider/`
- diagnostics orchestration is owned by `src/provider/application/diagnostics_service.rs`
- provider config command orchestration is owned by `src/provider/application/command_service.rs`
- provider clients and repository implementations are owned by provider infrastructure modules
- CLI remains a thin route and output adapter for provider commands
- characterization and parity suites pass

## Risks And Mitigation

- risk: layering drift where shell or generation paths bypass provider application services
- mitigation: route guard tests and port boundary tests

- risk: behavior drift across diagnostics and command outputs
- mitigation: characterization snapshots and deterministic response contracts

- risk: migration churn for call sites that used direct registry internals
- mitigation: staged compatibility wrappers and incremental call site migration

## Deliverables

- layered provider module split under `src/provider/`
- provider diagnostics command and generation application services
- provider ports for client and repository contracts
- infrastructure implementations for provider clients and XDG repository
- parity and boundary test suites for provider flows
