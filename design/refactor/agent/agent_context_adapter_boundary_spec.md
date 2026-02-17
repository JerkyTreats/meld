# Agent Context Adapter Boundary Spec

Date: 2026-02-17

## Objective

Define extraction for agent to context adapter contracts so adapter ownership lives under agent domain boundaries.

Related ownership specs:
- [Tooling Diffusion Map](../cli/tooling_diffusion_map.md)
- [Context Domain Structure Spec](../context/context_domain_structure.md)
- [Agent Provider Config Management Commands Spec](agent_provider_config_management_commands.md)

## Scope

This spec covers agent to context adapter boundary ownership.

- adapter contract shape for read write and generate flows
- concrete adapter that delegates to context APIs
- queue submit wait policy used by generate flow
- dependency wiring boundaries for caller modules

## Out Of Scope

This spec does not redesign domain behavior.

- no change to context query semantics
- no change to context mutation semantics
- no change to provider transport behavior

## Current Mix Of Concerns

Adapter ownership currently lives in legacy tooling path.

- `src/tooling/adapter.rs` owns adapter contract and implementation
- adapter implementation depends on broad context API surface
- queue wait policy is embedded in adapter implementation

This places a domain boundary contract in a non domain module.

## Target Ownership

### Agent domain owns

- context adapter port contract for agent callers
- concrete adapter implementation used by agent workflows
- adapter error mapping for agent call paths

### Context domain owns

- query mutation and orchestration behavior
- queue runtime behavior and processing semantics

### CLI shell owns

- dependency wiring only
- no adapter policy ownership

## Concerns To Move

### Adapter contract

- current area: `AgentAdapter` trait in `src/tooling/adapter.rs`
- target home: `src/agent/ports/context_adapter.rs`
- home status: missing dedicated agent port owner

### Adapter implementation

- current area: `ContextApiAdapter` in `src/tooling/adapter.rs`
- target home: `src/agent/adapters/context_api.rs`
- home status: missing dedicated agent adapter owner

### Queue wait policy in generate flow

- current area: queue submit wait timeout in `ContextApiAdapter::generate_frame`
- target home: `src/agent/adapters/context_api.rs` with queue policy delegated through context contracts
- home status: mixed policy and wiring in legacy module

## Proposed Module Shape

- `src/agent/ports/mod.rs`
- `src/agent/ports/context_adapter.rs`
- `src/agent/adapters/mod.rs`
- `src/agent/adapters/context_api.rs`

Compatibility wrapper during migration:

- `src/tooling/adapter.rs`

## Adapter Contracts

### Read contract

- input: node id plus context view
- output: read model for node context

### Write contract

- input: node id plus frame payload plus agent id
- output: frame id result

### Generate contract

- input: node id plus frame type plus agent id plus provider id
- output: generated frame id result

## Migration Plan

1. add characterization tests for current adapter read write and generate flows
2. introduce `src/agent/ports/context_adapter.rs` with compatibility reexport from legacy module
3. move `ContextApiAdapter` into `src/agent/adapters/context_api.rs`
4. migrate caller imports from `src/tooling/adapter.rs` to agent modules
5. keep legacy wrapper until no call sites remain
6. remove wrapper and legacy exports from tooling module

## Test Plan

### Behavior parity coverage

- parity for read context adapter behavior
- parity for write context adapter behavior
- parity for generate queue wait behavior

### Boundary coverage

- guard tests confirm adapter contracts are owned by agent ports
- guard tests confirm adapter implementation does not own context mutation policy
- route tests confirm CLI only wires dependencies

### Contract coverage

- deterministic error mapping checks
- timeout and queue wait contract checks
- trait contract mock tests for caller modules

## Acceptance Criteria

- adapter contract is owned by `src/agent/ports/context_adapter.rs`
- adapter implementation is owned by `src/agent/adapters/context_api.rs`
- legacy tooling adapter module is wrapper only during migration
- caller imports move to agent modules
- characterization and boundary suites pass

## Risks And Mitigation

- risk: hidden caller coupling to legacy import paths
- mitigation: staged reexports and incremental caller migration

- risk: queue timeout behavior drift
- mitigation: characterization tests for wait and timeout paths

- risk: boundary regression over time
- mitigation: contract guard tests and review ownership checks

## Deliverables

- agent port and adapter modules for context interaction
- compatibility wrapper strategy for legacy adapter path
- characterization and boundary tests for adapter behavior
- migration report with caller import updates and wrapper removal
