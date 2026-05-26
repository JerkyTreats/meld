# Graph Readiness Assessment

Status: ready
Depends on: `design/plan/events/assessment.md`
Design source: `design/cognitive_architecture/world_model/graph/README.md`, `design/cognitive_architecture/world_model/graph/spec.md`, `design/cognitive_architecture/world_model/graph/requirements.md`
Evidence date: 2026-05-21

## Verdict Summary

Graph is ready for the cognitive architecture plan.

The ready slice covers current anchors, lineage, provenance, relation adjacency, branch-scoped reads, and graph traversal over event facts.

## Conceptual Correctness

Graph solves replayable materialized state and relationship traversal without claiming belief, confidence, causation, regime identity, or execution policy.

This boundary lets upper world model layers consume auditable state while preserving separate authority for belief and planner projection.

## Completeness

The graph query families are complete enough for typed-loop support.

Graph provides subject identity, current anchor, provenance, and relation structure. It does not project directly to execution.

World model planner owns the conversion from graph plus belief state into `meld-lang::WorldState`.

## Boundary Clarity

Graph owns current anchor selection, anchor lineage, provenance, object identity, relation adjacency, bounded walks, object history, branch presence, branch-scoped federation, and replayable materialization from event facts.

Graph does not own belief confidence, contradiction handling, calibration, Bayesian revision, causal inference, regime detection, planner policy, task dispatch, or source-domain internals.

## Dependency Readiness

Events are ready and provide the object and relation facts that graph consumes.

No upstream blocker prevents the typed-loop slice.

## First-Slice Feasibility

Graph supports the typed loop by identifying the docs node subject and making it available to belief and planner projection.

Graph supports the runtime flywheel once sensory and execution publish graph-readable facts.

## Current Implementation Evidence

- `crates/meld-world-model/src/world_state/graph.rs`
- `crates/meld-world-model/src/world_state/graph/`
- `crates/meld-world-model/tests/world_model_queries.rs`
- `src/world_state.rs`
- `tests/integration/world_state_graph.rs`
- `tests/integration/branches_query.rs`
- `tests/integration/branches_runtime.rs`
- `tests/integration/execution_projection.rs`
- `design/completed/world_state/graph/README.md`

## Gaps

- Compatibility naming around `world_state` remains visible in code and must be handled at adapter boundaries.
- Source domains must publish graph-readable objects and relations for runtime flywheel coverage.

## Open Questions

- Which graph query becomes the first canonical input to `docs_freshness` belief projection.
- Which compatibility names remain public during world model crate cleanup.

## Recommendation

Proceed.
