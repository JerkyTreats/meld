# Observation Wait Semantics

Date: 2026-05-08
Status: active
Scope: runtime semantics for observation tasks as data-flow dependencies in the task network graph
Origin: relocated from program/await_observation_semantics.md — control graph dissolved, observation semantics preserved as data-flow dependency patterns

## Context

In the graphs-lower-graphs model, observation waits are expressed as data-flow dependencies. An observation task produces an artifact. Downstream tasks depend on that artifact through `DataFlow` dependency edges. The task network's ready-set computation naturally holds downstream tasks until the artifact is available.

This document specifies the runtime semantics: what happens when an observation task completes, fails, or times out. The core mechanism is simpler than the original control graph version — graph structure replaces explicit suspension and resume.

See [Planning Pipeline](planning_pipeline.md) for how observation waits map to dependency edges.

## Definition

An observation task is a normal task in the task network graph whose primary purpose is to produce an artifact that downstream tasks depend on. There is no special "observation task" type — any task can serve this role. The observation semantics emerge from the dependency structure.

```
observation_task ──DataFlow(decision_artifact)──▶ downstream_task_A
observation_task ──Conditional(guard)──▶ downstream_task_B
```

Downstream tasks do not enter the ready set until the observation task completes and its output artifact is available.

## Normal Completion

When the observation task completes and produces its expected artifact:

1. The artifact is stored in the task network's artifact state
2. DataFlow dependency edges referencing that artifact type are satisfied
3. Conditional dependency edges evaluate their guard expressions against the artifact
4. Downstream tasks whose dependencies are now fully satisfied enter the ready set
5. Conditional subtrees whose guards are not satisfied are pruned

This is the normal ready-set computation — no special observation logic needed. The graph structure handles it.

## Late Arrival (Already Complete)

If the observation task completes before its downstream tasks are evaluated (because the task was fast), the artifact is already available when the ready-set computation runs. Downstream tasks enter the ready set immediately.

This is the natural behavior of dependency-ordered graph execution. No special handling needed.

## Task Failure

If the observation task fails without producing the expected artifact:

1. The DataFlow and Conditional dependency edges from this task remain unsatisfied
2. Downstream tasks never enter the ready set
3. The planning loop receives the failure event
4. The planning loop re-evaluates via HTN lineage — it may:
   - Retry the observation task (inject a replacement)
   - Reselect the method at a parent task boundary
   - Abandon the goal if the observation is not achievable

This replaces the original control graph's `repair_entry` mechanism. The planning loop's cost-aware re-evaluation handles failure at the observation task the same way it handles any task failure.

## Timeout

Timeout policy applies to observation tasks the same way it applies to any task. The task network tracks task duration. If a timeout is exceeded:

1. The task network emits a timeout event (treated as task failure)
2. The planning loop receives it and re-evaluates
3. The planning loop may inject a replacement observation task, try an alternative method, or accept the timeout and prune the dependent subtree

Timeout is a task-level concern, not a graph-level concern. The observation task's task definition (or the planning loop's policy) determines timeout behavior.

## Relationship to Conditional Branching

Observation tasks and conditional dependency edges form the branching pattern:

```
planning loop emits:
  observation_task (produces decision_artifact)
      |
      |── Conditional(guard: should_execute = true) ──▶ action_subtree
      |
      |── Conditional(guard: should_execute = false) ──▶ skip_task
```

The observation task runs. When it completes:
- Guard expressions on conditional edges are evaluated
- The satisfied branch's tasks enter the ready set
- The unsatisfied branch's tasks are pruned

See [Guard Expression Semantics](guard_expression_semantics.md) for evaluation rules.

## Relationship to Continuation

For durable execution, the task network graph state must be checkpointable. An "in-progress observation wait" is represented as:

- The observation task is in the `in_flight` or `completed` set
- Downstream tasks are in the `pending` set (dependencies not yet satisfied)
- On resume after process restart, the ready-set computation re-evaluates and advances naturally

No special observation-specific continuation record is needed. The graph state and artifact repo are sufficient to reconstruct the wait.

## Read With

- [Planning Pipeline](planning_pipeline.md)
- [Guard Expression Semantics](guard_expression_semantics.md)
- [HTN Model](htn/README.md)
