# Events Readiness Assessment

Status: ready
Depends on: none
Design source: `design/cognitive_architecture/events/README.md`, `design/cognitive_architecture/events/multi_domain_spine.md`, `design/cognitive_architecture/events/CRATE.md`
Evidence date: 2026-05-26

## Verdict Summary

Events are ready for the cognitive architecture plan.

The ready slice covers durable append, replay, sequence, domain object references, event relations, and event ownership boundaries.

## Conceptual Correctness

The design solves the shared temporal substrate problem without moving product meaning into the ledger.

Events own canonical order, durable envelopes, replay, object references, and relation edges. Product domains own the meaning of facts and projections.

## Completeness

The core contract is complete enough for the typed loop and later runtime flywheel.

The typed loop uses `DomainObjectRef` and `EventRelation` as identity primitives consumed by `meld-lang`.

The runtime flywheel will require concrete semantic event records for observation, action, outcome, and belief calibration.

## Boundary Clarity

Events own append, replay, idempotence, envelope compatibility, runtime-wide sequence, `DomainObjectRef`, and `EventRelation`.

Events do not own `Proposition`, `Effect`, `Goal`, `Method`, `WorldState`, graph materialization, belief settlement, execution policy, telemetry summary, or CLI behavior.

## Dependency Readiness

Events have no upstream plan dependency.

`meld-lang` consumes event identity types. World model graph consumes event facts and relations. Sensory and execution publish semantic facts into the event spine.

## First-Slice Feasibility

Events support the typed loop through shared identity types.

Events support the runtime flywheel once sensory and execution define semantic fact records that graph and belief can consume.

## Current Implementation Evidence

- `crates/meld-events/src/lib.rs`
- `crates/meld-events/src/events.rs`
- `crates/meld-events/src/events/`
- `crates/meld-events/tests/event_store_contracts.rs`
- `src/events.rs`
- `tests/integration/event_spine.rs`
- `design/completed/events/README.md`
- `design/completed/events/PLAN.md`
- `design/completed/events/event_domain_extraction_spec.md`

## Gaps

- Runtime flywheel event names must be selected for observation, action, outcome, and belief calibration.
- Raw sensory streams remain outside events until promoted into compact semantic facts.
- Domain event vocabularies remain source-domain owned.

## Open Questions

- Which runtime flywheel fact names become canonical first.
- Which execution outcome records become graph-readable event facts.

## Recommendation

Proceed.
