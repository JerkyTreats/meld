# Agent Provider Config Management Commands Spec

Date: 2026-02-16

## Objective

Define a focused extraction for agent and provider config management commands so command handlers stop owning mutation orchestration and become thin routes.

Related ownership spec: [God Module Detangling Spec](../god_module_detangling_spec.md).

## Scope

This spec covers command orchestration for agent and provider config management.

- agent create edit remove validate status flows
- provider create edit remove validate flows
- config write and delete orchestration
- post mutation reload and consistency policy
- deterministic command result mapping for text and json adapters

## Out Of Scope

This spec does not redesign domain behavior.

- no change to agent or provider config schema
- no change to parse and help surface
- no change to provider diagnostics policy
- no change to transport client behavior

## Current Mix Of Concerns

`src/tooling/cli.rs` currently mixes shell and config command orchestration concerns.

- shell concern that should remain: parse command variants and route one service call
- orchestration concern to move: `handle_agent_create`
- orchestration concern to move: `handle_agent_edit`
- orchestration concern to move: `handle_agent_remove`
- orchestration concern to move: `handle_agent_validate`
- orchestration concern to move: `handle_provider_create`
- orchestration concern to move: `handle_provider_edit`
- orchestration concern to move: `handle_provider_remove`
- orchestration concern to move: `handle_provider_validate`
- orchestration concern to move: direct static config save and delete calls from command handlers

## Target Ownership

### Command services own

- command level mutation and validation workflows
- coordination with repository and validation services
- post mutation reload policy and consistency checks
- deterministic response contract for shell adapters

### CLI shell owns

- parse and route for command variants
- output envelope selection for text and json
- translation from service errors to CLI error surface

### Lower domains own

- in memory registry aggregate operations
- config repository file persistence primitives
- validation rule implementations

## Orchestration Concerns To Move

The list below tracks each orchestration concern, the target home, and current home status.

### Agent mutation workflows

- current shell area: `handle_agent_create`, `handle_agent_edit`, `handle_agent_remove`
- target home: agent command service
- home status: missing dedicated command service

### Agent validation workflow

- current shell area: `handle_agent_validate`
- target home: agent validation service plus agent command service
- home status: partial, registry validation exists, command level owner is missing

### Provider mutation workflows

- current shell area: `handle_provider_create`, `handle_provider_edit`, `handle_provider_remove`
- target home: provider command service
- home status: missing dedicated command service

### Provider validation workflow

- current shell area: `handle_provider_validate`
- target home: provider validation service plus provider command service
- home status: partial, registry validation exists, command level owner is missing

### Config persistence orchestration

- current shell area: direct `AgentRegistry` and `ProviderRegistry` static config save and delete calls in handlers
- target home: agent and provider config repositories coordinated by command services
- home status: partial, repository extraction is pending

### Post mutation registry reload policy

- current shell area: handler local reload logic after create edit remove
- target home: command service policy layer
- home status: missing centralized policy

## Proposed Service Shape

Create command services as orchestration owners.

- module: `src/agent/command_service.rs`
- module: `src/provider/command_service.rs`
- facades: `AgentCommandService`, `ProviderCommandService`
- operation groups: create edit remove validate status

## Request And Response Contracts

### Agent command requests

- agent id and command specific fields
- dry run and force flags when applicable

### Agent command responses

- mutation summary fields
- validation result fields
- deterministic adapter fields for text and json

### Provider command requests

- provider name and command specific fields
- connectivity related flags only for validate command

### Provider command responses

- mutation summary fields
- validation summary fields
- deterministic adapter fields for text and json

## Migration Plan

1. add characterization tests for current agent and provider command behavior in text and json
2. introduce command services behind existing handlers with no behavior change
3. move config persistence orchestration into repositories behind command services
4. move mutation and validation workflows into command services
5. move post mutation reload policy into command services
6. keep CLI handlers as parse route and output adapters only

## Test Plan

### Behavior parity coverage

- parity for agent create edit remove validate status flows
- parity for provider create edit remove validate flows
- parity for mutation summaries and validation outputs

### Boundary coverage

- route tests confirm one service call per command variant
- guard tests confirm shell does not call repository write methods directly
- error mapping tests for invalid input not found and persistence failures

### Policy coverage

- tests for post mutation reload consistency
- deterministic json field name tests for command outputs

## Acceptance Criteria

- config command orchestration is owned by agent and provider command services
- no config command orchestration business logic remains in `src/tooling/cli.rs`
- direct static repository mutation calls are removed from CLI handlers
- command behavior matches existing semantics for text and json
- characterization and parity suite passes for covered command flows

## Risks And Mitigation

- risk: policy drift between create edit remove and validate flows
- mitigation: one command service per aggregate and shared response contracts

- risk: persistence side effects differ from current behavior
- mitigation: characterization tests before migration and parity checks after migration

- risk: shell regains mutation logic over time
- mitigation: route guard tests and ownership rules in this spec

## Deliverables

- `src/agent/command_service.rs` and `src/provider/command_service.rs`
- CLI route wiring that delegates command workflows to services
- characterization and parity tests for agent and provider command flows
- migration report listing moved logic and boundary checks
