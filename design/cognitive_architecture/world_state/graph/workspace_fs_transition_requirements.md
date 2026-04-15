# Workspace FS Graph Transition Requirements

Date: 2026-04-13
Status: active
Scope: compatibility-led requirements for lifting `workspace_fs` into graph-capable `world_state/graph` inputs without breaking the existing filesystem model

## Thesis

`workspace_fs` must become a first-class input domain to `world_state/graph`.

That change must not replace the existing filesystem model in one cut.
It must preserve the current `NodeID` and path-based workspace behavior while adding a canonical lift into the spine and knowledge graph.

The core rule is simple:

- `NodeID` remains the local structural identity of `workspace_fs`
- `DomainObjectRef` becomes the cross-domain identity surface
- `world_state/graph` consumes lifted workspace facts and materializes current anchors from them

## Why This Change Exists

The current execution-fed graph slice proves that the spine and `world_state` can materialize replayable traversal state.
It does not yet make the actual workspace a first-class source of traversal truth.

Today the repo still assumes that filesystem structure is the universal anchor in many places.
That assumption is visible in:

- [src/types.rs](/home/jerkytreats/meld/src/types.rs)
  `NodeID` is the root structural identifier
- [src/context/frame.rs](/home/jerkytreats/meld/src/context/frame.rs)
  frame basis is keyed to `NodeID` and `FrameID`
- [src/heads.rs](/home/jerkytreats/meld/src/heads.rs)
  current frame heads are keyed by `NodeID`
- [src/workspace/commands.rs](/home/jerkytreats/meld/src/workspace/commands.rs)
  scan freshness, path resolution, and CLI targeting are all `NodeID` based
- [src/workspace/watch/runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
  watch emits legacy trace events, not canonical workspace facts
- [src/store/persistence.rs](/home/jerkytreats/meld/src/store/persistence.rs)
  durable node storage is keyed by raw `NodeID`
- [src/context/query/get.rs](/home/jerkytreats/meld/src/context/query/get.rs)
  context CLI reads still resolve to one `NodeID`
- [src/task/package/prepare.rs](/home/jerkytreats/meld/src/task/package/prepare.rs)
  workflow package targeting resolves to one `NodeID`

Those are all valid local behaviors for `workspace_fs`.
They become a problem only when they are treated as the system-wide anchor.

## Current State

The current graph slice already gives the repo these pieces:

- shared cross-domain object refs in [src/telemetry/contracts.rs](/home/jerkytreats/meld/src/telemetry/contracts.rs)
- graph-capable spine events in [src/telemetry/events.rs](/home/jerkytreats/meld/src/telemetry/events.rs)
- deterministic `world_state` materialization in [src/world_state/reducer.rs](/home/jerkytreats/meld/src/world_state/reducer.rs)
- indexed graph query surfaces in [src/world_state/query.rs](/home/jerkytreats/meld/src/world_state/query.rs)

What is still missing is canonical publication from `workspace_fs` itself.
Right now `workspace_fs` appears in the graph only when `execution` events happen to name a workspace node.

## Migration Goal

At the end of this change, the repo should support this layered model:

- `workspace_fs` owns filesystem structure and observation of workspace state
- `workspace_fs` publishes canonical workspace facts into the spine
- `world_state/graph` reduces those facts into current anchors
- execution, context, and workflow code continue to resolve and use `NodeID` while graph readers use `DomainObjectRef`

The first landing should make `workspace_fs` graph-capable without requiring a repo-wide replacement of `NodeID`.

## Non Breaking Rule

This transition must be additive.

The following existing behaviors must continue to work exactly as they do now:

- `meld init`
- `meld scan`
- path to node resolution
- direct `NodeID` CLI targeting
- head lookup by `NodeID`
- frame storage and replay
- workflow package targeting by path or `NodeID`
- watch-driven queueing and frame generation

The graph lift is successful only if old workspace flows remain valid while new canonical workspace facts become available to the spine and `world_state`.

## Domain Architecture Rules

This change must follow these boundaries:

- new runtime code for workspace fact publication lives under `src/workspace/`
- new runtime code for traversal materialization lives under `src/world_state/`
- cross-domain object contracts remain under `src/telemetry/contracts.rs`
- `world_state` must not reach into `workspace` internals
- workspace publication must go through public spine contracts
- no `mod.rs`
- compatibility wrappers come before any removal of old paths

## Canonical Identity Requirements

`NodeID` remains a valid local identity for filesystem structure.
It must gain a canonical cross-domain wrapper, not a replacement.

Required durable object refs:

- `workspace_fs:source`
  stable identity of one tracked workspace root
- `workspace_fs:snapshot`
  one observed tree state for that source
- `workspace_fs:node`
  one filesystem node represented by a `NodeID`
- `context:frame`
  one context frame

The minimum rule for this slice is:

- every canonical workspace fact must carry at least one `workspace_fs:source`
- node-specific facts must also carry the relevant `workspace_fs:node`
- snapshot facts must carry `workspace_fs:snapshot`

`NodeID` should be encoded into `DomainObjectRef.object_id` for `workspace_fs:node` as hex.

## Workspace Fact Families

The first canonical workspace fact families should be narrow and structural.

Required families:

- `workspace.source_attached`
- `workspace.scan_started`
- `workspace.snapshot_materialized`
- `workspace.scan_completed`
- `workspace.node_observed`
- `workspace.node_tombstoned`
- `workspace.node_restored`

Optional in a later slice:

- `workspace.watch_started`
- `workspace.batch_detected`
- `workspace.path_rebound`

The first slice should avoid raw per-file watch noise as canonical facts.
The watch runtime may still emit telemetry traces, but canonical workspace facts should reflect promoted structural outcomes.

## Fact Semantics

Each canonical workspace fact must be explicit about:

- source object refs
- node object refs if relevant
- optional snapshot object ref
- relation edges
- occurred time if known
- recorded time

Minimum relation types:

- `attached_to`
- `contains`
- `observed_in`
- `supersedes`

Example shape for one node observation:

```rust
ProgressEnvelope::new(
    session_id,
    "execution",
    "workspace.node_observed",
    payload,
)
```

The event family naming above should use the owning workspace domain in real code.
The exact domain string should be frozen before implementation.
The important requirement is domain ownership plus explicit object refs.

## Compatibility Requirements For Scan

Current scan behavior in [src/workspace/tooling.rs](/home/jerkytreats/meld/src/workspace/tooling.rs) and [src/workspace/commands.rs](/home/jerkytreats/meld/src/workspace/commands.rs) must remain valid.

The new requirements are:

- `scan_started` telemetry may remain for UX
- canonical workspace facts must be emitted in addition to legacy scan traces
- one scan must produce one stable `workspace_fs:source`
- one completed scan must produce one `workspace_fs:snapshot`
- root hash changes must map to new snapshot identity, not new source identity
- a brand new scan must be treated as source attachment plus first observation, not as a special case outside the temporal model

## Compatibility Requirements For Watch

Current watch behavior in [src/workspace/watch/runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs) must remain valid for batching and frame generation.

The new requirements are:

- watch may keep emitting trace style events for progress and debugging
- canonical workspace facts should be published only after promotion into stable structural outcomes
- repeated noisy file changes must not flood the canonical spine
- canonical publication should happen at batch or snapshot boundaries, not for every raw watcher pulse

## Context And Head Requirements

The graph lift must not break frame addressing.

Current code in [src/context/frame.rs](/home/jerkytreats/meld/src/context/frame.rs), [src/heads.rs](/home/jerkytreats/meld/src/heads.rs), and [src/context/query/get.rs](/home/jerkytreats/meld/src/context/query/get.rs) depends on `NodeID`.

Required posture:

- keep `Basis::Node` and `Basis::Both` intact for this change
- keep head lookup keyed by `NodeID`
- add graph-visible refs and relations instead of replacing frame basis
- allow `world_state/graph` anchors to attach to both `workspace_fs:node` and `context:frame`

This preserves current frame and head behavior while making current anchors discoverable in the graph.

## Storage Requirements

Current node persistence in [src/store/persistence.rs](/home/jerkytreats/meld/src/store/persistence.rs) is keyed by raw `NodeID` and path mapping.

The first graph-capable workspace slice must not rewrite that store.
Instead it must add canonical workspace fact publication beside it.

Required rule:

- node store remains the local source of structural workspace state
- spine publication becomes the temporal source for cross-domain workspace facts
- `world_state/graph` derives traversal state from spine facts, not by reaching into node storage

## API And CLI Requirements

Current CLI and API entrypoints must keep accepting path and `NodeID`.

Affected code includes:

- [src/workspace/commands.rs](/home/jerkytreats/meld/src/workspace/commands.rs)
- [src/context/query/get.rs](/home/jerkytreats/meld/src/context/query/get.rs)
- [src/task/package/prepare.rs](/home/jerkytreats/meld/src/task/package/prepare.rs)

Required posture:

- user-facing targeting remains path and `NodeID` based in this slice
- internal canonical publication adds `DomainObjectRef`
- no user-facing command should require graph ids yet

## Concurrency And Locking Requirements

Current locking in [src/concurrency.rs](/home/jerkytreats/meld/src/concurrency.rs) is keyed by `NodeID`.

That should remain unchanged in this slice.
The graph transition is about publication and traversal, not runtime lock identity.

Any attempt to switch locks to graph ids in the same change should be rejected.

## Required Implementation Phases

### Phase 0

Freeze the canonical workspace fact taxonomy, domain id, object ref shapes, and compatibility rules in design docs.

### Phase 1

Add workspace event builders under `src/workspace/` that produce canonical envelopes with object refs and relations.

### Phase 2

Wire `scan` to emit canonical workspace facts beside existing telemetry.

### Phase 3

Wire watch promotion to emit canonical structural workspace facts at stable batch boundaries.

### Phase 4

Add `world_state/graph` reducers that consume canonical workspace facts and materialize workspace-backed anchors.

### Phase 5

Add graph queries that let one claim traverse to its supporting workspace source, snapshot, node, and frame refs.

## Acceptance Gates

The change is complete only when all of these are true:

- old `init` and `scan` flows still work without graph-specific user input
- one brand new scan emits canonical workspace facts that can replay after restart
- one repeated scan of the same root keeps source identity stable and creates a new snapshot only when structural content changes
- one `workspace_fs:node` can be queried through `DomainObjectRef` without direct node store access
- one world state anchor can attach to both a `workspace_fs:node` and a `context:frame`
- deleting traversal projections and replaying the spine rebuilds the same workspace-backed traversal state
- watch noise does not become one canonical fact per raw watcher pulse

## Code Review Touchpoints

These files must be reviewed in any implementation branch for this change:

- [src/types.rs](/home/jerkytreats/meld/src/types.rs)
  confirms `NodeID` remains local workspace identity
- [src/context/frame.rs](/home/jerkytreats/meld/src/context/frame.rs)
  basis remains `NodeID` oriented in this slice
- [src/heads.rs](/home/jerkytreats/meld/src/heads.rs)
  head index remains keyed by `NodeID`
- [src/workspace/tooling.rs](/home/jerkytreats/meld/src/workspace/tooling.rs)
  entrypoint for scan command traces
- [src/workspace/commands.rs](/home/jerkytreats/meld/src/workspace/commands.rs)
  root hash freshness, node resolution, and scan flow
- [src/workspace/watch/runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
  watch batch promotion boundary
- [src/store/persistence.rs](/home/jerkytreats/meld/src/store/persistence.rs)
  local node store remains intact
- [src/context/query/get.rs](/home/jerkytreats/meld/src/context/query/get.rs)
  CLI context read compatibility
- [src/task/package/prepare.rs](/home/jerkytreats/meld/src/task/package/prepare.rs)
  workflow package targeting compatibility
- [src/init.rs](/home/jerkytreats/meld/src/init.rs)
  confirms `init` remains unrelated to graph-required workspace migration

## Explicit Rejections

Reject these moves in the first `workspace_fs` graph slice:

- replacing `NodeID` with graph ids in existing CLI and API surfaces
- rewriting frame basis away from `NodeID`
- rewriting head storage around graph ids
- publishing raw watcher pulses as canonical facts
- making `world_state` read node storage directly for truth
- forcing workflow and task packaging to target graph ids

## Recommendation

Implement the `workspace_fs` lift as a publication and reduction change, not as an identity replacement change.

That gives the repo a true path from filesystem structure into the traversal graph while preserving the existing workspace model and avoiding a breaking rewrite.

## Read With

- [World State Domain](../README.md)
- [Traversal](README.md)
- [Temporal Fact Graph](temporal_fact_graph.md)
- [Traversal Implementation Plan](implementation_plan.md)
- [Multi-Domain Spine](../../events/multi_domain_spine.md)
- [Spine Concern](../../spine/README.md)
