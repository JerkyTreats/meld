# Events Crate

Date: 2026-04-22
Status: active experiment
Scope: `meld-events` crate boundary for the canonical event ledger

## Intent

`meld-events` owns the durable temporal ledger.
It replaces the separate spine concern as the routing point for append, replay, sequencing, and cross-domain references.

The crate exists to make event truth independent from world-model, execution, CLI, and telemetry code.

## Target Crate

`meld-events`

## Owns

- canonical event envelope
- runtime-wide sequence
- durable append
- idempotent derived append through `record_id`
- replay after sequence
- event query primitives
- event subscription shape
- graph attachment primitives
- `DomainObjectRef`
- `EventRelation`
- stored envelope compatibility

## Does Not Own

- event meaning for product domains
- graph materialization
- belief revision
- planner policy
- task execution
- session lifecycle policy
- telemetry summaries

## Current Code Areas

- `src/events.rs`
- `src/events/contracts.rs`
- `src/events/ingress.rs`
- `src/events/query.rs`
- `src/events/runtime.rs`
- `src/events/store.rs`
- `src/events/subscription.rs`
- `src/events/compat.rs`

## Extraction Blockers

- `EventStore` still depends on session storage and session lifecycle operations.
- `telemetry` still re-exports event compatibility surfaces.
- Some names still say spine even though the owning crate should be events.

## Target Dependencies

| From | To | Reason |
| --- | --- | --- |
| `meld-events` | storage library crates | durable event trees |
| `meld-events` | serde and time crates | event serialization and timestamps |

## Forbidden Direction

`meld-events` must not depend on `meld-world-model`, `meld-execution`, or root `meld`.

Domain event builders may live in their owning domains.
They should construct event envelopes through the public event contract.

## Migration Path

1. Move event reference contracts into `meld-events`.
2. Remove session lifecycle methods from `EventStore`.
3. Keep telemetry compatibility as an adapter outside `meld-events`.
4. Move append, replay, ingress, runtime, and query code.
5. Add contract tests proving world-model and execution code can use only the public event API.

