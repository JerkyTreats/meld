# Graph

Date: 2026-04-20
Status: active
Scope: current anchor selection, lineage, provenance, and cross-domain graph walk inside `world_state`

## Thesis

`graph` answers what is current and how to reach it.

The graph contract is:

- the spine carries `DomainObjectRef` objects and `EventRelation` edges
- graph materialization derives current anchors and traversal views from spine history
- anchors are selected for a subject and perspective
- lineage records what an anchor replaced
- provenance records which facts made an anchor current
- traversal exposes bounded object walks and relation adjacency
- branch scoped reads preserve branch identity and presence

The job of `world_state/graph` is now to keep this materialization replayable, index backed, and narrow enough for belief to build on later.

## What Graph Owns

- current anchor selection
- anchor lineage
- provenance bundles
- cross-domain object walk through `DomainObjectRef`
- replayable graph materialization from spine facts

## What Graph Does Not Own

- confidence in whether the current anchor is still correct
- contradiction handling
- calibration
- Bayesian revision

Those belong to `world_state/belief`.

## Current Operational Truth

The current operational graph already exists in these forms:

- `workspace_fs` node identity
- frame basis attachment
- head selection by node and frame type
- task and capability work that advances or preserves those heads

The lift into explicit spine facts and traversal indexes is now baseline.

## Baseline Scope

The graph baseline stops at graph materialization and traversal.

It includes:

- canonical event spine publication with explicit object refs and relations
- workspace and execution contribution to current anchor state
- replayable current anchor materialization
- query surfaces for current anchor, lineage, provenance, neighbors, and bounded walks
- durable derived anchor events written back to the spine by `GraphRuntime`
- branch annotated federation over traversal stores
- one workflow task path that resolves final frame artifacts through traversal

Belief work is deferred.

## Remaining Limits

- `world_state/belief` is outside the graph baseline
- confidence, contradiction handling, calibration, and curation are not graph responsibilities
- traversal only reduces source domains that publish explicit graph objects and relations
- graph indexes are sled backed materializations, not a general graph database
- branch federation preserves per-branch presence and provenance, but does not merge branch facts into one global authority

## Read With

- [World State Domain](../README.md)
- [Belief](../belief/README.md)
- [Temporal Fact Graph](temporal_fact_graph.md)
- [Branch Federation Substrate](branch_federation_substrate.md)
- [Completed Graph Implementation History](../../../completed/world_state/graph/README.md)
- [Spine Concern](../../spine/README.md)
