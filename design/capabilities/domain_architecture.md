# Domain Architecture

Date: 2026-04-03
Status: draft
Scope: durable module boundaries for capability, task, provider execution, and domain capability publication

See [Capability And Task Design](README.md), [Capability Model](capability/README.md), [Task Network](../control/task_network.md), and [Control Design](../control/README.md).

## Core Position

If today’s workflow files become structured graph definitions, they are better modeled as `task` definitions than as a durable `workflow` domain.

That implies this shape:

- `capability`
  shared contract vocabulary and catalog
- `task`
  definition loading, normalization, validation, and compilation input
- `provider`
  provider-service execution for ready work
- `control`
  orchestration and batch release above ready work
- `<domain>/capability.rs`
  domain-owned capability publication and wiring

`workflow` should become a compatibility path, not the durable architecture center.

## Important Constraint

Do not introduce a generic top-level `src/graph`.

Reason:

- `graph` is a technical substrate, not a domain
- repository governance already prefers domain-first organization
- compiled graph structure belongs to `task`, not to a generic graph layer

If graph helpers are needed, they should live under a domain-owned path such as:

- `src/task/graph.rs`
- `src/task/compiler/graph.rs`

## Recommended Domain Split

### `src/capability`

Own the shared contract model used across all domains.

Recommended responsibilities:

- capability ids and version types
- scope, binding, input, output, effect, and execution contract types
- artifact contract ids and schema compatibility rules
- capability catalog assembly
- compiler-facing capability lookup interfaces

This domain should not own domain behavior.
It owns the vocabulary and catalog only.

### `src/task`

Own the authored task definition and its normalization path.

This is where structured YAML or JSON should enter the system.
A task file should load into a task-owned definition type, not straight into runtime execution.

Recommended responsibilities:

- task definition file schema
- parse and load
- normalize and canonicalize
- validate authored structure
- own compiled task record shape
- own graph structure used by task compilation
- own task-scoped artifact repo contract and persistence model
- own capability invocation record schema
- own artifact lookup and slot satisfaction data plane
- own task run creation from compiled task plus initialization payload
- compatibility lowering from old workflow files
- compile request assembly for downstream compiler

Recommended output types:

- `TaskDefinition`
- `TaskCompileInput`
- `TaskLoadError`
- `CompiledTaskRecord`
- `ArtifactRepoRecord`
- `TaskInitializationPayload`

This domain is also the natural home for integration fixtures.
A docs-writer definition should be a task fixture that compiles and then executes through the same path as production inputs.
That fixture should include task-owned artifact repo behavior, not only graph shape.

### `src/provider`

Own provider-service interaction for ready work.

Recommended responsibilities:

- provider binding resolution
- runtime override application
- client construction
- batching of provider-compatible requests
- throttling keyed by provider service lanes
- retry and backoff for provider interaction
- response correlation and result normalization

This domain should not own task readiness or dependency order.
It should receive ready work and execute it efficiently.

### `src/control`

Own orchestration above ready work.

Recommended responsibilities:

- consume traversal batches
- resolve batch release order
- coordinate batch barriers
- hand ready work to `context` and `provider`
- preserve compatibility orchestration while workflow remains user-visible

This domain should not own traversal algorithms or provider transport internals.
It should coordinate domain contracts.

### `src/<domain>/capability.rs`

Each substantive domain should publish its capability contracts through `capability.rs`.

Examples:

- `src/context/capability.rs`
- `src/provider/capability.rs`
- `src/heads/capability.rs`

That file should expose domain-owned contract records and thin capability wiring only.
The rest of the domain continues to own implementation details behind those contracts.

## Capability Catalog Over Registry

The system does need a central capability lookup surface, but the durable concept should be a `CapabilityCatalog`, not a mutable service-shaped registry.

Recommended posture:

- domains publish static capability contract records
- startup wiring assembles those records into one catalog
- task loading and compilation reason against a catalog snapshot
- runtime execution resolves by stable capability type id from that same snapshot

Why `catalog` is the better default term:

- it emphasizes published records over live mutable state
- it matches compiler and validation use better than a service object
- it keeps task reasoning deterministic

If a mutable runtime wrapper is later useful, it can sit on top of the catalog.
The durable contract surface should still be catalog-shaped.

## Plan Boundary

`plan` should not be a first-slice implementation concern in this layer.
It belongs in `control`.

In this design set:

- `task` is the compiled capability graph unit
- `plan` is the higher-order control concern that orders, executes, or modifies tasks

That boundary matches the control docs more closely:

- capability says what atomic behavior can run
- task says what compiled graph unit will run
- plan says how tasks are ordered, executed, selected, or modified over time

## Refactor-Phase Exception

Before `task` exists as a real execution input, `control` should still be introduced.

In that interregnum:

- `control` temporarily houses orchestration extracted from `context`
- `workflow` may still exist as a compatibility trigger path
- `control` coordinates traversal batches, context preparation, provider execution, and batch barriers

This is a refactor-phase ownership move, not a late-stage feature addition.

## Suggested First Slice Layout

```rust
src/
  capability.rs
  capability/
    artifact.rs
    catalog.rs
    contract.rs
    execution.rs
    scope.rs
  task.rs
  task/
    compile.rs
    definition.rs
    graph.rs
    load.rs
    normalize.rs
    validate.rs
  context/
    capability.rs
  control.rs
  control/
    orchestration.rs
    traversal_release.rs
    batch_barrier.rs
    compatibility.rs
  provider/
    capability.rs
    executor.rs
  heads/
    capability.rs
```

This keeps the graph substrate task-owned instead of generic.

## Migration Direction

The migration path should be:

1. extract orchestration out of `context` into `control`
2. add provider-native execution contracts and executor ownership under `provider`
3. treat current workflow files as compatibility task definitions and compatibility triggers
4. add `task` loading and normalization
5. assemble a `CapabilityCatalog` from domain capability publishers
6. compile task definitions into locked compiled task records
7. hand those compiled tasks to control-owned plan execution
8. remove `workflow` as a durable runtime abstraction

## Decision Summary

- adopt `src/capability`
- adopt `src/task`
- introduce `src/control` during the refactor phase
- keep provider execution in `src/provider`
- use `<domain>/capability.rs` for domain publication
- use `CapabilityCatalog` as the central lookup surface
- do not add generic `src/graph`
