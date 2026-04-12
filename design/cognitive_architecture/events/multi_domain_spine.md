# Multi-Domain Spine

Date: 2026-04-08
Status: proposed
Scope: generalization of the event spine from a single-domain application log to a shared temporal ledger across heterogeneous object graph domains

## Intent

Define the architectural model for attaching multiple independent merkle-like object graphs
to a single temporal event spine.

The current application is anchored in a filesystem representation. The spine refactor
described in [Event Manager Requirements](event_manager_requirements.md) targets task execution
events from one domain. This document defines the north star that constrains how the spine
envelope is designed so that future domains can attach without schema migration.

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
Object store: frame storage (content-addressed by `FrameID`).
Reference projection: `HeadIndex[(node_id, frame_type)] → FrameID`.

The lift from the current model: filesystem mutations that are currently implicit (detected
on scan) become explicit spine events with a domain_id of `workspace_fs`. The merkle tree
hash per scan becomes a spine event payload field. Current state remains a projection.

### task_graph

Task execution records as an append-only chain.

Each capability invocation record hashes the prior invocation record plus its inputs and
outputs. The chain for one task run is: `task_created → cap1_started → cap1_failed →
cap1_retried → cap2_started → cap2_completed → task_expanded → cap3_started →
cap3_completed → task_completed`.

Spine events: `task_requested`, `task_started`, `capability_invoked`, `capability_failed`,
`task_expanded`, `task_succeeded`, `task_failed`.
Object store: capability invocation records and artifact blobs (content-addressed).
Reference projection: task network state (active, blocked, completed, failed sets).

This domain already exists partially. The spine refactor in
[Telemetry Refactor](telemetry_refactor.md) targets this domain. Adding `domain_id` to the
envelope from day one means task_graph events are already domain-tagged.

### knowledge_graph

Conceptual entities such as thesis nodes, contextual facts, and decision records.

A "Core Thesis" is a node. Linked "contextual facts" are nodes. Decision records reference
the task graph events that produced them. Edges are typed and have provenance (created at
spine sequence S).

This domain is not being built now. Its inclusion here constrains the envelope design:
`domain_id` must be a first-class field, and cross-domain references must be expressible
via `DomainObjectRef`.

Spine events: `knowledge_node_added`, `knowledge_edge_added`, `knowledge_node_superseded`.
Object store: knowledge graph node content (content-addressed).
Reference projection: knowledge graph current view (latest version of each node and its edges).

## Required Envelope Fields

The current envelope shape is close to correct. Two fields must be added:

```rust
struct SpineEvent {
    ts: String,
    session: String,
    seq: u64,
    domain_id: DomainId,               // NEW — which domain this event belongs to
    event_type: String,
    content_hash: Option<Blake3Hash>,  // NEW — hash of the primary object this event touches
    data: serde_json::Value,
}
```

`domain_id` enables per-domain filtering and projection without parsing `event_type`.

`content_hash` enables content-addressed lookup from an event reference without loading the
full artifact. It is `Option` because not every event touches a content-addressed object
(e.g., `dispatch_requested` events have no primary object).

These two fields must be present from the first spine refactor commit. Adding them later
requires a migration of all stored events.

## DomainObjectRef

Cross-domain references require a stable identity type that is not tied to any one domain's
internal identifier scheme.

```rust
struct DomainObjectRef {
    domain_id: DomainId,
    object_kind: ObjectKind,
    content_hash: Blake3Hash,
}
```

A `NodeID` becomes:
```
DomainObjectRef { domain_id: "workspace_fs", object_kind: "node", content_hash: ... }
```

A task run becomes:
```
DomainObjectRef { domain_id: "task_graph", object_kind: "task_run", content_hash: ... }
```

A knowledge graph thesis becomes:
```
DomainObjectRef { domain_id: "knowledge_graph", object_kind: "thesis", content_hash: ... }
```

Cross-domain edges in the knowledge graph are typed references between `DomainObjectRef`
values. The spine records when those references were created. The knowledge graph node that
references a task_graph event carries a `DomainObjectRef` pointing to that event.

`DomainObjectRef` does not need to be used in the first spine refactor. It needs to be
defined as a type before the knowledge_graph domain is built. The constraint it imposes
on the current design is: `NodeID` should not be assumed to be the universal identity
anchor in new code.

## Temporal Queries Across Domains

With `domain_id` and monotonic `seq` in every event, cross-domain temporal queries become
straightforward:

**"What was the workspace state when task T completed?"**

1. Find the `task_succeeded` event for task T in the spine. Its sequence is S.
2. Find the latest `workspace_scan_completed` event for the relevant workspace with seq ≤ S.
3. That event's payload carries the merkle tree hash representing the workspace state at that time.

**"What decision was in effect when frame F was written?"**

1. Find the `frame_written` event for frame F. Its sequence is S.
2. Find the latest `task_artifact_emitted` event for `decision_artifact` for the same node
   with seq ≤ S.
3. Resolve the `DecisionArtifact` content from the content store.

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

The detailed diff (which nodes changed, their old and new hashes) is stored as a content-
addressed blob referenced by `change_summary_ref`. The spine event is compact. Consumers
that need the full diff load it from the content store.

## Genesis Events for Bootstrapping

When a domain is first attached to the spine, its current state predates the spine.
A genesis event captures the baseline:

```json
{
  "domain_id": "workspace_fs",
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

## Constraint on the Immediate Refactor

The [Telemetry Refactor](telemetry_refactor.md) is the concrete next work.
Its scope is the task_graph domain only.

The constraint this document imposes on that work is narrow:

1. Add `domain_id` to the spine event envelope. Default value for all current events:
   `"task_graph"`.
2. Add `content_hash` as an optional field.
3. Do not assume `NodeID` is the universal identity type in new event payload schemas.

Everything else in the telemetry refactor proceeds as planned.
The hub-and-spoke model is the north star, not the immediate implementation target.

## Read With

- [Event Manager Requirements](event_manager_requirements.md)
- [Telemetry Refactor](telemetry_refactor.md)
- [Event Management Research](research.md)
- [Execution Control](../execution/control/README.md)
