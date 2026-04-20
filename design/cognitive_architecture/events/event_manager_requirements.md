# Event Spine Requirements

Date: 2026-04-20
Status: active
Scope: canonical spine for promoted semantic facts across domains

## Objective

Define what must be true for the shared event spine.

This document is declarative.
Delivery history lives in [Completed Events](../../completed/events/README.md).

## Core Thesis

There is one canonical spine for promoted semantic facts.

That spine must:

- accept events from concurrent producers
- assign one runtime-wide deterministic order
- persist append-only records
- support idempotent append for derived facts
- support replay into reducer owned projections
- carry explicit graph attachment metadata
- allow downstream consumers without making them part of correctness

Only promoted semantic facts belong in the canonical spine.

## Ownership

`events` owns:

- ingress contract
- sequence assignment
- durable append contract
- replay contract
- subscription contract
- compatibility rules for stored envelopes
- `DomainObjectRef`
- `EventRelation`

Domain modules own:

- typed event families
- reducer logic
- projection logic
- promotion rules from local runtime behavior into canonical facts

`telemetry` owns:

- observability adapters
- summary mapping
- metrics and export sinks
- compatibility names for older progress surfaces

## Canonical Envelope

The canonical stored envelope is:

```rust
struct EventRecord {
    ts: String,
    recorded_at: String,
    record_id: Option<String>,
    session: String,
    seq: u64,
    domain_id: String,
    stream_id: String,
    event_type: String,
    occurred_at: Option<String>,
    content_hash: Option<String>,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
    data: serde_json::Value,
}
```

Required meaning of the fields:

- `ts`
  compatibility timestamp retained for older event readers
- `recorded_at`
  time the event was recorded into the spine
- `record_id`
  optional stable id for idempotent append
- `session`
  operator or runtime session grouping
- `seq`
  runtime-wide monotonic spine order
- `domain_id`
  owning domain label such as `execution`, `world_state`, `sensory`, or `workspace_fs`
- `stream_id`
  stable per-object, per-run, or per-source stream anchor
- `event_type`
  typed semantic event name
- `occurred_at`
  source occurrence time when known
- `content_hash`
  optional link to a content-addressed object or blob summary
- `objects`
  explicit graph objects attached by the fact
- `relations`
  explicit graph relations declared by the fact
- `data`
  typed payload encoded into one stable durable envelope

## Ordering Rules

`seq` is runtime wide.

Session local sequence is not enough for a real spine.
Without one shared order, cross-domain temporal questions have no precise answer.

The spine may later become distributed, but the contract remains one replayable total order per local spine.

## Idempotency Rules

Derived facts may provide `record_id`.

Appending the same `record_id` again must return the existing sequence and must not create a duplicate fact.

Source facts without a stable `record_id` remain append-only facts.

## Facts And Views

The spine stores facts.
Views are projections.

Facts include:

- workspace source and snapshot facts
- context frame and head facts
- execution task and artifact facts
- workflow turn and plan linkage facts
- control outcome facts
- world state derived anchor facts

Views include:

- progress summaries
- active task counts
- traversal indexes
- current anchors
- branch federated query results
- operator dashboards

## Raw Signal Rule

The spine is not raw sensory transport.

High-rate sensory lanes and raw watcher pulses lower into local observation forms first.
Only promoted semantic observations enter the spine.

## Reducer Rule

Workers may emit facts.
Reducers own canonical state transitions and projections.

Workers must not mutate reducer-owned state directly.

## Domain Event Families

Current first-class families include:

- `workspace_fs.*`
- `context.*`
- `execution.task.*`
- `execution.control.*`
- `execution.workflow.*`
- `world_state.anchor_*`

Reserved future families include:

- `sensory.observe.*`
- `sensory.promote.*`
- `world_state.belief.*`
- `world_state.calibration.*`

## Durability Requirements

- canonical records are append only
- replay must rebuild reducer projections after restart
- correction happens through later events, not in-place mutation
- session cleanup must not delete canonical spine history
- compatibility readers must keep old stored envelopes readable

## Ingress Requirements

- concurrent producers are allowed
- canonical ingress must be bounded
- shutdown and cancellation behavior must be explicit
- one slow consumer must not block append correctness

## Subscription Requirements

- reducers must resume from durable sequence
- downstream consumers may follow live or catch up from durable history
- slow downstream consumers must recover through replay or separate projection state
- diagnostics must not become the sole replay source

## Cross-Domain References

Cross-domain payloads use `DomainObjectRef`:

```rust
struct DomainObjectRef {
    domain_id: String,
    object_kind: String,
    object_id: String,
}
```

Typed graph edges use `EventRelation`:

```rust
struct EventRelation {
    relation_type: String,
    src: DomainObjectRef,
    dst: DomainObjectRef,
}
```

No domain may assume `NodeID` is the universal identity anchor.

## Acceptance Conditions

The spine requirements are satisfied when all of these remain true:

- one canonical envelope is shared
- runtime-wide sequence is explicit
- idempotent derived fact append is explicit
- typed event families are owned by domains
- reducer ownership of state transition is explicit
- replay can rebuild required projections
- telemetry remains downstream of the spine
- graph attachment uses explicit objects and relations
