# Guard Expression Semantics

Date: 2026-05-08
Status: active
Scope: evaluation rules for guard expressions on conditional dependency edges in the task network graph
Origin: relocated from program/guard_binding_semantics.md — control graph dissolved, guard semantics preserved as conditional dependency edge evaluation

## Context

In the graphs-lower-graphs model, conditional branching is expressed as conditional dependency edges with guard expressions. When an observation task completes and produces a decision artifact, conditional edges downstream evaluate their guard expressions. Subtrees whose guards are satisfied become ready. Subtrees whose guards are not satisfied are pruned.

This document specifies what a guard expression contains and how it is evaluated. The semantics are unchanged from the original control graph design — only the structural context has shifted from branch nodes to dependency edges.

See [Planning Pipeline](planning_pipeline.md) for how conditional dependency edges encode branching in the task network graph.

## Definition

A guard expression is a typed predicate that evaluates against an artifact produced by an upstream task. It is associated with a `TaskDependencyEdge` of kind `Conditional`.

```rust
enum GuardExpression {
    BooleanField { field_path: FieldPath, expected: bool },
    ThresholdField { field_path: FieldPath, threshold: f32, operator: ThresholdOperator },
    AlwaysTrue,
}

enum ThresholdOperator {
    GreaterThanOrEqual,
    LessThan,
}
```

`field_path` is a dot-separated path into the artifact's JSON content, e.g. `"should_execute"` or `"probability"`.

## Evaluation Rules

### Artifact Resolution

The guard expression evaluates against the output artifact of the upstream task (the `from` task on the conditional dependency edge). The artifact must match the `artifact_type` declared on the edge.

If the artifact is absent at evaluation time, the edge is not yet evaluable. The downstream task remains outside the ready set. This is not an error — it means the upstream task has not yet completed.

### Expression Evaluation

**BooleanField**

Reads the field at `field_path` from the artifact content. Compares to `expected`.
If the field is absent, evaluation fails (contract error).

Example: `BooleanField { field_path: "should_execute", expected: true }` evaluates to true when `artifact.content.should_execute == true`.

**ThresholdField**

Reads the field at `field_path` from the artifact content. Compares against `threshold` using the specified operator.
If the field is absent, evaluation fails (contract error).

Example: `ThresholdField { field_path: "probability", threshold: 0.6, operator: GreaterThanOrEqual }` evaluates to true when `artifact.content.probability >= 0.6`.

**AlwaysTrue**

Evaluates to true unconditionally. Used for the default branch of a conditional pattern.

## Conditional Branching Pattern

When the planning loop cannot resolve a decision at planning time (insufficient belief), it emits:

1. An observation task that will produce a decision artifact
2. Conditional dependency edges from the observation task to alternative downstream subtrees
3. Guard expressions on those edges that evaluate the decision artifact

```
observation_task (produces decision_artifact)
    |
    |── conditional edge (guard: BooleanField "should_execute" = true) ──▶ action_subtree
    |
    |── conditional edge (guard: BooleanField "should_execute" = false) ──▶ skip_task
```

When the observation task completes, the task network evaluates the conditional edges. The subtree whose guard is satisfied enters the ready set. The subtree whose guard is not satisfied is pruned from the graph.

### Standard Two-Edge Pattern

```
conditional edges from observation task:
  - to: action_subtree
    guard: BooleanField { field_path: "should_execute", expected: true }
  - to: skip_path
    guard: BooleanField { field_path: "should_execute", expected: false }
```

The `bayesian_evaluation` capability applies the threshold internally and emits `should_execute: bool`. The guard evaluates against the boolean, not the raw probability. This keeps threshold logic inside the capability contract and keeps the guard expression simple.

An alternative evaluates the raw probability at the edge:

```
conditional edges:
  - to: action_subtree
    guard: ThresholdField { field_path: "probability", threshold: 0.6, operator: GreaterThanOrEqual }
  - to: skip_path
    guard: ThresholdField { field_path: "probability", threshold: 0.6, operator: LessThan }
```

The raw probability form is useful when the threshold needs to vary per context.

## Guard Failure Behavior

A guard expression failure (field absent or type mismatch) is a contract error. It indicates a mismatch between the capability that produced the artifact and the guard that consumes it. This should surface as a validation error when the task network graph is assembled, not at runtime.

## Validation

When a planning agent commits a subgraph to the task network, the commit phase should validate:

- every conditional dependency edge references an artifact type that the upstream task declares as an output
- every guard expression references a field path that exists in the declared artifact schema
- the conditional edge set from a given observation task covers all possible outcomes (either exhaustively or with an `AlwaysTrue` fallback)

## Read With

- [Planning Pipeline](planning_pipeline.md)
- [Observation Wait Semantics](observation_wait_semantics.md)
- [Bayesian Evaluation Example](../examples/bayesian_evaluation.md)
