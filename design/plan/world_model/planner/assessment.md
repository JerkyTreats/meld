# Planner Projection Readiness Assessment

Status: complete for first slice
Depends on: `design/plan/world_model/graph/assessment.md`, `design/plan/world_model/belief/assessment.md`, `design/plan/meld-lang/assessment.md`
Design source: `design/cognitive_architecture/world_model/planner/README.md`, `design/cognitive_architecture/world_model/planner/spec.md`, `design/cognitive_architecture/world_model/public_interface.md`, `design/cognitive_architecture/meld-lang/world_state.md`
Evidence date: 2026-05-27

## Verdict Summary

Planner projection first slice is implemented for graph plus belief projection into `meld-lang::WorldState`.

It is not ready for full causal effect summaries, regime sensitivity summaries, broad risk envelopes, or complete abstention scoring.

Runtime projection from `BeliefView` into `meld-lang::WorldState` now exists in `meld-world-model`.

## Conceptual Correctness

Planner projection solves the boundary between internal world model state and execution-readable formal state.

The implemented first slice projects directly from `BeliefView` plus graph scope into `WorldState`. Broad `WorldModelView` remains deferred for richer future projection assembly.

## Completeness

The first projection emits:

- `Proposition::Holds` for configured belief confidence
- `Proposition::Holds` for generic stale state
- `Proposition::Holds` for generic observation-needed state
- `Proposition::Accessible` for the docs node scope when the scope is available
- source refs and hydration refs for belief and graph provenance

This is enough for the typed loop.

## Boundary Clarity

Planner projection owns action-relevant world model reads, belief summaries, uncertainty summaries, freshness summaries, contradiction summaries, observation opportunities, and the projection into `WorldState`.

Planner projection does not own task graphs, dispatch rules, continuation state, repair flow, retry policy, providers, or execution control semantics.

## Dependency Readiness

Graph is ready. Belief has landed the externally configured `docs_freshness` slice. `meld-lang` is ready for typed-loop values and operations.

Causation and regime are deferred for this slice.

## First-Slice Feasibility

The first slice projects one `WorldState` containing the docs node and its externally configured `docs_freshness` belief value.

Execution can evaluate the resulting `Goal` target mechanically.

This slice is implemented and verified.

## Current Implementation Evidence

- `crates/meld-world-model/src/world_state/query.rs`
- `crates/meld-world-model/src/world_state/query_runtime.rs`
- `crates/meld-world-model/src/world_state/graph/`
- `crates/meld-world-model/src/belief.rs`
- `crates/meld-world-model/src/belief/`
- `crates/meld-world-model/src/planner.rs`
- `crates/meld-world-model/src/planner/`
- `crates/meld-world-model/tests/belief.rs`
- `crates/meld-world-model/tests/planner.rs`
- `crates/meld-world-model/fuzz/fuzz_targets/fuzz_planner_projection_contract.rs`
- `tests/integration/world_state_graph.rs`
- `design/cognitive_architecture/world_model/planner/README.md`
- `design/cognitive_architecture/world_model/planner/spec.md`
- `design/cognitive_architecture/world_model/public_interface.md`
- `design/cognitive_architecture/meld-lang/world_state.md`

## Gaps

- Broad `WorldModelView` runtime remains deferred.
- View expiry and replay boundary semantics for broad planner views need implementation shape.
- Causal and regime fields remain deferred.
- Execution runtime consumption remains blocked beyond typed-loop evaluation.

## Open Questions

- How broad planner views should layer over the first direct `BeliefView` projection route.
- How execution outcomes become belief evidence after outcome publication is specified.

## Recommendation

Proceed with agent goal curation against the projected `WorldState`. Defer broad planner view, causal, and regime expansion.
