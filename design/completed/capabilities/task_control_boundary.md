# Task Control Boundary

Date: 2026-04-04
Status: active
Scope: exact ownership split between task and control during the interregnum and after task execution becomes durable

## Intent

Define the precise boundary between `task` and `control` for ordered parallel execution over capability graphs.

This document exists because the docs-writer style runtime makes one fact obvious:

- dependency structure is a task concern
- task-local capability progression is a task concern
- task-network release and continuation is a control concern

Those two facts are tightly related, but they are not the same responsibility.

## Core Position

`task` should own the durable dependency truth and the task-local execution agent.
`control` should own task-network execution truth.

That means:

- task defines what must precede what
- task defines which artifacts satisfy which later inputs
- task executor decides what is ready now inside one task
- task executor triggers ready capability work inside one task
- control decides which task runs or resumes now
- control waits on task-level progress and continuation events

This is the correction of ownership that the interregnum needs to codify.

## Docs Writer Example

For a bottom-up docs writer task:

- traversal-derived lower README work may run in parallel
- parent README work must wait until required child artifacts exist
- the graph should encode those dependencies durably
- task executor should release all currently ready leaves inside the task, then wait, then release the next ready region

So the system should not treat "bottom-up docs writer order" as one undifferentiated concern.
It splits into:

- task-owned structural dependency knowledge
- task-owned task-local executor behavior
- control-owned task-network dispatch and continuation behavior

## Task Owns

Task owns anything that can be compiled once and persisted with the task record.

That includes:

- compiled capability instances
- stable `capability_instance_id` values
- dependency edges among capability instances
- slot wiring and artifact compatibility
- the artifact repo and artifact lookup semantics
- producer to consumer relationships derived from declared inputs and outputs
- task-local invocation records
- task-local attempt lineage
- task-local ready-set evaluation
- task-local capability triggering through one task executor
- any docs-writer-specific dependency shape that can be expressed as durable graph structure

For docs writer, examples of task-owned knowledge are:

- `WorkspaceResolveNodeId` must precede `MerkleTraversal`
- `MerkleTraversal` emits ordered node batches used to derive generation regions
- one child `ContextGenerateFinalize` emission may satisfy one parent `ContextGeneratePrepare` input
- parent preparation stays blocked until the required child summary artifact exists

## Control Owns

Control owns anything that depends on task-network state and cross-task execution state.

That includes:

- dispatch and admission of tasks
- continuation and resume across tasks
- retry and repair intent
- inter-task ordering and task-network reduction
- event reduction and task-network state

Task executor owns, inside one task:

- ready-set evaluation against current artifact availability
- parallel release of independent ready capability instances
- waiting on task-local batch or join barriers
- retry attempt materialization after control issues retry intent

For docs writer, examples of control-owned knowledge are:

- this task should be dispatched or resumed now
- this failed task should receive retry intent rather than repair escalation
- another task has higher priority in the task network

For docs writer, examples of task-executor-owned knowledge are:

- all leaves in the current ready set can be released now
- a parent node is still blocked because one required child artifact is missing
- the next level becomes ready only after the last child finalization event lands
- a retry attempt should be materialized for one failed leaf because control issued retry intent

## Interregnum Rule

During the interregnum, `src/control` temporarily contains more docs-writer-specific execution logic than it should long term.

That is acceptable only as a compatibility posture.
The durable target remains:

- task owns the dependency model and task-local executor behavior
- control owns task-network dispatch and continuation

So any interregnum control code should be written as if it is operating over task-owned dependency truth, even before the full task compiler is in place.

## Structural Versus Live Knowledge

Use this test:

- if the knowledge can be serialized into the compiled task record, it belongs in task
- if the knowledge depends on current artifact availability inside one task, it belongs in task executor
- if the knowledge depends on task-network dispatch, repair, or continuation, it belongs in control

Examples:

- "parent README depends on child README summary artifacts"
  - task
- "three child invocations are ready right now"
  - task executor
- "leaf nodes for this subtree are parallel-safe"
  - task
- "the parent may now be released because all child outputs exist"
  - task executor
- "this task should yield to another task in the network"
  - control

## Artifact Repo And Readiness

The artifact repo is the bridge between task structure, task executor, and control.

Task owns:

- artifact persistence
- artifact identity
- producer and consumer linkage
- lookup semantics by slot compatibility

Task executor uses that task-owned information to answer live questions such as:

- are all required inputs satisfied
- is this capability instance ready
- which ready instances may run in parallel
- which blocked instances should be reevaluated after a new artifact lands

Control should not invent dependency meaning outside task records.
Task executor should evaluate readiness from task-owned structure plus current artifact repo state.

## Parallelism Rule

Parallelism should emerge from task structure and control evaluation together.
More precisely, it emerges from task structure and task-executor evaluation, under control-managed task dispatch policy.

The rule is:

- task declares independence through absence of blocking dependencies
- task executor exploits that independence by releasing all ready work allowed inside the active task
- control decides which task is active or resumed under broader policy

So the graph does not "run in parallel" by itself.
The graph defines where parallelism is legal.
Task executor decides when to exploit it inside the active task.

## Retry Rule

This same split applies to retry:

- control decides that retry should happen
- task executor executes the resulting retry attempt inside task scope
- task records the new invocation and any emitted artifacts
- task executor reevaluates task-local readiness using the updated task state
- control reevaluates task-network consequences from emitted task events

## Consequences For Implementation

The implementation should trend toward:

- task compiler emits enough dependency and artifact-wiring information that docs-writer ordering does not need to be re-invented in control
- task runtime becomes generic over task-local ready-set release, barrier waiting, and capability triggering
- control runtime becomes generic over task dispatch, continuation, retry intent, and repair
- interregnum control code should be treated as temporary housing for future task-executor behavior plus task-network control behavior

## Decision Summary

- task owns durable dependency truth
- task owns task-local execution truth
- control owns task-network execution truth
- docs-writer-specific dependency shape belongs in task once task compilation is real
- interregnum control code may temporarily host compatibility logic, but should not become the durable owner of task-local dependency knowledge

## Read With

- [Task Design](task/README.md)
- [Capability Model](capability/README.md)
- [Capabilities By Domain](capability/by_domain.md)
- [Interregnum Orchestration](../control/interregnum_orchestration.md)
- [Task Network](../control/task_network.md)
