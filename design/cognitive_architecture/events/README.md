# Events Design

Date: 2026-04-12
Status: active
Scope: implementable design set for the shared event spine

## Intent

This directory is the active plan for the event spine.
It should be read as one design set, not as disconnected notes.

The design goal is one durable spine for promoted semantic facts across `sensory`, `world_state`, `execution`, and attached object domains such as the workspace tree.

## Review Of Current State

The repo already has useful spine pieces:

- `src/telemetry/events.rs` defines a durable envelope
- `src/telemetry/routing/ingestor.rs` already serializes append through one ingestor
- `src/telemetry/sinks/store.rs` already persists ordered records
- `src/task/events.rs` already contains much of the execution vocabulary
- `src/workspace/watch/events.rs` already models local batching before promotion

The current system is not yet a true spine:

- sequence is session scoped rather than runtime wide
- `ProgressRuntime::emit_event` drains and flushes on every emit
- execution facts are split between durable telemetry and task-local `Vec<TaskEvent>`
- telemetry still owns too much transport and naming surface
- raw watch and workflow behavior are not separated cleanly from promoted semantic facts

## What This Design Set Must Produce

After this doc pass, the event spine plan should answer these questions without hand waving:

- what the canonical envelope is
- what sequence model the spine uses
- what belongs in the spine and what stays outside it
- which event families land first
- which current files change first
- what compatibility rules protect the migration
- what tests prove parity and replay correctness

## Read Order

1. [Event Spine Requirements](event_manager_requirements.md)
2. [Event Spine Refactor](telemetry_refactor.md)
3. [Multi-Domain Spine](multi_domain_spine.md)
4. [Event Management Research](research.md)

## Boundary

`events` is not `telemetry`.

- `events` owns ingress, sequence, durability, replay, subscription, and compatibility rules
- `sensory`, `world_state`, and `execution` own event meaning and reducer logic
- `telemetry` consumes the spine for observability, summaries, metrics, and external export

## Read With

- [Spine Concern](../spine/README.md)
- [Execution Control](../execution/control/README.md)
- [Task Network](../execution/control/task_network.md)
- [Runtime Model](../execution/control/runtime/README.md)
