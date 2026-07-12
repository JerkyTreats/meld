# Product Runtime Assembly Requirements

Date: 2026-06-17
Revised: 2026-07-12
Status: E5 authority assembly complete; runtime continuation active
Scope: Phase 1 requirements for root `ProductRuntimeAssembly`

## Purpose

`ProductRuntimeAssembly` is the root product assembly boundary for the durable flywheel runtime.

It resolves product configuration, consumes a root-resolved product event authority, opens the remaining product stores and the supervisor store, constructs direct handoff ports, builds the runtime factory registry, and produces inert runtime handles or handle factories for the supervisor.

It does not run the flywheel. It does not own semantic progress. It does not mediate every transition after ports are wired.

The product flywheel remains:

```text
world model runtime
-> execution runtime
-> event spine
-> world model runtime
```

Root assembly sits before this loop. It wires durable boundaries and hands lifecycle authority to the supervisor.

## Source Alignment

These requirements refine Phase 1 from `design/plan/integration/durable_flywheel_runtime_phase_design.md`.

They preserve the shared boundary rules in `design/plan/integration/runtime_requirements.md`, the supervisor ownership split in `design/plan/integration/runtime_supervisor_domain_plan.md`, the detailed supervisor handoff rules in `design/plan/integration/supervisor_runtime_requirements.md`, the storage and first proof expectations in `design/plan/integration/durable_runtime_first_slice.md`, and the pre implementation gap outcomes in `design/plan/integration/durable_runtime_pre_implementation_gaps.md`.

They are grounded in the current root storage and worker diagnostics surfaces in `src/runtime/storage.rs` and `src/runtime/contracts.rs`.

## E5 Implementation Reconciliation

The event-authority portion of these requirements is complete through `9350a90`.
The main product cutover is `97cc225`; closure storage and reliability corrections continue through `9350a90`.

Root composition resolves the branch product binding before assembly.
The binding uses durable `event_authority.json` `Preparing` and `Active` states, an `fs2` advisory lock, recoverable BLAKE3 migration mappings, and fail-closed source, marker, path, and identity validation.
The target ledger durably claims the branch product identity before binding or migration proceeds, so independent branch data homes cannot point at the same writable authority.
An empty legacy source receives a cutover marker, so a later legacy writer cannot create a second semantic history.

`ProductRuntimeAssembly::load_with_authority` consumes the resolved `EventAuthority` and builds append, replay, cursor, and shared graph adapters from its capabilities.
`OpenProductStores` opens projection and domain storage only.
It does not own, open, or flush canonical event storage.
The authority resolver owns the external product ledger, and durable append owns its flush acknowledgement.

Session compatibility remains in the configured legacy CLI database.
Configured non-event CLI paths for node, frame, prompt, belief, and session storage are preserved.
Semantic event publication uses only the bound product authority, and legacy semantic event appends are rejected after cutover.

The external product root policy is implemented in [storage_paths.rs](../../../src/config/workspace/storage_paths.rs).
Relative roots resolve below the workspace-specific XDG data root; absolute, normalized, or symlink-resolved roots inside the target workspace fail closed.
Dormant branch migration resolves each branch's own configuration, legacy source, product path, and ledger identity.

This reconciliation completes the E5 composition dependency.
It does not implement `RuntimeStatusPublisher` cadence, status-cache persistence, daemon lifecycle, real IPC, console frames, runtime action mapping, promotion policy, or the complete flywheel proof.

## Authority

`ProductRuntimeAssembly` owns assembly authority only.

Required authority:

- resolve the product root from config and environment
- derive `ProductStorageLayout`
- create storage directories required by the layout
- consume exactly one root-resolved product event authority
- open `OpenProductStores` without constructing canonical event storage
- open the supervisor store
- construct direct handoff port implementations
- construct root adapter ports for context, provider, prompt, and workspace access
- construct execution owned task artifact and task network store factories from product storage
- construct the process local runtime factory registry
- construct inert supervised runtime handles or handle factories
- pass the opened stores, ports, registry, desired runtime state, and lifecycle config into the supervisor
- expose a product flush boundary for the supervisor to call
- report assembly diagnostics for invalid config, failed opens, failed port construction, and invalid registry entries

`ProductRuntimeAssembly` must be the place where projection stores and concrete product adapters are assembled. Root event authority resolution is the one storage-construction exception. Domain crates remain the place where record meaning, replay rules, command validation, idempotency, and semantic cursors are owned.

## Non Authority

`ProductRuntimeAssembly` must not become a scheduler, interpreter, cursor owner, or semantic cache.

Forbidden authority:

- decide whether a goal should exist
- decide whether a goal is satisfied
- decide whether evidence supports a belief
- decide whether an event payload has product meaning
- decide whether a task network command is valid
- decide whether a task publication should remain pending or become published
- select or construct a competing canonical event history for the same product identity
- choose where a domain runtime resumes after restart
- route every world model to execution handoff after ports are wired
- route every execution to event handoff after ports are wired
- route every event to world model handoff after ports are wired
- start semantic work before supervisor lease acquisition
- append supervisor lifecycle records to the canonical event spine by default
- repair domain state during open, flush, startup, shutdown, or recovery

## Owned State

`ProductRuntimeAssembly` may hold only state needed to assemble and hand off product runtime infrastructure.

Allowed owned state:

- product root path
- product storage layout
- resolved product event authority capabilities
- opened product stores
- opened supervisor store handle
- direct handoff port values
- root adapter port values
- runtime factory registry
- inert runtime handle factories
- desired runtime state loaded from config
- lifecycle timing config for the supervisor
- work budget defaults for runtime factories
- clock source
- cancellation source
- process signal source
- provider client configuration
- provider client handles when they are passive before start
- assembly diagnostics

Allowed owned state must not become correctness state. Values may be reused across a process lifetime only because they are stable infrastructure references.

## Forbidden State

`ProductRuntimeAssembly` must not store, cache, derive, or checkpoint semantic progress.

Forbidden state:

- active goals
- pending publications
- published publication state
- belief views
- planner projection outputs
- event payload meaning
- event sequence as root progress
- graph replay cursor
- belief evidence cursor
- agent delivery cursor
- satisfaction review cursor
- task network state
- task network revision as root progress
- task dispatch claims
- task outcomes
- promoted evidence
- domain source cursors
- centralized semantic loop state
- local continuation variables required for correctness after reopen

Diagnostic copies are allowed only through bounded worker reports and supervisor status records. Diagnostic copies must be labeled as observation and must never drive resume, skip, satisfaction, publication, or correctness decisions.

## Required Inputs

Assembly inputs come from config, environment, and process services. Missing or invalid required inputs must fail before any runtime handle starts.

Required product inputs:

- product root path or workspace path from which the product root is derived
- docs freshness activation document path when the `docs_freshness` flywheel is enabled
- storage layout version or default layout selector
- supervisor store path or default `supervisor.sled` path below the product root
- enabled runtime ids
- disabled runtime ids when configured
- desired restart policy per runtime id or a configured default
- work budget defaults
- runtime specific config values required by the first proof
- event session id defaults used by event producing runtimes
- task network ids and storage keys
- task artifact repository ids
- branch ids, perspective keys, agent ids, belief family ids, and subject refs needed by domain factories
- method library or task package selection for execution factories
- provider profile and provider endpoint selection
- prompt artifact root selection when not derived from layout
- workspace root selection when not derived from layout

The first physical `docs_freshness` config surface is detailed in [Docs Freshness Physical Configuration Requirements](docs_freshness_physical_configuration_requirements.md).

For the product-visible `docs_freshness` slice, root assembly owns activation document loading and shape validation. The activation document is one physical file that may be TOML, YAML, or JSON behind a single DTO contract. Root assembly must split the validated DTO into owner-scoped runtime input packages before supervisor handoff.

Domain runtimes must not receive the activation document path, raw parsed document, or source format metadata. They receive typed input values for their domain only.

Required environment inputs:

- filesystem access to the product root
- provider credentials required by enabled provider factories
- process current working directory when relative paths are accepted
- process environment variables named by provider config
- clock source
- cancellation source
- process signal source
- bounded sleep or timer source used later by the supervisor

Assembly may validate presence, path shape, runtime id format, duplicate ids, storage root writability, provider config completeness, activation document schema version, and activation DTO syntax. Assembly must not validate semantic correctness by querying active goals, belief views, pending publications, event payloads, or task network state.

Domain fixture values such as subject refs and branch ids are opaque to root assembly. Root assembly may pass them into owning domain factories after syntactic validation required by those factories.

## Construction Sequence

Assembly construction must be deterministic and side effect limited.

Required construction order:

1. Load raw config and environment inputs.
2. Resolve the product root.
3. Derive `ProductStorageLayout`.
4. Create required product storage directories.
5. Consume the root-resolved product event authority and validate its product binding.
6. Open `OpenProductStores` without constructing canonical event storage.
7. Open the supervisor store.
8. Build direct handoff ports from authority capabilities, opened stores, and root adapters.
9. Build provider, context, prompt, and workspace adapters.
10. Build runtime factory registry.
11. Build inert runtime handles or handle factories.
12. Build the supervisor startup handoff package.

Steps one through eleven must not start domain work. Step twelve must hand resources to the supervisor, which owns lease acquisition, runtime start, heartbeat monitoring, restart, and shutdown.

## Product Storage Opening

Product storage paths must use `ProductStorageLayout`.
The root authority resolver opens the ledger entry, and product assembly uses `OpenProductStores` for the remaining stores.
E5 removed canonical event storage from `OpenProductStores`, so opening product stores cannot construct a second ledger.

Required product storage layout entries:

- `ledger.sled` for the root-resolved event authority only
- `workspace.sled` for workspace node records
- `world_model.sled` for graph reducer state, belief state, agent state, and compatibility world state
- `execution/goals.sled` for execution goals
- `execution/task_artifacts.sled` for task artifact repositories
- `execution/task_networks` for per network task network stores
- `context/frames` for context frame blobs
- `context/prompt_artifacts` for prompt context artifacts

Root authority resolution opens `ledger.sled` once.
Product assembly opens the remaining always available product stores once and wraps them in typed store handles.
It must not expose raw `sled::Db` handles through supervisor facing assembly fields.

Session compatibility is not a product-ledger field.
It remains in the configured legacy CLI database, while session semantic facts publish through the product authority.
Configured non-event CLI node, frame, prompt, belief, and session paths remain unchanged by the authority cutover.

The ledger storage entry must bind to one stable ledger identity for the product identity.
Root composition resolves that authority before product assembly, and product assembly must consume it rather than open another event store.
CLI compatibility storage must not remain a second writable canonical history.
After cutover its semantic event trees reject appends, including for an initially empty source.

Append, replay, subscription, watermark, and observability ports must derive from the same product event authority.
Physical separation between `ledger.sled` and projection stores remains required and does not imply separate logical event histories.

Root authority resolution opens the ledger.
Product assembly opens projection and domain stores.
Owning domains own record meaning.

Product storage open failure must return an assembly storage error before any runtime handle starts. Assembly must not delete or rewrite partially opened stores as a recovery shortcut.

## Product Flush Boundary

`OpenProductStores::flush_boundary` is the product checkpoint boundary for always opened stores.

It must flush or invoke the typed flush capability for stores it owns:

- workspace store
- traversal store
- belief store
- agent store
- legacy world state store
- execution goal store
- task artifact factory
- frame storage
- prompt artifact storage

The event authority is not owned by `OpenProductStores` and is not part of this flush method.
Durable authority append returns only after the event store flush contract completes.
Any future explicit authority checkpoint must use an authority-owned capability rather than add raw event storage back to product stores.

Task network stores are opened per network. Any task network store touched by a runtime must flush before `OpenProductStores::flush_boundary` is treated as a durable product checkpoint.

Assembly must expose the product flush boundary to the supervisor. The supervisor decides when to call it during shutdown and checkpoint style tests.

Assembly must not interpret flushed records. A successful flush means storage accepted the boundary. It does not mean root has proved semantic convergence.

If product flush fails, assembly returns the storage error to the supervisor. Assembly must not release leases, write final lifecycle records, or repair domain records. Those are supervisor and domain responsibilities.

## Supervisor Store Opening

Assembly must open a supervisor store beside product domain stores.

Default location:

```text
<product_root>/supervisor.sled
```

The supervisor store is a root supervisor domain store. It stores lifecycle records only.

Allowed supervisor store records:

- runtime instance records
- desired runtime state
- runtime leases
- heartbeats
- health snapshots
- restart records
- shutdown records
- lifecycle events
- operator status cache when needed

Forbidden supervisor store records:

- event sequence as owned progress
- graph replay cursor
- belief evidence cursor
- belief view
- agent delivery cursor
- execution goal lifecycle state
- planning result
- task network revision as owned progress
- task dispatch claim
- task outcome record
- pending publication state
- satisfaction decision
- any domain source cursor

Assembly must open the store and pass the handle to the supervisor. Assembly must not write supervisor lifecycle records except through an explicit supervisor initialization API that owns those records.

Supervisor store flush is separate from product store flush. Final shutdown durability requires product stores to flush before clean lease release, then supervisor lifecycle records to flush after final state is written.

## Port Construction

Assembly must build concrete port implementations once per product assembly and pass them to domain runtime factories.

Ports must be thin. They may adapt types, call owning domain APIs, and map errors. They must not store semantic progress or add root decisions.

### Event Append Port

The event append port must implement the execution callable `EventAppendSink` contract.

Requirements:

- backed by the resolved event authority append capability
- accepts an `EventEnvelope`
- delegates idempotent append through the canonical event capability
- returns the identity-bearing `AppendReceipt` assigned by the event authority
- preserves inserted or duplicate disposition without inventing publication state
- preserves event authority errors as append port errors
- does not inspect event payload meaning
- does not choose publication readiness
- does not store the latest appended sequence as root progress
- does not mark execution publications as published

Execution publication workers own publication selection and published marks. The event append port only appends idempotently.

### Event Replay Port

The event replay port must implement the world model callable bounded replay contract.

Requirements:

- backed by the resolved event authority replay capability
- accepts an identity-bearing ledger cursor and bounded limit
- rejects a foreign ledger identity before reading
- returns an identity-bearing page, next cursor, ordered records, and explicit coverage
- enforces bounded limits required by the replay contract
- maps storage and decode failures into replay errors
- stores no replay cursor
- stores no last seen event sequence
- does not inspect event payload meaning
- does not decide which world model reducer should run

World model runtimes own graph and belief replay cursors. Assembly only provides the read source.

### Execution Goal Command Port

The execution goal command port must let world model agent curation submit accepted goal commands into the execution goal set.

Requirements:

- backed by the execution goal store through the execution goal API
- accepts a validated world model goal command or a producer neutral goal acceptance request
- maps the command into an execution owned goal command shape
- persists through the execution goal set boundary
- returns the execution goal command outcome
- preserves idempotency metadata supplied by the producer
- maps execution validation and storage errors into command sink errors
- does not query active goals before acceptance
- does not decide if the goal is worthwhile
- does not cache accepted goal ids as root progress

World model owns curation. Execution owns goal lifecycle. Root assembly owns only the adapter binding.

### Execution Goal Mutation Port

The execution goal mutation port must let persisted world model satisfaction curation submit lifecycle mutation commands into execution.

Requirements:

- backed by the execution goal store through the execution goal API
- accepts a persisted `AgentGoalMutationCommand` or an equivalent domain mutation request
- maps satisfaction mutations through the root execution mutation adapter when needed
- persists through the execution goal set boundary
- returns the execution goal command outcome
- preserves mutation command idempotency
- maps invalid mutation and storage failures into mutation port errors
- does not decide whether satisfaction evidence is strong enough
- does not call pure satisfaction curation
- does not synthesize mutation commands from belief views
- does not query active goals for root progress

World model must persist the satisfaction curation decision before the mutation reaches this port.

### Planner Projection Port

The planner projection port must let execution planning request world model planner projections without making root a planning owner.

Requirements:

- backed by world model planner query APIs and world model stores
- accepts planner projection input supplied by execution planning
- returns planner projection output from the world model boundary
- maps world model projection errors into planner port errors
- stores no projection output in root assembly
- does not query belief views for supervisor progress
- does not decide which goals should be planned
- does not rewrite planner warnings

Execution planning decides when it needs a projection. World model owns projection semantics.

### Task Artifact Factory Port

The task artifact factory port must expose durable task artifact repository construction to execution runtimes.

Requirements:

- backed by `TaskArtifactRepoFactory` from `OpenProductStores`
- opens task scoped artifact repositories by repo id
- validates repo ids through execution owned factory rules
- flushes the shared artifact backing store through the product flush boundary
- lets runtime handles expose per repo flush hooks when needed
- does not inspect artifact payloads
- does not choose artifact ids
- does not decide task readiness from artifacts
- does not copy artifact records into supervisor state

Execution task runtimes own artifact production, linking, readiness, and publication use.

### Context Port

The context port must expose context frame storage needed by task and provider runtimes.

Requirements:

- backed by `FrameStorage`
- reads and writes frame blobs through context contracts
- validates paths and content address requirements required by context storage
- maps context storage errors into context port errors
- does not treat context frames as goal state
- does not infer belief or satisfaction from context content
- does not copy frame content into supervisor state

Context persistence is product infrastructure. Its semantic use belongs to the calling runtime.

### Provider Port

The provider port must expose configured provider capability to execution runtimes.

Requirements:

- built from provider config and environment credentials
- validates required provider profile fields before supervisor start
- fails assembly when enabled runtimes require a provider and credentials are missing
- redacts secrets from diagnostics and supervisor records
- exposes only the provider calls required by enabled runtime factories
- does not choose semantic work
- does not rewrite prompts based on root decisions
- does not persist provider responses outside owning runtime and artifact contracts

Provider construction may allocate clients. Provider calls that perform model or tool work must happen only after the supervisor starts the owning runtime handle.

### Prompt Port

The prompt port must expose prompt artifact storage and prompt assembly inputs required by execution runtimes.

Requirements:

- backed by `PromptContextArtifactStorage`
- persists prompt artifacts under the product context layout
- returns prompt artifact handles through prompt context contracts
- maps prompt storage failures into prompt port errors
- does not own prompt meaning
- does not decide which prompt a task should run
- does not cache prompt outputs as root progress

Execution task and provider runtimes own prompt usage and resulting artifacts.

### Workspace Port

The workspace port must expose workspace node record access required by runtime adapters.

Requirements:

- backed by `SledNodeRecordStore`
- validates workspace storage open and flush behavior through product storage
- maps workspace store errors into workspace port errors
- does not use workspace records to schedule semantic work
- does not infer active goals, belief status, or task network state from workspace records
- does not copy workspace records into supervisor lifecycle state

Workspace storage is a root product adapter. Semantic interpretation remains outside root assembly.

## Runtime Factory Registry

Assembly must build a process local runtime factory registry after stores and ports are available.

Registry requirements:

- keyed by stable runtime id
- runtime ids must be human readable and domain scoped
- duplicate runtime ids fail assembly
- invalid runtime ids fail assembly
- every factory must declare required ports and store handles
- every factory must declare whether it needs provider, context, prompt, workspace, task artifact, or task network resources
- factory registration must not start work
- factory registration must not open per network stores unless the handle remains inert
- factory registration must not query domain state for progress
- disabled desired runtime ids remain available for status but do not start
- enabled runtime ids with no factory become supervisor diagnostics during desired state reconciliation

Required first proof registry coverage:

- event runtime factory for event append and replay infrastructure
- world model graph replay runtime factory
- world model belief runtime factory
- world model agent goal curation runtime factory
- world model evidence ingestion runtime factory
- world model satisfaction curation runtime factory
- execution goal set runtime factory when needed for lifecycle observation
- execution planning runtime factory
- execution task network command runtime factory
- execution task dispatch runtime factory
- execution publication runtime factory

The exact runtime id list belongs to supervisor config. Assembly must be able to register the concrete factories needed by the first durable flywheel proof.

## Runtime Handle Construction

Assembly may construct handles eagerly or construct factories that later create handles. Both forms must be inert.

Inert handle requirements:

- has a stable runtime id
- references only opened stores, ports, config, and passive process services
- exposes lifecycle methods required by the supervisor
- exposes heartbeat or diagnostic report surfaces required by the supervisor
- exposes flush hooks for per network or per resource stores when needed
- does not call bounded worker ticks during construction
- does not replay events during construction
- does not query active goals during construction
- does not query belief views during construction
- does not scan pending publications during construction
- does not claim tasks during construction
- does not append events during construction
- does not mutate execution goals during construction
- does not start provider calls during construction

Runtime handles begin semantic work only after the supervisor acquires a lease and calls the handle start method with supervisor context.

## Startup Handoff To Supervisor

Assembly must hand startup to the supervisor after infrastructure is ready.

The startup handoff package must include:

- product root identity
- opened product stores
- opened supervisor store
- direct handoff ports
- root adapter ports
- runtime factory registry
- desired runtime state
- lifecycle timing config
- work budget defaults
- clock source
- cancellation source
- process signal source
- assembly diagnostics collected before handoff

The supervisor owns:

- runtime instance registration
- desired state reconciliation
- expired lease recovery
- lease acquisition
- runtime handle start
- heartbeat monitoring
- health snapshots
- restart policy
- operator status

Assembly may expose a convenience method such as `ProductRuntimeAssembly::start_supervisor`. That method must delegate lifecycle work to the supervisor and must not start runtime handles directly.

## Shutdown Handoff To Supervisor

The supervisor owns shutdown coordination. Assembly provides resources and flush functions.

Assembly shutdown responsibilities:

- keep product store handles available until supervisor shutdown completes
- expose `OpenProductStores::flush_boundary`
- expose supervisor store flush through supervisor owned APIs
- expose per resource flush hooks through runtime handles when the handle owns the open resource
- drop passive resources after supervisor shutdown completes

Assembly must not:

- signal runtime cancellation without supervisor coordination
- release leases
- write final heartbeat records
- mark runtime instances stopped
- force domain cursors forward
- mark publications published
- mark goals satisfied
- repair failed flushes

Shutdown durability requires the supervisor to stop handles at domain safe points, flush per network stores, flush product stores, write final lifecycle records, release leases only when clean, then flush the supervisor store.

## Error Model

Assembly must expose typed errors under a root runtime assembly error surface.

Required error classes:

- config error
- environment error
- storage layout error
- product storage open error
- product storage flush error
- supervisor store open error
- supervisor store flush error
- port construction error
- provider construction error
- runtime registry error
- runtime handle construction error
- unsupported runtime id error
- duplicate runtime id error
- invalid runtime id error
- supervisor handoff error

Error requirements:

- preserve source error text or source error chains for diagnostics
- redact secrets and sensitive provider config
- fail before supervisor start when required assembly inputs are missing
- fail before handle start when required stores or ports cannot be built
- never convert domain semantic rejection into root semantic truth
- never hide product storage flush failures
- never treat missing factory as domain runtime failure
- keep retryable runtime work errors inside runtime or supervisor health paths

Port errors after runtime start belong to the calling runtime and owning domain. Assembly must not reinterpret those errors as root progress decisions.

## Recovery Expectations

Recovery begins by resolving the same product event authority and constructing a fresh `ProductRuntimeAssembly` from the same product root and config.

Assembly recovery requirements:

- reopen existing product stores through `ProductStorageLayout`
- consume the reopened product event authority rather than construct event storage
- reopen the supervisor store
- rebuild direct handoff ports from reopened stores
- rebuild root adapter ports from reopened stores and config
- rebuild the runtime factory registry
- rebuild inert runtime handles or factories
- pass recovery to the supervisor
- read no domain progress to decide resume positions
- hold no remembered cursors from a prior process
- rely on domain stores and supervisor lifecycle records for recovery

The supervisor handles stale instance records, expired leases, desired runtime reconciliation, restart policy, and operator status.

Domain runtimes handle semantic resume:

- event runtime resumes from authority sequence and idempotency records
- world model graph runtime resumes from graph replay cursor in world model storage
- belief runtime resumes from belief evidence and revision state
- agent runtimes resume from agent delivery and decision stores
- execution goal runtime resumes from goal store lifecycle records
- planning runtime resumes by reading active goals and planner projection through execution and world model APIs
- task network runtimes resume from task network stores
- task runtimes resume from task claims, outcomes, and artifact repos
- publication runtime resumes from task network publication outbox and event append idempotency

Assembly must not bridge crash windows with local memory. Crash after publication append but before satisfaction must recover through event, task network, world model, and execution stores. Crash after product store flush but before lease release must recover through supervisor lease expiry and domain idempotency. Crash before flush must recover from whatever each owning store durably contains.

## Implemented Authority Evidence

Assembly-focused tests in [assembly.rs](../../../src/runtime/assembly.rs) include:

- `assembly_opens_product_and_supervisor_stores`
- `assembly_can_be_reopened_from_same_product_root`
- `event_append_and_replay_ports_are_wired_to_one_authority`
- `supplied_authority_and_graph_runtime_are_shared_across_assembly`
- `graph_replay_descriptor_declares_complete_authority_dependencies`
- `startup_package_exposes_runtime_handle_factories`

The real route proof is [product_event_authority_cutover.rs](../../../tests/integration/product_event_authority_cutover.rs):

- `real_cli_migrates_and_reuses_one_authority_for_event_and_runtime_routes`
- `binary_direct_commands_preserve_one_identity_across_processes`
- `real_route_rejects_a_mismatched_active_binding_without_fallback`

These tests prove the direct event CLI and `runtime run` share one authority and sequence, no second identity appears, legacy event rows remain unchanged, identity survives separate processes, and reopen restores identity, watermark, cursor, and next sequence.

Branch routing evidence in [branches_runtime.rs](../../../tests/integration/branches_runtime.rs) includes:

- `dormant_branch_migrations_keep_separate_product_authorities`
- `dormant_branch_migration_uses_its_configured_legacy_store`
- `active_branch_graph_status_reuses_the_open_product_projection`
- `binary_active_graph_query_routes_through_run_context`

Binding tests in [binding.rs](../../../src/events/binding.rs) cover `Preparing` resume, stable `Active` reopen, branch mismatch, deleted or substituted ledgers, source-marker validation, and no-fallback behavior.
They also cover separate branch-local binding homes attempting to share one target ledger.
Migration tests in [event_migration.rs](../../../crates/meld-events/tests/event_migration.rs) cover empty-source markers, target prefix preservation, BLAKE3 mappings, interruption and resume, malformed rows, record-id conflict parity, structural provenance, and post-cutover legacy append rejection while non-event compatibility trees remain usable.

Fresh reviews passed for migration recovery, route identity, constructor sealing, branch isolation, external storage policy, compatibility path preservation, and test honesty.

## E5 Breaking And Rollout Notes

- The first command after upgrade creates or resumes the product binding under `event_authority.lock`.
- Relative product roots now resolve under workspace-specific XDG data storage.
- Absolute or resolved product roots inside the target workspace fail with a typed configuration error.
- `Preparing` cutovers resume from durable mappings; `Active` bindings never fall back to legacy event trees.
- Migration is append-only into the target and does not delete the source history.
- Compatibility session storage and configured non-event CLI paths remain in place.
- Semantic legacy event appends fail after the source marker is active, including for a source that was empty at cutover.
- Production callers can no longer construct raw writable event or graph stores; explicit test-support fixtures replace those seams.

## Verification Requirements

Implementation verification must prove assembly is wiring infrastructure only.

Required unit tests:

- product storage layout derives expected paths
- assembly opens product stores through `OpenProductStores`
- assembly consumes a root-resolved product event authority without opening canonical event storage
- assembly opens `supervisor.sled`
- assembly constructs the execution event publication adapter from the resolved event authority append capability
- assembly constructs the world model replay adapter from the resolved event authority replay capability
- assembly constructs execution goal command port from goal store
- assembly constructs execution goal mutation port from goal store
- assembly constructs planner projection port from world model stores
- assembly exposes `TaskArtifactRepoFactory`
- assembly constructs context, provider, prompt, and workspace ports
- duplicate runtime ids fail registry construction
- invalid runtime ids fail registry construction
- runtime handle construction is inert

Required integration tests:

- assembly can be constructed, dropped, and reconstructed from the same product root
- reconstructed assembly builds equivalent ports and registry entries
- construction does not append events
- construction does not mutate execution goals
- construction does not query active goals
- construction does not query pending publications
- construction does not query belief views
- construction does not scan task network state
- runtime handles start only after supervisor lease acquisition
- supervisor can call product flush boundary during shutdown
- per network task stores flush before product flush in checkpoint tests
- supervisor store flush remains separate from product flush

Required boundary checks:

```sh
cargo test runtime::storage
cargo test runtime::contracts
cargo test runtime::assembly
rg -n "mod.rs" src/runtime
rg -n "active_goals|pending_publications|belief_view|source_cursor|centralized semantic loop" src/runtime
```

Search checks are guards only. Passing searches does not replace tests that prove ports and handles are inert.

## Acceptance Criteria

Phase 1 is acceptable when `ProductRuntimeAssembly` can build the product runtime infrastructure without running a semantic loop.

Required acceptance outcomes:

- product root is resolved from config and environment
- product stores open through `ProductStorageLayout` and `OpenProductStores`
- root-resolved product event authority is consumed without alternate writable construction
- supervisor store opens beside product stores
- product flush boundary is available to the supervisor
- supervisor store flush remains separate
- execution event publication port is wired to the resolved event authority append capability
- world model replay port is wired to the resolved event authority replay capability
- execution goal command port is wired to execution goal storage
- execution goal mutation port is wired to execution goal storage
- planner projection port is wired to world model projection APIs
- task artifact factory is available to execution runtime factories
- context port is wired to frame storage
- provider port is built from config and environment
- prompt port is wired to prompt artifact storage
- workspace port is wired to workspace storage
- runtime factory registry contains first proof factories
- runtime handles or handle factories are inert before supervisor start
- startup lifecycle is delegated to the supervisor
- shutdown lifecycle is delegated to the supervisor
- assembly errors are typed and diagnostic
- recovery rebuilds assembly from durable stores and config
- no domain cursor is owned by root assembly
- no active goal, pending publication, belief view, event payload meaning, or task network state is queried by assembly for progress
- no centralized semantic loop code appears in assembly

The durable flywheel is preserved when root assembly can be deleted and replaced by another product entrypoint without changing the domain handoff semantics:

```text
world model -> execution -> events -> world model
```
