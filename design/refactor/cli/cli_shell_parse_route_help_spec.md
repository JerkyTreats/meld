# CLI Shell Parse Route Help Spec

Date: 2026-02-16

## Objective

Define a focused extraction for the CLI shell domain so `src/cli` owns parse route help and output envelope decisions.

Related ownership spec: [God Module Detangling Spec](../god_module_detangling_spec.md).

## Scope

This spec covers shell concerns only.

- Parse user input into typed commands
- Route typed commands to use case services
- Render help and usage text
- Apply output envelope policy for text and json

## Out Of Scope

This spec does not redesign command behavior.

- No change to workspace mutation semantics
- No change to provider network policy
- No change to agent and provider validation rules
- No change to context generation logic

## Current Problems

`src/tooling/cli.rs` currently mixes shell and orchestration concerns.

- Parse types and route logic are mixed with business workflows
- `run_workspace_*` methods perform storage and lifecycle orchestration
- `handle_agent_*` and `handle_provider_*` methods perform config persistence and validation
- `handle_context_generate` and `build_generation_plan` perform generation orchestration and runtime control
- Help text ownership is coupled to large handlers, which raises drift risk during refactor

## Target Ownership

### Shell owns

- `Cli` and command enums used by clap
- Route map from command variants to service calls
- Help and usage text contract
- Uniform output envelope selection for text and json
- Error translation from service error types into CLI error surface

### Shell does not own

- Workspace lifecycle orchestration
- Agent and provider config mutation workflows
- Provider diagnostics and connectivity checks
- Context generation planning and queue lifecycle
- Progress session policy and event emission details

## Orchestration Concerns To Move

The list below tracks each orchestration concern, the target home, and current home status.

### Workspace command orchestration

- Current shell area: `run_workspace_validate`, `run_workspace_delete`, `run_workspace_restore`, `run_workspace_compact`, `run_workspace_list_deleted`
- Target home: workspace lifecycle application service
- Home status: missing dedicated service, building blocks exist in `tree`, `store`, `ignore`, and `workspace_status`

### Agent command orchestration

- Current shell area: `handle_agent_create`, `handle_agent_edit`, `handle_agent_remove`, `handle_agent_validate`, `handle_agent_status`
- Target home: agent application use case services
- Home status: partial, registry exists, command use case layer is not yet extracted

### Provider command orchestration

- Current shell area: `handle_provider_create`, `handle_provider_edit`, `handle_provider_remove`, `handle_provider_validate`, `handle_provider_status`, `handle_provider_test`
- Target home: provider application use case services plus provider diagnostics service
- Home status: partial, registry exists, diagnostics and use case layer are not yet extracted

### Context generation orchestration

- Current shell area: `handle_context_generate`, `build_generation_plan`, queue runtime start and stop wiring
- Target home: context domain orchestration and queue services
- Home status: partial, generation and queue modules exist, runtime boundary ownership is still in shell

### Telemetry session lifecycle orchestration

- Current shell area: CLI `execute` session start finish prune and summary event emission scaffolding
- Target home: telemetry session and emission services
- Home status: partial, telemetry primitives exist under `progress`, lifecycle policy is still embedded in shell

### Unified status assembly orchestration

- Current shell area: `handle_unified_status` and status fan in logic for workspace agent provider summaries
- Target home: domain status services plus CLI unified status assembler
- Home status: missing dedicated assembler and explicit domain status service boundaries

## Proposed Module Shape

Move CLI boundary modules under one sharp root.

- `src/cli/mod.rs`
- `src/cli/parse.rs`
- `src/cli/route.rs`
- `src/cli/help.rs`
- `src/cli/output.rs`
- `src/cli/status_assembler.rs`

Compatibility wrapper during migration:

- `src/tooling/cli.rs` delegates to `src/cli` until call sites move

## Route Contract

Each route maps one command variant to one use case call.

- Input contract: typed command struct plus workspace and config context
- Output contract: typed result enum with text payload or json payload
- Error contract: typed service error mapped to stable CLI error category and message

Route handlers must stay thin.

- No direct file writes
- No direct store mutation
- No direct queue runtime lifecycle
- No direct registry persistence calls

## Help Contract

Help text is versioned as a shell contract.

- Command names and flags remain stable unless approved by explicit CLI change spec
- Examples remain executable against current command surface
- Deprecated aliases are listed with removal version and migration command

## Migration Plan

1. Add characterization tests for parse route help and output envelope for top level commands.
2. Introduce shell route layer with no behavior changes and keep current handlers behind adapter calls.
3. Move workspace command orchestration from `run_workspace_*` methods into services.
4. Move agent and provider command orchestration from `handle_agent_*` and `handle_provider_*` into services.
5. Move context generation orchestration from `handle_context_generate` and `build_generation_plan` into context domain services.
6. Move status fan in logic from `handle_unified_status` into `src/cli/status_assembler.rs`.
7. Use domain read services for workspace status agent status and provider status.
8. Introduce `src/cli` modules for parse route help output and dependency wiring.
9. Keep `src/tooling/cli.rs` as a temporary wrapper.
10. Remove wrapper after caller migration completes.

## Test Plan

### Parse coverage

- Valid command matrix for top level and nested subcommands
- Invalid flag and argument matrix with expected error text
- Default value coverage for format and command options

### Route coverage

- One test per command variant to verify route target
- Guard tests to ensure shell does not call storage or registry directly
- Error mapping tests for each service error category

### Help coverage

- Snapshot tests for `--help` at top level and nested scopes
- Snapshot tests for usage examples that appear in help text

### Output coverage

- Snapshot tests for text envelope and json envelope
- Stability tests for json field names consumed by automation

## Acceptance Criteria

- `src/cli` contains parse route help output and dependency wiring only
- No `run_workspace_*` methods remain in CLI boundary modules
- No `handle_agent_*` methods remain in CLI boundary modules
- No `handle_provider_*` methods remain in CLI boundary modules
- No `handle_context_generate` or `build_generation_plan` remains in CLI boundary modules
- No `handle_unified_status` fan in logic remains in CLI boundary modules
- Help output for existing commands remains stable except approved deltas
- Integration tests confirm command behavior parity for text and json modes

## Risks And Mitigation

- Risk: output drift in text and json shape
- Mitigation: characterization and snapshot tests before moving handlers

- Risk: hidden coupling to telemetry runtime lifecycle
- Mitigation: isolate telemetry policy in service layer and keep shell unaware of lifecycle internals

- Risk: route table sprawl over time
- Mitigation: enforce one route entry per command variant and one service call per route

## Deliverables

- New shell module split under `src/cli`
- Unified status assembler in `src/cli/status_assembler.rs`
- Service boundary adapters for workspace agent provider and context command families
- Characterization and snapshot test suite for parse route help output
- Final parity report with changed files and no behavior regressions
