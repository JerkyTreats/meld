# Events Readiness Assessment

Status: ledger mechanics ready; event foundation not closed
Depends on: [Event Foundation Closeout Program](event_foundation_closeout_program.md)
Design source: `design/cognitive_architecture/events/README.md`, `design/cognitive_architecture/events/multi_domain_spine.md`, `design/cognitive_architecture/events/CRATE.md`
Evidence date: 2026-07-10

## Verdict Summary

Ledger mechanics are hardened by the completed [Event Spine Overhaul](event_spine_overhaul_program.md) and the first observability surfaces are delivered by the completed [Event Observability PLAN](event_observability_program.md).
The events foundation is not yet ready for resumed runtime visibility, daemon, or semantic wiring.

The ready slice covers atomically sequenced durable and best-effort append, seek-based replay whose cost is independent of history size, single-writer group-commit ingress, the commit watermark and barrier, the subscription surface with consumer-owned cursors, the retention boundary contract with genesis facts, domain object references, event relations, and event ownership boundaries. Contract suites prove ordering under concurrent producers, crash durability of acked events across kill cycles, idempotency, and replay determinism; criterion baselines are recorded in the overhaul PLAN.

Foundation closure still requires the E1 through E6 work: remaining correctness regressions, persisted ledger identity, one authority aggregate, unified append and watermark truth, identity-bearing and coverage-honest observability, transport-neutral remote conformance, domain port migration, compatibility history migration, and direct product CLI cutover.

## Conceptual Correctness

The design solves the shared temporal substrate problem without moving product meaning into the ledger.

Events own canonical order, ledger identity, authority capabilities, durable envelopes, replay, structural provenance, and observability truth. Product domains own the meaning of facts and projections.

## Completeness

The implemented mechanics are complete enough for the typed loop.
They are not complete enough to claim one product event authority or resume the runtime workstream.

The typed loop uses `DomainObjectRef` and `EventRelation` as identity primitives consumed by `meld-lang`.

The runtime flywheel also requires the closed event authority so every producer, replay consumer, subscription, cursor, watermark, and observability surface shares one ledger identity and one sequence space.

## Boundary Clarity

Events own ledger identity, authority construction, append, replay, subscription, watermark, idempotence, envelope compatibility, runtime-wide sequence, structural observability, `DomainObjectRef`, and `EventRelation`.

Events do not own `Proposition`, `Effect`, `Goal`, `Method`, `WorldState`, graph materialization, belief settlement, execution policy, telemetry summary, supervisor scheduling, runtime status publication, daemon hosting, real IPC, or health promotion policy.

## Dependency Readiness

Events foundation readiness depends on E1 through E6 of the active closeout.

`meld-lang` consumes event identity types. World model graph consumes event facts and relations. Sensory and execution publish semantic facts into the event spine.

Runtime visibility, daemon, and semantic wiring resume only after E6 closes the foundation.

## First-Slice Feasibility

Events support the typed loop through shared identity types.

Events support the runtime flywheel only after the product CLI cutover proves one direct authority and one sequence through the real command route.

## Current Implementation Evidence

- `crates/meld-events/src/lib.rs`
- `crates/meld-events/src/events.rs`
- `crates/meld-events/src/events/`
- `crates/meld-events/tests/event_store_contracts.rs`
- `crates/meld-events/tests/spine_concurrency.rs`
- `crates/meld-events/tests/spine_determinism.rs`
- `crates/meld-events/tests/spine_recovery.rs`
- `crates/meld-events/benches/spine.rs`
- `crates/meld-events/benches/spine_flywheel.rs`
- `src/events.rs`
- `tests/integration/event_spine.rs`
- [Event Spine Overhaul PLAN](event_spine_overhaul_program.md)
- [Event Observability PLAN](event_observability_program.md)
- [Event Foundation Closeout Program](event_foundation_closeout_program.md)
- [Spine Compaction Design](spine_compaction_design.md)
- `design/completed/events/README.md`
- `design/completed/events/PLAN.md`
- `design/completed/events/event_domain_extraction_spec.md`

## Gaps

- Exact session partitioning, atomic cursor advancement, bounded read limits, relation-only traces, and trace coverage semantics need regression closure.
- No persisted `LedgerIdentity` or one production `EventAuthority` prevents alternate writable histories.
- Graph-derived and execution append paths do not yet prove one writer, watermark, and notification source.
- Observability reports do not yet carry ledger identity and honest scanned-range or truncation state everywhere.
- Provenance must rely only on explicit structural contracts rather than arbitrary payload strings.
- The transport-neutral remote authority contract and reusable loopback conformance suite do not exist.
- CLI compatibility history and product history remain separate writable sequence spaces until E5 migration and cutover.
- Direct `meld event` routing does not yet prove that it reads the configured product authority.

## Ownership Decisions

- Events supplies `EventHealthReport` and stable runtime status mapping inputs.
- Runtime owns status mapping, `RuntimeStatusPublisher` invocation, cache persistence, staleness, console and action records, daemon hosting, and real IPC.
- Events durably appends a supplied promoted health fact.
- A producer-owned runtime-health or sensory concern owns thresholds, hysteresis, process epochs, retry and outbox state, and promotion decisions.

## Recommendation

Execute E1 through E6 of the active closeout.
Do not resume runtime visibility, daemon, or full semantic wiring until the events foundation is closed.
