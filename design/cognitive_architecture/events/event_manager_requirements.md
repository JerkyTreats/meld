# Event Spine Requirements

Date: 2026-04-12
Status: active
Scope: canonical spine for promoted semantic facts across `sensory`, `world_state`, `execution`, and attached object domains

## Objective

Define what must be true for the event spine.
This document closes the core design choices.
Migration sequencing lives in [Event Spine Refactor](telemetry_refactor.md).

## Current Baseline

The current code already gives the spine a partial shape:

- `src/telemetry/events.rs` defines `ProgressEvent`
- `src/telemetry/routing/bus.rs` provides in-process ingress
- `src/telemetry/routing/ingestor.rs` serializes append through one ingestor
- `src/telemetry/sinks/store.rs` persists ordered records in `sled`
- `src/telemetry/sessions/service.rs` exposes the current emission path
- `src/task/events.rs` defines execution event vocabulary
- `src/task/executor.rs` keeps a task-local event log outside the durable store
- `src/workspace/watch/events.rs` batches local file changes before any semantic promotion

The current gaps are:

- `seq` is session scoped rather than runtime wide
- emission drains and flushes on every event
- execution facts are split across two event paths
- raw local batching and promoted semantic facts are not separated clearly
- telemetry still owns too much of the canonical path

## Core Thesis

There must be one canonical spine for promoted semantic facts.

That spine must:

- accept events from concurrent producers
- assign one runtime-wide deterministic order
- persist append-only records
- support replay into reducer owned projections
- allow downstream consumers without making them part of correctness

Raw sensory pulses, transient worker chatter, and presentation summaries do not belong in the canonical spine.
Only promoted semantic facts belong there.

## Ownership

`events` owns:

- ingress contract
- sequence assignment
- durable append contract
- replay contract
- subscription contract
- compatibility rules for stored envelopes

`execution`, `world_state`, and `sensory` own:

- typed event families
- reducer logic
- projection logic
- promotion rules from local runtime behavior into canonical facts

`telemetry` owns:

- observability adapters
- summary mapping
- metrics and export sinks

## Canonical Envelope

The canonical stored envelope for the first spine should be:

```rust
struct SpineEvent {
    ts: String,
    session: String,
    seq: u64,
    domain_id: String,
    stream_id: String,
    #[serde(rename = "type")]
    event_type: String,
    content_hash: Option<String>,
    data: serde_json::Value,
}
```

Required meaning of the fields:

- `ts`
  event creation time as observed by the publisher
- `session`
  operator or runtime session grouping
- `seq`
  runtime-wide monotonic spine order
- `domain_id`
  one domain label such as `execution`, `world_state`, `sensory`, or `workspace_fs`
- `stream_id`
  one stable per-object or per-run stream anchor such as `task_run_id`
- `event_type`
  typed semantic event name
- `content_hash`
  optional link to one content-addressed object or blob summary
- `data`
  typed payload encoded into one stable durable envelope

## Closed Design Decisions

### One runtime-wide order

`seq` must be runtime wide.
Session local sequence is not enough for a real spine.

Without one shared order, cross-domain temporal questions have no precise answer.
The first implementation should use one local sequencer and one local append path.

### Stream identity is required

`stream_id` is required from the first landing.

The spine needs a stable per-run or per-object anchor that is not overloaded onto `session`.
The first execution slice should use values such as:

- `task_run_id`
- `plan_id`
- `workflow_id`

### Facts, not views

The spine stores facts.
Views are projections.

Facts for the first execution slice include:

- `execution.task.requested`
- `execution.task.started`
- `execution.task.progressed`
- `execution.task.blocked`
- `execution.task.succeeded`
- `execution.task.failed`
- `execution.task.artifact_emitted`
- `execution.repair.requested`
- `execution.repair.applied`
- `execution.control.dispatch_requested`
- `execution.control.dispatch_started`
- `execution.control.dispatch_completed`

Derived views include:

- progress percentage
- active task count
- TUI lane summaries
- retry dashboards

### Raw sensory stays outside

The spine is not the raw sensory transport.
High-rate sensory lanes lower into local observation forms first.
Only promoted semantic observations enter the spine.

### Reducers own state transitions

Workers may emit facts.
Reducers own canonical state transitions and projections.
Workers must not mutate reducer-owned state directly.

## Domain Event Families

The first landing should standardize these families:

- `execution.session.*`
- `execution.task.*`
- `execution.control.*`
- `execution.repair.*`
- `execution.artifact.*`

The next families should reserve namespace only:

- `sensory.observe.*`
- `sensory.promote.*`
- `world_state.claim.*`
- `world_state.belief.*`
- `world_state.calibration.*`

## Durability Requirements

- canonical records are append only
- replay must rebuild reducer projections after restart
- the design must define durable commit versus buffered commit
- durability policy must be explicit rather than hidden inside one emit call
- correction happens through later events, not in-place mutation

## Ingress Requirements

- concurrent producers are allowed
- canonical ingress must be bounded
- shutdown and cancellation behavior must be explicit
- one slow consumer must not block append correctness

The current `std::sync::mpsc` bus is acceptable as a transitional local mechanism.
The design requirement is bounded ingress semantics, not that this exact channel survive unchanged.

## Subscription Requirements

- reducers must resume from durable sequence
- downstream consumers may follow live or catch up from durable history
- slow downstream consumers must recover through replay or separate projection state
- diagnostics must not become the sole replay source

## Cross-Domain References

Cross-domain payloads should use one stable reference shape:

```rust
struct DomainObjectRef {
    domain_id: String,
    object_kind: String,
    object_id: String,
}
```

`object_id` may be a content hash, task run id, workspace node hash, or another stable domain identifier.
The first spine commit does not need full `DomainObjectRef` adoption in every payload.
It does need to stop assuming that `NodeID` is the universal anchor.

## First Landing Constraints

The first implementation slice is `execution`.

That slice must:

- keep existing storage and reader continuity where practical
- introduce the canonical envelope fields now
- shift sequence assignment to runtime-wide order now
- make `TaskEvent` compatible with canonical spine publication
- keep raw workspace watch batches outside the canonical spine until semantic promotion exists

## Non Requirements

This design does not require:

- distributed consensus in the first landing
- an external broker in the first landing
- a full CQRS framework
- raw sensory ingestion into the canonical spine
- world-state ECS work before the execution slice lands

## Acceptance Conditions

The spine requirements are satisfied when all of these are true:

- one canonical envelope is defined and shared
- runtime-wide sequence is explicit
- typed event families are named for the first execution slice
- reducer ownership of state transition is explicit
- replay can rebuild required projections
- telemetry is downstream of the spine
- compatibility and parity gates are defined for the migration
- the acceptance criteria in [Event Spine Refactor](telemetry_refactor.md) are all satisfied
