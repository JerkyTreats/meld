# Goals Readiness Assessment

Status: conditionally ready
Depends on: `design/plan/world_model/agent/assessment.md`, `design/plan/meld-lang/assessment.md`
Design source: `design/cognitive_architecture/execution/goals/README.md`, `design/cognitive_architecture/execution/GAPS.md`, `design/cognitive_architecture/world_model/agent/goal_curation.md`, `design/cognitive_architecture/meld-lang/goals_and_methods.md`
Evidence date: 2026-05-21

## Verdict Summary

Execution goals are conditionally ready for one goal set using `meld-lang::Goal`.

The ready slice covers storing one ground active goal, exposing a curation API, and supporting typed-loop satisfaction.

## Conceptual Correctness

Goals are desired belief states owned as execution data.

World model agent constructs goals through normative judgment. Execution stores and evaluates them mechanically.

## Completeness

The first goal shape is imported from `meld-lang`.

Execution goal storage must reject non-ground active goals.

The public curation API must support add, modify, remove, and satisfy.

Satisfaction may be set by planning evaluation or by agent curation.

## Boundary Clarity

Execution owns the goal set, lifecycle state, priority data, satisfaction state, and curation API.

World model agent owns goal construction and normative judgment.

`meld-lang` owns `Goal`, `GoalPriority`, `GoalSource`, `GoalLifecycle`, and target `Proposition`.

Goals do not own task decomposition, provider execution, belief settlement, regime inference, or event ledger authority.

## Dependency Readiness

`meld-lang` is ready for typed-loop goal values.

Agent is conditionally ready for one ground goal.

Runtime planning remains blocked beyond typed-loop evaluation.

## First-Slice Feasibility

Goals support the typed loop by storing one active `docs_freshness` goal and exposing it for mechanical evaluation.

No task dispatch is required.

## Current Implementation Evidence

- `crates/meld-execution/src/task.rs`
- `crates/meld-execution/src/workflow.rs`
- `crates/meld-execution/src/workflow/`
- `tests/integration/task_executor.rs`
- `tests/integration/workflow_task_compatibility.rs`
- `tests/integration/workflow_contracts_conformance.rs`
- `design/cognitive_architecture/execution/goals/README.md`
- `design/cognitive_architecture/execution/GAPS.md`
- `design/cognitive_architecture/meld-lang/goals_and_methods.md`

## Gaps

- Concrete goal curation API operations need implementation shape.
- Goal persistence and replay semantics need implementation shape.
- Multi-goal conflict and preemption strategy is deferred.
- Multi-agent goal coordination is deferred.

## Open Questions

- Which execution module owns the first goal set store.
- Whether `Proposed` goals are admitted in the first implementation or only `Active` goals.

## Recommendation

Proceed with one ground active `Goal`.
