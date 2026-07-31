# Runtime Operator Visibility Plan Skeleton

Date: 2026-07-02
Status: skeleton
Scope: runtime startup, action observability, status cache, and operator command behavior

## Purpose

This skeleton records the work needed to make runtime operation visible enough to prove whether the product flywheel is doing useful work.

The immediate operator problem is that `meld runtime run` can own the runtime process and still print nothing until shutdown. The companion problem is that `meld runtime status` can fail while the runtime is active because normal CLI startup touches product storage before the runtime status path is reached.

This plan starts with visibility and command behavior. It does not replace the existing runtime requirements for world model, execution, events, product assembly, or supervisor boundaries.

Execution of this runtime plan starts after the [Event Foundation Closeout Program](../events/event_foundation_closeout_program.md) reaches E6.
Events supplies `EventHealthReport`, stable status mapping inputs, one authority, and a transport-neutral remote contract.
Runtime owns status mapping, supervisor cadence, `RuntimeStatusPublisher` invocation, status cache persistence and staleness, console and action publication, daemon hosting, real IPC, and process integration.

## Source Anchors

- Runtime requirements index: `design/plan/integration/runtime_requirements.md`
- Supervisor runtime requirements: `design/plan/integration/supervisor_runtime_requirements.md`
- Event runtime requirements: `design/plan/integration/event_runtime_requirements.md`
- Execution runtime requirements: `design/plan/integration/execution_runtime_requirements.md`
- World model runtime requirements: `design/plan/integration/world_model_runtime_requirements.md`
- Current runtime CLI tooling: `src/runtime/tooling.rs`
- Current runtime presentation: `src/runtime/presentation.rs`
- Current CLI route context: `src/cli/route.rs`
- Current runtime assembly: `src/runtime/assembly.rs`
- Current worker report contracts: `src/runtime/contracts.rs`
- Current supervisor store: `src/runtime/supervisor/store.rs`

## Current Assessment

`meld runtime run` is currently expected to look blank. The command loads product runtime assembly, starts the foreground supervisor, enters a tick loop, and formats only a final stopped summary after duration expiry or Ctrl-C. The current text formatter is named as a completed foreground run formatter.

That behavior is not acceptable for runtime operation. The operator cannot tell whether startup succeeded, whether handles are concrete or inert, whether a tick made progress, whether the flywheel is stalled, or whether a domain is retrying.

`meld runtime status` can fail while `runtime run` is active because the top level CLI constructs `RunContext` before command dispatch. That context loads the full CLI assembly and catches up the graph runtime, which can touch the locked product store before the runtime status code runs. Runtime status later reads supervisor lifecycle rows, but the route has already done too much work.

Current concrete runtime work is narrow. The supervisor starts enabled handles, records desired state, acquires leases, writes heartbeats, writes health snapshots, evaluates restarts, and shuts down cleanly. Root assembly has descriptors for the first proof runtimes, but the only concrete semantic handle currently wired by assembly is `world_model.graph_replay`. Other factory descriptors are present and mostly inert at the supervisor boundary.

The existing `WorkerTickReport` is a useful diagnostic seed. It can show bounded input and output checkpoints, attempted item count, committed item count, issue counts, and budget exhaustion. It is not rich enough by itself to describe domain object lifecycle actions.

## Goals

- Every runtime emits startup data that explains identity, availability, concrete or inert handle state, lease ownership, and initial health.
- Every runtime emits bounded action data that names the domain object touched, the action attempted, the cause, the outcome, and the relevant domain checkpoint.
- `meld runtime run` prints startup state, live tick frames, action summaries, error summaries, and final shutdown state.
- `meld runtime status` returns without opening product stores and without querying locked domain databases.
- Runtime status reads a cache projection written by the running supervisor process.
- The status cache remains an observation layer and never becomes the source of semantic truth.
- Domain object lifecycle visibility is designed domain by domain before broad implementation.
- End to end proof can distinguish idle healthy runtimes from useless spinning and from real workflow progress.

## Non Goals

- Do not move semantic progress into the supervisor.
- Do not make event replay the source of runtime health.
- Do not make status cache data authoritative for cursor recovery.
- Do not have root runtime inspect execution, event, or world model internals to decide correctness.
- Do not add a generic logging dump as a substitute for typed action data.
- Do not require status readers to open product sled stores while the runtime process owns them.

## Command Behavior Target

Foreground `meld runtime run` may remain a long lived process host while it owns the supervisor. It must not remain visually blank. It must print an initial startup frame, then print on progress, failure, restart, shutdown, and periodic idle heartbeat.

Operator read commands must be non-blocking in the practical sense that they do not wait on product database locks. `meld runtime status` must use passive config plus status cache files only.

Detached launch is a separate process control decision. A safe detached default needs a process record, readiness handshake, log paths, and a stop command. Until those exist, the safest first slice is foreground run with live frames and non-blocking status.

## Status Cache Shape

The status cache is a single writer and many reader projection. The running supervisor writes it. CLI status readers read it. It should use ordinary files with atomic replacement for the latest snapshot and bounded append for recent actions.

Proposed files under the runtime product root:

- `runtime/status/latest.json`
- `runtime/status/actions.jsonl`
- `runtime/status/process.json`

The cache writer writes after startup, after each tick with changed data, after each restart decision, after every shutdown phase change, and before final process exit.

The cache reader must tolerate missing, partial, stale, and older version cache files. A missing cache means no active status has been observed. A stale cache means the reader reports stale age and does not infer semantic failure.

The status cache contains operational and diagnostic projections only:

- product root
- supervisor store path
- status cache path
- cache version
- writer process id
- instance id
- run mode
- launch status
- started time
- ready time
- last write time
- shutdown state
- desired runtime set
- runtime availability
- concrete handle state
- active lease summary
- latest heartbeat summary
- health summary
- restart summary
- latest lifecycle event
- latest worker report summary
- recent action summaries
- cache staleness hints

The status cache does not contain:

- graph replay cursor as supervisor owned progress
- belief evidence cursor as supervisor owned progress
- agent delivery cursor as supervisor owned progress
- execution goal lifecycle authority
- planning attempt authority
- task network revision authority
- dispatch claim authority
- publication outbox authority
- event spine sequence authority
- provider request payloads
- raw model output

## Runtime Status Reader

`runtime status` should become a lightweight route. It should not construct `RunContext`, should not load `CliRuntimeAssembly`, and should not trigger graph catch up.

The reader inputs are:

- workspace root
- optional config path
- passive runtime description
- status cache latest snapshot
- status cache recent actions when requested
- wall clock for age calculations

The reader output should show:

- product root
- cache age
- instance id
- process id when known
- launch status
- runtime count by health class
- each runtime id
- enabled state
- factory availability
- concrete handle state
- lease state
- heartbeat age
- health
- last meaningful action
- last progress checkpoint as diagnostic text

The JSON output should preserve the full typed snapshot. Text output should be terse by default and should have a future verbose mode for recent actions.

## Runtime Run Console

`runtime run` should collate supervisor lifecycle data and domain worker action data into console frames.

Startup frame:

- command id or session id when available
- instance id
- product root
- supervisor store path
- status cache path
- enabled runtime count
- disabled runtime count
- concrete handle count
- inert handle count
- runtime ids grouped by domain
- startup failures by runtime id

Tick frame:

- tick number
- elapsed time
- active runtime count
- changed runtime count
- progress action count
- retryable issue count
- fatal issue count
- budget exhaustion count
- per runtime compact delta

Action line:

- timestamp
- runtime id
- domain id
- object ref
- action kind
- outcome
- attempted count
- committed count
- checkpoint change
- issue code when present

Idle heartbeat:

- tick number
- elapsed time
- no progress marker
- active runtime count
- latest heartbeat age range

Shutdown frame:

- shutdown id
- requested time
- safe point count
- flush result count
- stopped runtime count
- released lease count
- final health summary

The default text mode should print startup and changes. A future `jsonl` mode can print every frame as one JSON record. The existing `json` mode can remain a final object or become a bounded final snapshot after the detailed format decision.

## Runtime Action Contracts

The implementation should introduce typed runtime action contracts in the runtime domain. These contracts are for observation and event emission. They do not replace domain stores.

`RuntimeStatusCacheRecord`:

- cache version
- product root
- status cache path
- writer identity
- snapshot
- recent action window
- write time

`RuntimeStatusSnapshot`:

- instance summary
- process summary
- shutdown summary
- runtime rows
- aggregate health counts
- cache warnings

`RuntimeActionRecord`:

- action id
- observed time
- runtime id
- domain id
- actor id
- object ref
- action kind
- cause
- outcome
- metrics
- checkpoint observations
- issue summaries
- redaction state

`RuntimeActionKind`:

- startup
- shutdown
- tick
- lease
- heartbeat
- health
- restart
- append
- replay
- reduction
- projection
- command
- mutation
- dispatch
- provider
- artifact
- publication
- scan
- curation

`RuntimeObjectRef`:

- domain id
- object type
- object id
- branch id
- perspective key
- parent refs
- source refs

`RuntimeActionCause`:

- operator command
- supervisor tick
- lease recovery
- replay batch
- domain command
- task readiness
- retry
- shutdown

`RuntimeActionOutcome`:

- started
- succeeded
- no work
- duplicate
- rejected
- blocked
- retryable failure
- fatal failure
- cancelled

`RuntimeTickConsoleFrame`:

- frame id
- tick number
- elapsed time
- changed runtime rows
- progress actions
- issue actions
- aggregate health

`RuntimeStatusPublisher`:

- publish startup snapshot
- publish tick snapshot
- publish action
- publish shutdown snapshot
- flush cache

`RuntimeStatusReader`:

- read latest snapshot
- read recent actions
- derive stale status
- format text
- format JSON

## Event Emission Boundary

Runtime action records should also map to canonical event spine envelopes where useful. The mapper belongs to the runtime domain. The event spine stores and sequences records, but it does not own runtime payload meaning.

Runtime lifecycle event families should include:

- `runtime.supervisor.instance_registered`
- `runtime.supervisor.instance_status_changed`
- `runtime.supervisor.desired_state_recorded`
- `runtime.handle.start_requested`
- `runtime.handle.start_succeeded`
- `runtime.handle.start_failed`
- `runtime.lease.acquired`
- `runtime.lease.renewed`
- `runtime.lease.released`
- `runtime.lease.expired`
- `runtime.heartbeat.accepted`
- `runtime.heartbeat.stale`
- `runtime.health.snapshot_recorded`
- `runtime.tick.started`
- `runtime.tick.completed`
- `runtime.tick.failed`
- `runtime.restart.scheduled`
- `runtime.restart.skipped`
- `runtime.shutdown.requested`
- `runtime.shutdown.safe_point_reached`
- `runtime.shutdown.flush_started`
- `runtime.shutdown.completed`
- `runtime.shutdown.failed`

Lifecycle writes must not depend on event append success. The supervisor should commit lifecycle state, update status cache, then bridge to events with idempotent record ids. Bridge progress is bridge state only, not runtime correctness.

## Domain Action Lifecycle Skeleton

### Supervisor

Objects:

- process
- runtime instance
- desired runtime row
- runtime handle
- lease
- heartbeat
- health snapshot
- restart record
- shutdown record

Actions:

- process launch recorded
- instance registered
- desired state recorded
- handle start requested
- handle start succeeded
- handle start failed
- lease acquired
- lease renewed
- lease released
- lease expired
- heartbeat accepted
- heartbeat detected stale
- health snapshot recorded
- tick started
- tick completed
- tick failed
- restart scheduled
- restart skipped
- shutdown requested
- shutdown safe point reached
- shutdown flush completed
- instance stopped

Visibility gap:

- no process record
- no readiness handshake
- no status cache
- no stop command
- no live console frames

### Event Runtime

Objects:

- event envelope
- event record
- record id index
- sequence metadata
- replay batch
- retention floor
- bridge checkpoint

Actions:

- append requested
- append accepted
- duplicate record id replayed
- append rejected
- sequence assigned
- record id index written
- replay requested
- replay window served
- replay rejected by retention
- sequence metadata repaired
- bridge checkpoint advanced

Visibility gap:

- event append and replay exist, but runtime action events for append, replay, duplicate, retention, and repair are not emitted through a dedicated runtime action contract.

### World Model Graph Replay

Objects:

- event batch
- graph object
- graph relation
- graph anchor
- selected anchor
- provenance ref
- reducer cursor
- derived event envelope

Actions:

- replay window selected
- event record reduced
- source intent mapped
- object fact materialized
- relation fact materialized
- anchor selected
- anchor ended
- anchor superseded
- derived event append requested
- derived event append accepted
- cursor advanced
- no work batch recorded

Visibility gap:

- graph replay has the strongest current concrete handle coverage, but console output and status cache do not yet expose these actions as domain object lifecycle records.

### Belief Assessment

Objects:

- belief family config
- config snapshot
- evidence item
- evidence assignment
- dirty belief key
- assessment lease
- belief revision
- belief head
- belief view
- rejection record

Actions:

- config snapshot loaded
- evidence normalized
- evidence rejected
- evidence assigned
- dirty key scheduled
- dirty key claimed
- assessment lease acquired
- revision drafted
- revision committed
- belief head updated
- belief view projected
- assessment lease completed
- assessment lease abandoned
- expired lease recovered

Visibility gap:

- belief domain state and leased runtime behavior exist, but supervised status does not expose equal action depth for evidence normalization, revision commit, view projection, and lease recovery.

### Planner Projection

Objects:

- projection request
- projection rule version
- belief view input
- graph input
- source ref
- hydration ref
- warning
- world state frame

Actions:

- projection requested
- input views selected
- graph refs selected
- warning captured
- world state frame built
- projection returned
- projection snapshot stored when enabled

Visibility gap:

- planner projection is query oriented and lacks a dedicated supervised runtime id, worker report, and durable action record. The plan must decide whether it remains a called port with action data or becomes a runtime handle with cached projection snapshots.

### Agent Goal Curation

Objects:

- directive
- seed agent
- activated agent
- subscription
- delivery cursor
- projection frame
- curation decision
- goal command
- sink receipt

Actions:

- directive loaded
- seed agent registered
- agent activated
- subscription registered
- delivery selected
- projection requested
- curation decision persisted
- goal command submitted
- duplicate command observed
- sink receipt recorded
- delivery cursor advanced
- sink failure retained

Visibility gap:

- agent curation has useful domain reports, but the runtime action view should show decision ids, command ids, sink receipts, and cursor movement without leaking domain internals.

### Event Evidence Ingestion

Objects:

- source mapping
- replay batch
- execution outcome event
- promoted evidence item
- assignment
- dirty belief key
- ingestion cursor
- rejection record

Actions:

- replay window selected
- mapping selected
- event accepted for ingestion
- event normalized to evidence
- evidence rejected
- evidence assignment written
- dirty key scheduled
- ingestion cursor advanced
- ingestion batch exhausted budget

Visibility gap:

- promoted evidence ingestion exists, but a first class durable ingestion cursor per mapping, belief family, perspective, and branch is not clearly established in the supervised runtime surface.

### Satisfaction Curation

Objects:

- active goal summary
- review source
- projection frame
- satisfaction decision
- goal mutation command
- sink receipt
- review cursor

Actions:

- active goal set loaded
- review candidate selected
- projection requested
- satisfaction decision persisted
- mutation command submitted
- duplicate mutation observed
- sink receipt recorded
- review cursor advanced
- sink failure retained

Visibility gap:

- satisfaction decisions and sink receipts exist, but review cursoring after sink durability is not yet a first class runtime visibility contract.

### Execution Goal Set

Objects:

- goal command
- goal record
- lifecycle transition
- mutation command
- command outcome
- source identity

Actions:

- command received
- command validated
- duplicate command replayed
- goal accepted
- goal activated
- goal suspended
- goal resumed
- goal satisfied
- goal removed
- mutation rejected
- lifecycle state persisted

Visibility gap:

- execution goal lifecycle is durable, but runtime status should expose accepted, duplicate, rejected, active, and satisfied transitions as action summaries.

### Execution Planning

Objects:

- active goal
- planning attempt
- planner projection request
- method library snapshot
- composition result
- task network mutation
- task network command result

Actions:

- active goal selected
- projection requested
- method evaluated
- planning result satisfied
- planning result composed
- planning result no method
- composition lowered
- mutation command submitted
- planning attempt persisted
- goal satisfaction submitted when already satisfied

Visibility gap:

- planning reports can convert to worker reports, but status should expose the attempt identity, projection frame, result class, and task network command id.

### Task Network Command

Objects:

- network
- command request
- command response
- journal record
- reduced state
- ready set
- claim
- outcome
- publication outbox record

Actions:

- mutation accepted
- mutation rejected
- revision committed
- ready set computed
- claim accepted
- claim rejected
- outcome accepted
- outcome rejected
- publication queued
- publication marked
- command duplicate replayed
- request hash conflict rejected

Visibility gap:

- task network has strong durable state, but claim, rejection, ready set, and publication status are not emitted as first class runtime action records.

### Task Dispatch

Objects:

- ready task
- claim request
- accepted claim
- task initialization
- task run
- capability instance
- artifact repository
- task outcome

Actions:

- ready task observed
- claim requested
- claim accepted
- claim rejected
- initialization loaded
- artifact repository opened
- capability invocation planned
- capability invocation started
- capability invocation completed
- capability invocation failed
- artifact written
- task outcome built
- outcome command submitted

Visibility gap:

- local task execution can run, but status summaries need ready count, running claim count, invocation count, artifact count, last event, and block reason.

### Provider And Capability

Objects:

- capability contract
- invocation request
- provider profile
- provider request
- provider response
- retry state
- redacted diagnostic

Actions:

- capability contract selected
- invocation request built
- provider profile selected
- provider request started
- provider response received
- provider request failed
- retry scheduled
- redacted summary stored
- capability result returned

Visibility gap:

- provider progress exists in command telemetry, but runtime status should show recent provider request state, retry state, provider binding, and redaction state without exposing raw prompts or model output.

### Publication

Objects:

- pending publication
- event envelope
- event append result
- mark publication command
- retry record

Actions:

- pending publication selected
- event envelope prepared
- append requested
- append accepted
- duplicate append observed
- append failed
- publication marked
- retry scheduled
- failed publication surfaced

Visibility gap:

- terminal task outcome events exist, but publication status changes and failures need direct runtime visibility.

### Workflow And Task Package

Objects:

- workflow thread
- workflow turn
- gate
- prompt link
- task package trigger
- prepared task run
- workflow event

Actions:

- workflow target selected
- turn opened
- provider turn requested
- task package trigger prepared
- task run prepared
- gate evaluated
- workflow event emitted
- turn completed
- turn failed

Visibility gap:

- workflow progress and turn events exist, but status does not show workflow task package progress after partial failure or stalled provider work.

### Workspace And Docs Freshness

Objects:

- workspace scan
- tree node
- changed path
- context frame
- docs candidate
- docs freshness task

Actions:

- scan requested
- tree walked
- node changed
- context frame created
- docs candidate produced
- task package prepared
- task dispatch requested
- publication completed

Visibility gap:

- docs freshness proof needs operator visible evidence that workspace scanning, task creation, dispatch, publication, event replay, graph update, belief update, planning, and satisfaction ran end to end.

## First Slice

The first implementation slice should be small and decisive.

- Add status cache contracts and file writer in the runtime supervisor domain.
- Write startup, tick, action, and shutdown snapshots from `RuntimeSupervisor`.
- Convert current supervisor lifecycle records and `WorkerTickReport` values into `RuntimeActionRecord`.
- Make `runtime status` bypass full CLI context and read the cache.
- Keep fallback passive desired state output when no cache exists.
- Make foreground `runtime run` print startup, progress, idle heartbeat, issue, and shutdown frames.
- Add an integration test where `runtime run` is active and a separate `runtime status` invocation on the same root returns without product store lock failure.

This first slice does not need every domain action publisher. It must still make clear when a runtime is inert versus doing concrete work.

## Delivery Waves

These waves are runtime work and do not gate event closure.

Wave 1: status cache substrate.

- Add cache record contracts under runtime supervisor ownership.
- Add atomic snapshot writer and bounded action log writer.
- Add tolerant snapshot reader.
- Add cache stale derivation.
- Add tests for partial, missing, stale, and older version cache reads.

Wave 2: CLI route isolation.

- Route `runtime status` through a lightweight path.
- Avoid `RunContext` construction for runtime status.
- Avoid product store opens for status.
- Preserve text and JSON output.
- Add lock regression coverage.

Wave 3: run console collation.

- Add startup frames.
- Add tick delta frames.
- Add idle heartbeat.
- Add issue and failure frames.
- Add shutdown frames.
- Add text snapshots and JSON snapshot behavior.

Wave 4: supervisor lifecycle action bridge.

- Convert supervisor lifecycle records to runtime action records.
- Add runtime owned event envelope mapper.
- Add idempotent record id policy.
- Add event bridge tests that duplicate bridge drain does not duplicate event records.

Wave 5: domain worker action depth.

- Expand `WorkerTickReport` or add companion action records for concrete domain lifecycle actions.
- Add graph replay action records.
- Add belief assessment action records.
- Add agent goal curation action records.
- Add satisfaction action records.
- Add execution planning action records.
- Add task network command and dispatch action records.
- Add publication action records.

Wave 6: process control.

- Add process launch record.
- Add readiness handshake.
- Add log path records.
- Add shutdown request command.
- Add `runtime stop`.
- Add detached launch option.
- Decide whether detached launch becomes the default later.

Wave 7: end to end proof.

- Run docs freshness workflow with visible action records from workspace scan through execution and back into world model.
- Prove restart safety by killing and reopening the runtime.
- Prove status remains available while the supervisor owns product stores.
- Prove action data distinguishes idle, stalled, retrying, and progressing runtimes.

## Testing Skeleton

Focused checks:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test runtime::supervisor --lib`
- `cargo test runtime::contracts --lib`
- `cargo test runtime::presentation --lib`
- `cargo test --test integration_tests runtime_cli`
- `cargo test --test integration_tests progress_observability`
- `cargo test --test integration_tests event_spine`
- `cargo test --test integration_tests goal_acceptance`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

New required tests:

- status cache writer round trip
- status cache reader tolerates missing cache
- status cache reader tolerates partial action log line
- status cache reader marks stale cache without failing
- runtime status bypasses full CLI assembly
- runtime status works while foreground runtime run owns product storage
- runtime run prints startup before first sleep
- runtime run prints idle heartbeat when no work is done
- runtime run prints progress when graph replay commits work
- runtime run final summary includes shutdown and stopped runtime counts
- lifecycle action event bridge is idempotent
- worker action records are bounded and redacted
- concrete versus inert runtime rows are visible

Static scans:

- no `mod.rs` under changed Rust modules
- no status reader import from domain store internals
- no supervisor use of domain checkpoints for resume decisions
- no raw provider request payload in runtime status cache

## Open Design Questions

- Should `runtime run` stay foreground by default with live frames, or should it become a detached launch after process control exists.
- Should `json` for long running run mode mean final snapshot or should a new `jsonl` mode own streaming frames.
- What retention window is enough for `actions.jsonl`.
- Should status cache live beside `supervisor.sled` or under a separate `runtime/status` directory.
- Should status cache include last emitted runtime lifecycle event sequence, labeled only as bridge progress.
- Which planner projection actions are best represented as port call action records versus a supervised planner runtime handle.
- What redaction level is required for provider, workflow, and task package action summaries.

## Acceptance Criteria

The plan is complete enough for implementation when it can answer these operator questions from cache and console data:

- Is the supervisor alive.
- Which process owns it.
- Which runtimes were desired.
- Which runtimes are concrete versus inert.
- Which runtimes acquired leases.
- Which runtimes are ticking.
- Which runtimes made domain progress.
- Which runtime last failed.
- Which object was touched by the last meaningful action.
- Whether the flywheel is idle, stalled, retrying, or progressing.
- Whether a docs freshness workflow traversed from workspace scan to execution publication and back into world model state.

## Handoff Packets

Implementation should split into these packets:

- Runtime status cache contracts and file persistence.
- CLI route isolation for runtime status.
- Run console frame formatter.
- Supervisor lifecycle action mapper.
- Event bridge for runtime lifecycle actions.
- World model action publishers.
- Execution action publishers.
- Process control and detached launch.
- End to end docs freshness runtime proof.
