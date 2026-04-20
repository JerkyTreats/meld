# Multi-Domain Spine

Date: 2026-04-20
Status: active
Scope: shared temporal ledger across cognitive domains

## Intent

Define how many domains share one temporal spine without sharing one ownership model.

The spine is the shared history ledger.
Object stores, reducers, reference projections, and query surfaces remain domain-owned.

## Problem

Cross-domain temporal questions require a shared clock.

Examples:

- what workspace snapshot was current when a task emitted an artifact
- what frame head was current when a workflow turn completed
- what graph anchor was selected after an execution outcome
- what belief was current when a planner chose an action

Without runtime-wide spine sequence and cross-domain object refs, these questions collapse into ad hoc joins across local clocks.

## Domain Concerns

Every domain has three concerns:

- objects
  durable objects owned by that domain
- references
  named current-state pointers or projections
- history
  ordered semantic facts in the spine

The spine stores history.
Domains own objects and references.

## Known Domains

### workspace_fs

The workspace filesystem graph.

Semantic facts include:

- source attached
- scan started
- scan completed
- snapshot materialized
- snapshot selected
- node observed

### context

Frames, frame heads, and prompt context artifacts.

Semantic facts include:

- frame added
- head selected
- prompt context lineage where promoted

### execution

Planning, task progression, workflow turns, repair, action outcomes, and artifacts.

Semantic facts include:

- task requested
- task started
- task artifact emitted
- task failed
- control node completed
- control node failed
- workflow turn completed

### world_state

Graph anchors, graph traversal materialization, and future belief records.

Semantic facts include:

- anchor selected
- anchor superseded
- claim added
- evidence attached
- future belief revised
- future calibration recorded

### sensory

Promoted observations from lowered sensory pipelines.

Raw high-rate lanes do not publish directly into the spine.
Only promoted semantic observations enter the spine.

## Envelope Anchors

The spine envelope carries:

- runtime-wide `seq`
- owning `domain_id`
- `stream_id`
- `event_type`
- optional `record_id`
- optional `content_hash`
- explicit `objects`
- explicit `relations`

`domain_id` enables per-domain filtering and projection without parsing `event_type`.

`stream_id` anchors one task run, workspace source, branch, belief stream, or sensory lane.

`content_hash` links an event to content-addressed data when one primary object exists.

## DomainObjectRef

Cross-domain references use a stable identity type that is not tied to any one domain internal identifier.

```rust
struct DomainObjectRef {
    domain_id: String,
    object_kind: String,
    object_id: String,
}
```

A workspace node becomes:

```rust
DomainObjectRef { domain_id: "workspace_fs", object_kind: "node", object_id: "..." }
```

A task run becomes:

```rust
DomainObjectRef { domain_id: "execution", object_kind: "task_run", object_id: "..." }
```

A world-state entity becomes:

```rust
DomainObjectRef { domain_id: "world_state", object_kind: "entity", object_id: "..." }
```

Cross-domain edges use typed `EventRelation` values between `DomainObjectRef` values.

## Temporal Queries

With `domain_id`, `stream_id`, `DomainObjectRef`, relations, and runtime-wide `seq`, cross-domain temporal queries become replayable:

- find the source event
- take its sequence
- query the latest relevant projection at or before that sequence
- hydrate provenance through object refs and relations

The spine sequence is the shared clock.

## Batch Facts

Large domains may publish compact batch facts.

A batch fact should preserve:

- source object
- summary counts
- content hash or artifact ref for detailed data
- relation edges needed for graph traversal
- sequence position for replay

The detailed payload may live in a domain-owned object store.
The spine event stays compact.

## Genesis Facts

When a domain joins the spine with existing state, a genesis fact may capture the baseline.

After genesis, subsequent mutations emit delta facts.
Replaying genesis plus later deltas reconstructs that domain projection.

## Read With

- [Event Spine Requirements](event_manager_requirements.md)
- [World State Domain](../world_state/README.md)
- [Graph](../world_state/graph/README.md)
- [Execution Control](../execution/control/README.md)
