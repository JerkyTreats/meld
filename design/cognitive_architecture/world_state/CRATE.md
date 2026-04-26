# World Model Crate

Date: 2026-04-26
Status: declarative
Scope: `meld-world-model` crate for graph, anchors, provenance, and planner facing world model reads

## Identity

`meld-world-model` is the source of truth for graph materialization and legacy world state claim projections.
Root `meld` consumes this crate through a compatibility shim in [src/world_state.rs](../../../src/world_state.rs).

The live implementation is in:

- [crates/meld-world-model/src/lib.rs](../../../crates/meld-world-model/src/lib.rs)
- [crates/meld-world-model/src/world_state.rs](../../../crates/meld-world-model/src/world_state.rs)
- [crates/meld-world-model/src/world_state](../../../crates/meld-world-model/src/world_state)

The product facing module name remains `world_state` for compatibility even though the authority crate name is `meld-world-model`.

## Owns

- graph contracts
- traversal store and query
- current anchor selection
- anchor lineage
- graph walk queries
- planner facing query runtime
- legacy claim projections
- world state claim storage
- evidence attachment and provenance queries

## Does Not Own

- canonical event append
- execution policy
- provider execution
- workspace source truth
- context source truth
- task dispatch
- CLI formatting

## Public Surface

Primary exports are:

- graph contracts and query types from `world_state::graph`
- claim and evidence contracts from `world_state::contracts`
- `WorldModelQueries`
- `WorldStateQuery`

Compatibility only surfaces remain available through root `meld::compat`:

- `GraphRuntime`
- `TraversalStore`
- `WorldStateStore`

## Dependency Rule

`meld-world-model` depends on `meld-events` for replay source and object graph contracts.

`meld-world-model` does not depend on root `meld`, CLI, provider internals, context internals, workspace internals, or execution internals.

Source domains publish facts through events.
The world model reduces those facts without importing source domain reducers.

## Product Integration

Root `meld` keeps:

- branch and CLI adapters
- compatibility paths for runtime and store seams
- product specific presentation and routing

The authority implementation no longer lives under `src/world_state`.
