# Events Crate Migration

Date: 2026-04-25
Status: working plan
Purpose: make [CRATE.md](CRATE.md) true through code changes in `src`

## Intent

`meld-events` should own the canonical event ledger, append, replay, sequencing, subscription shape, and cross domain graph attachment contracts.

This document records the migration work needed for that to be true in code.

## Ground Truth In `src`

The current event authority lives in:

- [src/events.rs](../../../src/events.rs)
- [src/events/contracts.rs](../../../src/events/contracts.rs)
- [src/events/store.rs](../../../src/events/store.rs)
- [src/events/runtime.rs](../../../src/events/runtime.rs)
- [src/events/ingress.rs](../../../src/events/ingress.rs)
- [src/events/subscription.rs](../../../src/events/subscription.rs)

The main blockers are visible in code:

- `EventStore` embeds `SessionStore` in [src/events/store.rs](../../../src/events/store.rs#L28)
- `EventStore` exposes session lifecycle methods in [src/events/store.rs](../../../src/events/store.rs#L66) through [src/events/store.rs](../../../src/events/store.rs#L218)
- telemetry progress runtime composes events and sessions together in [src/telemetry/sessions/service.rs](../../../src/telemetry/sessions/service.rs#L14)
- telemetry and sink compatibility still alias event store and contracts in [src/telemetry/contracts.rs](../../../src/telemetry/contracts.rs), [src/telemetry/sinks/store.rs](../../../src/telemetry/sinks/store.rs), and [src/events/query.rs](../../../src/events/query.rs)
- `EventRuntime::emit_envelope_idempotent` currently forwards to the non idempotent path in [src/events/runtime.rs](../../../src/events/runtime.rs#L64)

## Target State

When `CRATE.md` becomes declarative truth, `meld-events` should:

- expose envelope and record contracts
- own sequence allocation and durable append
- support `record_id` based idempotent append at the public runtime layer
- expose replay and query primitives
- expose graph attachment contracts through `DomainObjectRef` and `EventRelation`
- have no dependency on session lifecycle policy or telemetry compatibility

## Required Code Changes

### 1. Remove session storage from `EventStore`

The event ledger must not own command session lifecycle.

Required work:

- remove `SessionStore` from `EventStore`
- delete `put_session`, `get_session`, `list_sessions`, `put_meta`, `get_meta`, `mark_interrupted_sessions`, `prune_completed`, and `delete_session`
- move any session retention logic fully into `src/session`

Primary files:

- [src/events/store.rs](../../../src/events/store.rs)
- [src/session/storage.rs](../../../src/session/storage.rs)
- [src/session/runtime.rs](../../../src/session/runtime.rs)

### 2. Make idempotent append real at the runtime API

Store level idempotence already exists.
Runtime level idempotence does not.

Required work:

- route `emit_envelope_idempotent` to `append_envelope_idempotent`
- route batch idempotent emit through the same semantics
- add tests that prove duplicate derived envelopes with the same `record_id` do not append twice

Primary files:

- [src/events/runtime.rs](../../../src/events/runtime.rs)
- [src/events/store.rs](../../../src/events/store.rs)

### 3. Move telemetry compatibility out of the events authority

Telemetry should observe the event ledger, not define its surface.

Required work:

- stop using telemetry aliases as public event contracts
- keep telemetry sink adapters outside the extracted crate
- make telemetry consume the event public API like any other observer

Primary files:

- [src/telemetry/contracts.rs](../../../src/telemetry/contracts.rs)
- [src/telemetry/sinks/store.rs](../../../src/telemetry/sinks/store.rs)
- [src/telemetry/sessions/service.rs](../../../src/telemetry/sessions/service.rs)

### 4. Clean up naming and query surfaces

The code still mixes `spine` naming and event crate naming.
That is workable during migration, but the extracted crate should present one vocabulary.

Required work:

- rename public `spine` facing APIs to event ledger names
- keep compatibility wrappers only where older call sites still exist
- ensure `EventQueryStore` style aliases do not become the long term public surface

Primary files:

- [src/events/store.rs](../../../src/events/store.rs)
- [src/events/query.rs](../../../src/events/query.rs)
- [src/events.rs](../../../src/events.rs)

## Ordered Migration Plan

### Step 1

Separate session lifecycle from event storage.

Deliverables:

- `EventStore` contains event data only
- session code composes with events from outside the store

### Step 2

Fix runtime idempotent append behavior.

Deliverables:

- public idempotent emit matches store behavior
- tests cover duplicate derived append

### Step 3

Move telemetry aliases and compatibility types out of the extracted boundary.

Deliverables:

- telemetry depends on events
- events no longer depend on telemetry or session

### Step 4

Finalize event crate public API naming and compatibility wrappers.

Deliverables:

- public API uses event ledger terminology
- legacy names are compatibility only

## Exit Criteria

`CRATE.md` is ready to become declarative once all of the following are true:

- `EventStore` has no session lifecycle dependency
- public runtime idempotent append is correct
- telemetry consumes event APIs as an observer
- `meld-events` can be used by world model and execution through public contracts only

## Non Goals For This Migration

- changing domain event meanings
- adding distributed sequencing
- moving world model or execution logic into the event crate
