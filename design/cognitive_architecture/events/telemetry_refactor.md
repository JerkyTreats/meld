# Event Spine Refactor

Date: 2026-04-12
Status: active
Scope: directly implementable migration from the current telemetry runtime into the first real spine

## Objective

Turn the current telemetry centered implementation into a canonical event spine for the first `execution` slice.

This document is the implementation plan.
Target requirements live in [Event Spine Requirements](event_manager_requirements.md).

## Closed Decisions

- the first landing domain is `execution`
- `seq` becomes runtime wide
- `domain_id`, `stream_id`, and optional `content_hash` land in the first envelope update
- telemetry becomes downstream and compatibility focused
- raw sensory lanes stay outside the canonical spine

## Current Code Reality

The first refactor should build from the existing code rather than replace it wholesale.

- `src/telemetry/events.rs` already defines the durable envelope
- `src/telemetry/routing/ingestor.rs` already centralizes sequence assignment
- `src/telemetry/sinks/store.rs` already persists the ordered log
- `src/telemetry/sessions/service.rs` still drains and flushes on every emit
- `src/task/events.rs` and `src/task/executor.rs` still hold authoritative execution facts locally
- `src/control/orchestration.rs` still emits raw progress style names directly
- `src/task/runtime.rs` still emits workflow compatibility events through `ProgressRuntime`

## First Deliverable

The first implementation pass should produce one working execution spine with these properties:

- one canonical stored envelope with runtime-wide `seq`
- one canonical execution event family
- one append path for execution facts
- compatibility readers for current telemetry consumers
- replay tests that rebuild at least one execution projection

## Phase 1

### Envelope And Store

Edit these files first:

- `src/telemetry/events.rs`
- `src/telemetry/routing/ingestor.rs`
- `src/telemetry/sinks/store.rs`
- `src/telemetry/sessions/service.rs`

Required changes:

- introduce `SpineEvent` and `SpineEnvelope`, or rename the current types while preserving serde compatibility
- add `domain_id`, `stream_id`, and `content_hash`
- move sequence assignment from session meta to one runtime-wide spine counter
- keep `session` as grouping metadata rather than the ordering boundary
- keep old stored records readable through serde defaults or decode shims

Store changes:

- event keys should sort by runtime-wide `seq`
- session indexes may remain as secondary lookup paths during migration
- session metadata may remain for lifecycle state, but no longer owns the primary sequence

## Phase 2

### Canonical Execution Event Contracts

Edit these files next:

- `src/task/events.rs`
- `src/task/executor.rs`
- `src/control/orchestration.rs`
- `src/task/runtime.rs`

Required changes:

- define the first canonical execution event family
- standardize names under `execution.task.*`, `execution.control.*`, and `execution.repair.*`
- ensure each emitted event can derive one `stream_id`
- ensure payloads carry enough identifiers for replay, artifact lookup, and projection

Immediate stream rules:

- task lifecycle events use `task_run_id`
- generation plan events use `plan_id`
- workflow turn compatibility events use `workflow_id`

Compatibility rule:

`Vec<TaskEvent>` may remain as a local cache during migration, but it must stop being treated as authoritative.

## Phase 3

### Canonical Append Path

Edit these files after the contracts exist:

- `src/telemetry/sessions/service.rs`
- `src/telemetry/routing/bus.rs`
- `src/telemetry/routing/ingestor.rs`

Required changes:

- one canonical append path for execution events
- bounded ingress semantics
- explicit durable commit policy
- remove implicit full drain and flush from every low value emit on the correctness path

The implementation may still remain single-runtime and local.
Do not add broker or distributed sequencing work in this phase.

## Phase 4

### Reducer Owned Execution Projection

Add one reducer owned execution projection that proves replay is real.

The first projection set should derive at least:

- active task set
- blocked task set
- completed task set
- failed task set
- artifact availability

This reducer is the first proof that execution facts now flow through one canonical path.

## Phase 5

### Downstream Telemetry Cutover

After canonical append and replay are proven:

- move summaries and TUI readers to the canonical spine or derived projections
- keep telemetry sinks under `src/telemetry`
- remove direct ownership of business event meaning from telemetry code

## Recommended Pull Request Slices

1. envelope and store migration with runtime-wide `seq`
2. canonical execution event family and compatibility bridge
3. `TaskExecutor` cutover from local authority to spine publication
4. reducer owned execution projection and replay tests
5. downstream telemetry cutover

## Required Tests

Before old paths are removed, add characterization and parity coverage for:

- runtime-wide sequence monotonicity across more than one session
- stored record compatibility for pre-refactor events
- replay into one reducer owned execution projection
- `TaskExecutor` parity between local behavior and durable spine output
- current telemetry readers continuing to function during the transition

## Acceptance Criteria

The refactor should not be considered complete until these outcomes are proven by tests:

### Runtime-Wide Sequence

Test name:
`runtime_wide_sequence_is_monotonic`

Expected outcome:

- appends across two or more sessions produce one strictly increasing `seq`
- the stored event order is no longer session scoped

### Legacy Read Compatibility

Test name:
`legacy_events_remain_readable`

Expected outcome:

- pre-refactor stored records can still be decoded through the new envelope layer
- migration does not orphan historical events

### Canonical Execution Publication

Test name:
`task_executor_publishes_canonical_events`

Expected outcome:

- one small compiled task publishes canonical `execution.task.*` facts into the spine
- local `TaskEvent` history is no longer treated as the source of truth

### Replay-Owned State

Test name:
`replay_rebuilds_execution_projection`

Expected outcome:

- replay of one known execution stream rebuilds the expected execution projection
- active, blocked, completed, failed, and artifact state come from the reducer rather than ad hoc runtime mutation

### Live And Replay Parity

Test name:
`projection_matches_live_execution`

Expected outcome:

- the final projection produced by live execution matches the final projection produced by replay from durable events
- replay is a real correctness path rather than a diagnostic approximation

### Telemetry Downstream Isolation

Test name:
`telemetry_is_downstream_only`

Expected outcome:

- telemetry summaries still work from canonical spine records
- canonical append and projection correctness do not depend on telemetry consumers being active

### Slow Consumer Isolation

Test name:
`slow_or_missing_consumer_does_not_break_append`

Expected outcome:

- one stalled downstream consumer does not break append correctness
- replay and projection rebuild still succeed

## Explicit Non Goals

This plan does not yet include:

- raw sensory ingestion
- world-state reduction
- distributed brokers
- cross-machine sequencing
- ECS internals for world state

## Done When

This refactor is complete when all of these are true:

- canonical execution facts append through one spine path
- runtime-wide sequence is live in storage
- replay rebuilds at least one execution projection
- telemetry is downstream rather than authoritative
- `TaskEvent` local history is no longer the source of truth
- the acceptance criteria tests all pass
