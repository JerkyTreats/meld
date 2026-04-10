# Guard Binding Semantics

Date: 2026-04-08
Status: active
Scope: evaluation rules for guard bindings on branch edges in the control graph

## Intent

Define how guard bindings are expressed, what they evaluate against, and what the evaluation
outcomes mean for control flow.

## Context

`branch` nodes carry guard bindings on their outgoing edges.
`guard_binding` is named in [Control Graph Model](control_graph.md) as a required record.
This document specifies what a guard binding contains and how it is evaluated.

## Definition

A guard binding is a typed predicate expression that evaluates against a named artifact in
the task network state. It is always associated with one outgoing edge of a `branch` node.

The branch node selects the outgoing edge whose guard evaluates to true.
Exactly one edge should evaluate to true per branch evaluation.
If no guard evaluates to true, the branch is a control error.

## Node and Edge Records

```rust
struct BranchNode {
    node_id: ControlNodeId,
    artifact_ref: GuardArtifactRef,
    edges: Vec<BranchEdge>,
}

struct BranchEdge {
    target_node_id: ControlNodeId,
    guard: GuardExpression,
    label: Option<String>,
}

struct GuardArtifactRef {
    artifact_type_id: ArtifactTypeId,
    schema_version: SchemaVersion,
}

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

`field_path` is a dot-separated path into the artifact's JSON content, e.g.
`"should_execute"` or `"probability"`.

## Evaluation Rules

### Artifact Resolution

The branch node resolves its guard artifact from the task network state using
`artifact_ref.artifact_type_id`. The artifact must have been placed in the task network
state by the preceding `await_observation` node.

If the artifact is absent at evaluation time, evaluation is blocked. This is not a branch
failure; it means the `await_observation` node has not yet resumed. The reducer should not
reach branch evaluation in this state under correct control flow.

### Expression Evaluation

**BooleanField**

Reads the field at `field_path` from the artifact content. Compares to `expected`.
If the field is absent, evaluation fails.

Example: `BooleanField { field_path: "should_execute", expected: true }` evaluates to true
when `artifact.content.should_execute == true`.

**ThresholdField**

Reads the field at `field_path` from the artifact content. Compares against `threshold`
using the specified operator.
If the field is absent, evaluation fails.

Example: `ThresholdField { field_path: "probability", threshold: 0.6, operator: GreaterThanOrEqual }`
evaluates to true when `artifact.content.probability >= 0.6`.

**AlwaysTrue**

Evaluates to true unconditionally. Used for the default or fallthrough edge of a branch.

### Standard Two-Edge Branch Pattern

The standard pattern for a `DecisionArtifact` branch is:

```
branch:
  artifact_ref: decision_artifact
  edges:
    - target: invoke_docs_writer
      guard: BooleanField { field_path: "should_execute", expected: true }
      label: "execute"
    - target: skip_node
      guard: BooleanField { field_path: "should_execute", expected: false }
      label: "skip"
```

The `bayesian_evaluation` capability applies the threshold internally and emits
`should_execute: bool`. The branch evaluates against the boolean, not the raw probability.

This keeps the threshold logic inside the capability contract where it belongs and keeps
the guard expression simple.

An alternative pattern evaluates the raw probability at the branch:

```
edges:
  - target: invoke_docs_writer
    guard: ThresholdField { field_path: "probability", threshold: 0.6, operator: GreaterThanOrEqual }
  - target: skip_node
    guard: ThresholdField { field_path: "probability", threshold: 0.6, operator: LessThan }
```

The raw probability form is useful when the threshold needs to vary per invocation (e.g.,
overridden by operator command). For the first slice, the boolean form is preferred.

## Guard Failure Behavior

A guard expression failure (field absent or type mismatch) is a control error, not a repair
trigger. It indicates a contract mismatch between the capability that produced the artifact
and the guard that consumes it. This should surface as a compile-time validation error when
the control program is assembled.

## Compile-Time Validation

When the control program is compiled, the assembler should validate:

- every `branch` node references an artifact type that some upstream `await_observation` node
  declares as its expected artifact type
- every `GuardExpression` references a field path that exists in the declared artifact schema
- the branch edge set covers all possible outcomes (either exhaustively or with an `AlwaysTrue`
  fallback edge)

This validation catches contract mismatches before runtime.

## Read With

- [Control Graph Model](control_graph.md)
- [Await Observation Semantics](await_observation_semantics.md)
- [Impact Assessment](../impact_assessment.md)
- [Bayesian Evaluation](../../capabilities/capability/bayesian_evaluation/README.md)
