# Cognitive Architecture Implementation Plan

Date: 2026-05-27
Status: active
Scope: declarative implementation readiness and dependency order for the cognitive architecture

## Purpose

This directory defines implementation readiness, dependency order, scope cuts, and contract closure for the cognitive architecture.

Durable architecture lives under `design/cognitive_architecture`. This plan states what can be built, what must be built first, and what remains blocked.

Each assessment is current truth. It names the exact slice that is ready, the exact contracts it depends on, and the exact work that is deferred.

## Strategy

The first vertical slice threads the thinnest possible path through every layer of the cognitive flywheel using the `docs_freshness` scenario. Each layer implements the minimum needed to pass its output to the next. The goal is one complete flywheel turn before deepening any individual layer.

The typed loop has already proven the full contract chain through `meld-lang` pure types and operations. The vertical slice now makes each layer real with runtime code that materializes state, projects belief, curates goals, and dispatches work.

## Vertical Slice: Implementation Order

The `docs_freshness` scenario threads through every phase. Each phase produces the input the next phase consumes.

### Phase 1: Shared Language — `complete`

Pure types and operations shared between world model and execution. `Term`, `Proposition`, `Condition`, `Effect`, `WorldState`, `Goal`, `Method`, `Composition`, `Operator`. Pure operations: `evaluate`, `unify`, `substitute`, `validate`, `apply_effects`. Full evaluation loop proven end-to-end in integration tests.

Owner: `meld-lang`
Plan: [meld-lang/PLAN.md](meld-lang/PLAN.md)

### Phase 2: World Model Graph — `complete`

Materialize current anchors from event spine facts. Subject identity lookup for a docs node, current anchor reads, provenance. Enough for belief to consume.

Owner: `meld-world-model`

### Phase 3: Belief Layer — `complete`

One belief family: `docs_freshness`. Graph anchor and promoted evidence normalization. One configured comparator that produces a confidence value. One compact planner-facing belief view. First layer that exercises epistemic judgment.

Owner: `meld-world-model`
Blocked by: Phase 2
Plan: [world_model/belief/PLAN.md](world_model/belief/PLAN.md)

### Phase 4: Planner Projection — `not started` ← next

Convert public graph and belief views into ground `meld-lang::WorldState`. First projection emits `Proposition::Holds` for `docs_freshness` confidence and freshness state. Bridge that makes world model state consumable by execution.

Owner: `meld-world-model`
Blocked by: Phase 3

### Phase 5: Agent Goal Curation — `not started`

One agent, one perspective, one curation rule: if `docs_freshness` confidence is below 0.7, create an active `Goal` requiring confidence above 0.7. First point where the system generates operational intent from belief.

Owner: `meld-world-model`
Blocked by: Phase 4

### Phase 6: Execution Planning Runtime — `not started`

Method library loading. Goal evaluation against `WorldState`, method matching via `unify`, composition preparation via `substitute` and `validate`. Bridges from typed planning substrate to runtime orchestration. Task network execution is deferred. First slice stops at a validated composition ready for dispatch.

Owner: `meld-execution`
Blocked by: Phase 5

### Phase 7: Task Dispatch and Outcome — `not started`

Bridge validated composition to the existing task and capability engine. Dispatch one task. Publish outcome events to the spine. World model reducer consumes them. The flywheel turns once.

Owner: `meld-execution`
Blocked by: Phase 6

### Phase 8: Sensory — `not started`

Diff-native observation for the docs node. Publishes to the event spine. Closes the loop: the system observes changes it caused and re-enters the cycle.

Owner: sensory domain
Parallel with: Phases 6 and 7. Requires only the event spine contract.

## Foundation

These components predate the vertical slice and support all phases.

| Component | Status | Owner |
|---|---|---|
| `events` | complete | `meld-events` |
| `integration/typed_loop` | complete | `meld-lang` integration tests |

## Dependency Order

1. `events` — complete
2. `meld-lang` — complete, Phase 1
3. `world_model/graph` — complete, Phase 2
4. `world_model/belief` — complete, Phase 3
5. `world_model/planner` — not started, Phase 4
6. `world_model/agent` — not started, Phase 5
7. `execution/goals` — type contract complete in `meld-lang`, runtime Phase 6
8. `integration/typed_loop` — complete
9. `execution/planning` — type substrate complete in `meld-lang`, runtime Phase 6
10. `execution/dispatch` — not started, Phase 7
11. `sensory` — not started, Phase 8
12. `world_model/causation` — deferred past vertical slice
13. `world_model/regime` — deferred past vertical slice
14. `world_model` — full integration deferred
15. `execution` — full integration deferred

## Implementation Plans

Implementation plans decompose assessed areas into phased, dependency-ordered work with tasks, exit criteria, and verification commands.

- [meld-lang/PLAN.md](meld-lang/PLAN.md) — complete
- [world_model/belief/PLAN.md](world_model/belief/PLAN.md) — complete
- [world_model/planner/PLAN.md](world_model/planner/PLAN.md) — ready to execute, next required

## Assessment Inventory

- `design/plan/events/assessment.md`
- `design/plan/meld-lang/assessment.md`
- `design/plan/world_model/graph/assessment.md`
- `design/plan/world_model/belief/assessment.md`
- `design/plan/world_model/planner/assessment.md`
- `design/plan/world_model/agent/assessment.md`
- `design/plan/execution/goals/assessment.md`
- `design/plan/integration/typed_loop.md`
- `design/plan/execution/planning/assessment.md`
- `design/plan/sensory/assessment.md`
- `design/plan/world_model/causation/assessment.md`
- `design/plan/world_model/regime/assessment.md`
- `design/plan/world_model/assessment.md`
- `design/plan/execution/assessment.md`

## Scope Cuts

The vertical slice excludes full causal effect summaries, regime sensitivity summaries, broad risk envelopes, multi-agent divergence, learned normative policy, multi-agent coordination, goal conflict resolution, broad utility estimation, task network graph execution, graph mutation acceptance, plan diffing, and switching cost model.

Each phase implements the minimum needed for one `docs_freshness` flywheel turn. Deepening happens after the loop turns once end-to-end.

## Blocked Areas

- task network graph executor
- graph mutation acceptance
- plan diffing
- switching cost model
- workflow integration strategy
- causal effect estimation
- regime inference

## Assessment Shape

Each assessment includes:

- verdict summary
- conceptual correctness
- completeness
- boundary clarity
- dependency readiness
- first-slice feasibility
- current implementation evidence
- gaps
- open questions
- recommendation

## Validation Rules

Plan files must avoid hidden chronology and stale state language. They must state current truth only.

Plan files must avoid literal parenthesis characters in Markdown prose.

Every readiness verdict must name its exact scope.

Every deferred runtime concern must be explicit.
