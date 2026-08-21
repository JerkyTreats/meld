# Observation Wait Semantics

Scope: runtime semantics for observation tasks as data-flow dependencies in the task network graph

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

## Late Arrival After Completion

If the observation task completes before its downstream tasks are evaluated, the artifact is already available when the ready-set computation runs. Downstream tasks enter the ready set immediately.

This is the natural behavior of dependency-ordered graph execution. No special handling needed.

## Task Failure

If the observation task fails without producing the expected artifact:

1. The DataFlow and Conditional dependency edges from this task remain unsatisfied
2. Downstream tasks never enter the ready set
3. Execution records and publishes the failure
4. Task-local retry follows the exact authorized task policy
5. After retry exhaustion, Execution may select another still-authorized alternative or return typed rejection
6. Strategy may construct a replacement proposal from revised state
7. Only the Agent may suspend or abandon the Goal

Semantic replacement returns through Strategy and Agent authorization. Execution owns only the wait and transition mechanics for authorized work.

## Timeout

Timeout policy applies to observation tasks the same way it applies to any task. The task network tracks task duration. If a timeout is exceeded:

1. The Task Network emits a timeout Event that is treated as Task failure
2. The planning loop receives it and re-evaluates
3. Execution may realize another still-authorized observation alternative or report typed timeout and rejection for renewed Strategy

Timeout is a task-level concern, not a graph-level concern. The authorized task definition and Execution policy determine timeout behavior.

## Relationship to Conditional Branching

Observation tasks and conditional dependency edges form the branching pattern:

```
authorized Composition contains:
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
- Downstream Tasks are in the `pending` set because dependencies are not yet satisfied
- On resume after process restart, the ready-set computation re-evaluates and advances naturally

No special observation-specific continuation record is needed. The graph state and artifact repo are sufficient to reconstruct the wait.

## Read With

- [Planning Pipeline](planning_pipeline.md)
- [Guard Expression Semantics](guard_expression_semantics.md)
