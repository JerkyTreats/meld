# Event Domain Extraction Spec

Date: 2026-04-17
Status: active
Scope: durable extraction of canonical event ownership from `telemetry`, with downstream consumers moved off the correctness path and no premature public API commitments

## Objective

Make `events` the durable correctness domain for append, replay, sequencing, subscription, and compatibility.

Do not use this extraction to freeze a permanent downstream domain name.
The goal is to separate canonical event concerns from downstream consumer concerns.
Whether the downstream home later lands as `observability`, a narrower logging area, or some other operator facing domain is intentionally left open here.

The immediate goal is not a broad event rewrite.
The immediate goal is a durable boundary that lets `execution`, `world_state`, `sensory`, and branch federation grow without continuing to route correctness through `telemetry`.

## Current Assessment

The current code already contains most of the first spine substrate, but it lives under the `telemetry` domain.

Observed ownership today:

- `src/telemetry.rs` exports event contracts, event runtime, routing, sinks, and summaries as one domain surface
- `src/telemetry/events.rs` defines the canonical envelope and stored record shape
- `src/telemetry/sessions/service.rs` owns both session lifecycle and canonical append
- `src/telemetry/sinks/store.rs` mixes session records and the durable event log in one store type
- `src/task/executor.rs` still keeps a local `Vec<TaskEvent>` that behaves like an authoritative event source during execution
- `src/task/runtime.rs` mirrors those task local events into the durable store after the fact
- `src/control/projection.rs` already proves replay over the durable event log is valuable, but it still reads through telemetry owned storage types
- `src/context/queue.rs` and `src/workspace/tooling.rs` still emit raw progress style names through `ProgressRuntime::emit_event_best_effort`
- `src/telemetry/emission/engine.rs` still emits summary events through the same correctness path

This is workable for the first spine slice, but it is the wrong durable boundary for future enrichment.

## Scope Guard

This extraction is intentionally narrow.

It does not define:

- a public API for external event consumers
- a plugin runtime
- a first class tracing export surface
- a first class terminal UI surface

It does define the internal boundaries needed so those concerns can land later without pulling canonical event ownership back out of `events`.

## Problem Statement

The current system has four structural problems.

### 1. Telemetry still owns correctness

Canonical event append still flows through `ProgressRuntime`.
That means the observability domain still owns the primary write path for business facts.

### 2. Session lifecycle and event append are fused

Session records and canonical events are stored and managed through one runtime and one store type.
This makes it harder to treat sessions as telemetry concerns while treating events as a cross domain substrate.

### 3. Producers still publish through telemetry flavored APIs

Large parts of the codebase emit events through `emit_event_best_effort`.
That blurs the line between canonical semantic facts and telemetry flavored operator updates.

### 4. Telemetry storage owns the replay surface

Reducers and projections that should depend on `events` currently depend on `telemetry::sinks::store::ProgressStore`.
That keeps `telemetry` on the correctness path even when the projection itself is not telemetry.

## Desired Boundary

The durable boundary should become:

- `events` owns append, sequencing, replay, subscription, event record compatibility, and canonical event storage
- `execution`, `world_state`, `sensory`, `workspace`, and later `branches` own event families and reducers
- downstream consumer domains own session lifecycle, summaries, logging, operator read models, and best effort derived consumption as needed

In that target model, a downstream consumer may depend on `events`.
`events` never depends on that downstream consumer.

## Target Ownership

### `events` owns

- canonical `EventEnvelope`
- canonical `EventRecord`
- runtime wide sequence assignment
- append only event store
- replay and catch up reads
- live subscription and fanout contract
- durable compatibility shims for older stored event shapes
- secondary indexes needed for replay and selective reads

### downstream consumer domains own

- command session records
- session status policy and pruning
- summary event production
- derived operator read models
- logging and other observability specific contracts where applicable
- export and presentation concerns
- downstream health diagnostics

### other domains own

- typed event builders
- semantic event naming
- reducer logic
- projection logic
- promotion rules from local runtime activity into canonical facts

## Target Module Layout

Recommended target landing zones:

```text
src/events.rs
src/events/contracts.rs
src/events/runtime.rs
src/events/store.rs
src/events/ingress.rs
src/events/subscription.rs
src/events/compat.rs
src/events/query.rs

src/telemetry.rs
src/telemetry/runtime.rs
src/telemetry/sessions.rs
src/telemetry/summary.rs
src/telemetry/emission.rs
```

Recommended compatibility posture during migration:

- keep `src/telemetry/events.rs` as a thin compatibility shim for one migration window
- keep `ProgressEvent` and `ProgressEnvelope` as aliases to the canonical `events` types during transition
- keep `ProgressRuntime` as a compatibility facade over whichever downstream runtime remains during migration

## Target Runtime Model

### `EventRuntime`

`EventRuntime` becomes the canonical write and replay surface.

Required responsibilities:

- accept one canonical `EventEnvelope`
- assign one runtime wide `seq`
- persist one `EventRecord`
- expose replay after `seq`
- expose optional live subscription
- support explicit flush policy
- support append batching

Recommended public surface:

```rust
pub struct EventRuntime { ... }

impl EventRuntime {
    pub fn append_envelope(&self, envelope: EventEnvelope) -> Result<EventRecord, ApiError>;
    pub fn append_batch(&self, envelopes: Vec<EventEnvelope>) -> Result<Vec<EventRecord>, ApiError>;
    pub fn replay_after(&self, after_seq: u64) -> Result<Vec<EventRecord>, StorageError>;
    pub fn subscribe_after(&self, after_seq: u64) -> Result<EventSubscription, StorageError>;
    pub fn flush(&self) -> Result<(), StorageError>;
}
```

### downstream consumer runtime

One downstream runtime may remain during migration to own session lifecycle and other non canonical event concerns.

Required responsibilities:

- create and finish command sessions
- persist session records
- emit `telemetry.session.*` or equivalent session lifecycle facts through `EventRuntime`
- emit telemetry owned summary facts through `EventRuntime`
- coordinate sink consumers that follow the canonical event stream

The downstream runtime must depend on `EventRuntime`.
`EventRuntime` must not depend on the downstream runtime.

## Target Storage Split

### `EventStore`

The event store should own these durable concerns:

- event log tree
- runtime wide sequence metadata
- compatibility decoding for old event rows
- optional secondary indexes by session, stream, and domain

Recommended first trees:

```text
events_log
events_meta
events_by_session
events_by_stream
events_by_domain
legacy_events
```

### downstream consumer store

One downstream store may remain to own these durable concerns:

- session records
- session status metadata
- optional telemetry specific caches
- optional sink checkpoint state

Recommended first trees:

```text
telemetry_sessions
telemetry_session_meta
consumer_checkpoints
```

The initial implementation may keep both stores in one sled database.
The important change is ownership and type separation, not physical database separation.

## Event Type Migration Rule

Canonical semantic facts must stop being emitted through unscoped raw names.

Examples of current leak points:

- `scan_started`
- `queue_stats`
- queue lifecycle names emitted through `emit_event_best_effort`
- summary names emitted through telemetry helpers without a clear ownership split

Required rule:

- semantic facts use domain scoped names and canonical builders
- telemetry owned summaries use telemetry scoped names
- raw operator pulses that are not correctness relevant do not automatically enter the canonical stream

## Required Type Moves

These contracts should move under `events`:

- `DomainObjectRef`
- `EventRelation`
- canonical envelope type
- canonical stored record type

These contracts should remain outside `events`:

- session policy
- summary payload helpers
- logging configuration
- downstream export adapters

## Minimal Session Definition

The minimal correct meaning of `session` here is:

- one bounded runtime lifecycle
- used for correlation and operator visibility
- separate from canonical event ownership
- not a generic lifecycle abstraction for every domain entity

### Minimal Contract

```rust
pub struct SessionRecord {
    pub session_id: String,
    pub session_kind: SessionKind,
    pub label: String,
    pub started_at_ms: u64,
    pub ended_at_ms: Option<u64>,
    pub status: SessionStatus,
    pub error: Option<String>,
}

pub enum SessionKind {
    Command,
}

pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Interrupted,
}
```

### Boundary

`session` must not own:

- event append
- event sequence allocation
- replay
- subscription
- business stream identity
- business lifecycle semantics for tasks, turns, claims, or branches

Those belong to `events` or to the owning business domain.

The minimal session runtime supports start, finish, interruption sweep, and pruning.
It may emit lifecycle facts into `events`.
It does not own canonical event append.

### Source Layout

Recommended minimal source layout:

```text
src/session.rs
src/session/contracts.rs
src/session/runtime.rs
src/session/storage.rs
src/session/policy.rs
src/session/events.rs
src/cli/session.rs
```

## Reducer Class Taxonomy

The extraction should make reducer classes explicit.

### correctness reducers

These are on the correctness path.
They rebuild durable domain state from canonical events.

Examples in the current repo:

- execution projection
- world state graph catch up
- future branch presence materialization

Correctness reducers must:

- replay deterministically from canonical events
- own no hidden side channel authority
- be testable through live versus replay parity

### downstream consumer reducers

These are not on the correctness path.
They exist for operator visibility, logging enrichment, summaries, and later external tooling.

They may:

- reduce directly from canonical events
- reduce from correctness projections that are themselves replayable from canonical events

They must not:

- become the only place where business state can be reconstructed
- require themselves to be running for canonical append to succeed

## Public Consumer Model

This extraction intentionally stops short of freezing a public API.

What it does require now is a stable internal posture that can later support one.

Required posture:

- canonical event storage is owned by `events`
- replay and follow semantics are owned by `events`
- downstream consumers reduce from canonical events or replayable correctness projections
- no consumer depends on private producer callbacks for correctness relevant information

This preserves domain fidelity now and leaves room for a later public consumer design once the real requirements are known.

## Plugin Boundary

Plugin support is not defined in this extraction.

The extraction does need to preserve the boundary that would make plugin support possible later.

That boundary is:

- producers publish canonical events into `events`
- reducers consume canonical events through replay and follow contracts
- external style consumers are downstream only

No plugin specific transport, packaging, or ABI should be designed in this document.

## Stable Subscription Contract

This extraction should define one stable internal subscription posture, not a public protocol.

Minimum contract:

- follow from a durable `seq`
- catch up after restart from a durable `seq`
- bounded producer ingress
- slow consumer isolation from append correctness

This is enough to support internal reducers now and future public consumers later.

It is intentionally not yet a commitment to a public transport or external compatibility promise.

## Stable Public Projection Contract For Operator Tooling

This extraction does not define a public projection API.

It does define the posture operator tooling should rely on later:

- operator tooling should consume replayable reductions
- those reductions should come either from canonical events directly or from correctness projections rebuilt from canonical events
- operator reductions must remain downstream and replaceable

That gives later operator tooling a clean place to attach without making this extraction solve the public surface too early.

## Consumer Model

Telemetry should consume canonical events through one of two paths:

- direct replay and follow from `EventRuntime`
- replay and follow from reducer owned projections that are themselves rebuilt from `events`

Telemetry must not rely on private side channels from producers.
If telemetry needs data, that data must be present in the canonical event or in a reducer owned projection that is replayable from canonical events.

## Migration Plan

### Phase 1

Freeze ownership and extract canonical event types.

Code landing zones:

```text
src/events.rs
src/events/contracts.rs
src/events/compat.rs
src/telemetry/events.rs
src/telemetry.rs
```

Required changes:

- define canonical `events` types
- re export through compatibility shims
- keep serde compatibility for historical records
- make `telemetry` depend on `events`

Acceptance gate:

- old stored records decode through the new `events` surface
- current callers still compile through compatibility re exports

### Phase 2

Extract store and runtime ownership.

Code landing zones:

```text
src/events/store.rs
src/events/runtime.rs
src/events/ingress.rs
src/telemetry/runtime.rs
src/telemetry/sessions.rs
```

Required changes:

- split event storage from telemetry session storage
- move canonical append to `EventRuntime`
- make the remaining downstream runtime compose `EventRuntime`
- retain current sled location and existing data continuity

Acceptance gate:

- append works with no telemetry sink active
- session records still persist and prune correctly

### Phase 3

Cut producers over to `events` first APIs.

Code landing zones:

```text
src/task/events.rs
src/task/runtime.rs
src/task/executor.rs
src/control/orchestration.rs
src/workspace/events.rs
src/context/events.rs
src/context/queue.rs
src/workspace/tooling.rs
```

Required changes:

- domain code emits canonical envelopes through `EventRuntime`
- `TaskExecutor` local `Vec<TaskEvent>` stops being authoritative
- raw names are replaced by domain scoped event builders where the fact is correctness relevant
- telemetry best effort helpers remain only for telemetry owned outputs

Acceptance gate:

- one execution path writes canonical facts without `ProgressRuntime`
- one workspace path writes canonical facts without raw `scan_started`

### Phase 4

Move replay consumers to `events`.

Code landing zones:

```text
src/control/projection.rs
src/world_state/graph/runtime.rs
src/world_state/graph/reducer.rs
src/branches/query.rs
```

Required changes:

- replay consumers depend on `events::store::EventStore` or `events::runtime::EventRuntime`
- replay APIs stop importing telemetry owned event store types
- telemetry becomes one consumer among many

Acceptance gate:

- execution projection replays from `events`
- graph catch up replays from `events`

### Phase 5

Make downstream consumers downstream only.

Code landing zones:

```text
src/telemetry/emission.rs
src/telemetry/summary.rs
src/telemetry/runtime.rs
```

Required changes:

- summary emission routes through canonical append without owning it
- downstream read models become explicitly downstream

Acceptance gate:

- disabling downstream consumers does not change canonical append or replay correctness
- summaries still work as telemetry outputs

## Compatibility Rules

- keep historical event decode readable through serde defaults or explicit decode shims
- keep `ProgressEvent`, `ProgressEnvelope`, and `ProgressRuntime` as aliases or facades during the transition window
- do not rename on disk trees in the same step as the ownership extraction unless a tested compatibility layer is present
- do not require a one shot data migration before new code can read existing stores

## Required Tests

### event append survives with no telemetry consumer

Expected outcome:

- canonical append succeeds when no telemetry sink is registered
- replay after append returns the written events

### session lifecycle composes with event runtime

Expected outcome:

- starting and finishing a command session writes session records through telemetry storage
- session lifecycle facts append through `EventRuntime`

### legacy event records remain readable

Expected outcome:

- historical stored rows decode through `events::compat`
- old rows remain visible to replay consumers

### execution projection replays from events

Expected outcome:

- `ExecutionProjection` rebuilds from `events` storage rather than telemetry owned storage

### task executor no longer treats local history as authority

Expected outcome:

- `TaskExecutor` may retain an in memory cache for convenience
- canonical correctness comes from durable appended events

### downstream summaries are downstream only

Expected outcome:

- summary emission still functions
- downstream consumer failure does not block canonical event append

## Non Goals

- distributed brokers
- multi machine sequencing
- belief layer implementation
- full sensory promotion
- downstream consumer feature completeness
- deletion of all compatibility names in the first extraction pass

## Done When

This extraction is complete when all of these are true:

- canonical event append no longer lives under `telemetry`
- replay consumers import `events` rather than telemetry owned storage
- downstream consumers depend on `events`, not the reverse
- session lifecycle remains durable without owning canonical append
- downstream summaries continue to function as downstream consumers
- compatibility tests prove old event rows remain readable
