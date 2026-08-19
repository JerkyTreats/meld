# Events Domain

Date: 2026-04-22
Status: active
Scope: declarative design for the shared event ledger

## Intent

`events` is the canonical ledger for promoted semantic facts.

The design goal is one durable temporal ledger across `workspace_fs`, `context`, `execution`, `world_state`, and `sensory`.

## Ledger Authority Invariant

One product identity has exactly one canonical event ledger authority and one event sequence space.
One authoritative binding maps that product identity to a stable ledger identity and one local or process-owned authority endpoint.

Every semantic producer, replay consumer, reducer, subscription, and observability surface must receive capabilities derived from that authority.
CLI, runtime, daemon, TUI, and HTTP surfaces are clients of the authority.
They must not construct or select an alternate canonical history for the same product identity.

Physical storage layout does not change this invariant.
The canonical ledger may share a database with projections or live in a dedicated database, but storage placement must not create a second logical ledger authority.
These layouts are alternative bindings for the authority.
Changing the binding is a ledger migration, not creation of another semantic stream.
The selected binding remains stable across CLI, supervised runtime, daemon, and presentation clients.
Each client reaches that binding directly or through an authority-preserving process boundary.
Binding or ledger identity mismatches fail instead of selecting a fallback history.

A compatibility ledger may exist only as a migration source.
It must not remain a concurrent semantic writer after authority cutover.
Migration requires characterization, parity proof, an explicit cutover boundary, and preservation of semantic history.

## Boundary

`events` owns:

- canonical envelope
- ledger identity and authority capabilities
- ingress
- sequence
- durable append
- replay
- subscription
- commit watermark and notification truth
- health, flow, trace, session, and paging contracts
- transport-neutral authority requests, responses, durability outcomes, identity validation, and conformance
- stored envelope compatibility
- graph attachment primitives

Domain owners own event meaning:

- `workspace_fs` owns workspace facts
- `context` owns frame and head facts
- `execution` owns task, control, workflow, and artifact facts
- world model owns derived graph, belief, Agent curation, and Strategy judgment facts
- `sensory` owns observation promotion rules
- a producer-owned runtime-health or sensory concern owns health thresholds, hysteresis, process epochs, retry and outbox state, and promotion decisions

`events` durably appends promoted facts presented through its authority capability.
It does not promote raw health signals or define the policy that decides when a health observation becomes semantic.

An Agent-authorized Strategy decision is a fact about semantic judgment. Goal admission is a fact about authorized intent entering Execution. `NoMethodAvailable` is a world-model runtime error stating that bounded Strategy construction could connect no reusable or novel theory of action to a Goal draft. An accepted task-network commitment is a fact about operational intent. None is an observed external outcome or evidence of Goal satisfaction.

`telemetry` is downstream.
It consumes event history for summaries, metrics, operator feedback, and compatibility.

## Event Contract

The event contract requires:

- one stable ledger identity per product identity
- one runtime-wide sequence assigned atomically with the append
- append-only canonical history
- stable `record_id` support for idempotent derived facts
- domain and stream identity
- explicit event type
- explicit recorded time
- optional occurred time
- optional content hash
- graph object refs
- graph relation edges
- identity-bearing observability reports
- explicit scanned range and truncation state for bounded reads
- structural provenance through object refs, relation edges, and record identity
- one-time migration of legacy stored rows into the canonical ledger

## Durability And Delivery Contract

- Two producer classes: durable emits ack after the flush that made them durable; best-effort emits enqueue without blocking, and a full queue drops them with counted diagnostics, never silently.
- One ordered writer owns producer appends; concurrent producers share group commits.
- A commit watermark exposes the highest durably committed sequence; consumers wake on it instead of polling, and a barrier proves everything enqueued earlier has reached the store.
- Replay cost is proportional to events returned, never to total history.
- Consumer cursors live in consumer stores; the ledger never persists a consumer position.
- A retained lower boundary bounds replay: cursors below it receive a typed retention gap rather than silently skipped history, and genesis facts record rebuilt-from-snapshot bases so replay from zero is never required.

## Inclusion Rule

Only promoted semantic facts belong in the canonical event ledger.

Raw sensory pulses, raw file watcher noise, transient worker chatter, and presentation summaries stay outside the canonical correctness path.

## Active Documents

- [Multi-Domain Event Ledger](multi_domain_spine.md)
  cross-domain ledger model and reference contract
- [Events Crate](CRATE.md)
  `meld-events` crate boundary, owned modules, extraction path, and forbidden dependencies

## Read With

- [World Model Domain](../world_model/README.md)
- [Graph](../world_model/graph/README.md)
- [Execution Domain](../execution/README.md)
