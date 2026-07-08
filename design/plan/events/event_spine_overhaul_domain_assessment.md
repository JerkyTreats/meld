# Event Spine Overhaul Assessment By Domain

Date: 2026-07-08
Status: active
Evidence date: 2026-07-08
Method: [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md)

## Concern Definition

The event spine overhaul changes the mechanics beneath the canonical event contract: sequencing atomicity, seek-based replay, single-writer group-commit ingress with durability classes, watermark notification, session index layout, legacy tree sunset, and a retention contract.
The envelope shape, `DomainObjectRef`, `EventRelation`, idempotency keys, and consumer-owned cursors do not change.
Producers keep their emit signatures; consumers keep pull-replay semantics and gain a wake primitive.

## In Scope

- `crates/meld-events` internals: store, ingress, runtime facade, new subscription surface
- graph replay cursor-advance correctness in `crates/meld-world-model`
- supervisor tick wake source and lifecycle key width in `src/runtime`
- test and bench harness for the spine
- retention-gap contract and its handling by the graph reducer

## Out Of Scope

- event payload semantics and domain event vocabularies, which stay producer-owned
- the eleven inert runtime handles, owned by the runtime wiring workstream
- sensory lane stores and promotion reducers
- belief, planner, and agent runtime behavior beyond replay-source usage

## Domain Snapshot

Generated 2026-07-08 with the policy command:

```
agent api branches capability cli compat concurrency config context control error
events execution heads ignore init lib logging merkle_traversal metadata
prompt_context provider runtime session store task telemetry types views
workflow workspace world_state
```

## Domain Assessment Table

| Domain | Needed Integration | Current Integration | Completeness | Evidence | Non Integration Rationale | Follow Up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | No spine calls | `not needed` | No emit or replay sites in `src/agent` | Agent identity config does not produce or consume spine mechanics | none |
| `api` | `adapter` | Routes context envelopes to emit facade | `complete` | `src/api.rs:929-955` | | Signatures unchanged; re-verify at ingress phase gate |
| `branches` | `consume` | Calls graph catch-up before branch queries | `complete` | `src/branches/runtime.rs:78` | | Benefits from seek reads; no code change expected |
| `capability` | `none` | No spine calls | `not needed` | No emit or replay sites in `src/capability` | Capability contracts are invocation boundaries, not history producers today | none |
| `cli` | `observe` | Progress panel polls session reads; runtime commands drive tick loop | `partial` | `src/cli/progress.rs:57-68`, `src/runtime/tooling.rs:281-316` | | Panel keeps polling; tick loop gains wake-on-watermark in read path phase |
| `compat` | `observe` | Legacy aliases only | `complete` | `crates/meld-events/src/events/compat.rs` | | Unchanged |
| `concurrency` | `none` | Node lock manager unrelated to spine | `not needed` | `src/concurrency.rs` scopes node generation locks | Spine ordering is owned by the writer, not shared lock infrastructure | none |
| `config` | `observe` | Storage paths resolve spine database location | `complete` | `src/cli/runtime_assembly.rs:26-44` | | Group-commit window may add config later; none required now |
| `context` | `publish` | Emits frame and head events; backfills legacy heads | `complete` | `src/context/events.rs:53-160`, `src/context/head.rs:117` | | Emit signatures preserved |
| `control` | `publish` and `consume` | Emits control events; execution projection replays | `partial` | `src/control/orchestration.rs:91-311`, `src/control/projection.rs:21-27` | | Projection gains seek reads free; durable cursor adoption deferred to wiring workstream |
| `error` | `observe` | Maps storage and API errors | `partial` | `crates/meld-events/src/error.rs` | | Retention-gap typed error added in retention phase |
| `events` | `own` | Owns all spine mechanics | `partial` | `crates/meld-events` | | This program |
| `execution` | `publish` | Task, workflow, and publication events through append sink | `complete` | `crates/meld-execution/src/task_network/publication.rs:236`, `src/execution/ports.rs:663-670` | | `EventAppendSink` signature preserved; durable class adopted at ingress phase |
| `heads` | `observe` | Legacy head index backfilled into spine | `complete` | `src/context/head.rs:117` | | Interacts with legacy sunset; characterization tests before backfill removal |
| `ignore` | `none` | No spine calls | `not needed` | No emit or replay sites | File selection policy has no history concern | none |
| `init` | `none` | No spine calls | `not needed` | No emit or replay sites | Bootstrap assets do not touch the ledger | none |
| `lib` | `adapter` | Re-exports event types | `complete` | `src/lib.rs` | | Verify exports after subscription surface lands |
| `logging` | `none` | Tracing only | `not needed` | Spine emits `tracing::warn` on failures | Logs are not ledger participants | none |
| `merkle_traversal` | `none` | No spine calls | `not needed` | No emit or replay sites | Tree traversal strategy is orthogonal | none |
| `metadata` | `none` | No spine calls | `not needed` | No emit or replay sites | Metadata schema has no current history contract | none |
| `prompt_context` | `none` | Lineage promotion flows through context events | `not needed` | `multi_domain_spine.md` context domain section | Prompt artifacts publish through context, not directly | none |
| `provider` | `publish` | Provider progress events | `complete` | `src/provider/tooling.rs:194,218` | | Best-effort class fits; unchanged signatures |
| `runtime` | `consume` and `adapter` | Ports wrap append and bounded replay; supervisor drives ticks | `partial` | `src/runtime/ports.rs:310-339`, `src/runtime/supervisor/entrypoint.rs:437-499` | | Wake-on-watermark and drop-counter diagnostics land here |
| `session` | `publish` | Session lifecycle events | `complete` | `src/telemetry/sessions/service.rs:31-43` | | Unchanged |
| `store` | `none` | Node persistence separate from spine trees | `not needed` | `src/store` owns node records in separate trees | Spine storage is events-domain owned per storage policy | none |
| `task` | `publish` | Task events through execution ports | `complete` | `crates/meld-execution/src/task/events.rs:111-137` | | Unchanged |
| `telemetry` | `publish` | Legacy telemetry and summaries | `complete` | `src/telemetry/emission/engine.rs:26-37` | | Canonical best-effort class consumer |
| `types` | `none` | No spine types re-declared | `not needed` | Event types live in meld-events | Shared types crate must not duplicate envelope truth | none |
| `views` | `none` | No spine reads | `not needed` | Read models source from domain stores | Presentation reads projections, not the ledger | none |
| `workflow` | `publish` | Workflow turn events through execution ports | `complete` | `crates/meld-execution/src/workflow/events.rs:55-75` | | Unchanged |
| `workspace` | `publish` | Scan, snapshot, and watch events | `complete` | `src/workspace/events.rs:59-213`, `src/workspace/watch/runtime.rs:111-532` | | Watch batching is the proto lane pattern; unchanged here |
| `world_state` | `consume` and `publish` | Reducers replay source events and append derived facts | `partial` | `crates/meld-world-model/src/world_state/graph/runtime.rs:123-170` | | Cursor-advance fix and retention-gap handling land in correctness and retention phases |

## Gaps And Follow Ups

- The cursor-advance fix crosses into world_state ownership; it lands as a consumer-correctness change with world-model tests, not as spine ownership creep.
- Control projection and the claim reducer keep in-memory cursors; durable cursor adoption belongs to the runtime wiring workstream and is explicitly not pulled into this program.
- The `error` row gains one typed variant at the retention phase; no broader error remapping.
- CLI progress panel polling is acceptable at its 200ms cadence; converting it to the subscription surface is optional follow-up, not a gate.

## Non Integration Notes

Domains marked `not needed` either have no history concern or publish through an owning domain that already integrates.
No integration is added merely because a domain exists; every `publish` row cites an existing emit site rather than a proposed one.
