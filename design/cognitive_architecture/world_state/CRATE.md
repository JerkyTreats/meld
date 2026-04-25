# World Model Crate

Date: 2026-04-22
Status: active experiment
Scope: `meld-world-model` crate boundary for graph, anchors, provenance, belief, and planner-facing views

## Intent

`meld-world-model` owns the system world model.
It materializes event history into graph anchors, provenance, evidence, belief revisions, and planner-facing views.

The current Rust module name is `world_state`.
That name remains a compatibility path until the crate migration is complete.

## Target Crate

`meld-world-model`

Rust import name:

```rust
meld_world_model
```

## Owns

- graph contracts
- traversal store and query
- current anchor selection
- anchor lineage
- provenance bundles
- branch annotated world-model reads
- legacy claim compatibility during migration
- belief keys
- evidence attachment
- belief revision
- belief views for planning

## Does Not Own

- event append and sequencing
- source-domain truth for workspace, context, task, or workflow
- planner policy
- task dispatch
- provider execution
- CLI formatting

## Current Code Areas

- `src/world_state.rs`
- `src/world_state/contracts.rs`
- `src/world_state/events.rs`
- `src/world_state/graph.rs`
- `src/world_state/graph`
- `src/world_state/legacy_claims.rs`
- `src/world_state/projection.rs`
- `src/world_state/query.rs`
- `src/world_state/reducer.rs`
- `src/world_state/store.rs`

## Extraction Blockers

- graph reduction directly calls source-domain reducer functions in `workspace`, `context`, and `task`
- public exports expose `TraversalStore` and `WorldStateStore` too broadly
- belief contracts are not yet stable enough for a public crate boundary
- the module and docs still mix world-state and world-model language

## Target Dependencies

| From | To | Reason |
| --- | --- | --- |
| `meld-world-model` | `meld-events` | replay source, object refs, relation edges, derived fact append |

## Forbidden Direction

`meld-world-model` must not depend on `meld-execution`, root `meld`, CLI, provider, task internals, context internals, or workspace internals.

Source domains publish event facts.
The world model consumes those facts through `meld-events`.

## Migration Path

1. Rename docs to World Model while keeping `world_state` as the current code path.
2. Move world-model contracts behind a narrow public API.
3. Invert graph reducer source hooks so source domains do not get imported by graph reduction.
4. Extract graph and legacy claim compatibility into `meld-world-model`.
5. Add belief internals after `BeliefKey` and `BeliefView` are stable.
6. Add a temporary `world_state` compatibility re-export from root `meld`.

