# Product Event Authority Assessment By Domain

Date: 2026-07-09
Revised: 2026-07-10
Status: assessed
Evidence date: 2026-07-10
Method: [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md)

## Concern Definition

The product event authority concern is whether every semantic producer, replay consumer, reducer, subscription, and observability surface for one product identity uses one canonical ledger identity and one sequence space.

The current CLI compatibility assembly and product runtime assembly can create separate writable event histories during one `meld runtime run` command.
The required integration replaces that state with one identity-bearing authority and treats the compatibility ledger as a migration source rather than a concurrent writer.

## In Scope

- event authority identity and capability contracts
- root product and CLI composition
- semantic producer and consumer injection
- compatibility history migration and write cutover
- direct configured-authority observability routing
- transport-neutral remote authority contract and loopback conformance
- parity, reopen, and route-level proof

## Out Of Scope

- product event payload meaning
- projection database co-location
- supervisor leases and heartbeat storage
- sensory modality implementation
- daemon process management, IPC framing, reconnects, endpoint lifecycle, and authentication
- real direct-versus-daemon process integration
- compaction implementation

## Domain Snapshot

Generated with the policy command:

```text
agent api branches capability cli compat concurrency config context control error
events execution heads ignore init lib logging merkle_traversal metadata
prompt_context provider runtime session store task telemetry types views
workflow workspace world_state
```

## Domain Assessment

| Domain | Needed Integration | Current Integration | Completeness | Evidence | Non Integration Rationale | Follow Up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Agent facts publish through world model paths | `not needed` | [agent design](../../cognitive_architecture/world_model/agent/README.md) | Agent identity does not select event storage | none |
| `api` | `adapter` | Root API receives the CLI `ProgressRuntime` | `partial` | [API runtime field](../../../src/api.rs) | | Consume injected event capabilities without selecting authority |
| `branches` | `observe` | Branch commands observe graph state built from the CLI history | `partial` | [branch runtime](../../../src/branches/runtime.rs) | | Observe projections sourced from the product authority |
| `capability` | `none` | Capability contracts do not select event storage | `not needed` | [capability root](../../../src/capability.rs) | Capability invocation is separate from event authority | none |
| `cli` | `adapter` | Normal routing constructs CLI assembly before `runtime run` and binds `meld event` to CLI progress | `partial` | [CLI route](../../../src/cli/route.rs) | | Resolve and consume one product authority before command adapters |
| `compat` | `adapter` | Compatibility reexports expose legacy construction paths | `partial` | [compat root](../../../src/compat.rs) | | Retire writable construction seams after cutover |
| `concurrency` | `none` | Process coordination does not own ledger identity | `not needed` | [concurrency root](../../../src/concurrency.rs) | Authority identity is not a lock manager concern | none |
| `config` | `adapter` | Config resolves separate CLI store and product root locations | `partial` | [storage paths](../../../src/config/workspace/storage_paths.rs) | | Resolve one product ledger binding and preserve external storage policy |
| `context` | `publish` | Context publications use CLI `ProgressRuntime` | `partial` | [context tooling](../../../src/context/tooling.rs) | | Inject the product event appender |
| `control` | `publish` | Control paths publish through existing root event facades | `partial` | [control orchestration](../../../src/control/orchestration.rs) | | Consume the product event appender |
| `error` | `adapter` | Errors map storage failures without ledger identity mismatch | `partial` | [error root](../../../src/error.rs) | | Add stable authority and migration error mapping |
| `events` | `own` | Events guarantees coherence inside one supplied database but accepts independent store construction | `partial` | [event store](../../../crates/meld-events/src/events/store.rs), [event runtime](../../../crates/meld-events/src/events/runtime.rs) | | Define identity-bearing authority and seal production construction paths |
| `execution` | `publish` | Product publication can use product port while compatibility paths can use CLI history | `partial` | [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs), [runtime ports](../../../src/runtime/ports.rs) | | Consume the events-owned append capability |
| `heads` | `none` | Legacy head history publication is owned by context | `not needed` | [head backfill](../../../src/context/head.rs) | Context owns the event handoff | none |
| `ignore` | `none` | Ignore policy does not publish or consume semantic events | `not needed` | [ignore root](../../../src/ignore.rs) | File selection policy does not select ledger authority | none |
| `init` | `none` | Initialization does not own semantic event authority | `not needed` | [init root](../../../src/init.rs) | Initialization assets are outside event history ownership | none |
| `lib` | `adapter` | Root reexports event storage and runtime types broadly | `partial` | [events facade](../../../src/events.rs), [library root](../../../src/lib.rs) | | Reexport authority capabilities rather than raw production construction seams |
| `logging` | `none` | Logging is non-authoritative | `not needed` | [logging root](../../../src/logging.rs) | Logs are not semantic event history | none |
| `merkle_traversal` | `none` | Traversal mechanics do not select event authority | `not needed` | [traversal root](../../../src/merkle_traversal.rs) | Tree traversal is orthogonal | none |
| `metadata` | `none` | Metadata policy does not select event storage | `not needed` | [metadata root](../../../src/metadata.rs) | Metadata changes publish through owning domains | none |
| `prompt_context` | `none` | Prompt lineage publishes through context | `not needed` | [prompt context root](../../../src/prompt_context.rs) | Context owns the event handoff | none |
| `provider` | `publish` | Provider progress uses CLI `ProgressRuntime` | `partial` | [provider tooling](../../../src/provider/tooling.rs) | | Inject the product event appender or keep nonsemantic progress outside the ledger |
| `runtime` | `consume` | Product runtime opens its own ledger after CLI authority already exists | `partial` | [runtime tooling](../../../src/runtime/tooling.rs), [runtime storage](../../../src/runtime/storage.rs) | | Consume the root-resolved product authority |
| `session` | `publish` | Session lifecycle facts emit through compatibility `ProgressRuntime` | `partial` | [session service](../../../src/telemetry/sessions/service.rs) | | Separate lifecycle storage from canonical event publication and inject authority |
| `store` | `none` | Node persistence is separate from canonical event ownership | `not needed` | [store root](../../../src/store.rs) | Node stores remain domain storage | none |
| `task` | `publish` | Task events can flow through compatibility and product execution paths | `partial` | [task events](../../../src/task/events.rs) | | Consume the product event appender through execution contracts |
| `telemetry` | `adapter` | `ProgressRuntime` constructs an event runtime from the compatibility database | `partial` | [progress service](../../../src/telemetry/sessions/service.rs) | | Inject authority publication and stop constructing canonical event storage |
| `types` | `none` | Shared root types do not own ledger identity | `not needed` | [types root](../../../src/types.rs) | Ledger identity belongs to events | none |
| `views` | `none` | Views consume projections and presentation data | `not needed` | [views root](../../../src/views.rs) | Presentation must not select canonical storage | none |
| `workflow` | `publish` | Workflow commands publish through CLI progress | `partial` | [workflow tooling](../../../src/workflow/tooling.rs) | | Inject the product event appender |
| `workspace` | `publish` | Scan and watch facts publish through CLI progress | `partial` | [workspace tooling](../../../src/workspace/tooling.rs), [watch events](../../../src/workspace/watch/events.rs) | | Inject the product event appender and prove the product replay adapter sees the facts |
| `world_state` | `consume` | CLI and product graph runtimes can consume different stores and append derived facts directly | `partial` | [graph runtime](../../../crates/meld-world-model/src/world_state/graph/runtime.rs) | | Consume authority replay and append capabilities with one ledger identity |

## Gaps And Follow Ups

- No stable ledger identity connects CLI and product capabilities.
- Root composition opens two event histories for `meld runtime run`.
- Compatibility producers continue writing after the product ledger exists.
- `meld event` reads the compatibility authority rather than resolving the product authority.
- `EventStore`, `EventRuntime`, and writer construction remain available outside one authority aggregate.
- Events lacks an identity-bearing canonical append capability that a root adapter can use to satisfy execution's publication sink contract.
- World model appends directly to the store and bypasses the authority writer.
- No migration and cutover proof preserves existing CLI history.
- No transport-neutral authority conformance suite proves identity and durability semantics without a daemon.

The complete implementation sequence is recorded in the [Event Foundation Closeout Program](../events/event_foundation_closeout_program.md).
The root composition and compatibility migration detail is recorded in [Product Event Authority Cutover](product_event_authority_cutover.md).

## Non Integration Notes

Domains marked `not needed` neither select canonical event storage nor publish directly through an authority boundary.
Their behavior remains unchanged as long as any semantic facts they cause are published by the owning integrated domain.
