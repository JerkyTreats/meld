# Interregnum Orchestration

Date: 2026-04-03
Status: active
Scope: refactor-phase control ownership before task features land

## Intent

Define the temporary but real orchestration role of `src/control` during the refactor window.

This window exists after orchestration leaves `context` and before `task` becomes the durable execution unit.
Current CLI and workflow-triggered flows still need to run end to end during that period.

## Core Position

`control` is the honest place for the remaining orchestration logic during the refactor phase.

That logic includes:

- consuming traversal batches
- releasing work batch by batch
- waiting on batch barriers
- coordinating `context` and `provider`
- preserving current workflow-visible behavior through compatibility entry paths

This logic should not remain hidden in `context`.
It should not be pushed into `provider`.
It should not stay trapped inside legacy workflow internals.

## Why This Exists

The current system still needs ordered orchestration for cases such as docs writer behavior.

That behavior requires:

- bottom-up structural order over Merkle nodes
- batch barriers between lower and higher levels
- context preparation for the current ready batch
- provider execution for that ready batch
- result handling before the next batch is released

Those are orchestration concerns.

## Interregnum Responsibility Split

The refactor-phase split is:

- `merkle_traversal`
  returns ordered structural node batches
- `control`
  consumes those batches and advances execution
- `context`
  prepares generation input and finalizes generation results
- `provider`
  executes ready requests with batching and throttling
- `workflow`
  remains a compatibility trigger path while delegating orchestration inward

## Traversal Handoff

The traversal handoff should be structural rather than use-case specific.

Recommended traversal output:

- `ordered_merkle_node_batches`

Properties:

- each batch contains nodes that are parallel-safe relative to one another
- batches are ordered by the selected traversal strategy
- for `bottom_up`, lower structural levels appear before higher ones

Traversal does not decide provider timing.
Traversal does not decide docs-writer eligibility.
Traversal does not decide retries.

## Control Handoff

`control` consumes traversal batches and turns them into ready execution waves.

For each batch, `control` should:

1. resolve current execution targets from node refs
2. ask `context` to prepare generation input for ready targets
3. hand provider-ready requests to `provider`
4. wait for completion across the released batch
5. ask `context` to finalize results
6. decide whether the next batch can be released

This is the minimum orchestration loop needed to preserve current behavior without leaving control logic inside `context`.

## Workflow Compatibility

During the refactor period, the outer workflow trigger path should remain callable.

That means:

- CLI may still enter through workflow-facing commands
- workflow compatibility code may still resolve authored workflow inputs
- orchestration should delegate into `control`
- `workflow` should stop being the hidden owner of execution order

The external trigger path may stay stable while internal ownership changes.

## Suggested Refactor-Phase Shape

```text
src/
  control.rs
  control/
    orchestration.rs
    traversal_release.rs
    batch_barrier.rs
    compatibility.rs
```

Suggested meanings:

- `orchestration.rs`
  main refactor-phase coordination loop
- `traversal_release.rs`
  batch release logic from traversal output
- `batch_barrier.rs`
  wait and completion barrier handling
- `compatibility.rs`
  adapters for current workflow and legacy entry paths

## Boundaries

`control` should own:

- batch release
- wave completion barriers
- orchestration lineage
- compatibility delegation during the refactor window

`control` should not own:

- provider transport internals
- prompt rendering
- frame persistence
- provider-specific batching algorithms
- Merkle traversal algorithms

## Migration Direction

The migration path is:

1. extract traversal ordering into `merkle_traversal`
2. move orchestration release logic into `control`
3. narrow `context` to preparation and finalization
4. narrow `provider` to service execution
5. keep workflow as a compatibility facade
6. replace compatibility orchestration later with task-driven control execution

## Decision Summary

- add `src/control` during the refactor phase
- use it as temporary housing for orchestration logic
- preserve current workflow trigger paths through compatibility delegation
- keep traversal structural and provider execution transport-focused
