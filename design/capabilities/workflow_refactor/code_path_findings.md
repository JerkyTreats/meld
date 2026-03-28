# Workflow Refactor Code Path Findings

Date: 2026-03-28
Status: active
Scope: current workflow code paths and the removal lift required to clear the way for capability and plan

## Intent

Capture how workflows work today and where they are embedded in the current codebase.
This document is not a code review.
It is a current-state map for removal planning.

## Source Specs

- [Workflow Refactor](README.md)
- [Capability And Plan Design](../README.md)
- [Capability And Plan Implementation Plan](../PLAN.md)

## Baseline Findings

### W1 The current workflow system is a profile-driven turn runner, not a general planning layer

The model in [profile.rs](/home/jerkytreats/meld/src/workflow/profile.rs) is a workflow profile made of ordered turns, gates, thread policy, artifact policy, and failure policy.
The main built-in profile in [builtin.rs](/home/jerkytreats/meld/src/workflow/builtin.rs) is `docs_writer_thread_v1`.

In practical terms, the current workflow subsystem is mostly a specialized multi-turn runner for docs writing.
It is not the durable planning substrate the new direction wants.

### W2 Current behavior is concentrated around one meaningful builtin flow

The builtin path in [builtin.rs](/home/jerkytreats/meld/src/workflow/builtin.rs) defines a four-turn docs writer chain:

- evidence gather
- verification
- readme struct
- style refine

This matters for removal scope.
The workflow subsystem is not broad in active behavior.
It is concentrated around one meaningful example.

### W3 The workflow registry is simple, but it is a live dependency

The registry in [registry.rs](/home/jerkytreats/meld/src/workflow/registry.rs) loads workflow YAML from workspace and user locations, merges builtins, validates prompt references, and exposes resolved profiles by `workflow_id`.

The registry itself is not a large lift to remove.
The lift comes from how many other surfaces assume it exists.

### W4 Workflow execution is deeply coupled to context generation internals

The executor in [executor.rs](/home/jerkytreats/meld/src/workflow/executor.rs) is not a thin shell over a separate engine.
It directly uses prompt contracts, provider preparation, completion execution, metadata validation, frame creation, and frame persistence from the context-generation stack.

That means removing workflows does not remove core generation behavior.
It removes a multi-turn wrapper that is tightly interwoven with current generation seams.

### W5 Workflow dataflow is implemented as turn output maps and gate checks

The resolver in [resolver.rs](/home/jerkytreats/meld/src/workflow/resolver.rs) builds turn inputs from prior turn outputs or target context.
The gate layer in [gates.rs](/home/jerkytreats/meld/src/workflow/gates.rs) validates turn outputs after model execution.

So the current workflow model owns one local dataflow system:
- prior turn outputs become later turn inputs
- gate checks validate outputs between turns

That dataflow will need a replacement once docs writer is reintroduced as capability graph compilation.

### W6 Workflow state is durable and operator-visible

The workflow state store in [state_store.rs](/home/jerkytreats/meld/src/workflow/state_store.rs) persists thread, turn, gate, and prompt-link records under the workspace data area.

This makes workflow more than a transient executor.
It has durable operational state, resume behavior, and telemetry coupling.
Removing workflows therefore means deciding what happens to these records and the operators that rely on them.

### W7 Workflow is embedded in context execution selection

Agent binding and execution selection tie workflow into generation mode:

- agent binding is validated in [binding.rs](/home/jerkytreats/meld/src/workflow/binding.rs)
- execution mode selection happens in [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs)
- target execution request and result types carry workflow fields in [program.rs](/home/jerkytreats/meld/src/context/generation/program.rs)

This means workflow is not isolated behind its own command surface.
It shapes how context generation decides what path to take.

### W8 Queue execution has workflow-specific branching and retry behavior

The queue in [queue.rs](/home/jerkytreats/meld/src/context/queue.rs) branches on `TargetExecutionProgramKind::Workflow`.
It routes workflow work to [facade.rs](/home/jerkytreats/meld/src/workflow/facade.rs) and also has workflow-specific retry classification logic.

That makes workflow a live part of generation transport and failure handling.

### W9 Watch mode also runs workflows directly

Watch mode in [runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs) checks agent workflow binding, resolves the registry entry, constructs a workflow execution request, and runs the workflow executor directly.

So removal work is not limited to CLI and queue.
It also includes workspace watch behavior.

### W10 CLI owns a full workflow command surface

The CLI route in [route.rs](/home/jerkytreats/meld/src/cli/route.rs) exposes:

- workflow list
- workflow validate
- workflow inspect
- workflow execute

The command service in [commands.rs](/home/jerkytreats/meld/src/workflow/commands.rs) depends on registry, facade, and execution request construction.

These are straightforward to remove, but they are user-visible surfaces and should be treated as an intentional break.

## Removal Shape

The smallest durable interpretation of this system is:

- a profile registry
- one turn executor
- one simple state store
- several integration seams that depend on workflow routing

So the code volume is not the hard part.
The hard part is the integration density.

## Removal Lift

The lift to remove workflows is best understood as concentrated rather than broad.

It is smaller than a large planning rewrite because:

- there is one main builtin flow
- the registry and command surfaces are simple
- workflow is not the source of truth for generation behavior itself

It is larger than deleting `src/workflow` because workflow concepts currently shape:

- agent binding
- context execution mode
- queue routing
- watch mode
- telemetry
- durable thread and turn state
- docs-writer multi-step behavior

## Immediate Removal Targets

The first things that can be targeted once replacement seams exist are:

- agent `workflow_id` semantics
- `TargetExecutionProgramKind::Workflow`
- workflow-specific queue routing
- workflow watch-mode execution path
- workflow CLI commands

## Replacement Dependencies

The parts that need replacement before full removal are:

- docs-writer multi-step behavior
- turn-to-turn dataflow currently implemented as output maps
- gate-like output validation semantics where they still matter
- any required operator-visible progress and resume behavior

## Practical Conclusion

Workflows are removable.
They are not the deepest domain behavior in the system.

The lift is meaningful because workflows sit at several routing seams.
But the removal is very feasible if the replacement plan is:

- pull reusable implementation seams out first
- replace workflow-owned docs-writer behavior with capability-graph compilation later
- then remove workflow routing and workflow state as a dedicated concept
