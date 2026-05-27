# Belief Readiness Assessment

Status: first slice implemented
Depends on: `design/plan/events/assessment.md`, `design/plan/world_model/graph/assessment.md`
Design source: `design/cognitive_architecture/world_model/belief/README.md`, `design/cognitive_architecture/world_model/belief/belief_families.md`, `design/cognitive_architecture/world_model/belief/fact_to_belief.md`, `design/cognitive_architecture/world_model/belief/comparator_model.md`, `design/cognitive_architecture/world_model/belief/substrate.md`, `design/cognitive_architecture/world_model/belief/curation.md`, `design/cognitive_architecture/world_model/belief/microarchitecture.md`, `design/cognitive_architecture/world_model/belief/spec.md`
Evidence date: 2026-05-27

## Verdict Summary

Belief design has landed its first externally configured belief family path and one comparator engine path.

The first loaded belief family configuration is `docs_freshness`.

The implementation lives in `crates/meld-world-model/src/belief.rs` and `crates/meld-world-model/src/belief/*.rs`.

## Conceptual Correctness

Belief solves credibility, uncertainty, freshness, contradiction, and observation need over graph state.

Graph says what is current. Belief says how credible and action-relevant that current state is.

## Completeness

The landed slice covers one externally configured `docs_freshness` belief key, graph-anchor evidence normalization, promoted outcome evidence normalization, one generic comparator engine path, append-only revisions, and compact planner-facing belief views.

Belief produces values that world model planner converts into `WorldState` propositions.

Belief does not create `Goal`, `Method`, `Composition`, or task-network work.

## Boundary Clarity

Belief owns belief identity, runtime family configuration loading, evidence normalization, priors, posteriors, uncertainty, precision, freshness, contradiction, supersession, hypothesis sets, calibration, observation opportunities, and planner-facing belief values.

Belief does not own graph anchors, event append, causal claims, regime identity, goal lifecycle, task dispatch, or execution side effects.

## Dependency Readiness

Events and graph are ready.

Belief can consume current anchors, provenance, object history, graph walks, and promoted outcome facts.

## First-Slice Feasibility

Belief supports the typed-loop design path by loading `docs_freshness` runtime configuration and producing a configured confidence value below `0.7`.

World model planner projects that value into `Proposition::Holds` inside `WorldState`.

## Current Implementation Evidence

- `crates/meld-world-model/src/belief.rs`
- `crates/meld-world-model/src/belief/contracts.rs`
- `crates/meld-world-model/src/belief/config.rs`
- `crates/meld-world-model/src/belief/evidence.rs`
- `crates/meld-world-model/src/belief/comparator.rs`
- `crates/meld-world-model/src/belief/store.rs`
- `crates/meld-world-model/src/belief/runtime.rs`
- `crates/meld-world-model/src/belief/query.rs`
- `crates/meld-world-model/tests/belief.rs`
- `design/plan/world_model/belief/PLAN.md`
- `design/cognitive_architecture/world_model/belief/README.md`
- `design/cognitive_architecture/world_model/belief/belief_families.md`
- `design/cognitive_architecture/world_model/belief/fact_to_belief.md`
- `design/cognitive_architecture/world_model/belief/comparator_model.md`
- `design/cognitive_architecture/world_model/belief/spec.md`

## Gaps

- Full prior and posterior record shape remains outside the first slice.
- Broad comparator catalog remains outside the first slice.
- Runtime family configuration loading and replay snapshotting are implemented for the first slice only.
- Outcome-driven calibration requires the outcome publication bridge.
- Multi-agent divergence requires explicit perspective handling beyond the first key.
- Planner projection into `meld-lang::WorldState` is not implemented here.
- Semantic settlement and rule comparator paths remain deferred.

## Open Questions

- Which public world-model route should expose the first belief-backed planner projection.
- How execution outcomes become promoted belief evidence after outcome publication is specified.

## Recommendation

Proceed to planner projection and typed-loop integration on top of the landed `docs_freshness` belief slice.
