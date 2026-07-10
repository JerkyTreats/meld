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
- stable ledger identity and event authority contract
- `DomainObjectRef`
- `EventRelation`
- runtime wide sequencing, atomic with the append
- durable and best-effort append classes
- idempotent append through `record_id`
- seek-based replay after sequence
- single-writer ingress with group commit
- commit watermark and barrier primitives
- consumer subscription surface with consumer-owned cursors
- health, flow, trace, session, and paging contracts over one supplied authority
- transport-neutral authority requests, responses, durability outcomes, and identity validation
- reusable authority conformance contract for local and remote implementations
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
- runtime status publication and cache persistence
- supervisor scheduling and daemon process ownership
- IPC framing, reconnect, endpoint lifecycle, and authentication
- health thresholds, hysteresis, process epochs, retry and outbox state, and promotion decisions

## Public Surface

Primary cross-domain exports are:

- `EventEnvelope`
- `EventRecord`
- stable ledger identity
- append capability
- replay capability
- subscription capability
- observability capability
- `CommitWatermark`
- `DomainObjectRef`
- `EventRelation`

Event runtime, writer, and store types are event-owned persistence implementation.
They are available to event authority assembly and focused tests, not as construction seams for producer or consumer domains.

## Dependency Rule

`meld-events` does not depend on `meld-world-model`, `meld-execution`, or root `meld`.

Domain builders in other crates construct envelopes through this public contract and do not extend the event crate with domain specific meaning.

Domain consumers receive append, replay, subscription, and observability capabilities derived from one event authority.
They do not construct event stores, writers, or alternate event runtimes from raw database handles.

Root `meld` selects the physical storage binding once for a product identity and constructs the event authority through the events contract.
The storage engine seam remains behind that authority.
It must not become a public route for creating competing canonical histories.

## Product Integration

Root `meld` keeps:

- telemetry session lifecycle compatibility
- adapter level reexports
- higher level domain event builders

`telemetry` compatibility now sits above the event authority crate rather than inside it.

Compatibility adapters may translate legacy calls into authority capabilities.
They must not retain a second writable semantic ledger after cutover.
