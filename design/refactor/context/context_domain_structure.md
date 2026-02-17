# Context Domain Structure Spec

Date: 2026-02-17

## Objective

Define a domain first structure for context concerns so context query, mutation, queue, and orchestration concerns land under `src/context`.

Related ownership spec: [God Module Detangling Spec](../god_module_detangling_spec.md).

## Scope

This spec defines target structure and ownership for the context domain.

- context query use cases
- context mutation use cases
- context generation orchestration
- context queue runtime and request lifecycle
- context types and contracts used across callers

## Out Of Scope

This spec does not change business semantics.

- no change to CLI command shape
- no change to provider behavior contracts
- no change to tree and store primitives

## Domain First Principle

Everything that defines context behavior should live in `src/context`.

- cli owns parse route help and output rendering
- context domain owns use case orchestration and policy
- provider domain owns provider clients and provider use cases
- API facade remains as a compatibility surface during migration

## Target Domain Shape

- `src/context/mod.rs`
- `src/context/facade.rs`
- `src/context/types.rs`
- `src/context/query/mod.rs`
- `src/context/query/service.rs`
- `src/context/query/view_policy.rs`
- `src/context/query/composition.rs`
- `src/context/query/head_queries.rs`
- `src/context/mutation/mod.rs`
- `src/context/mutation/service.rs`
- `src/context/orchestration/mod.rs`
- `src/context/orchestration/service.rs`
- `src/context/orchestration/plan.rs`
- `src/context/queue/mod.rs`
- `src/context/queue/runtime.rs`
- `src/context/queue/request.rs`

## Concern Landing Map

### Query concerns

- current area: `src/api.rs` query methods and convenience methods
- target home: `src/context/query/service.rs`

### View policy concerns

- current area: `src/views.rs`
- target home: `src/context/query/view_policy.rs`

### Composition concerns

- current area: `src/composition.rs`
- target home: `src/context/query/composition.rs`

### Mutation concerns

- current area: `src/api.rs` put and ensure style methods
- target home: `src/context/mutation/service.rs`

### Queue concerns

- current area: `src/frame/queue.rs`
- target home: `src/context/queue/runtime.rs` and `src/context/queue/request.rs`

### Generation orchestration concerns

- current area: `src/generation/orchestrator.rs` and `src/generation/plan.rs`
- target home: `src/context/orchestration/service.rs` and `src/context/orchestration/plan.rs`

### CLI route concerns

- current area: `src/tooling/cli.rs` context handlers
- target home: stay in cli route layer only, with service delegation to context domain

## API Compatibility Strategy

Keep existing API entrypoints while moving implementation ownership.

- `src/api.rs` delegates context query and mutation calls to `src/context/facade.rs`
- call sites migrate incrementally to context domain services
- wrappers are removed only after parity tests pass

## Migration Plan

1. create `src/context` module tree with facade and type contracts
2. move query logic from `src/api.rs` and `src/views.rs` into context query modules
3. move composition logic into context query composition module
4. move queue logic from `src/frame/queue.rs` into context queue modules
5. move generation orchestrator and plan into context orchestration modules
6. move mutation use cases from `src/api.rs` into context mutation module
7. keep CLI shell as thin route and output adapter
8. keep API wrappers for compatibility until parity suite is green

## Test Plan

### Behavior parity coverage

- context get parity for path and node id targets
- context generate parity for recursive and single target modes
- queue parity for dedupe retry and completion behavior
- mutation parity for frame creation and head update behavior

### Boundary coverage

- guard tests confirm CLI does not own context orchestration logic
- guard tests confirm queue callers use context queue contracts
- guard tests confirm provider calls go through provider domain contracts

### Contract coverage

- deterministic output field checks for context text and json flows
- deterministic selection ordering checks for context query
- context facade wrapper parity checks during migration

## Acceptance Criteria

- context concerns are owned by `src/context` modules
- queue and orchestration are submodules of context domain
- CLI context handlers are thin routes with service delegation
- API compatibility wrappers preserve behavior during migration
- characterization and parity suites pass

## Risks And Mitigation

- risk: import churn and temporary duplicate logic during staged moves
- mitigation: keep wrappers thin and remove old logic in small steps

- risk: queue behavior drift during module relocation
- mitigation: characterization tests for dedupe retry and sync wait paths

- risk: boundary leakage from CLI into context domain policy
- mitigation: route guard tests and explicit ownership rules in this spec

## Deliverables

- context domain structure under `src/context`
- migrated query mutation queue and orchestration modules
- compatibility wrappers in API facade during transition
- updated refactor specs that point context concerns to context domain modules
