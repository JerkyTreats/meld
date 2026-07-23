# Supervisor Runtime Requirements

Date: 2026-06-17
Status: historical supervisor contract evidence
Scope: root runtime supervisor requirements only

Current runtime-completion authority is [Runtime Completion Ground Map](runtime_completion_ground_map.md), followed by [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md). These lifecycle contracts remain evidence only where they agree with the operational-parity objective and its deferred failure mechanics.

## Purpose

The root runtime supervisor is the product process owner for domain runtime lifecycle and operator visibility.

It starts enabled runtime handles, prevents duplicate active handles for the same runtime id, records leases and heartbeats, coordinates graceful shutdown, applies conservative restart policy, and exposes supervisor status.

It is not the flywheel center. Domain runtimes own semantic work and carry momentum through durable handoff ports.

The flywheel remains:

```text
world model runtime
-> execution runtime
-> event spine
-> world model runtime
```

The supervisor sits beside that loop:

```text
root supervisor
-> start domain runtimes
-> monitor lifecycle
-> coordinate shutdown
-> report health
```

The supervisor may observe bounded worker reports as diagnostics. Those reports are never correctness state.

## Owned State

The supervisor owns only operational lifecycle state.

Required owned state:

- runtime identity catalog
- available runtime factory registry
- desired runtime set
- runtime instance records
- runtime lease records
- heartbeat records
- health snapshots
- restart counters and restart causes
- graceful shutdown state
- bounded diagnostic report summaries
- operator status projections
- supervisor lifecycle event records
- supervisor storage flush state

Runtime identity is the stable name of a supervised process role. It is not a domain aggregate id, event id, cursor, task id, publication id, or goal id.

Runtime ids must be stable, human readable, and domain scoped. Examples:

```text
events.ledger
world_model.graph.reducer
world_model.belief.docs_freshness
world_model.agent.seed.docs_freshness
execution.goals
execution.planning
execution.task_network.network-docs
execution.publication.network-docs
```

Runtime id validation must reject empty ids, whitespace, empty path segments, path traversal, and values outside the allowed runtime id character set. The first allowed set should be lower case ASCII letters, digits, `.`, `_`, and `-`.

The available runtime factory registry is process local. It maps runtime ids to lifecycle handle factories. It may be constructed by root assembly, but it must not persist domain handles or domain state.

The desired runtime set is supervisor owned configuration. It records which runtime ids should be enabled and which restart policy applies to each id. Desired state may be loaded from config and persisted in the supervisor store after validation so operator status can explain what was requested.

Runtime instance records describe one root supervisor process attached to one product root. They must include a generated instance id, product root identity, start time, stop time when known, and current instance status.

Lease records describe ownership of one runtime id by one instance and lease id. A lease is a duplicate process guard. It is not a work claim inside the owning domain.

Heartbeat records describe the latest observation for one active lease. A heartbeat may carry the last normalized `WorkerTickReport` summary, but it must store the report as observation only.

Health snapshots are supervisor summaries derived from handle state, lease state, heartbeat age, restart history, and last diagnostic outcome. They must not query or copy semantic domain state.

Lifecycle event records are operational audit entries such as instance registered, runtime start requested, lease acquired, heartbeat stale, restart scheduled, shutdown requested, lease released, and instance stopped.

## Required Inputs

The supervisor requires these inputs from root assembly and the process environment:

- product root path
- product storage layout
- opened product stores
- supervisor store handle
- available runtime factory registry
- desired runtime set
- configured lease duration
- configured heartbeat interval
- configured heartbeat grace duration
- configured shutdown grace duration
- configured restart policy defaults
- bounded work budget defaults
- clock source
- cancellation source
- process signal source
- lifecycle handle factory for each enabled runtime id
- direct handoff ports created by root assembly

Root assembly may provide event append, event replay, goal sink, artifact, context, prompt, and workspace adapters. The supervisor may pass these ports into domain runtime factories. After startup, domain runtimes call the ports directly.

The supervisor must not require active goals, belief views, task network state, pending publications, event records, promoted evidence, satisfaction commands, or any domain cursor as input.

## Durable Outputs

The supervisor must persist operational records in `supervisor.sled` under the product root.

Required durable output families:

- runtime instances
- desired runtime state
- runtime leases
- runtime heartbeats
- health snapshots
- restart records
- shutdown records
- lifecycle events
- operator status cache when needed

Durable supervisor records may reference runtime ids, lease ids, instance ids, timestamps, lifecycle statuses, restart counts, diagnostic issue counts, and bounded report summaries.

Durable supervisor records must not persist:

- event spine sequence as owned progress
- graph replay cursor
- belief evidence cursor
- belief view revision as owned progress
- agent delivery cursor
- curation decision as owned progress
- execution goal lifecycle state
- planning result
- task network revision as owned progress
- task dispatch claim
- task outcome record
- pending publication record
- published publication state
- satisfaction evidence decision
- source cursor for any domain

If a diagnostic report includes an input checkpoint or output checkpoint, the supervisor stores it only as report data. It must not use that checkpoint to resume work, skip work, or decide domain correctness.

## Ports

The supervisor exposes lifecycle and observability ports only.

Required supervisor command ports:

- start supervisor instance
- stop supervisor instance
- start enabled runtimes
- stop runtime by runtime id
- request graceful shutdown
- request status snapshot
- request health snapshot

Required supervisor store ports:

- register runtime instance
- load desired runtime set
- persist desired runtime state
- acquire runtime lease
- renew runtime lease
- release runtime lease
- expire stale runtime lease
- write heartbeat
- write health snapshot
- write lifecycle event
- flush supervisor store

Required runtime handle ports:

- read runtime id
- start with supervisor context
- request stop
- wait for safe point
- read heartbeat
- flush runtime owned open resources when exposed by the handle

Required process ports:

- current time
- cancellation token
- process signal subscription
- bounded sleep or timer

The supervisor context passed to a runtime handle may include runtime id, instance id, lease id, cancellation token, clock, work budget defaults, and already wired domain ports.

The supervisor context must not include supervisor-held semantic progress memory.

## Lease Rules

Every enabled runtime must acquire a supervisor lease before its handle starts.

There must be at most one active lease for a runtime id.

Lease acquisition must be atomic with respect to the active lease index. If another active unexpired lease exists for the runtime id, acquisition fails and no handle starts.

A lease is identified by runtime id, lease id, instance id, acquired time, expiry time, and lease status.

Allowed lease statuses:

- acquiring
- active
- renewing
- released
- expired
- superseded
- abandoned

Only the active lease owner may renew a lease.

Only the active lease owner may write heartbeats for that runtime id.

Only the active lease owner may release a lease during graceful shutdown.

Renewal must compare the provided lease id and instance id with the active lease record. A stale owner must receive a stale lease error.

Lease expiry does not prove domain work failed. It proves the supervisor can no longer trust the process owner for that runtime id.

Startup recovery may mark expired active leases as expired. It must not steal an unexpired active lease.

Restart after lease expiry must create a new lease id and a new runtime handle. The old lease remains historical state.

Lease records must not be used as domain locks, domain cursors, task claims, goal ownership markers, or publication ownership markers.

## Heartbeat Rules

Each running runtime must publish a heartbeat after successful start and then at the configured interval.

Each heartbeat must be bound to runtime id, lease id, instance id, observed time, and runtime health.

A heartbeat may include:

- handle state
- worker report summary
- retryable issue count
- fatal issue count
- budget exhausted flag
- last successful tick time
- last error code
- last restart cause

A heartbeat must not include domain state snapshots, active goal records, belief views, task network state clones, publication bodies, event payloads, or domain cursor ownership.

The supervisor must reject a heartbeat if the lease id is not the active lease for the runtime id.

The supervisor must treat missing heartbeat, stale heartbeat, retryable report, fatal report, and budget exhaustion as distinct health signals.

Heartbeat age must be computed from the supervisor clock and the last accepted observed time.

The latest heartbeat may be overwritten or stored as a bounded latest record. If historical heartbeat retention is added, retention must be bounded by count or time.

Worker report checkpoints inside a heartbeat are diagnostics. They may explain progress to an operator, but they must not drive resume position or semantic assertions.

## Startup And Shutdown Rules

Startup must follow this order:

1. Resolve the product root.
2. Build the product storage layout.
3. Open product stores.
4. Open supervisor store.
5. Register the supervisor instance.
6. Load and validate the desired runtime set.
7. Build direct handoff ports.
8. Recover expired leases.
9. Reconcile desired runtime ids with available factories.
10. Acquire a lease for each enabled runtime.
11. Start each runtime handle only after its lease is acquired.
12. Write an initial heartbeat or start failure event for each enabled runtime.
13. Start heartbeat monitoring.
14. Surface operator status through supervisor query APIs.

Shutdown must follow this order:

1. Mark the supervisor instance as stopping.
2. Stop accepting new runtime start requests.
3. Signal cancellation to running runtime handles.
4. Ask each runtime handle to stop at a domain safe point.
5. Wait until each handle stops or reaches the shutdown grace limit.
6. Flush per runtime and per network resources exposed by handles.
7. Flush product stores through `OpenProductStores::flush_boundary`.
8. Write final heartbeat, stopped record, or failed stop record for each runtime.
9. Release active supervisor leases for stopped runtimes.
10. Write health snapshots.
11. Mark the supervisor instance stopped.
12. Flush the supervisor store.

Shutdown must be idempotent. Running shutdown twice must not corrupt leases, duplicate lifecycle events in a misleading way, or rewrite domain state.

If a runtime fails to reach a safe point before the grace limit, the supervisor must record a failed stop status and leave the domain stores as the source of recovery truth.

The supervisor must not force a domain cursor advance during shutdown.

## Recovery Rules

Recovery begins when a supervisor opens a product root that may contain prior runtime lifecycle records.

Recovery must:

- register a new supervisor instance
- mark prior instances with no stopped time as stale when their leases have expired
- scan active leases
- expire leases whose expiry time is before the current supervisor time
- keep unexpired active leases intact
- refuse to start a runtime when another unexpired active lease owns that runtime id
- write lifecycle events for every recovered lease
- rebuild health snapshots from supervisor records only
- start enabled runtimes from desired state after lease acquisition

Recovery must not read domain stores to decide where any runtime should resume. Domain runtimes decide resume position by reading their owning stores after they start.

Crash after domain store flush but before lease release must recover by lease expiry and restart. The restarted domain runtime must be idempotent against the domain store.

Crash before domain store flush must recover from whatever the owning stores durably contain. The supervisor must not infer missing semantic progress from heartbeats or reports.

Crash after publication append but before satisfaction must rely on reopened event, task network, world model, and execution stores. The supervisor may report the prior publication worker checkpoint as diagnostic history only.

Recovery must preserve `RTG-6` reopen proof expectations. Product root and fixture constants may cross checkpoint boundaries. Goal records, task network state, publications, events, promoted evidence, belief views, mutation commands, and cursors must be reloaded through owning domain APIs.

## Restart Rules

The first restart policy set is intentionally conservative.

Required policy values:

- `never`
- `on_retryable_failure`
- `on_heartbeat_expiry`

`never` records the failure and leaves the runtime stopped until an operator or caller requests another start.

`on_retryable_failure` may restart a handle when the active runtime reports retryable failure and no fatal condition requires operator intervention.

`on_heartbeat_expiry` may restart a handle after heartbeat expiry and lease expiry recovery.

Fatal worker reports must not be translated into domain truth. They should mark the runtime unhealthy and require explicit policy before restart.

Every restart must:

- record the restart cause
- increment a restart counter
- apply a bounded backoff
- stop or abandon the old handle
- expire or supersede the old lease when allowed
- acquire a new lease before start
- start a fresh runtime handle
- rely on domain stores for resume

Restart policy must not inspect active goals, belief views, task network revisions, publication state, event payloads, satisfaction decisions, or domain cursors.

Restart loops must be bounded. The supervisor must stop restarting after a configured attempt limit or backoff limit and expose the runtime as unhealthy.

## Operator Status

Operator status is a supervisor view over operational records.

Required status fields:

- supervisor instance id
- product root identity
- supervisor instance status
- runtime id
- desired enabled state
- available factory state
- current handle state
- active lease id when present
- lease status
- lease expiry time
- last heartbeat time
- heartbeat age
- health status
- restart count
- last restart cause
- retryable issue count
- fatal issue count
- budget exhausted flag
- last diagnostic actor id
- last diagnostic checkpoint names and values
- last lifecycle event

Operator status must be answerable from supervisor store records and process local handle state. It must not query semantic domain internals.

Status output may show diagnostic checkpoints copied from `WorkerTickReport`. It must label them as diagnostics and never as supervisor-held cursors.

Missing factory for an enabled runtime id must produce a supervisor diagnostic, not a domain error.

Disabled desired runtime ids must be visible as disabled and must not acquire leases.

## Store Flush Boundary

The supervisor is responsible for ordering flush boundaries during shutdown and checkpoint style tests. It is not responsible for interpreting flushed domain records.

Task network stores are opened per network. Any open task network stores touched by a runtime must be flushed before `OpenProductStores::flush_boundary` is treated as a product checkpoint.

`OpenProductStores::flush_boundary` flushes always opened product stores, including event store, workspace store, traversal store, belief store, agent store, legacy world state store, execution goal store, and task artifact factory.

The supervisor store has a separate flush boundary. Supervisor lifecycle records become durable only after the supervisor store flush succeeds.

Required shutdown flush ordering:

1. Runtime handles stop at domain safe points.
2. Runtime exposed per network stores flush.
3. Runtime exposed per resource stores flush.
4. `OpenProductStores::flush_boundary` runs.
5. Final heartbeat and stopped records are written.
6. Runtime leases are released.
7. Health snapshots are written.
8. Supervisor store flush runs.

Stores flushed in step two and step three remain owned by their domains. The supervisor only calls flush through explicit adapters or handle ports.

If a flush fails, the supervisor must record the failure in supervisor state, keep leases from being misleadingly released as clean shutdown when possible, and surface unhealthy status.

The supervisor must never repair domain state during flush failure handling.

## Forbidden Responsibilities

The supervisor must not own or persist:

- event truth
- event sequence
- graph replay cursor
- belief evidence cursor
- belief view
- agent delivery cursor
- agent curation decision
- execution goal lifecycle
- planning state
- planning result
- task network revision
- task dispatch claim
- task outcome record
- pending publication state
- published publication state
- satisfaction evidence decision
- domain retry cursor
- domain source cursor
- root ordered semantic loop state

The supervisor must not:

- schedule semantic work item by item across domains
- route every world model to execution transition after ports are wired
- route every execution to events transition after ports are wired
- route every events to world model transition after ports are wired
- decide whether a goal exists
- decide whether a goal is satisfied
- decide whether evidence supports a belief
- decide whether a task network command is valid
- decide whether publication should be pending or published
- decide where a domain runtime resumes after restart
- append supervisor leases or heartbeats to the canonical event spine by default

Supervisor lifecycle records may become product facts only through an explicit future policy that promotes an operational condition into domain meaning.

## Implementation Phases

### Phase S1 - Contracts

Define supervisor contracts for runtime ids, desired runtime state, runtime instances, leases, heartbeats, health snapshots, lifecycle events, shutdown state, restart policy, and supervisor errors.

Exit criteria:

- runtime id validation is covered by tests
- records serialize and round trip
- lease status changes are explicit
- contracts contain no domain cursor ownership fields

### Phase S2 - Supervisor Store

Add `supervisor.sled` to the product storage layout and implement store operations for instances, desired state, leases, heartbeats, health snapshots, restart records, shutdown records, and lifecycle events.

Exit criteria:

- supervisor storage opens beside existing product stores
- supervisor store flushes independently
- no domain cursor is stored as owned supervisor progress
- corrupted supervisor records fail loudly with supervisor errors

### Phase S3 - Desired Runtime Registry

Implement an in process runtime factory registry and desired runtime reconciliation.

Exit criteria:

- enabled runtime ids map to concrete factories
- disabled runtime ids do not start
- missing factories produce supervisor diagnostics
- desired runtime state is visible in operator status

### Phase S4 - Lease Manager

Implement lease acquire, renew, release, expiry recovery, stale owner rejection, and active lease indexing.

Exit criteria:

- duplicate active lease acquisition fails
- expired leases are recovered at startup
- stale lease owners cannot renew
- stale lease owners cannot heartbeat
- restart creates a new lease id

### Phase S5 - Runtime Handles

Wrap event, world model, and execution runtimes behind lifecycle handles that expose start, stop, safe point wait, heartbeat, and flush hooks where needed.

Exit criteria:

- supervisor starts handles only after lease acquisition
- each running handle writes heartbeat records
- domain work still flows through direct handoff ports
- handle context contains no supervisor-held semantic progress

### Phase S6 - Shutdown And Flush

Implement coordinated cancellation, safe point waits, flush ordering, final heartbeats, lease release, health snapshots, and stopped instance records.

Exit criteria:

- shutdown can run twice without corrupting supervisor state
- task network stores flush before product boundary
- product stores flush before final lease release
- supervisor store flush persists final lifecycle state
- stopped instance is visible in operator status

### Phase S7 - Recovery

Implement startup recovery for prior instances, expired leases, stale health, and desired runtime reconciliation.

Exit criteria:

- expired leases are marked expired during startup
- unexpired active leases are not stolen
- prior stale instances are visible in lifecycle records
- restarted domains resume from domain stores

### Phase S8 - Restart Policy

Implement `never`, `on_retryable_failure`, and `on_heartbeat_expiry` with restart counters, backoff, attempt limits, and restart cause records.

Exit criteria:

- retryable failure can restart when policy allows
- heartbeat expiry can restart when policy allows
- fatal reports become unhealthy status unless policy explicitly changes later
- restart does not read semantic domain state

### Phase S9 - Operator Status

Expose supervisor query APIs and later a thin CLI adapter.

Exit criteria:

- operator can see desired state, factory state, lease owner, heartbeat age, health, restart count, and last diagnostic outcome
- status output is available without reading domain internals
- CLI adapter does not duplicate supervisor logic

## Verification Requirements

Contract tests must cover:

- valid runtime ids
- invalid runtime ids
- desired state round trip
- instance record round trip
- lease record round trip
- heartbeat record round trip
- health snapshot round trip
- restart policy round trip

Lease tests must cover:

- acquire succeeds with no active lease
- acquire fails with active unexpired lease
- renew succeeds for active owner
- renew fails for stale owner
- heartbeat succeeds for active owner
- heartbeat fails for stale owner
- release succeeds for active owner
- release is idempotent for already released lease
- expired lease recovery clears the active lease index

Lifecycle tests must cover:

- enabled runtimes start after lease acquisition
- disabled runtimes do not start
- missing factory becomes supervisor diagnostic
- startup recovery does not steal unexpired lease
- shutdown writes final heartbeat before lease release
- shutdown flushes product stores before final lease release
- shutdown flushes supervisor store after final lifecycle records

Restart tests must cover:

- `never` leaves failed runtime stopped
- `on_retryable_failure` restarts after retryable failure
- `on_heartbeat_expiry` restarts after stale heartbeat and lease expiry
- restart creates a new lease id
- old lease owner cannot renew after restart
- restart counter and cause are visible in status
- restart attempt limit stops restart loops

Flywheel boundary tests must prove:

- supervisor does not query active goals for progress
- supervisor does not query belief views for progress
- supervisor does not query task network state for progress
- supervisor does not query pending publications for progress
- supervisor does not store supervisor-held source cursors
- domain runtimes resume from owning stores after restart
- `RTG-6` reopen checkpoints still read correctness through domain APIs

Suggested focused checks:

```sh
cargo test runtime::supervisor
cargo test runtime::storage
cargo test runtime::contracts
rg -n "supervisor-held semantic cursor|shared cursor table" src design/plan/integration
rg -n "mod.rs" src/runtime
```

Before acceptance, inspect `supervisor.sled` record definitions and tests to confirm no owned semantic progress fields are present.

## Acceptance Criteria

The requirements are satisfied when the root runtime supervisor can start, observe, stop, recover, and restart domain runtime handles without owning their semantic progress.

The implementation must provide:

- stable runtime identity validation
- durable desired runtime state
- durable runtime instance registration
- one active lease per runtime id
- active lease renewal and stale owner rejection
- heartbeat records bound to active leases
- health snapshots for operator visibility
- graceful shutdown with domain safe point waits
- flush ordering that respects per network stores, product stores, and supervisor store
- conservative restart policy with bounded attempts
- operator status that explains lifecycle and diagnostics
- tests proving supervisor records do not replace domain stores

The durable flywheel is preserved when domain momentum still flows through direct ports and owning stores:

```text
world model -> execution -> events -> world model
```
