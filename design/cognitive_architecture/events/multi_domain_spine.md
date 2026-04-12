# Multi-Domain Spine

Date: 2026-04-12
Status: proposed
Scope: north star for extending the first execution spine into one shared temporal ledger across cognitive domains

## Intent

Define how the first spine grows beyond `execution`.

The immediate refactor lands one real spine for `execution`.
This document constrains that work so `sensory`, `world_state`, and attached object domains can join later without another envelope reset.

## The Problem

The application currently has one implicit domain: the workspace filesystem.
`NodeID` traces to filesystem paths. `FrameID` basis traces to `NodeID`.
Everything anchors there.

Adding a second filesystem, attaching task execution records as a first-class queryable
structure, or representing conceptual entities like "Core Thesis linked to contextual facts
and decision points" all require a shared temporal anchor that the current model cannot provide.

Without a shared anchor, cross-domain temporal queries are impossible:
"what was the filesystem state when this task succeeded?" has no answer because the two
domains do not share a clock or a sequence.

## Every Domain Reduces to Three Concerns

Regardless of what a domain represents, it has three concerns:

- **Objects**: content-addressed blobs. Each domain has its own object store.
  Key: `Blake3Hash → bytes`. The bytes live exactly once.
- **References**: named current-state pointers. Each domain maintains its own projection.
  Examples: `HeadIndex`, task network state, knowledge graph current view.
- **History**: ordered mutations. All domains share the spine.
  The spine records when each mutation happened and what it touched.

The spine is the shared history ledger. Object stores and reference projections are
domain-owned. This is the hub-and-spoke model.

## Known Domains

### workspace_fs

The current filesystem merkle tree.

Spine events: `workspace_scan_completed`, `workspace_node_changed`, `workspace_node_removed`.
Object store: frame storage keyed by `FrameID`.
Reference projection: `HeadIndex` over node and frame type.

The lift from the current model is simple:
filesystem mutations that are currently implicit during scan become explicit spine events with a `domain_id` of `workspace_fs`.
The merkle tree hash per scan becomes a spine event payload field.
Current state remains a projection.

### execution

Execution facts from planning, task progression, repair, and action outcome publication.

The first spine refactor targets this domain.
The first event families are `execution.task.*`, `execution.control.*`, `execution.repair.*`, and `execution.artifact.*`.

Object store: task invocation records, execution artifacts, and related content-addressed blobs.
Reference projection: task-network state and other execution projections.

This domain already exists partially in code.
The first refactor should tag all canonical facts with `domain_id = "execution"` from day one.

### world_state

Curated claims, belief revisions, calibration records, and world-model projections.

This domain is not landing in the first spine refactor.
Its inclusion here constrains the envelope and reference model now.

Spine events: `world_state.claim_added`, `world_state.belief_revised`, `world_state.claim_superseded`, `world_state.calibration_recorded`.
Object store: evidence blobs, claim payloads, and projection snapshots where needed.
Reference projection: current belief view and knowledge graph materialization.

### sensory

Promoted observations from lowered sensory pipelines.

This domain does not publish raw high-rate lanes into the spine.
It publishes promoted semantic observations only.

Spine events: `sensory.observe.promoted`, `sensory.observe.retracted`, `sensory.observe.window_closed`.
Object store: optional observation summaries or referenced evidence blobs.
Reference projection: local observation windows or promoted observation indexes.

## Required Envelope Fields

The first spine envelope should include four durable identity anchors:

```rust
struct SpineEvent {
    ts: String,
    session: String,
    seq: u64,
    domain_id: DomainId,
    stream_id: String,
    event_type: String,
    content_hash: Option<Blake3Hash>,
    data: serde_json::Value,
}
```

`domain_id` enables per-domain filtering and projection without parsing `event_type`.

`stream_id` anchors one task run, workspace root, belief stream, or sensory lane.

`content_hash` enables content-addressed lookup from an event reference without loading the full artifact.
It is optional because not every event touches one primary content object.

These fields must be present from the first spine refactor commit.
Adding them later would force another stored event migration.

## DomainObjectRef

Cross-domain references require a stable identity type that is not tied to any one domain's
internal identifier scheme.

```rust
struct DomainObjectRef {
    domain_id: DomainId,
    object_kind: ObjectKind,
    object_id: String,
}
```

A `NodeID` becomes:
```
DomainObjectRef { domain_id: "workspace_fs", object_kind: "node", object_id: ... }
```

A task run becomes:
```
DomainObjectRef { domain_id: "execution", object_kind: "task_run", object_id: ... }
```

A world-state entity becomes:
```
DomainObjectRef { domain_id: "world_state", object_kind: "entity", object_id: ... }
```

Cross-domain edges in the knowledge graph are typed references between `DomainObjectRef` values.
The spine records when those references were created.
World-state nodes that reference execution outcomes carry `DomainObjectRef` values that point back to the execution stream.

`DomainObjectRef` does not need full adoption in the first spine refactor.
It does need to exist before the world-state graph grows into broader cross-domain references.
The practical constraint is simple:
new payload schemas should stop assuming that `NodeID` is the universal identity anchor.

## Temporal Queries Across Domains

With `domain_id`, `stream_id`, and runtime-wide `seq`, cross-domain temporal queries become straightforward:

**"What was the workspace state when task T completed?"**

1. Find the `execution.task.succeeded` event for task T in the spine. Its sequence is S.
2. Find the latest `workspace_scan_completed` event for the relevant workspace with seq ≤ S.
3. That event's payload carries the merkle tree hash representing the workspace state at that time.

**"What world-state belief was in effect when frame F was written?"**

1. Find the `frame_written` event for frame F. Its sequence is S.
2. Find the latest `world_state.belief_revised` event for the relevant belief stream with seq ≤ S.
3. Resolve the referenced world-state object or projection snapshot.

These queries do not require joins across systems with different clocks. The spine sequence
is the shared clock.

## Batch Events for Bulk Operations

A workspace scan may change thousands of nodes. Emitting one spine event per node is
impractical under the current eager flush model and may be impractical even under a batched
model for large workspaces.

The solution is batch events:

```json
{
  "domain_id": "workspace_fs",
  "stream_id": "workspace::<root>",
  "event_type": "workspace_scan_batch",
  "content_hash": "hash_of_new_merkle_root",
  "data": {
    "workspace_id": "...",
    "previous_merkle_root": "...",
    "new_merkle_root": "...",
    "changed_node_count": 47,
    "change_summary_ref": "hash_of_detailed_diff_blob"
  }
}
```

The detailed diff that lists changed nodes plus old and new hashes is stored as a content-addressed blob referenced by `change_summary_ref`.
The spine event stays compact.
Consumers that need the full diff load it from the content store.

## Genesis Events for Bootstrapping

When a domain is first attached to the spine, its current state predates the spine.
A genesis event captures the baseline:

```json
{
  "domain_id": "workspace_fs",
  "stream_id": "workspace::<root>",
  "event_type": "workspace_domain_genesis",
  "content_hash": "hash_of_initial_merkle_root",
  "data": {
    "workspace_id": "...",
    "initial_merkle_root": "...",
    "node_count": 312,
    "genesis_reason": "spine_attach"
  }
}
```

After genesis, all subsequent mutations emit delta events. Replaying the genesis event
plus all subsequent delta events reconstructs the full domain state.

## Constraint On The Immediate Refactor

The [Event Spine Refactor](telemetry_refactor.md) is the concrete next work.
Its scope is the `execution` domain only.

The constraints this document imposes on that work are narrow:

1. Add `domain_id`, `stream_id`, and optional `content_hash` to the spine envelope.
2. Default all first-slice canonical facts to `domain_id = "execution"`.
3. Make `seq` runtime wide from the first real spine commit.
4. Do not assume `NodeID` is the universal identity anchor in new payload schemas.
5. Keep raw sensory lanes outside the canonical spine.

Everything else in the first refactor may stay scoped to execution.
The hub-and-spoke model is the north star, not the immediate implementation target.

## Read With

- [Event Spine Requirements](event_manager_requirements.md)
- [Event Spine Refactor](telemetry_refactor.md)
- [Event Management Research](research.md)
- [Execution Control](../execution/control/README.md)
