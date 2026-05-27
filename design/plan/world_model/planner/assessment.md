# Planner Projection Readiness Assessment

Status: conditionally ready
Depends on: `design/plan/world_model/graph/assessment.md`, `design/plan/world_model/belief/assessment.md`, `design/plan/meld-lang/assessment.md`
Design source: `design/cognitive_architecture/world_model/planner/README.md`, `design/cognitive_architecture/world_model/planner/spec.md`, `design/cognitive_architecture/world_model/public_interface.md`, `design/cognitive_architecture/meld-lang/world_state.md`
Evidence date: 2026-05-27

## Verdict Summary

Planner projection design is conditionally ready for graph plus belief projection into `meld-lang::WorldState`.

It is not ready for full causal effect summaries, regime sensitivity summaries, broad risk envelopes, or complete abstention scoring.

Runtime projection from belief view into `meld-lang::WorldState` is not implemented.

## Conceptual Correctness

Planner projection solves the boundary between internal world model state and execution-readable formal state.

`WorldModelView` remains the world model side of projection assembly. `WorldState` is the cross-domain contract consumed by execution.

## Completeness

The first projection emits:

- `Proposition::Holds` for `docs_freshness`
- `Proposition::Accessible` for the docs node scope when the scope is available
- `Proposition::Related` only when required by the first method precondition

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

This is a design-ready slice. The code path does not yet exist.

## Current Implementation Evidence

- `crates/meld-world-model/src/world_state/query.rs`
- `crates/meld-world-model/src/world_state/query_runtime.rs`
- `crates/meld-world-model/src/world_state/graph/`
- `crates/meld-world-model/src/belief.rs`
- `crates/meld-world-model/src/belief/`
- `crates/meld-world-model/tests/belief.rs`
- `tests/integration/world_state_graph.rs`
- `design/cognitive_architecture/world_model/planner/README.md`
- `design/cognitive_architecture/world_model/planner/spec.md`
- `design/cognitive_architecture/world_model/public_interface.md`
- `design/cognitive_architecture/meld-lang/world_state.md`

## Gaps

- First projection fields must be implemented as concrete code contracts.
- Projection from `BeliefView` to `meld-lang::WorldState` does not exist yet.
- View expiry and replay boundary semantics need implementation shape.
- Causal and regime fields remain deferred.
- Execution runtime consumption remains blocked beyond typed-loop evaluation.

## Open Questions

- Whether first projection emits confidence plus freshness state as separate propositions or a single structured proposition.
- Whether first projection includes observation-needed state directly or leaves it to `Indeterminate`.

## Recommendation

Proceed with graph plus belief projection into `WorldState`.
