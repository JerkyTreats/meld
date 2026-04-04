# Workflow Cleanup Technical Spec

Date: 2026-03-28
Status: active
Scope: implementation-facing extraction of workflow-owned orchestration into compatibility and control seams

## Intent

Provide one implementation-facing execution spec for workflow cleanup.
This spec defines the concrete extraction work required to clear the ground for capability and task as the durable implementation model.

The goal is not to preserve workflows as a durable architecture center.
The goal is to remove workflow-owned orchestration and push that responsibility into `control` while keeping current trigger flows alive through compatibility seams.

## Source Synthesis

This specification synthesizes:

- [Workflow Refactor](README.md)
- [Workflow Refactor Code Path Findings](code_path_findings.md)
- [Context Code Path Findings](../context/code_path_findings.md)
- [Context Technical Spec](../context/technical_spec.md)
- [Interregnum Orchestration](../../control/interregnum_orchestration.md)
- [Merkle Traversal Technical Spec](../capability/merkle_traversal/technical_spec.md)

## Boundary

Start condition:
- workflow exists as a first-class runtime path
- agent configuration can bind workflow behavior
- generation selection carries workflow-specific execution mode
- queue and watch mode both branch on workflow routing
- workflow CLI surfaces are user-visible
- workflow thread and turn records are durable operational state

End condition:
- workflow no longer exists as a durable orchestration abstraction
- agent, context, queue, and provider no longer depend on workflow-owned execution order
- workflow may remain as a compatibility trigger path during the refactor window
- docs writer workflow behavior is preserved only through delegated control execution during the interregnum
- reusable implementation seams have been extracted where needed for later capability and task work

## Functional Goal

The functional goal of this cleanup is to remove workflow-specific orchestration ownership without destabilizing the deeper domain behavior that will later back capability execution.
The functional goal of this cleanup is also to preserve the thin compatibility seams that future task loading and compilation can target.

This means:

- preserve reusable generation and artifact seams
- move ordered execution into `control`
- reduce workflow command and watch surfaces to compatibility entry paths where still needed
- remove workflow-specific durable state as an active orchestration model
- preserve current end-to-end trigger flows during the cleanup window

## Cleanup Gates

These gates define what must be true before the cleanup is considered complete:

- workflow is no longer a valid agent-facing execution concern
- workflow is no longer a valid generation program kind
- workflow is no longer a queue dispatch branch
- workflow orchestration has moved into `control`
- workflow is reduced to compatibility entry surfaces only where still needed
- docs-writer behavior is preserved through delegated control execution until task exists

## Change To Outcome Map

### W0 Preserve reusable execution seams before removing workflow routing

Code changes:
- identify and preserve the generation and artifact seams in context that should survive workflow removal
- avoid coupling cleanup work to a rewrite of provider execution, frame persistence, or generation orchestration

Outcome:
- workflow cleanup removes routing and orchestration shells rather than destabilizing reusable domain behavior

Verification:
- preserved context seams remain callable without workflow-owned orchestration

### W1 Move ordered workflow execution into `control`

Code changes:
- introduce `src/control` as a refactor-phase orchestration home
- move traversal batch release, wave barriers, and ordered execution progression out of workflow internals
- keep workflow entry paths delegating into `control`

Outcome:
- workflow stops owning execution order
- current behavior survives through control-owned orchestration

Verification:
- a workflow-triggered run can still complete end to end
- ordered release no longer depends on workflow-internal execution loops

### W2 Remove workflow from agent-facing configuration and binding

Code changes:
- remove workflow binding assumptions from [binding.rs](/home/jerkytreats/meld/src/workflow/binding.rs)
- remove agent workflow binding usage from generation selection and watch mode callers
- stop treating `workflow_id` as a durable agent concern

Outcome:
- agents no longer opt into workflow execution
- the durable agent model stops carrying workflow-specific semantics

Verification:
- no active agent execution path requires workflow binding
- agent resolution no longer produces workflow-specific selection behavior

### W3 Remove workflow from generation program selection

Code changes:
- remove `TargetExecutionProgramKind::Workflow` and related workflow fields from [program.rs](/home/jerkytreats/meld/src/context/generation/program.rs)
- remove workflow branch selection from [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs)
- collapse generation selection onto non-workflow execution paths

Outcome:
- generation program selection no longer treats workflow as a first-class mode
- context-facing execution contracts become cleaner and more capability-ready

Verification:
- generation program types compile without workflow variants
- selection logic no longer branches on workflow binding

### W4 Remove workflow routing from queue execution and retry handling

Code changes:
- remove workflow dispatch branching from [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
- remove workflow-specific retry or failure classification from queue-local logic
- ensure queue remains transport and dispatch logic rather than orchestration logic

Outcome:
- queue no longer owns workflow routing
- workflow-specific retry behavior no longer shapes general generation transport

Verification:
- queue execution compiles and runs without workflow facade calls
- queue retry handling no longer references workflow-specific paths

### W5 Reduce workflow routing in watch mode to compatibility only

Code changes:
- remove workflow-owned execution logic from [runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
- if watch mode still triggers workflow-shaped behavior, delegate into `control`

Outcome:
- watch mode no longer acts as a second owner of workflow orchestration
- compatibility triggering stays thin

Verification:
- watch runtime does not contain workflow-owned orchestration
- any remaining workflow-shaped trigger path delegates inward

### W6 Reduce workflow CLI product surfaces to compatibility triggers

Code changes:
- reduce workflow list, validate, inspect, and execute routing in [route.rs](/home/jerkytreats/meld/src/cli/route.rs)
- reduce [commands.rs](/home/jerkytreats/meld/src/workflow/commands.rs) to compatibility helpers while migration is active

Outcome:
- workflow stops presenting itself as the durable implementation model
- CLI can still trigger compatibility flows where needed

Verification:
- surviving workflow CLI behavior is clearly compatibility-only
- CLI-triggered workflow runs delegate into control-owned orchestration

### W7 Remove workflow registry, executor, and state as live orchestration dependencies

Code changes:
- isolate or delete [registry.rs](/home/jerkytreats/meld/src/workflow/registry.rs), [facade.rs](/home/jerkytreats/meld/src/workflow/facade.rs), [executor.rs](/home/jerkytreats/meld/src/workflow/executor.rs), [resolver.rs](/home/jerkytreats/meld/src/workflow/resolver.rs), [gates.rs](/home/jerkytreats/meld/src/workflow/gates.rs), and [state_store.rs](/home/jerkytreats/meld/src/workflow/state_store.rs) once compatibility delegation no longer depends on them
- remove workflow thread and turn state as live operational records

Outcome:
- workflow internals stop shaping the runtime
- remaining reusable behavior can be reintroduced later through capability, task, and control-owned plan execution, not through preserved workflow shells

Verification:
- no production routing path depends on workflow registry or executor
- workflow thread and turn records are no longer written as part of normal execution

### W8 Reframe docs writer as a future capability graph example while preserving interim execution

Code changes:
- stop treating `docs_writer_thread_v1` as a durable orchestration primitive
- preserve only the behavior knowledge needed for later capability graph compilation
- during the refactor window, keep current docs-writer behavior only through compatibility delegation into `control`

Outcome:
- docs writer remains useful as a task-compilation example
- workflow abstraction no longer survives as the hidden owner of docs-writer execution

Verification:
- docs writer can still run during the interregnum
- design docs continue to reference docs writer as future capability graph input rather than durable workflow architecture

## File Level Execution Order

1. the new `src/control` entry and orchestration files
2. [binding.rs](/home/jerkytreats/meld/src/workflow/binding.rs)
3. [program.rs](/home/jerkytreats/meld/src/context/generation/program.rs)
4. [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs)
5. [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
6. [runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
7. [route.rs](/home/jerkytreats/meld/src/cli/route.rs)
8. [commands.rs](/home/jerkytreats/meld/src/workflow/commands.rs)
9. [facade.rs](/home/jerkytreats/meld/src/workflow/facade.rs)
10. [registry.rs](/home/jerkytreats/meld/src/workflow/registry.rs)
11. [executor.rs](/home/jerkytreats/meld/src/workflow/executor.rs)
12. [resolver.rs](/home/jerkytreats/meld/src/workflow/resolver.rs)
13. [gates.rs](/home/jerkytreats/meld/src/workflow/gates.rs)
14. [state_store.rs](/home/jerkytreats/meld/src/workflow/state_store.rs)

## Verification Matrix

Routing gates:
- agent resolution no longer carries workflow execution behavior
- generation selection no longer emits workflow program variants
- queue and watch mode no longer own workflow orchestration
- workflow entry paths delegate into `control`

Surface gates:
- CLI presents workflow only as a compatibility trigger where needed
- docs writer remains runnable during the interregnum through delegated control execution

State gates:
- workflow thread and turn records are no longer written during normal execution
- workflow-owned durable state is no longer part of active orchestration

Boundary gates:
- reusable generation seams remain available for later capability work
- workflow cleanup does not reintroduce workflow semantics through compatibility names or routing wrappers

## Completion Criteria

1. ordered workflow execution has moved into `control`
2. workflow binding is gone from active agent behavior
3. workflow execution mode is gone from generation program selection
4. queue and watch mode no longer branch into workflow-owned execution
5. workflow CLI surfaces are reduced to explicit compatibility helpers where still needed
6. workflow registry, executor, and state store are no longer live orchestration dependencies
7. docs writer remains runnable during the interregnum and remains future capability graph design input

## Read With

- [Workflow Refactor](README.md)
- [Workflow Refactor Code Path Findings](code_path_findings.md)
- [Context Code Path Findings](../context/code_path_findings.md)
- [Context Technical Spec](../context/technical_spec.md)
- [Merkle Traversal Technical Spec](../capability/merkle_traversal/technical_spec.md)
- [Capability And Task Implementation Plan](../PLAN.md)
