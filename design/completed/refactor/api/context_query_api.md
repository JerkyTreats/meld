# Context Query API Spec

Date: 2026-02-17

## Objective

Define a focused extraction for context query read paths so query policy moves into context domain modules and `src/api.rs` becomes a compatibility facade.

Related ownership specs:
- [God Module Detangling Spec](../god_module_detangling_spec.md)
- [Context Domain Structure Spec](../context/context_domain_structure.md)

## Scope

This spec covers context query behavior.

- node context lookup for active nodes
- frame selection policy using view filters ordering and bounds
- query convenience methods and response helpers
- deterministic query error and empty result behavior
- head lookup helpers needed by read callers

## Out Of Scope

This spec does not redesign business behavior.

- no change to frame write semantics
- no change to tombstone restore and compact semantics
- no change to CLI parse help and output formatting

## Current Mix Of Concerns

`src/api.rs` currently mixes context query logic with mutation and lifecycle logic.

- query concern to move: `ContextApi::get_node`
- query concern to move: `ContextApi::latest_context`
- query concern to move: `ContextApi::context_by_type`
- query concern to move: `ContextApi::context_by_agent`
- query concern to move: `ContextApi::combined_context_text`
- query concern to move: `ContextApi::get_head`
- query concern to move: `ContextApi::get_all_heads`
- query helper concern to move: `ContextViewBuilder`
- query helper concern to move: `NodeContext` read helper methods

## Target Ownership

### Context query modules own

- node existence and tombstone gate for read path
- view policy translation and frame selection
- frame hydration and missing frame skip policy
- convenience query methods that compose one base query
- deterministic response contract for CLI and adapter callers

### API facade owns

- stable public API surface during migration
- delegation from wrappers to context query service
- dependency wiring only with no new query policy logic

### Other context modules own

- frame mutation and head update ordering
- lifecycle policies for tombstone restore and compact

## Concerns To Move

The list below tracks each query concern, the target home, and current home status.

### Core node context query

- current area: `ContextApi::get_node`
- target home: `src/context/query/service.rs`
- home status: exists in monolithic API file only

### Query convenience methods

- current area: `ContextApi::latest_context`, `ContextApi::context_by_type`, `ContextApi::context_by_agent`, `ContextApi::combined_context_text`
- target home: `src/context/query/service.rs`
- home status: partial, wrappers exist with no dedicated owner

### Query response helpers

- current area: `NodeContext` helper methods and `ContextViewBuilder`
- target home: `src/context/query/types.rs`
- home status: partial, helper logic exists but is co located with broad API logic

### View policy and composition helpers

- current area: `src/views.rs` and `src/composition.rs`
- target home: `src/context/query/view_policy.rs` and `src/context/query/composition.rs`
- home status: partial, split across standalone modules

### Head lookup read helpers

- current area: `ContextApi::get_head`, `ContextApi::get_all_heads`
- target home: `src/context/query/head_queries.rs`
- home status: partial, shared by queue and CLI callers

## Proposed Module Shape

Keep `src/api.rs` as a compatibility facade while splitting query units into context domain modules.

- `src/api.rs`
- `src/context/mod.rs`
- `src/context/facade.rs`
- `src/context/query/mod.rs`
- `src/context/query/service.rs`
- `src/context/query/types.rs`
- `src/context/query/view_policy.rs`
- `src/context/query/composition.rs`
- `src/context/query/head_queries.rs`

## Query Contracts

### Query request

- node id
- context view policy with max frames ordering and filters
- active node requirement with tombstoned nodes rejected

### Query response

- node id
- node record
- selected frames in deterministic policy order
- total frame count derived from current head index view
- read only behavior with no side effects

### Error contract

- `ApiError::NodeNotFound` for missing or tombstoned nodes
- storage passthrough errors for node store head index and frame storage failures
- empty context response when no heads are available

## Behavior Compatibility Rules

- keep head index based selection semantics in this phase
- keep missing frame blob behavior as skip not failure
- keep convenience methods as pure composition over base query
- keep `NodeContext` helpers read facing and free of mutation logic

## Migration Plan

1. add characterization tests for current query behavior and convenience helpers
2. introduce context query service behind `ContextApi` wrappers with no behavior change
3. move `get_node` logic into `src/context/query/service.rs`
4. move convenience query methods and head lookup helpers into context query modules
5. keep compatibility wrappers in `src/api.rs` while external callers migrate
6. remove duplicated query policy from `src/api.rs` after migration completion

## Test Plan

### Behavior parity coverage

- deterministic parity for repeated `get_node` calls with identical view
- parity for node not found and tombstoned node behavior
- parity for empty context response when no heads exist
- parity for convenience methods latest by type by agent and combined text

### Contract coverage

- ordering and filter matrix parity with view policy
- max frame bound and total count behavior
- missing frame blob skip behavior
- head lookup helper parity for queue and CLI callers

### Boundary coverage

- guard tests confirm query modules do not perform mutations
- adapter and CLI tests confirm read calls flow through query service wrappers
- regression tests for queue prompt context build path

## Acceptance Criteria

- context query logic is owned by `src/context/query` modules
- `src/api.rs` no longer contains core query policy implementation
- public query methods remain behavior compatible
- `NodeContext` convenience helpers stay query focused and avoid mutation concerns
- characterization and parity tests pass for API and CLI query flows

## Risks And Mitigation

- risk: hidden coupling with head index internals and queue call patterns
- mitigation: parity tests for queue preflight and query call behavior

- risk: behavior drift in ordering filters and frame counts
- mitigation: deterministic assertions in characterization tests

- risk: boundary leakage through broad facade helpers
- mitigation: explicit ownership guardrails and staged wrapper deprecation

## Deliverables

- query module split under `src/context/query`
- `src/api.rs` wrappers that delegate to context query services
- characterization and parity tests for context query behavior
- migration report listing moved logic and compatibility wrappers
