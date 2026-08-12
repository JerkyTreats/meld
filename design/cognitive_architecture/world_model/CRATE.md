# World Model Crate

Date: 2026-05-27
Status: active
Scope: `meld-world-model` crate for graph, anchors, provenance, belief assessment, and planner-facing world model projection

## Identity

`meld-world-model` is the source of truth for graph materialization, belief assessment, planner projection, and legacy world state claim projections.
Root `meld` consumes this crate through a compatibility shim in [src/world_state.rs](../../../src/world_state.rs).

Crate routing:

- [crates/meld-world-model/src/lib.rs](../../../crates/meld-world-model/src/lib.rs)
- [crates/meld-world-model/src/world_state.rs](../../../crates/meld-world-model/src/world_state.rs)
- [crates/meld-world-model/src/world_state](../../../crates/meld-world-model/src/world_state)
- [crates/meld-world-model/src/belief.rs](../../../crates/meld-world-model/src/belief.rs)
- [crates/meld-world-model/src/belief](../../../crates/meld-world-model/src/belief)
- [crates/meld-world-model/src/planner.rs](../../../crates/meld-world-model/src/planner.rs)
- [crates/meld-world-model/src/planner](../../../crates/meld-world-model/src/planner)

The product facing module name remains `world_state` for compatibility even though the authority crate name is `meld-world-model`.

## Owns

- graph contracts
- traversal store and query
- current anchor selection
- anchor lineage
- graph walk queries
- planner-facing query runtime
- planner projection contracts
- `BeliefView` to `meld-lang::WorldState` projection
- planner source refs and hydration refs
- runtime belief family configuration loading
- belief evidence normalization and assignment
- belief comparator assessment
- append-only belief revisions
- planner-safe belief views
- assessment leases, recovery, dirty keys, and storm coalescing
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
- belief contracts, store, runtime, comparator, normalizer, and query facade from `belief`
- planner contracts, `project_world_state`, and `PlannerQuery` from `planner`
- claim and evidence contracts from `world_state::contracts`
- `WorldModelQueries`
- `WorldStateQuery`

Compatibility only surfaces remain available through root `meld::compat`:

- `GraphRuntime`
- `TraversalStore`
- `WorldStateStore`

## Dependency Rule

`meld-world-model` depends on `meld-events` for replay source and object graph contracts.

`meld-world-model` depends on `meld-lang` for ground `WorldState`, `Proposition`, `Condition`, and `Term` values.

`meld-world-model` does not depend on root `meld`, CLI, provider internals, context internals, workspace internals, or execution internals.

Source domains publish facts through events.
The world model reduces those facts without importing source domain reducers.

## Product Integration

Root `meld` keeps:

- branch and CLI adapters
- compatibility paths for runtime and store seams
- product specific presentation and routing
