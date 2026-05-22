# Cognitive Architecture Implementation Plan

Date: 2026-05-21
Status: active
Scope: declarative implementation readiness and dependency order for the cognitive architecture

## Purpose

This directory defines implementation readiness, dependency order, scope cuts, and contract closure for the cognitive architecture.

Durable architecture lives under `design/cognitive_architecture`. This plan states what can be built, what must be built first, and what remains blocked.

Each assessment is current truth. It names the exact slice that is ready, the exact contracts it depends on, and the exact work that is deferred.

## Active Targets

`typed loop` is the first target. It proves that `meld-lang` can express world state, goals, methods, compositions, effects, and satisfaction without runtime dispatch.

`world model projection` projects graph and belief state into `meld-lang::WorldState`.

`goal curation` creates one ground `meld-lang::Goal` from one agent perspective and one watched belief family.

`runtime planning substrate` evaluates goals, selects methods, validates compositions, and prepares task network mutations.

`runtime flywheel` dispatches tasks, publishes outcome facts, and closes belief revision through the event spine.

## Dependency Order

1. `events`
2. `meld-lang`
3. `world_model/graph`
4. `world_model/belief`
5. `world_model/planner`
6. `world_model/agent`
7. `execution/goals`
8. `integration/typed_loop`
9. `execution/planning`
10. `sensory`
11. `world_model/causation`
12. `world_model/regime`
13. `world_model`
14. `execution`

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

The typed loop excludes runtime dispatch, event publication, workflow migration, task network execution, provider calls, storage, and async runtime behavior.

World model projection excludes full causal effect summaries, regime sensitivity summaries, broad risk envelopes, and multi-agent divergence.

Goal curation excludes learned normative policy, multi-agent coordination, goal conflict resolution, and broad utility estimation.

Runtime planning substrate excludes task network execution until the graph executor, mutation acceptance, plan diffing, and switching cost contracts are specified.

Runtime flywheel excludes broad multi-agent coordination and full causation or regime inference.

## Blocked Areas

- task network graph executor
- graph mutation acceptance
- plan diffing
- switching cost model
- outcome publication bridge
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
