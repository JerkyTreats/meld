# Events Readiness Assessment

Status: event foundation closed and ready for runtime consumption
Depends on: [Event Foundation Closeout Program](event_foundation_closeout_program.md)
Design source: `design/cognitive_architecture/events/README.md`, `design/cognitive_architecture/events/multi_domain_spine.md`, `design/cognitive_architecture/events/CRATE.md`
Evidence date: 2026-07-12

## Verdict Summary

The events foundation is closed. `meld-events` now provides one correct, observable, identity-bearing authority that does not require a supervisor. The direct product CLI and product runtime assembly resolve the same durable product binding, ledger identity, sequence, watermark, replay source, and observability source.

The completed [Event Spine Overhaul](event_spine_overhaul_program.md) established durable ledger mechanics. The completed [Event Observability PLAN](event_observability_program.md) established the first operator read surfaces. The completed [Event Foundation Closeout Program](event_foundation_closeout_program.md) repaired the remaining correctness defects, introduced the authority aggregate, hardened observability and structural provenance, migrated domain ports, defined remote conformance, migrated compatibility history, and cut the real product route over to one authority.

Runtime visibility, daemon hosting, and semantic wiring may resume from this stable foundation. Their completion is not claimed here.

## Conceptual Correctness

The implementation provides a shared temporal substrate without moving product meaning into the ledger.

Events own canonical order, persisted ledger identity, authority capabilities, durable envelopes, replay, structural provenance, bounded observability truth, retention boundaries, and transport-neutral authority semantics. Product domains own the meaning of facts and projections.

## Completeness

The event-owned foundation is complete.

One `EventAuthority` derives identity-bearing append, replay, subscription, watermark, consumer registry, and observability capabilities. All canonical appends advance one durable sequence and one notification source. Reopen restores ledger identity, watermark, cursor state, and next-sequence behavior. Local and serde loopback clients satisfy one transport-neutral conformance suite.

The product cutover is complete for direct single-process operation. Existing history migrates through a recoverable, parity-tested protocol. Active bindings fail closed on path or identity mismatch. The target ledger durably claims its branch product identity, so separate branch-local binding files cannot alias one authority. Legacy event trees reject new semantic writes after cutover. Direct `meld event` commands and `meld runtime run` reuse the resolved authority and shared graph runtime.

## Boundary Clarity

Events own ledger identity, authority construction, append, replay, subscription, watermark, idempotence, envelope compatibility, runtime-wide sequence, structural provenance, structural observability, `DomainObjectRef`, and `EventRelation`.

Events do not own `Proposition`, `Effect`, `Goal`, `Method`, `WorldState`, graph materialization, belief settlement, execution policy, telemetry summary, supervisor scheduling, runtime status publication, daemon hosting, real IPC, or health promotion policy.

Production domains cannot construct writable raw event stores or writer handles. Execution retains its publication sink. World model retains replay, derived-event sink, and cursor-reporting ports. Root adapters bridge those domain contracts to authority capabilities without interpreting semantic payloads.

## Dependency Readiness

The dependency gate on events is satisfied.

`meld-lang` consumes event identity types. World model graph consumes authority-backed event facts and relations. Execution publishes through its authority-backed sink. The direct CLI and product runtime assembly resolve one product binding.

Runtime now owns the downstream R1 through R4 sequence: status publisher and cache, daemon and real IPC, console and action publishers, and full semantic flywheel proof.

## Current Implementation Evidence

- `crates/meld-events/src/events/authority.rs`
- `crates/meld-events/src/events/migration.rs`
- `crates/meld-events/src/events/observability.rs`
- `crates/meld-events/src/events/remote.rs`
- `crates/meld-events/src/events/remote/conformance.rs`
- `crates/meld-events/src/events.rs`
- `src/events/binding.rs`
- `src/runtime/assembly.rs`
- `tests/integration/product_event_authority_cutover.rs`
- [Event Spine Overhaul PLAN](event_spine_overhaul_program.md)
- [Event Observability PLAN](event_observability_program.md)
- [Event Foundation Closeout Program](event_foundation_closeout_program.md)
- [Product Event Authority Cutover](../integration/product_event_authority_cutover.md)
- [Spine Compaction Design](spine_compaction_design.md)

## Verification Evidence

E1 landed in commits `b9ba7a3` and `c09c2ae`. E2 landed in `a20390d` and `215ef32`. E3 landed in `303d9c1`, `d19d742`, and `01e4f59`. E4 landed in `3c6b26d`, `e73dfa0`, and `b7781a2`. E5 landed in `97cc225`, with repeated-gate hardening through `9350a90`, including product-identity fencing, bounded reopen recovery, and stable performance verification.

The feature-enabled event suite passed 174 tests and 3 doctests. The authority, cursor, concurrency, and recovery suites passed 25 consecutive normal runs and 25 consecutive serial runs. The full workspace all-targets suite passed three consecutive times. The formatter, build, warning-denying clippy, domain boundaries, focused crate suites, route-level integration suites, local conformance suite, and serde loopback conformance suite all passed.

Fresh migration, routing, constructor-sealing, recovery, concurrency, architecture, and compatibility reviews were clean. The final architecture review found and then cleared the shared-target aliasing blocker through the persisted product claim and its failed-flush retry proof.
An independent verifier reproduced the clean CI no-lock ladder at `9350a90`, including the first full workspace all-targets run and focused reopen and timing stress.

The closing benchmarks stayed inside the ten percent threshold. The largest protected regression was eight durable writers at 4.2 percent. Replay at tip changed by 2.5 percent, health by 0.3 percent, and all other protected measurements improved.

## Remaining Runtime-Owned Work

- R1 owns status publisher invocation, status-cache persistence, and staleness.
- R2 owns daemon lifecycle, process ownership, real IPC framing, reconnects, endpoints, and authentication.
- R3 owns console frames, runtime action records, and `event.append` heartbeat mapping.
- R4 owns the complete semantic flywheel and operator-visibility proof.
- A producer-owned runtime-health or sensory concern owns thresholds, hysteresis, process epochs, retry and outbox state, and promotion decisions.

The existing `SelfObservationWatcher` remains provisional. Its presence does not move promotion policy into events.

## Recommendation

Resume the runtime workstream from R1. Consume the resolved event authority rather than reopening or reconstructing canonical event storage.
