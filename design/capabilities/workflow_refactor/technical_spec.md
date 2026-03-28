# Workflow Cleanup Technical Spec

Date: 2026-03-28
Status: active
Scope: implementation-facing removal of current workflow routing and workflow-owned orchestration seams

## Intent

Provide one implementation-facing execution spec for workflow cleanup.
This spec defines the concrete removal work required to clear the ground for capability and plan as the durable orchestration model.

The goal is not to preserve workflows.
The goal is to remove workflow-owned routing, workflow-owned state, and workflow-owned product surfaces so later capability graph compilation can reintroduce multi-step behavior at the correct abstraction layer.

## Source Synthesis

This specification synthesizes:

- [Workflow Refactor](README.md)
- [Workflow Refactor Code Path Findings](code_path_findings.md)
- [Context Generate Technical Spec](../context/technical_spec.md)
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
- workflow no longer exists as a durable runtime abstraction
- agent, context, queue, watch mode, and CLI no longer route through workflow-specific seams
- docs writer workflow behavior is no longer available through workflow execution
- reusable implementation seams have been extracted where needed for later capability and plan work

## Functional Goal

The functional goal of this cleanup is to remove workflow-specific routing without destabilizing the deeper domain behavior that will later back capability execution.

This means:

- preserve reusable generation and artifact seams
- remove workflow-specific execution selection
- remove workflow-specific command and watch surfaces
- remove workflow-specific durable state as an active orchestration model
- accept that current workflow functionality breaks during the cleanup window

## Cleanup Gates

These gates define what must be true before the cleanup is considered complete:

- workflow is no longer a valid agent-facing execution concern
- workflow is no longer a valid generation program kind
- workflow is no longer a queue dispatch branch
- workflow is no longer a watch-mode execution path
- workflow is no longer a primary CLI product surface
- docs-writer behavior is preserved only as a future capability graph example, not as a live workflow runner

## Change To Outcome Map

### W0 Preserve reusable execution seams before removing workflow routing

Code changes:
- identify and preserve the generation and artifact seams in context that should survive workflow removal
- avoid coupling cleanup work to a rewrite of provider execution, frame persistence, or generation orchestration

Outcome:
- workflow cleanup removes routing and orchestration shells rather than destabilizing reusable domain behavior

Verification:
- preserved context seams remain callable without workflow-owned orchestration

### W1 Remove workflow from agent-facing configuration and binding

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

### W2 Remove workflow from generation program selection

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

### W3 Remove workflow routing from queue execution and retry handling

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

### W4 Remove workflow routing from watch mode

Code changes:
- remove workflow execution branching from [runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
- keep watch mode focused on non-workflow generation behavior during the cleanup window

Outcome:
- watch mode no longer acts as a second workflow entry point
- workflow cleanup is complete across interactive and watched execution paths

Verification:
- watch mode does not resolve or execute workflow profiles
- watch runtime no longer depends on workflow execution requests

### W5 Remove workflow CLI product surfaces

Code changes:
- remove workflow list, validate, inspect, and execute routing from [route.rs](/home/jerkytreats/meld/src/cli/route.rs)
- remove or reduce [commands.rs](/home/jerkytreats/meld/src/workflow/commands.rs) to migration-only helpers if any temporary helpers still matter

Outcome:
- workflow is no longer presented as an active product surface
- operator expectations align with the cleanup direction

Verification:
- CLI no longer exposes workflow execution as a normal command surface
- any surviving helper behavior is clearly migration-only

### W6 Remove workflow registry, executor, and state as live orchestration dependencies

Code changes:
- isolate or delete [registry.rs](/home/jerkytreats/meld/src/workflow/registry.rs), [facade.rs](/home/jerkytreats/meld/src/workflow/facade.rs), [executor.rs](/home/jerkytreats/meld/src/workflow/executor.rs), [resolver.rs](/home/jerkytreats/meld/src/workflow/resolver.rs), [gates.rs](/home/jerkytreats/meld/src/workflow/gates.rs), and [state_store.rs](/home/jerkytreats/meld/src/workflow/state_store.rs) once no active caller depends on them
- remove workflow thread and turn state as live operational records

Outcome:
- workflow internals stop shaping the runtime
- remaining reusable behavior can be reintroduced later through capability and plan, not through preserved workflow shells

Verification:
- no production routing path depends on workflow registry or executor
- workflow thread and turn records are no longer written as part of normal execution

### W7 Reframe docs writer as a future capability graph example only

Code changes:
- stop treating `docs_writer_thread_v1` as a live orchestration primitive
- preserve only the behavior knowledge needed for later capability graph compilation
- remove current runtime dependence on docs-writer workflow execution

Outcome:
- docs writer remains useful as a planning and compilation example
- workflow abstraction no longer survives only because docs writer uses it today

Verification:
- docs writer workflow execution is no longer part of active runtime routing
- design docs continue to reference docs writer only as a future capability graph example

## File Level Execution Order

1. [binding.rs](/home/jerkytreats/meld/src/workflow/binding.rs)
2. [program.rs](/home/jerkytreats/meld/src/context/generation/program.rs)
3. [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs)
4. [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
5. [runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
6. [route.rs](/home/jerkytreats/meld/src/cli/route.rs)
7. [commands.rs](/home/jerkytreats/meld/src/workflow/commands.rs)
8. [facade.rs](/home/jerkytreats/meld/src/workflow/facade.rs)
9. [registry.rs](/home/jerkytreats/meld/src/workflow/registry.rs)
10. [executor.rs](/home/jerkytreats/meld/src/workflow/executor.rs)
11. [resolver.rs](/home/jerkytreats/meld/src/workflow/resolver.rs)
12. [gates.rs](/home/jerkytreats/meld/src/workflow/gates.rs)
13. [state_store.rs](/home/jerkytreats/meld/src/workflow/state_store.rs)

## Verification Matrix

Routing gates:
- agent resolution no longer carries workflow execution behavior
- generation selection no longer emits workflow program variants
- queue and watch mode no longer route through workflow

Surface gates:
- CLI no longer presents workflow as a live product surface
- docs writer is no longer runnable through workflow execution

State gates:
- workflow thread and turn records are no longer written during normal execution
- workflow-owned durable state is no longer part of active orchestration

Boundary gates:
- reusable generation seams remain available for later capability work
- workflow cleanup does not reintroduce workflow semantics through compatibility names or routing wrappers

## Completion Criteria

1. workflow binding is gone from active agent behavior
2. workflow execution mode is gone from generation program selection
3. queue and watch mode no longer branch into workflow execution
4. workflow CLI surfaces are removed or reduced to explicit migration-only helpers
5. workflow registry, executor, and state store are no longer live runtime dependencies
6. docs writer remains only as future capability graph design input

## Read With

- [Workflow Refactor](README.md)
- [Workflow Refactor Code Path Findings](code_path_findings.md)
- [Context Generate Technical Spec](../context/technical_spec.md)
- [Merkle Traversal Technical Spec](../capability/merkle_traversal/technical_spec.md)
- [Capability And Plan Implementation Plan](../PLAN.md)
