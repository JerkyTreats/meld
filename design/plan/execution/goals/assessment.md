# Goals Readiness Assessment

Status: first slice implemented
Depends on: `design/plan/world_model/agent/assessment.md`, `design/plan/meld-lang/assessment.md`
Design source: `design/cognitive_architecture/execution/goals/README.md`, `design/cognitive_architecture/execution/GAPS.md`, `design/cognitive_architecture/world_model/agent/goal_curation.md`, `design/cognitive_architecture/meld-lang/goals_and_methods.md`
Evidence date: 2026-05-31

## Verdict Summary

Execution goals are implemented for one first slice goal set using `meld-lang::Goal`.

The implemented slice covers storing one ground active goal, exposing a curation API, supporting typed-loop satisfaction, and persisting execution goal lifecycle state.

Runtime goal set storage is implemented in `meld-execution` through in memory and durable store variants.

## Conceptual Correctness

Goals are desired belief states owned as execution data.

World model agent constructs goals through normative judgment. Execution stores and evaluates them mechanically.

## Completeness

The first goal shape is imported from `meld-lang`.

Execution goal storage rejects non-ground active goals.

The public curation API supports add, modify, remove, and satisfy.

Satisfaction may be set by planning evaluation or by agent curation.

## Boundary Clarity

Execution owns the goal set, lifecycle state, priority data, satisfaction state, and curation API.

World model agent owns goal construction and normative judgment.

`meld-lang` owns `Goal`, `GoalPriority`, `GoalSource`, `GoalLifecycle`, and target `Proposition`.

Goals do not own task decomposition, provider execution, belief settlement, regime inference, or event ledger authority.

## Dependency Readiness

`meld-lang` is ready for typed-loop goal values.

Agent design is ready for one ground goal.

Execution planning runtime is in progress for one goal to composition.

## First-Slice Feasibility

Goals support the typed loop by storing one active `docs_freshness` goal and exposing it for mechanical evaluation.

No task dispatch is required.

This is implemented for the first execution planning slice.

## Current Implementation Evidence

- `crates/meld-lang/src/goal.rs`
- `crates/meld-lang/tests/evaluation_loop.rs`
- `crates/meld-execution/src/goals.rs`
- `crates/meld-execution/src/goals/contracts.rs`
- `crates/meld-execution/src/goals/store.rs`
- `crates/meld-execution/src/goals/persistent_store.rs`
- `crates/meld-execution/src/goals/query.rs`
- `crates/meld-execution/tests/goals.rs`
- `design/cognitive_architecture/execution/goals/README.md`
- `design/cognitive_architecture/execution/GAPS.md`
- `design/cognitive_architecture/meld-lang/goals_and_methods.md`

## Gaps

- Execution runtime assembly has not selected the persistent goal store path.
- Agent goal command ingestion into execution goal storage is not wired.
- Multi-goal conflict and preemption strategy is deferred.
- Multi-agent goal coordination is deferred.

## Open Questions

- Which runtime assembly path opens the execution goal database.
- How world model `AgentGoalCommand` delivery enters the execution goal store.

## Recommendation

Proceed with execution planning runtime integration using durable goal storage for authoritative lifecycle state.
