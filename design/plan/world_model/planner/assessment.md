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

## Spec-extracted open items, 2026-08-12

These open items were extracted from the planner spec's per-type open-gap subsections on 2026-08-12. The spec keeps the durable type and pipeline contracts; the open implementation questions live here.

- `DecisionContext`: `SubjectScope`, `BranchScope`, `ReferenceTime`, `TransactionTime`, `BeliefPredicate`, `InterventionRef`, and `DecisionHorizon` need shared contracts.
- `ViewSnapshot`: source cursor shape is not yet shared across graph, belief, causation, and regime.
- `PlannerAgentLens`: agent lens contracts are not yet defined as typed public records.
- `PlannerGraphInput`: `ObjectHistory`, `BranchPresence`, and graph source cursor contracts are not fully exposed in current implementation.
- `PlannerBeliefInput`: the belief layer needs typed public records for posterior, uncertainty, precision, freshness, contradiction, origin, coverage, and assessment state.
- `PlannerCausalInput`: the causation layer needs read contracts for causal variables, effect estimates, identification status, confounder risk, selection warnings, and assumption refs.
- `PlannerRegimeInput`: the regime layer needs typed read contracts for active segment state, mixture prediction, stress metrics, and sensitivity sets.
- `WorldModelView`: the public interface has route names but no typed response envelope with errors, warnings, cursor metadata, and partial result semantics.
- `ActionableBeliefView`: decision relevance scoring needs an explicit scale, threshold basis, and tie-break rule.
- `ObservationOpportunityView`: observation cost and channel refs need a cross-domain contract with execution capabilities.
- `PreconditionAssessment`: the boundary between world-facing condition and execution method precondition needs a shared test fixture with execution.
- `CausalEffectSummary`: causal input records are design-only and need implementation contracts.
- `RiskEnvelope`: risk severity scale, blocking threshold, and mitigation hint vocabulary are undefined.
- `AbstentionState`: execution needs a clear mapping from abstention state to planner handoff behavior.
- `ConflictSummary`: belief contradiction taxonomy needs stable typed values.
- `SensitivitySummary`: regime sensitivity and stress metrics need a threshold vocabulary.
- `AssumptionSet`: assumption refs need shared formatting across causal, regime, belief, and planner records.
- `HydrationHandle`: hydration handle dereference routes are not defined in the public interface.

Extracted the same day from the spec's pipeline-stage gap annotations:

- canonical serialization for `DecisionContext` is undefined.
- agent lens records and default lens policy are undefined.
- source domains do not yet expose all packet reads.
- version negotiation between planner and lower source packets is not specified.
- planner-specific belief packet adaptation is not yet available in code.
- the graph implementation exposes anchors, walks, and provenance, but object history, branch presence, and source cursor envelopes need public route shape.
- decision relevance scoring needs scale and threshold rules.
- shared fixtures are needed to keep planner preconditions and execution readiness from overlapping.
- evidence channel refs and observation cost records need a shared contract with execution capabilities.
- belief contradiction taxonomy needs stable typed values.
- causal source records are design-only and need public read contracts.
- effect support categories and blocked identification semantics need typed values.
- regime source records are design-only and need public read contracts.
- risk severity scale and blocking thresholds are undefined.
- flip and weakening condition vocabulary is undefined.
- assumption refs and violation effects need typed records.
- execution handoff semantics for hard and soft abstention are not yet specified.
- hydration dereference routes are not defined.
- source cursor set shape is undefined.
- typed response envelope is missing for partial views, warnings, and route metadata.

Extracted the same day, the spec's gap register table:

| Gap | Blocks | Needed owner |
|---|---|---|
| canonical `DecisionContext` serialization | cache keys, replay keys, route idempotence | planner |
| source cursor set format | deterministic replay across layers | graph, belief, causation, regime |
| typed agent lens contract | perspective-scoped admissibility | agent |
| typed belief view contract | belief input projection | belief |
| typed causal output contract | causal effect summary and abstention | causation |
| typed regime output contract | risk, sensitivity, expiry | regime |
| hydration dereference routes | explanation and execution handoff | public interface |
| decision relevance scale | ranking and blocking | planner |
| risk severity scale | abstention and execution handoff | planner |
| partial result semantics | robust query behavior during missing lower-layer data | public interface |
| context canonicalization | route idempotence and cache keys | planner |
| agent lens read port | admissibility filtering | agent |
| graph packet contract | graph input projection | graph |
| belief packet contract | belief projections | belief |
| causal packet contract | causal summaries | causation |
| regime packet contract | risk, sensitivity, expiry | regime |
| source cursor set | replay and snapshot | all source layers |
| score thresholds | ranking, blocking, abstention tests | planner |
| execution handoff contract | abstention and precondition consumption | execution, planner |
| partial response envelope | robust public routes | public interface |
