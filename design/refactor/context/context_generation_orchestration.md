# Context Generation Orchestration Spec

Date: 2026-02-17

## Objective

Define extraction for context generation orchestration so CLI handlers stay thin and orchestration ownership moves into the context domain.

Related ownership specs:
- [God Module Detangling Spec](../god_module_detangling_spec.md)
- [Context Domain Structure Spec](../context/context_domain_structure.md)

## Scope

This spec covers orchestration for `context generate`.

- generation target resolution by path or node id
- generation plan construction and ordering policy
- subtree precondition checks for recursive generation
- runtime boundary ownership for async execution
- queue lifecycle wiring and orchestrator invocation
- deterministic result mapping for text and json adapters

## Out Of Scope

This spec does not redesign generation behavior.

- no changes to frame content policy
- no changes to provider transport implementation
- no changes to queue dedupe semantics
- no changes to CLI parse and help shape

## Current Mix Of Concerns

`src/tooling/cli.rs` currently mixes shell and orchestration concerns in the context generate path.

- shell concern that should remain: parse `ContextCommands` and route one service call
- orchestration concern to move: `handle_context_generate`
- orchestration concern to move: `build_generation_plan`
- orchestration concern to move: runtime creation and block boundary management
- orchestration concern to move: queue start stop and worker lifecycle wiring
- orchestration concern to move: command result summary assembly for generation outcomes

## Target Ownership

### Context orchestration service owns

- target resolution and precondition policy
- generation plan construction and ordering
- runtime boundary ownership for async execution
- queue orchestration and orchestrator invocation
- deterministic response model for shell adapters

### CLI shell owns

- parse and route for `context generate`
- output envelope selection for text and json
- translation from service errors to CLI error surface

### Lower domains own

- queue worker processing internals inside context queue modules
- provider client request execution through provider contracts
- frame storage and head update persistence
- telemetry event transport and sink primitives

## Orchestration Concerns To Move

The list below tracks each orchestration concern, the target home, and current home status.

### Target resolution and precondition checks

- current shell area: `handle_context_generate`
- target home: `src/context/orchestration/service.rs`
- home status: partial, API and generation modules exist, orchestration ownership still in shell

### Plan construction

- current shell area: `build_generation_plan`
- target home: `src/context/orchestration/plan.rs`
- home status: partial, plan types exist, plan building still in shell

### Runtime boundary ownership

- current shell area: runtime creation and block boundary logic in `handle_context_generate`
- target home: `src/context/orchestration/service.rs`
- home status: missing dedicated owner

### Queue lifecycle and invocation

- current shell area: queue start stop and orchestrator invocation in `handle_context_generate`
- target home: `src/context/queue/runtime.rs`
- home status: partial, queue and orchestrator modules exist, lifecycle policy still in shell

### Deterministic command result mapping

- current shell area: context generate success and failure output assembly in `handle_context_generate`
- target home: context orchestration response models
- home status: missing dedicated response contract

## Proposed Domain Shape For Orchestration

Create context domain modules as orchestration owners.

- module: `src/context/orchestration/mod.rs`
- module: `src/context/orchestration/service.rs`
- module: `src/context/orchestration/plan.rs`
- module: `src/context/queue/mod.rs`
- module: `src/context/queue/runtime.rs`
- module: `src/context/queue/request.rs`

## Request And Response Contracts

### Generate context request

- target selector by path or node id
- agent id and provider name
- frame type override
- recursive and force flags
- sync mode policy

### Generate context response

- target node count
- generated frame count
- skipped frame count
- failure count and categorized failures
- deterministic summary fields for text and json adapters

## Migration Plan

1. add characterization tests for current `context generate` behavior in text and json
2. introduce context orchestration services behind current handler with no behavior change
3. move target resolution and precondition checks into context orchestration service
4. move plan construction into context orchestration plan module
5. move runtime boundary and queue lifecycle ownership into context orchestration and queue modules
6. keep CLI handler as parse route and output adapter only
7. remove `build_generation_plan` and generation orchestration logic from `src/tooling/cli.rs`

## Test Plan

### Behavior parity coverage

- parity for single target generation by path
- parity for single target generation by node id
- parity for recursive generation and ordering
- parity for force behavior and skip behavior
- parity for sync output fields in text and json

### Boundary coverage

- route tests confirm one service call for `context generate`
- guard tests confirm shell does not own runtime and queue lifecycle
- error mapping tests for target resolution failures and provider failures

### Determinism coverage

- deterministic response fields for identical inputs
- deterministic output envelope field names for json mode

## Acceptance Criteria

- generation orchestration is owned by context domain modules
- no generation orchestration logic remains in `src/tooling/cli.rs`
- queue lifecycle ownership moves to `src/context/queue/runtime.rs`
- `build_generation_plan` no longer exists in `src/tooling/cli.rs`
- `context generate` behavior remains stable for text and json modes
- characterization and parity suite passes for context generate scenarios

## Risks And Mitigation

- risk: runtime lifecycle regressions during extraction
- mitigation: isolate runtime boundary in one context service and add targeted lifecycle tests

- risk: behavior drift in recursive plan ordering
- mitigation: characterization tests and deterministic ordering assertions

- risk: output drift in command summaries
- mitigation: snapshot tests and explicit response contract fields

## Deliverables

- context orchestration module split under `src/context/orchestration`
- context queue module split under `src/context/queue`
- CLI route wiring that delegates context generation to context services
- characterization and parity tests for context generation flows
- migration report listing moved logic and boundary checks
