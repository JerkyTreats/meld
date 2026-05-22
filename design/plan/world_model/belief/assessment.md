# Belief Readiness Assessment

Status: conditionally ready
Depends on: `design/plan/events/assessment.md`, `design/plan/world_model/graph/assessment.md`
Design source: `design/cognitive_architecture/world_model/belief/README.md`, `design/cognitive_architecture/world_model/belief/belief_families.md`, `design/cognitive_architecture/world_model/belief/fact_to_belief.md`, `design/cognitive_architecture/world_model/belief/comparator_model.md`, `design/cognitive_architecture/world_model/belief/substrate.md`, `design/cognitive_architecture/world_model/belief/curation.md`, `design/cognitive_architecture/world_model/belief/microarchitecture.md`, `design/cognitive_architecture/world_model/belief/ECS.md`
Evidence date: 2026-05-21

## Verdict Summary

Belief is conditionally ready for one belief family and one comparator path.

The first belief family is `docs_freshness`.

## Conceptual Correctness

Belief solves credibility, uncertainty, freshness, contradiction, and observation need over graph state.

Graph says what is current. Belief says how credible and action-relevant that current state is.

## Completeness

The ready slice covers one `docs_freshness` belief key, one evidence normalization path, one comparator path, one revision record, and one compact planner-facing belief value.

Belief produces values that world model planner converts into `WorldState` propositions.

Belief does not create `Goal`, `Method`, `Composition`, or task-network work.

## Boundary Clarity

Belief owns belief identity, evidence normalization, priors, posteriors, uncertainty, precision, freshness, contradiction, supersession, hypothesis sets, calibration, observation opportunities, and planner-facing belief values.

Belief does not own graph anchors, event append, causal claims, regime identity, goal lifecycle, task dispatch, or execution side effects.

## Dependency Readiness

Events and graph are ready.

Belief can consume current anchors, provenance, object history, graph walks, and promoted outcome facts.

## First-Slice Feasibility

Belief supports the typed loop by producing a `docs_freshness` confidence value below `0.7`.

World model planner projects that value into `Proposition::Holds` inside `WorldState`.

## Current Implementation Evidence

- `crates/meld-world-model/src/world_state/legacy_claims.rs`
- `crates/meld-world-model/src/world_state/query.rs`
- `crates/meld-world-model/src/world_state/projection.rs`
- `crates/meld-world-model/src/world_state/graph/`
- `tests/integration/world_state_graph.rs`
- `tests/integration/execution_projection.rs`
- `design/cognitive_architecture/world_model/belief/README.md`
- `design/cognitive_architecture/world_model/belief/belief_families.md`
- `design/cognitive_architecture/world_model/belief/fact_to_belief.md`
- `design/cognitive_architecture/world_model/belief/comparator_model.md`

## Gaps

- Full prior and posterior record shape remains outside the first slice.
- Broad comparator catalog remains outside the first slice.
- Outcome-driven calibration requires the outcome publication bridge.
- Multi-agent divergence requires explicit perspective handling beyond the first key.

## Open Questions

- Which graph evidence fields feed the first `docs_freshness` comparator.
- Which compact fields must be carried into planner projection.

## Recommendation

Proceed with the `docs_freshness` scope cut.
