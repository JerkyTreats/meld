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
- runtime wide sequencing
- durable append
- idempotent append through `record_id`
- replay after sequence
- event bus ingestion
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
- `EventBus`
- `EventIngestor`
- `SharedIngestor`
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
