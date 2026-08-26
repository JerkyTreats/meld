# Product Event Authority Assessment By Domain

Date: 2026-07-09
Revised: 2026-07-12
Status: complete
Evidence date: 2026-07-12
Evidence revision: `9350a90`
Method: canonical assessment-by-domain skill

## Concern Definition

The product event authority concern is whether every semantic producer, replay consumer, reducer, subscription, and observability surface for one product identity uses one canonical ledger identity and one sequence space.

That integration is complete.
Root composition now binds the branch product identity to one external event authority before constructing CLI and product runtime adapters.
The target ledger persists the inverse product claim, preventing separate branch-local bindings from aliasing one ledger path.
Legacy event history is a recoverable migration source, not a concurrent writer.
Direct CLI routes and `meld runtime run` consume the same authority and shared graph runtime.

The main implementation is `97cc225`.
Closure reliability corrections continue through `9350a90`.

## In Scope

- event authority identity and capability contracts
- root product and CLI composition
- semantic producer and consumer injection
- recoverable compatibility history migration and write cutover
- direct configured-authority observability routing
- transport-neutral remote authority contract and loopback conformance
- branch identity and storage isolation
- parity, reopen, and real route proof

## Out Of Scope

- product event payload meaning
- supervisor status publication cadence and cache persistence
- runtime status staleness policy
- daemon process management, IPC framing, reconnects, endpoint lifecycle, and authentication
- console frames, runtime action publication, and promotion policy
- real direct-versus-daemon process integration
- the complete semantic flywheel and operator-visibility proof
- compaction implementation

## Domain Snapshot

Generated with the policy command:

```text
agent api branches capability cli compat concurrency config context control error
events execution heads ignore init lib logging merkle_traversal metadata
prompt_context provider runtime session store task telemetry tree types views
workflow workspace world_state
```

## Domain Assessment

| Domain | Needed Integration | Completed Integration | Completeness | Evidence | Follow Up |
| --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Agent facts remain owned by world-model publication paths | `not needed` | [agent design](../../cognitive_architecture/world_model/agent/README.md) | none |
| `api` | `adapter` | API receives a `ProgressRuntime` backed by the bound append capability and separate session compatibility runtime | `complete` | [API root](../../../src/api.rs), [session service](../../../src/telemetry/sessions/service.rs) | none |
| `branches` | `observe` | Active graph queries reuse the shared product projection; dormant migration resolves each branch's configured source and authority | `complete` | [branch runtime](../../../src/branches/runtime.rs), `dormant_branch_migrations_keep_separate_product_authorities` | none |
| `capability` | `none` | Capability invocation remains separate from event authority selection | `not needed` | [capability root](../../../src/capability.rs) | none |
| `cli` | `adapter` | `RunContext` resolves authority before adapters; direct event and runtime routes reuse it | `complete` | [CLI route](../../../src/cli/route.rs), [cutover route tests](../../../tests/integration/product_event_authority_cutover.rs) | none |
| `compat` | `adapter` | Production raw event construction is sealed; explicit test support contains compatibility construction | `complete` | [events facade](../../../src/events.rs), [boundary gate](../../../scripts/check_domain_boundaries.sh) | Remove deployed-data readers only after their documented compatibility horizon |
| `concurrency` | `none` | Process coordination does not own ledger identity; binding cutover uses an external advisory lock | `not needed` | [binding resolver](../../../src/events/binding.rs) | Runtime process ownership remains R2 |
| `config` | `adapter` | Relative roots resolve under workspace XDG data; workspace-contained roots and symlink escapes fail closed | `complete` | [storage paths](../../../src/config/workspace/storage_paths.rs) | none |
| `context` | `publish` | Context publication uses the injected product appender while configured frame and prompt paths remain compatible | `complete` | [context tooling](../../../src/context/tooling.rs), [CLI assembly](../../../src/cli/runtime_assembly.rs) | none |
| `control` | `publish` | Control publication consumes root event capability adapters | `complete` | [control orchestration](../../../src/control/orchestration.rs), [runtime ports](../../../src/runtime/ports.rs) | none |
| `error` | `adapter` | Storage policy, binding, identity, migration, and fail-closed route errors map through stable root errors | `complete` | [root errors](../../../src/error.rs), [event errors](../../../crates/meld-events/src/error.rs) | none |
| `events` | `own` | Persisted identity, one authority aggregate, unified writer and watermark, observability, migration, and remote conformance are complete; raw production construction is sealed | `complete` | [authority](../../../crates/meld-events/src/events/authority.rs), [migration](../../../crates/meld-events/src/events/migration.rs), [event tests](../../../crates/meld-events/tests) | Runtime hosts the remote edge in R2 |
| `execution` | `publish` | Publication receipts carry ledger identity and root adapts the execution sink to the authority append capability | `complete` | [publication contract](../../../crates/meld-execution/src/task_network/publication.rs), [runtime ports](../../../src/runtime/ports.rs) | none |
| `heads` | `none` | Context owns legacy head-history publication | `not needed` | [head backfill](../../../src/context/head.rs) | none |
| `ignore` | `none` | File-selection policy does not select ledger authority | `not needed` | [ignore root](../../../src/ignore.rs) | none |
| `init` | `none` | Initialization assets remain outside event-history ownership | `not needed` | [init root](../../../src/init.rs) | none |
| `lib` | `adapter` | Public root composition exposes authority capabilities rather than writable raw store constructors | `complete` | [library root](../../../src/lib.rs), [events facade](../../../src/events.rs) | none |
| `logging` | `none` | Logs remain non-authoritative | `not needed` | [logging root](../../../src/logging.rs) | none |
| `merkle_traversal` | `none` | Traversal mechanics remain independent of authority selection | `not needed` | [traversal root](../../../src/merkle_traversal.rs) | none |
| `metadata` | `none` | Metadata policy publishes through owning domains | `not needed` | [metadata root](../../../src/metadata.rs) | none |
| `prompt_context` | `none` | Context owns prompt-lineage publication | `not needed` | [prompt context root](../../../src/prompt_context.rs) | none |
| `provider` | `publish` | Provider progress uses the injected product-backed progress facade | `complete` | [provider tooling](../../../src/provider/tooling.rs), [session service](../../../src/telemetry/sessions/service.rs) | Runtime action mapping remains R3 |
| `runtime` | `consume` | Product assembly consumes the root-resolved authority and shared graph runtime without opening event storage | `complete` | [runtime assembly](../../../src/runtime/assembly.rs), `supplied_authority_and_graph_runtime_are_shared_across_assembly` | Status, daemon, action, and flywheel work remains R1 through R4 |
| `session` | `publish` | Compatibility session records stay in the legacy CLI database; promoted session facts publish through the product authority | `complete` | [session service](../../../src/telemetry/sessions/service.rs), `real_cli_migrates_and_reuses_one_authority_for_event_and_runtime_routes` | none |
| `store` | `none` | Node persistence remains separate and its configured legacy CLI path is preserved | `not needed` | [store root](../../../src/store.rs), [CLI assembly](../../../src/cli/runtime_assembly.rs) | none |
| `task` | `publish` | Task publication reaches authority through execution-owned contracts | `complete` | [task events](../../../src/task/events.rs), [publication contract](../../../crates/meld-execution/src/task_network/publication.rs) | none |
| `telemetry` | `adapter` | `ProgressRuntime` receives append and session capabilities and cannot construct canonical event storage | `complete` | [session service](../../../src/telemetry/sessions/service.rs) | Threshold and promotion policy remain runtime-health owned |
| `tree` | `none` | Merkle tree construction and hashing remain separate from event-authority selection | `not needed` | [tree root](../../../src/tree.rs) | none |
| `types` | `none` | Ledger identity remains owned by events | `not needed` | [types root](../../../src/types.rs) | none |
| `views` | `none` | Presentation does not select canonical storage | `not needed` | [views root](../../../src/views.rs) | none |
| `workflow` | `publish` | Workflow publication uses the injected product-backed progress facade | `complete` | [workflow tooling](../../../src/workflow/tooling.rs) | none |
| `workspace` | `publish` | Scan and watch facts use the product appender and are replayed by the shared graph adapter | `complete` | [workspace tooling](../../../src/workspace/tooling.rs), `real_cli_migrates_and_reuses_one_authority_for_event_and_runtime_routes` | none |
| `world_state` | `consume` | Graph runtime consumes authority replay, append, and cursor ports; direct and supervised paths share one runtime | `complete` | [graph runtime](../../../crates/meld-world-model/src/world_state/graph/runtime.rs), [runtime assembly](../../../src/runtime/assembly.rs) | none |

## Closure Evidence

Binding and migration evidence includes `Preparing` resume, `Active` fail-closed validation, BLAKE3 source-to-target mappings, target-prefix preservation, malformed-source preflight, empty-source marker creation, semantic legacy-write rejection, and compatibility session-tree writes.
The focused contracts are in [binding.rs](../../../src/events/binding.rs) and [event_migration.rs](../../../crates/meld-events/tests/event_migration.rs).

Route evidence in [product_event_authority_cutover.rs](../../../tests/integration/product_event_authority_cutover.rs) proves:

- the real CLI migrates history and direct `meld event` reads the bound authority;
- a workspace fact is replayed by the shared graph adapter in the same identity and sequence space;
- the real `runtime run` route creates no second identity or legacy semantic writes;
- separate binary invocations preserve the identity;
- mismatch fails without fallback;
- reopen restores identity, watermark, consumer cursor, and next sequence.

Branch evidence in [branches_runtime.rs](../../../tests/integration/branches_runtime.rs) proves configured legacy-source selection, separate dormant branch authority identities and paths, and active graph route reuse.

The external root policy is covered by tests in [storage_paths.rs](../../../src/config/workspace/storage_paths.rs), including relative XDG resolution, lexical and symlink workspace containment, parent escape rejection, and configured legacy-root diagnostics.
Existing configured non-event CLI storage paths remain unchanged.

## Remaining Runtime Integration

The product event authority concern is closed, but the runtime program is not.
The completed authority is an input to:

- R1 status publisher cadence, status cache persistence, and staleness;
- R2 daemon lifecycle and real IPC;
- R3 console frames, runtime actions, and heartbeat mapping;
- R4 the complete semantic flywheel and operator-visibility proof.

These are runtime-owned consumers and do not reopen this domain assessment unless they violate the authority identity, capability, or route contracts.
