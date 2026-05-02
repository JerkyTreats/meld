# Graph ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/graph` as a reducer-heavy substrate

## Thesis

`graph` is the most system-heavy and least perspective-sensitive world model domain.

Its job is not to interpret trust, causality, or action.
Its job is to reduce shared facts into current anchors, lineage, provenance, adjacency, and traversal surfaces.

If ECS is used internally, `graph` should use it for reduction and projection state, while public reads remain graph-shaped queries.

## Entities

The core graph entities should be:

- `WorldObject`
  stable object identity keyed by `DomainObjectRef`
- `Anchor`
  one selected current pointer for a subject and perspective
- `RelationEdge`
  one typed adjacency between world objects
- `LineageRecord`
  one supersession or replacement relation between anchors
- `ProvenanceRecord`
  one explanation bundle for why an anchor or relation is current
- `BranchPresence`
  one object presence record scoped to a branch

## Components

The most useful graph components are:

- object identity
- subject ref
- target ref
- perspective key
- relation type
- validity interval
- transaction time
- reference time
- branch membership
- current status
- lineage parent ref
- provenance fact refs
- traversal index cache

## Systems

The core graph systems should be:

- fact lowering
  turn spine facts into graph-relevant intents
- anchor selection
  choose the current target for one subject and perspective
- lineage update
  record what an anchor superseded
- provenance projection
  attach the fact bundle that made a graph state current
- adjacency projection
  maintain relation neighborhoods
- traversal projection
  maintain bounded-walk ready indexes
- branch federation
  preserve branch-local presence and reads
- derived graph publication
  publish idempotent derived anchor facts back to the spine

## Role In The Set

`graph` should do shared structural work once for all consumers.

`belief`, `causation`, `regime`, `planner`, and `agent` should consume graph outputs, not rebuild graph structure per perspective.

## Read With

- [Graph](README.md)
- [World Model Domain](../README.md)
