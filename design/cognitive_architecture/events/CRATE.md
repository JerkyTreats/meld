# Events Crate

Date: 2026-04-26
Status: declarative
Scope: `meld-events` crate for canonical event ledger ownership

## Identity

`meld-events` is the source of truth for canonical event storage and replay.
Root `meld` consumes this crate through a thin reexport shim in [src/events.rs](../../../src/events.rs).

The live implementation is in:

- [crates/meld-events/src/lib.rs](../../../crates/meld-events/src/lib.rs)
- [crates/meld-events/src/events.rs](../../../crates/meld-events/src/events.rs)
- [crates/meld-events/src/events](../../../crates/meld-events/src/events)

## Owns

- canonical `EventEnvelope` and `EventRecord`
- `DomainObjectRef`
- `EventRelation`
- runtime wide sequencing, atomic with the append
- durable and best-effort append classes
- idempotent append through `record_id`
- seek-based replay after sequence
- single-writer ingress with group commit
- commit watermark and barrier primitives
- consumer subscription surface with consumer-owned cursors
- retention boundary contract and genesis facts
- one-time legacy store migrations at open
- event runtime helpers
- stored envelope compatibility aliases

## Does Not Own

- product domain meaning
- graph materialization
- world model reduction
- execution policy
- session lifecycle policy
- telemetry summaries
- CLI behavior

## Public Surface

Primary exports are:

- `EventEnvelope`
- `EventRecord`
- `EventRuntime`
- `SpineWriter`
- `CommitWatermark`
- `DomainObjectRef`
- `EventRelation`
- `store::EventStore`

## Dependency Rule

`meld-events` does not depend on `meld-world-model`, `meld-execution`, or root `meld`.

Domain builders in other crates construct envelopes through this public contract and do not extend the event crate with domain specific meaning.

## Product Integration

Root `meld` keeps:

- telemetry session lifecycle compatibility
- adapter level reexports
- higher level domain event builders

`telemetry` compatibility now sits above the event authority crate rather than inside it.
