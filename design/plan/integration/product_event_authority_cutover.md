# Product Event Authority Cutover — E5 Detail

Date: 2026-07-09
Revised: 2026-07-10
Status: pending E2 through E4 in the active event closeout
Scope: E5 direct product and CLI cutover to one event authority

## Requirement

One product identity must have exactly one canonical event ledger identity and one event sequence space.

Every semantic event producer, replay consumer, reducer, subscription, and observability surface must consume capabilities derived from that authority.
One physical authority binding is selected for the product identity and remains stable across process shapes.
Physical layout must not create another logical history for the same product.

Source intent:

- [Events Domain](../../cognitive_architecture/events/README.md)
- [Multi-Domain Event Ledger](../../cognitive_architecture/events/multi_domain_spine.md)
- [Product Event Authority Assessment](product_event_authority_domain_assessment.md)
- [Event Foundation Closeout Program](../events/event_foundation_closeout_program.md)

## Program Position

This document is the detailed E5 plan inside the active event foundation closeout.
E2 establishes the authority core, E3 hardens observability and the transport-neutral remote contract, and E4 migrates domain ports.
E5 then cuts CLI and product assembly over in direct single-process mode.

Real daemon hosting and remote process integration start only after event closure.

## Current Problem

Normal command dispatch constructs `RunContext` and `CliRuntimeAssembly` before selecting the command implementation.
That assembly opens the current CLI database and constructs `ProgressRuntime`, event emission, and graph replay over its event trees.

`meld runtime run` then constructs `ProductRuntimeAssembly` and opens the product ledger at `product_root/ledger.sled`.
Product append, replay, graph work, runtime health facts, and product observability use that second event history.

`meld event` commands remain bound to the CLI-owned `ProgressRuntime` history.
There is no shared ledger identity, migration cursor, cutover marker, or rule preventing both histories from receiving semantic writes.

This is not a valid steady state.
The CLI remains a permanent adapter.
Its independent event-store construction and the legacy event trees are compatibility seams pending cutover to the product event authority.

## Ownership

| Concern | Owner | Rule |
| --- | --- | --- |
| Ledger identity and authority capabilities | `meld-events` | Define one identity-bearing append, replay, subscription, and observability authority |
| Product identity to ledger identity binding | root `meld` with `config` | Persist and resolve one binding before CLI or product runtime adapters are assembled |
| CLI command routing | `cli` | Consume injected authority capabilities and never select a canonical store |
| Session compatibility | `telemetry` and `session` | Session lifecycle storage may remain separate, but promoted session facts publish through the product authority |
| Product runtime assembly | root `meld` with `runtime` | Consume the resolved authority rather than open another event store |
| Graph replay and derived facts | `world_state` | Consume replay and append capabilities without direct event store construction |
| Legacy history migration | `events` with root `meld` | Characterize, migrate, verify parity, record cutover, and stop legacy writes |
| Direct operator event reads | `events` with `cli` | Read the configured product authority |
| Remote event contract | `meld-events` | Define transport-neutral requests, responses, identity checks, and conformance |
| Remote process hosting | `runtime` | Own daemon lifecycle, IPC, reconnects, endpoints, and authentication after event closure |

## Required Work

### 1. Required Inputs From E2 Through E4

- Persisted `LedgerIdentity` and one `EventAuthority` aggregate exist.
- Append, replay, subscription, watermark, and observability capabilities carry and validate that identity.
- Identity mismatch and split-brain writable opens fail without fallback.
- Production domains cannot construct an alternate writable canonical store.
- Execution publication and world model replay retain their domain contracts through thin root adapters.
- Graph-derived and execution appends advance the same writer and notification source.

### 2. Root composition cutover

- Resolve the product event authority before constructing CLI or runtime adapters.
- Inject the authority appender into `ProgressRuntime` and other compatibility facades.
- Make `ProductRuntimeAssembly` consume the resolved authority.
- Ensure `meld runtime run` does not create a compatibility event authority before opening the product runtime.
- Resolve local authority capabilities for the direct single-process product route.
- Never fall back to the CLI event trees when the configured authority is unavailable or identity validation fails.
- Keep projection databases physically separate from the canonical ledger where product storage requires it.

### 3. Producer and consumer cutover

- Route workspace, context, execution, workflow, task, provider, control, and promoted session facts through the product authority.
- Route graph replay and derived event append through authority capabilities.
- Remove direct production use of writable `EventStore` values outside the events authority.
- Prove every semantic cursor advances against the same ledger identity.

### 4. Compatibility migration

- Characterize event history in existing CLI stores.
- Treat that history as the active ordinary-command writer until the exclusive cutover begins.
- Define source and target ledger identities and a deterministic migration order.
- Preserve envelopes, object refs, relations, record ids, and source ordering semantics.
- Record a durable migration and cutover marker.
- Make the compatibility event trees read-only after successful cutover.
- Retain parity tests until the compatibility write path is removed.

### 5. Direct Authority Client Routing

- Bind one-shot `meld event` commands to the configured product authority.
- Bind semantic CLI publication to the same local append capability.
- Reject unavailable or mismatched configured authority without compatibility-store fallback.
- Keep status cache data non-authoritative.

## Acceptance Gates

- One product identity resolves to one stable ledger identity.
- One authoritative binding resolves that ledger identity to one direct local authority for E5.
- Authority identity mismatch and split-brain configuration fail without fallback.
- `meld runtime run` creates no second writable semantic event history.
- A CLI-produced workspace fact is visible to the product replay adapter and product event observability in the same sequence space.
- Command session facts either use the product authority or remain explicitly nonsemantic outside the canonical ledger.
- Append, replay, reducer, cursor, and observability capabilities report the same ledger identity.
- Existing compatibility history migrates without loss, duplication, or silent reinterpretation.
- A completed cutover prevents new semantic writes to compatibility event trees.
- Direct construction of an alternate writable event authority is unavailable to normal production domains.
- Reopen tests recover the same authority and sequence space.
- Direct `meld event` reads return data from the configured product authority.
- The real `meld runtime run` route uses one authority for CLI and product assembly.

## Required Proof

The product integration proof must demonstrate this path:

```text
CLI workspace fact
-> product event authority
-> product graph replay adapter
-> product event observability
```

The proof must assert one ledger identity and one monotonic event sequence across every step.

It must also start `meld runtime run` through the real command route and prove that no compatibility event stream receives concurrent semantic writes.
The proof validates routing and sequence authority without requiring supervisor tick cadence or a complete semantic flywheel turn.

## Dependencies And Handoffs

- E5 starts only after E2 authority core, E3 observability and remote-contract hardening, and E4 domain port migration pass.
- E6 closes the event foundation after this cutover and its route-level proof pass.
- Runtime visibility, daemon hosting, and semantic wiring resume after event closure.
- The future daemon edge consumes the event-owned remote contract and must preserve the same ledger identity.
- Compatibility removal follows the repository compatibility shim policy and requires characterization and parity proof.

## Non Goals

- Recombine product projection databases with the ledger.
- Make supervisor heartbeats or leases canonical semantic events.
- Move event payload meaning into `meld-events`.
- Make the events domain resolve workspace configuration or XDG paths.
- Require one operating system process for all event clients.
- Invoke `RuntimeStatusPublisher` or persist the runtime status cache.
- Build daemon lifecycle, IPC framing, reconnects, endpoint management, or authentication.
- Require real direct-versus-daemon process tests for E5 closure.
