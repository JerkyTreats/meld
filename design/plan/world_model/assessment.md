# World Model Readiness Assessment

Status: conditionally ready
Depends on: `design/plan/events/assessment.md`, `design/plan/meld-lang/assessment.md`, `design/plan/world_model/graph/assessment.md`, `design/plan/world_model/belief/assessment.md`, `design/plan/world_model/planner/assessment.md`, `design/plan/world_model/agent/assessment.md`, `design/plan/world_model/causation/assessment.md`, `design/plan/world_model/regime/assessment.md`
Design source: `design/cognitive_architecture/world_model/README.md`, `design/cognitive_architecture/world_model/public_interface.md`, `design/cognitive_architecture/meld-lang/README.md`
Evidence date: 2026-05-21

## Verdict Summary

World model is conditionally ready for graph to belief to planner projection to agent goal curation.

It is ready to project one `WorldState` and construct one `Goal`.

## Conceptual Correctness

The world model owns epistemic authority. It transforms event facts into graph state, graph state into belief, belief into planner-facing state, and planner-facing state into agent goal curation.

`meld-lang` is the bridge between world model and execution. It provides formal values without moving belief semantics into execution.

## Completeness

The ready slice covers:

- graph object identity and provenance
- one externally configured `docs_freshness` belief value
- one projection into `WorldState`
- one agent curation rule
- one ground `Goal`

Full causal inference, regime inference, multi-agent coordination, learned policy, and broad planner risk projection remain outside this slice.

## Boundary Clarity

World model owns graph materialization, belief views, causal and regime interpretation, planner-facing projection, agent perspective, and goal construction through public execution APIs.

World model does not own canonical event append, sensory raw lanes, execution goal storage, task decomposition, task dispatch, provider execution, workflow runtime, or telemetry summaries.

## Dependency Readiness

Events are ready.

`meld-lang` is ready for typed-loop values and operations.

Graph is ready. Belief, planner projection, and agent are conditionally ready for the first externally configured `docs_freshness` slice.

Causation and regime are deferred.

## First-Slice Feasibility

World model supports the typed loop by projecting one `WorldState` and curating one `Goal`.

Runtime loop closure remains blocked by execution runtime planning and outcome publication.

## Current Implementation Evidence

- `crates/meld-world-model/src/lib.rs`
- `crates/meld-world-model/src/world_state.rs`
- `crates/meld-world-model/src/world_state/graph.rs`
- `crates/meld-world-model/src/world_state/graph/`
- `src/world_state.rs`
- `tests/integration/world_state_graph.rs`
- `tests/integration/execution_projection.rs`
- `tests/integration/branches_query.rs`
- `tests/integration/branches_runtime.rs`
- `design/completed/world_state/README.md`
- `design/completed/world_state/graph/README.md`
- `design/cognitive_architecture/world_model/README.md`
- `design/cognitive_architecture/world_model/public_interface.md`
- `design/cognitive_architecture/meld-lang/README.md`

## Gaps

- First `WorldState` projection must be specified as concrete code.
- Belief runtime configuration loading must be specified before the first belief implementation.
- World model runtime bootstrap and concurrency model must be implemented from the consolidated domain specs.
- First agent curation rule must be implemented against projected belief.
- Causation remains deferred.
- Regime remains deferred.
- Runtime loop closure depends on execution runtime planning and outcome publication.

## Open Questions

- Whether the first projection includes confidence only or confidence plus freshness age.
- Which public world model route exposes the first `WorldState`.
- How execution outcomes become belief evidence after outcome publication is specified.

## Recommendation

Proceed with graph, belief, planner projection, and agent curation for the typed loop. Defer full causal, regime, and runtime closure behavior.
