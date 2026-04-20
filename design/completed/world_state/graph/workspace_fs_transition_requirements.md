# Workspace FS Graph Transition Status

Date: 2026-04-20
Status: implemented first slice
Scope: compatibility-led lift of `workspace_fs` into graph-capable `world_state/graph` inputs without breaking the existing filesystem model

## Thesis

`workspace_fs` is now a first-class input domain to `world_state/graph`.

The change preserved the existing filesystem model.
`NodeID` and path-based workspace behavior remain local workspace identity, while `DomainObjectRef` is the cross-domain graph identity surface.

The core rule remains:

- `NodeID` remains the local structural identity of `workspace_fs`
- `DomainObjectRef` is the cross-domain identity surface
- `world_state/graph` consumes lifted workspace facts and materializes current anchors from them

## Implemented State

The repo now has:

- canonical workspace fact builders in [src/workspace/events.rs](/home/jerkytreats/meld/src/workspace/events.rs)
- scan publication in [src/workspace/commands.rs](/home/jerkytreats/meld/src/workspace/commands.rs)
- watch batch publication in [src/workspace/watch/runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
- workspace reducer intents in [src/workspace/reducer.rs](/home/jerkytreats/meld/src/workspace/reducer.rs)
- traversal materialization in [src/world_state/graph](/home/jerkytreats/meld/src/world_state/graph.rs)
- scan and watch verification in [tests/integration/workspace_traversal.rs](/home/jerkytreats/meld/tests/integration/workspace_traversal.rs) and workspace watch unit tests

## Preserved Local Identity

The graph lift did not replace local workspace identity.

The following remain `NodeID` and path oriented:

- `meld init`
- `meld scan`
- path to node resolution
- direct `NodeID` CLI targeting
- head lookup by `NodeID`
- frame storage and replay
- workflow package targeting by path or `NodeID`
- watch-driven queueing and frame generation
- node locks

The graph lift adds publication and traversal beside those paths.

## Canonical Workspace Object Refs

Implemented object refs:

- `workspace_fs:source`
  stable identity of one tracked workspace root
- `workspace_fs:snapshot`
  one observed tree state for that source
- `workspace_fs:snapshot_head`
  current snapshot selector for a workspace source
- `workspace_fs:node`
  one filesystem node represented by a `NodeID`

`workspace_fs:node` stores `NodeID` as hex in `DomainObjectRef.object_id`.

`workspace_fs:source` stores the normalized canonical workspace path.

## Implemented Fact Families

Implemented canonical workspace fact families:

- `workspace_fs.source_attached`
- `workspace_fs.scan_started`
- `workspace_fs.scan_completed`
- `workspace_fs.snapshot_materialized`
- `workspace_fs.snapshot_selected`
- `workspace_fs.node_observed`

Watch still emits trace style events for progress and debugging, but promoted structural outcomes publish the same canonical workspace fact families as scan.

## Implemented Relations

Workspace events publish explicit graph relations:

- `belongs_to`
- `attached_to`
- `selected`
- `supersedes`
- `observed_in`
- `contains`

The reducer uses `workspace_fs.snapshot_selected` to select the current `workspace_fs:snapshot_head` anchor for a source.

## Compatibility Posture

The node store remains the local source of structural workspace state.

The spine is the temporal source for cross-domain workspace facts.

`world_state/graph` derives traversal state from spine facts, not by reaching into node storage for authority.

Frame basis and head lookup remain `NodeID` oriented.
Graph-visible refs and relations make current anchors discoverable without replacing frame addressing.

## Acceptance Status

Implemented and covered:

- old `init` and `scan` flows still work without graph-specific user input
- brand new scan emits canonical workspace facts
- repeated scan reuses source identity
- new snapshot selection happens only when the root hash changes
- watch batch promotion emits canonical workspace facts
- watch does not publish one canonical fact per raw watcher pulse
- current snapshot for a workspace source is queryable through traversal
- replay rebuilds workspace-backed traversal state

Still deferred:

- node tombstone and restore fact families
- path rebound fact family
- user-facing commands that target graph ids directly
- replacing local lock identity with graph ids

## Code Review Touchpoints

Review these files when changing the workspace graph lift:

- [src/types.rs](/home/jerkytreats/meld/src/types.rs)
  confirms `NodeID` remains local workspace identity
- [src/context/frame.rs](/home/jerkytreats/meld/src/context/frame.rs)
  basis remains `NodeID` oriented
- [src/heads.rs](/home/jerkytreats/meld/src/heads.rs)
  head index remains keyed by `NodeID`
- [src/workspace/commands.rs](/home/jerkytreats/meld/src/workspace/commands.rs)
  root hash freshness, node resolution, scan flow, and workspace fact publication
- [src/workspace/watch/runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
  watch batch promotion boundary and canonical publication
- [src/workspace/events.rs](/home/jerkytreats/meld/src/workspace/events.rs)
  canonical workspace object and relation shape
- [src/workspace/reducer.rs](/home/jerkytreats/meld/src/workspace/reducer.rs)
  reducer intents for workspace snapshot anchors
- [src/store/persistence.rs](/home/jerkytreats/meld/src/store/persistence.rs)
  local node store remains intact
- [src/context/query/get.rs](/home/jerkytreats/meld/src/context/query/get.rs)
  CLI context read compatibility
- [src/task/package/prepare.rs](/home/jerkytreats/meld/src/task/package/prepare.rs)
  workflow package targeting compatibility

## Explicit Rejections

Still reject these moves in this slice:

- replacing `NodeID` with graph ids in existing CLI and API surfaces
- rewriting frame basis away from `NodeID`
- rewriting head storage around graph ids
- publishing raw watcher pulses as canonical facts
- making `world_state` read node storage directly for truth
- forcing workflow and task packaging to target graph ids

## Read With

- [World State Domain](../../../cognitive_architecture/world_state/README.md)
- [Graph](../../../cognitive_architecture/world_state/graph/README.md)
- [Temporal Fact Graph](../../../cognitive_architecture/world_state/graph/temporal_fact_graph.md)
- [Graph Implementation Status](implementation_plan.md)
- [Multi-Domain Spine](../../../cognitive_architecture/events/multi_domain_spine.md)
- [Spine Concern](../../../cognitive_architecture/spine/README.md)
