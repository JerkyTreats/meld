# Await Observation Semantics

Date: 2026-04-08
Status: active
Scope: runtime semantics for the await_observation control node type

## Intent

Define precisely what happens when control execution reaches an `await_observation` node,
how it suspends, and under what conditions it resumes.

## Context

`await_observation` is one of the control node families defined in
[Control Graph Model](control_graph.md). It was named there but not specified.
This document provides that specification.

## Definition

An `await_observation` node suspends control program execution until a designated observation
task has emitted a specific artifact type into the task network state.

It is always paired with a subsequent `branch` node. The `await_observation` node makes an
artifact available. The `branch` node evaluates a guard binding against that artifact.

```
dispatch_task: ImpactAssessmentTask
    |
await_observation: artifact_type=decision_artifact, task_ref=impact_assessment_instance
    |
branch: guard=decision_artifact.should_execute
   |                          |
dispatch_task: DocsWriter    terminate
```

## Node Record

```rust
struct AwaitObservationNode {
    node_id: ControlNodeId,
    observation_task_ref: ObservationTaskRef,
    expected_artifact_type_id: ArtifactTypeId,
    expected_artifact_schema_version: SchemaVersion,
    timeout_policy: AwaitTimeoutPolicy,
    on_timeout: ControlNodeId,
}

enum ObservationTaskRef {
    TaskType(TaskTypeId),
    TaskInstance(TaskInstanceId),
}

enum AwaitTimeoutPolicy {
    None,
    AfterDuration { seconds: u64, escalate_to_repair: bool },
}
```

## Suspension Rules

When execution reaches `await_observation`:

1. The control runtime records the current continuation at this node.
2. Execution does not advance past this node.
3. The control runtime registers an interest in `task_artifact_emitted` events for the
   specified `(observation_task_ref, expected_artifact_type_id)` pair.
4. The task network continues to dispatch and execute other ready tasks normally.

The observation task must already have been dispatched before `await_observation` is reached.
The control graph is responsible for ordering the dispatch. `await_observation` does not
dispatch anything; it only waits.

## Resume Rules

The control runtime resumes execution past `await_observation` when:

1. A `task_artifact_emitted` event arrives from the referenced observation task for the
   expected artifact type.
2. The reducer records the emitted artifact in the task network state.
3. The reducer advances the continuation past `await_observation` to the next node.

The artifact is now available in the task network state under its artifact type id.
The subsequent `branch` node reads it from there.

## Late Arrival

If the observation task emits the expected artifact before execution reaches
`await_observation` (because the task was fast relative to control dispatch), the
`await_observation` node resumes immediately on entry. The artifact is already present
in the task network state.

This means `await_observation` is a rendezvous point, not a strict barrier. The ordering
guarantee is: execution does not pass `await_observation` until the artifact exists, but
the artifact may have arrived before the node is reached.

## Absence on Task Failure

If the observation task fails without emitting the expected artifact:

- A `task_failed` event arrives in the task network.
- If the `await_observation` node has registered interest in the artifact, the reducer
  transitions the continuation to the repair path rather than leaving it suspended.
- Repair intent is raised for the observation task. The control program does not advance
  past `await_observation` until either:
  - the observation task retries and emits the expected artifact, or
  - repair escalates to method reselection at a parent task boundary.

## Timeout Policy

When `timeout_policy` is set:

- If the observation task has not emitted the expected artifact within the specified duration,
  the timeout fires.
- If `escalate_to_repair: true`, the reducer treats the timeout as an implicit task failure
  and follows the failure path above.
- If `escalate_to_repair: false`, the continuation advances to `on_timeout` node without
  the artifact. The `on_timeout` node must handle the absent artifact case explicitly
  (typically a `terminate` or a skip path).

For the first slice, timeout is not required. It should be defined in the record but may
be `AwaitTimeoutPolicy::None` in initial implementations.

## Relationship to Continuation Model

`await_observation` is one of the suspension points that the continuation model must record.

A suspended `await_observation` node in a serialized continuation record should carry:

- the `observation_task_ref`
- the `expected_artifact_type_id`
- the continuation node to resume at

This allows the runtime to restore observation interests after a restart by replaying
the registered interest from durable continuation state.

See [Continuation Model](../runtime/continuation_model.md) for the broader continuation record.

## Read With

- [Control Graph Model](control_graph.md)
- [Guard Binding Semantics](guard_binding_semantics.md)
- [Impact Assessment](../impact_assessment.md)
- [Runtime Model](../runtime/README.md)
- [Continuation Model](../runtime/continuation_model.md)
