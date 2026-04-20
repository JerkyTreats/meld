# Events Design

Date: 2026-04-20
Status: active
Scope: declarative design for the shared event spine

## Intent

`events` is the canonical spine for promoted semantic facts.

The design goal is one durable temporal ledger across `workspace_fs`, `context`, `execution`, `world_state`, and later `sensory`.

Implementation history lives in [Completed Events](../../completed/events/README.md).

## Boundary

`events` owns:

- canonical envelope
- ingress
- sequence
- durable append
- replay
- subscription
- stored envelope compatibility
- graph attachment primitives

Domain owners own event meaning:

- `workspace_fs` owns workspace facts
- `context` owns frame and head facts
- `execution` owns task, control, workflow, and artifact facts
- `world_state` owns derived graph and later belief facts
- `sensory` owns observation promotion rules

`telemetry` is downstream.
It consumes spine history for summaries, metrics, operator feedback, and compatibility.

## Spine Contract

The spine contract requires:

- one runtime-wide sequence
- append-only canonical history
- stable `record_id` support for idempotent derived facts
- domain and stream identity
- explicit event type
- explicit recorded time
- optional occurred time
- optional content hash
- graph object refs
- graph relation edges
- legacy read compatibility

## Inclusion Rule

Only promoted semantic facts belong in the canonical spine.

Raw sensory pulses, raw file watcher noise, transient worker chatter, and presentation summaries stay outside the canonical correctness path.

## Active Documents

- [Event Spine Requirements](event_manager_requirements.md)
  canonical envelope, ownership, ordering, durability, and replay requirements
- [Multi-Domain Spine](multi_domain_spine.md)
  cross-domain ledger model and reference contract

## Completed History

- [Completed Events](../../completed/events/README.md)
  extraction plan, refactor plan, and research history

## Read With

- [Spine Concern](../spine/README.md)
- [World State Domain](../world_state/README.md)
- [Graph](../world_state/graph/README.md)
- [Execution Control](../execution/control/README.md)
