# Regime Readiness Assessment

Status: deferred
Depends on: `design/plan/world_model/graph/assessment.md`, `design/plan/world_model/belief/assessment.md`, `design/plan/world_model/causation/assessment.md`
Design source: `design/cognitive_architecture/world_model/regime/README.md`, `design/cognitive_architecture/world_model/regime/spec.md`, `design/cognitive_architecture/world_model/regime/requirements.md`
Evidence date: 2026-05-27

## Verdict Summary

Regime is deferred for the typed loop and runtime planning substrate.

Use a default stable regime assumption where planner projection needs a placeholder.

## Conceptual Correctness

Regime solves structural change, changepoints, recurring modes, archived priors, and mixture prediction.

The boundary is correct because local belief updates are not treated as structural world changes by default.

## Completeness

The first usable slice is one active segment, one continuation signal, one break signal, and an archive hook.

Full changepoint inference, recurrence matching, run length updates, mixture prediction, and structural stress scenarios are outside the first execution sequence.

## Boundary Clarity

Regime owns changepoint inference, run length beliefs, regime identity, continuation and break comparison, mixture prediction, archived priors, and structural stress scenarios.

Regime does not own raw contradiction handling, graph truth maintenance, task orchestration, generic planner goals, causal effect estimation, or belief settlement.

## Dependency Readiness

Graph is ready. Belief has landed its first slice. Causation is deferred.

Runtime flywheel can proceed first with a default stable regime assumption.

## First-Slice Feasibility

Regime is not required for the typed loop.

Regime should not be implemented before runtime flywheel behavior works without changepoint inference.

## Current Implementation Evidence

- `crates/meld-world-model/src/world_state/graph/`
- `crates/meld-world-model/src/belief/`
- `crates/meld-world-model/src/world_state/query.rs`
- `crates/meld-world-model/src/world_state/projection.rs`
- `tests/integration/world_state_graph.rs`
- `tests/integration/execution_projection.rs`
- `design/cognitive_architecture/world_model/regime/README.md`
- `design/cognitive_architecture/world_model/regime/spec.md`

## Gaps

- Changepoint scoring is not specified enough for implementation.
- Recurring regime archive lookup is not operationalized.
- Mixture prediction lacks update rules.
- Structural stress metrics are not specified.

## Open Questions

- Which belief or outcome signals count as first break pressure.
- Whether cost belief priors need regime scope before causal outcome links exist.

## Recommendation

Defer regime and use a default stable regime assumption.

## Evidence Note 2026-08-12

Evidence note 2026-08-12: the regime requirements document previously carried a first-slice and deferred split. That slice boundary is recorded here.

First slice scope: `ActiveSegment`, `ChangepointCandidate`, one continuation model, one break model, continuation versus break comparison, `RegimePosterior`, `RunLengthBelief`, active segment rollover, one recurring regime archive path, known regime entry prior selection, novel regime widened prior selection, mixture prediction, regime summary projection, and replay tests.

Deferred past the first slice: rich regime library matching, retrospective smoothing over break timing, multi-scale nested regimes, full structural stress scenario library, agent goal-prior integration beyond public prior refs, online nonparametric regime discovery, and multi-process regime worker coordination.
