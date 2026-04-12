# Control Graph Model

Date: 2026-04-08
Status: active
Scope: durable control graph structure above compiled tasks, defining task dispatch, control transfer, and observation branching

## Intent

Define the concrete program model the task network executes.

The task network is the stateful orchestrator.
The control graph is the compiled program it runs.
Tasks are the objects being orchestrated.
Control nodes are the instructions.

## Summary

The control graph is the executable program artifact above the compiled task layer.
It owns control transfer, task dispatch, and observation branching.
Compiled tasks remain responsible for capability dependency structure and execution.

## Vocabulary

This document uses the application's settled vocabulary:

- `task` — a compiled capability graph; the unit of work dispatched by control
- `task_network` — the stateful orchestrator that executes the control program
- `CompiledTask` — a locked compiled task record; what `dispatch_task` references
- `DecisionArtifact` — the typed output of an observation task; what `branch` reads
- `region` — a named sub-unit within a task expansion; distinct from a whole task

Prior versions of this document used `region` and `invoke_region` where this document
uses `task` and `dispatch_task`. The rename reflects the settled task and task_network
vocabulary. See task_network.md "What should change in the design set."

## Required Records

```rust
struct CompiledControlProgram {
    program_id: ControlProgramId,
    entry_node: ControlNodeId,
    nodes: Vec<ControlNode>,
    edges: Vec<ControlEdge>,
}

struct ControlNode {
    node_id: ControlNodeId,
    kind: ControlNodeKind,
}

struct ControlEdge {
    from: ControlNodeId,
    to: ControlNodeId,
    guard: Option<GuardBinding>,
}
```

`GuardBinding` is specified in [Guard Binding Semantics](guard_binding_semantics.md).

## Control Node Families

Control nodes divide into two categories with different relationships to tasks.

### Task-Operating Nodes

These nodes act on tasks directly. Tasks are their primary object.

**`dispatch_task`**
Dispatches a `CompiledTask` for execution by the task network.
Creates a live task run. The task network owns the resulting task run identity and state.

**`await_observation`**
Suspends the control program until a designated observation task emits a specific artifact.
Always followed by a `branch` node that reads that artifact.
Full semantics in [Await Observation Semantics](await_observation_semantics.md).

**`repair_entry`**
Enters repair scope for a failed or blocked task.
Identifies the failing task boundary and the available repair paths.
Full semantics in [Repair Entry Model](../repair/repair_entry.md).

### Control Flow Nodes

These nodes navigate the program. They do not act on tasks directly.
They route execution based on task outcomes already recorded in task network state.

**`entry`**
Program entry point. No task operation. Marks where execution begins.

**`branch`**
Conditional routing. Reads a guard artifact from task network state via a `GuardBinding`.
Selects exactly one outgoing edge. Does not dispatch anything.
Full semantics in [Guard Binding Semantics](guard_binding_semantics.md).

**`join`**
Barrier. Holds until all upstream parallel paths have reached this node.
Advances only when the task network confirms all expected `task_succeeded` events have landed.

**`loop_header`**
Marks the head of a bounded revisit path.
Loop back edges are only valid here.
Every loop path must declare a checkpoint posture before re-entry.

**`terminate`**
Ends the control program. Terminal node. No outgoing edges.

## Edge Rules

- control edges express control transfer only
- dependency edges remain inside compiled tasks, not in the control graph
- `branch` edges carry `GuardBinding` expressions
- loop back edges are only valid targeting `loop_header` nodes
- every reachable path must reach `terminate` or suspend at `await_observation`

## Task Dispatch Rules

- one `dispatch_task` node references one `CompiledTask` by `CompiledTaskRef`
- the `CompiledTask` remains an independently valid capability DAG
- repeated dispatch of the same `CompiledTask` across loop iterations must be explicit;
  the task network does not infer repetition from graph structure
- dispatch creates a live task run; the task network owns task run identity and state
- the control graph does not own task run records

## Observation and Branch Pattern

`await_observation` and `branch` always appear together:

```
dispatch_task: ImpactAssessmentTask
       |
await_observation: artifact_type=decision_artifact, task_ref=impact_assessment
       |
branch: guard=GuardBinding(decision_artifact.should_execute)
  true  |                          | false
dispatch_task: DocsWriterTask    terminate (or skip path)
```

The observation task is dispatched before `await_observation` is reached.
`await_observation` does not dispatch; it only waits.
`branch` does not dispatch; it only routes.

## Loop Rules

- loop behavior must live in `loop_header` semantics, not in compiled task rewiring
- side-effecting task dispatches inside loop paths must rely on capability replay posture
  from the capability contract
- checkpoint state must be declared before the loop back edge is taken

## First Slice

The first slice should support:

- straight-line task dispatch (`entry` → `dispatch_task` → `terminate`)
- conditional branch via `DecisionArtifact` guard (`dispatch_task` → `await_observation` → `branch`)
- parallel dispatch with `join` barrier
- bounded revisit via `loop_header`
- repair entry via `repair_entry`

## Read With

- [Execution Control](../README.md)
- [Task Network](../task_network.md)
- [Await Observation Semantics](await_observation_semantics.md)
- [Guard Binding Semantics](guard_binding_semantics.md)
- [Execution Planning](../planning/README.md)
- [Repair Entry Model](../repair/repair_entry.md)
- [Task Design](../../capabilities/task/README.md)
