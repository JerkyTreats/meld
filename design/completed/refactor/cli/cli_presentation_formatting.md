# CLI Presentation Formatting Spec

Date: 2026-02-16

## Objective

Define a focused extraction for CLI presentation formatting so command handlers and route modules stop owning rendering details.

Related ownership spec: [God Module Detangling Spec](../god_module_detangling_spec.md).

## Scope

This spec covers text and json presentation responsibilities for CLI outputs.

- text rendering for command outputs
- json shape rendering for automation outputs
- table rendering and truncation policy
- shared formatting contract for status and command summaries

## Out Of Scope

This spec does not redesign command behavior.

- no change to underlying business semantics
- no change to parse and help surface
- no change to service orchestration policies
- no change to event storage formats

## Current Mix Of Concerns

`src/tooling/cli.rs` currently mixes shell routing and presentation formatting concerns.

- shell concern that should remain: choose output mode and route to one formatter call
- presentation concern to move: inline formatter helpers for agent list and show outputs
- presentation concern to move: inline formatter helpers for provider list and show outputs
- presentation concern to move: inline formatter helpers for context get text and json outputs
- presentation concern to move: initialization summary text formatting helper
- presentation concern to move: command level formatting decisions spread across handlers

## Target Ownership

### Presentation module owns

- text rendering helpers
- json rendering helpers
- table and truncation policy for formatter outputs
- compatibility contract for json field names consumed by automation

### CLI shell owns

- parse and route
- output mode selection
- hand off domain responses to presentation module

### Services own

- domain response models used as formatter input

## Concerns To Move

The list below tracks each concern, the target home, and current home status.

### Agent formatter helpers

- current shell area: `format_agent_list_text`, `format_agent_list_json`, `format_agent_show_text`, `format_agent_show_json`
- target home: CLI presentation module
- home status: missing dedicated module

### Provider formatter helpers

- current shell area: `format_provider_list_text`, `format_provider_list_json`, `format_provider_show_text`, `format_provider_show_json`
- target home: CLI presentation module
- home status: missing dedicated module

### Context formatter helpers

- current shell area: `format_context_text_output`, `format_context_json_output`
- target home: CLI presentation module
- home status: missing dedicated module

### Initialization summary formatter

- current shell area: `format_init_summary`
- target home: CLI presentation module
- home status: missing dedicated module

### Handler local formatting decisions

- current shell area: formatter selection logic spread across command handlers
- target home: shell output adapter plus presentation module boundary
- home status: partial, output mode selection exists, formatter ownership is mixed in shell

## Proposed Module Shape

Create one presentation package with command focused formatter units under `src/cli`.

- module: `src/cli/presentation/mod.rs`
- module: `src/cli/presentation/agent.rs`
- module: `src/cli/presentation/provider.rs`
- module: `src/cli/presentation/context.rs`
- module: `src/cli/presentation/init.rs`
- module: `src/cli/presentation/shared.rs`

Compatibility wrapper during migration:

- `src/tooling/cli.rs` delegates formatter calls to `src/cli/presentation`

## Formatting Contracts

### Text contract

- deterministic section ordering
- deterministic table column ordering
- stable truncation policy and marker behavior

### Json contract

- stable field names
- stable key presence policy for optional fields
- deterministic list ordering where applicable

## Migration Plan

1. add snapshot tests for existing text and json outputs
2. move formatter helpers from `src/tooling/cli.rs` into presentation modules with no behavior change
3. replace handler local formatting branches with adapter calls to presentation module
4. keep output mode selection in shell route layer
5. remove old formatter helpers from `src/tooling/cli.rs`

## Test Plan

### Snapshot coverage

- snapshots for agent outputs in text and json
- snapshots for provider outputs in text and json
- snapshots for context get outputs in text and json
- snapshots for initialization summary output

### Boundary coverage

- route tests confirm shell selects output mode and delegates rendering
- guard tests confirm handlers do not contain inline formatter logic

### Compatibility coverage

- json field stability tests for automation critical outputs
- table layout stability tests for key status outputs

## Acceptance Criteria

- presentation formatting is owned by `src/cli/presentation`
- inline formatter helpers are removed from CLI boundary modules
- shell keeps parse route and output mode selection only
- text and json outputs remain compatible with current behavior
- snapshot and compatibility suite passes

## Risks And Mitigation

- risk: output drift in json field names
- mitigation: compatibility tests with stable field assertions

- risk: output drift in table layouts
- mitigation: snapshot tests on representative command outputs

- risk: formatter duplication across modules
- mitigation: shared formatter utilities in presentation shared module

## Deliverables

- new presentation modules under `src/cli/presentation`
- CLI route updates to delegate formatting to presentation adapters
- snapshot and compatibility tests for presentation outputs
- migration report listing moved formatters and compatibility checks
