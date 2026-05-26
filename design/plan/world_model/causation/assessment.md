# Causation Readiness Assessment

Status: deferred
Depends on: `design/plan/events/assessment.md`, `design/plan/world_model/graph/assessment.md`, `design/plan/world_model/belief/assessment.md`
Design source: `design/cognitive_architecture/world_model/causation/README.md`, `design/cognitive_architecture/world_model/causation/spec.md`, `design/cognitive_architecture/world_model/causation/requirements.md`
Evidence date: 2026-05-26

## Verdict Summary

Causation is deferred for the typed loop and runtime planning substrate.

It becomes relevant after outcome publication exists.

## Conceptual Correctness

Causation solves the distinction between temporal evidence, intervention, measured outcome, selection path, confounding, and causal effect.

The boundary is correct because graph replacement and event order are not treated as causal proof.

## Completeness

The first usable slice is intervention and outcome link recording.

Causal effect estimation, counterfactual query semantics, confounder discovery, and mechanism versioning are outside the first execution sequence.

## Boundary Clarity

Causation owns causal variable families, intervention semantics, outcome semantics, selection and measurement semantics, confounder hypotheses, effect estimates, uncertainty, and counterfactual query contracts.

Causation does not own event append, temporal truth materialization, generic belief freshness, planner policy ranking, task dispatch, or regime identity.

## Dependency Readiness

Events and graph are ready.

Belief is conditionally ready for externally configured `docs_freshness`.

Outcome publication is not specified, so causation cannot participate in loop closure yet.

## First-Slice Feasibility

Causation is not required for the typed loop.

Runtime flywheel can close first through outcome facts and belief revision without causal effect estimation.

## Current Implementation Evidence

- `crates/meld-events/src/events.rs`
- `crates/meld-world-model/src/world_state/graph/`
- `crates/meld-execution/src/task.rs`
- `crates/meld-execution/src/workflow.rs`
- `tests/integration/event_spine.rs`
- `tests/integration/task_executor.rs`
- `tests/integration/execution_projection.rs`
- `design/cognitive_architecture/world_model/causation/README.md`
- `design/cognitive_architecture/world_model/causation/spec.md`

## Gaps

- Outcome publication bridge is required before causation can record intervention and outcome links.
- Effect estimation is not specified enough for implementation.
- Counterfactual query semantics are not operationalized.
- Confounder handling is not specified enough for implementation.

## Open Questions

- Which outcome facts become candidate intervention records.
- Which selection warnings are required for first belief calibration.

## Recommendation

Defer causation until outcome publication exists.
