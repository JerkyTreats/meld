# Events Domain

Date: 2026-04-22
Status: active
Scope: declarative design for the shared event ledger

## Intent

`events` is the canonical ledger for promoted semantic facts.

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
- world model owns derived graph and later belief facts
- `sensory` owns observation promotion rules

`telemetry` is downstream.
It consumes event history for summaries, metrics, operator feedback, and compatibility.

## Event Contract

The event contract requires:

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

Only promoted semantic facts belong in the canonical event ledger.

Raw sensory pulses, raw file watcher noise, transient worker chatter, and presentation summaries stay outside the canonical correctness path.

## Active Documents

- [Event Ledger Requirements](event_manager_requirements.md)
  canonical envelope, ownership, ordering, durability, and replay requirements
- [Multi-Domain Event Ledger](multi_domain_spine.md)
  cross-domain ledger model and reference contract
- [Events Crate](CRATE.md)
  `meld-events` crate boundary, owned modules, extraction path, and forbidden dependencies

## Completed History

- [Completed Events](../../completed/events/README.md)
  extraction plan, refactor plan, and research history

## Read With

- [World Model Domain](../world_model/README.md)
- [Graph](../world_model/graph/README.md)
- [Execution Domain](../execution/README.md)
