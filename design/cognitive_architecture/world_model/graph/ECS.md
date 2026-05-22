# Graph ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/graph` as a reducer-heavy substrate

## Thesis

`graph` is the most system-heavy and least perspective-sensitive world model domain.

Its job is not to interpret trust, causality, or action.
Its job is to reduce shared facts into current anchors, lineage, provenance, adjacency, and traversal surfaces.

If ECS is used internally, `graph` uses it for reduction and projection state, while public reads remain graph-shaped queries.

Detailed graph entity, component, system, and requirement definitions live in:

- [Graph Entities](entities.md)
- [Graph Components](components.md)
- [Graph Systems](systems.md)
- [Graph Requirements](requirements.md)

## Entities

The core graph entities are:

- `TraversalFact`
  one graph-readable spine fact lowered into traversal state
- `WorldObject`
  stable object identity keyed by `DomainObjectRef`
- `Anchor`
  one selected current pointer for a subject and perspective
- `RelationEdge`
  one typed adjacency between world objects
- `ObjectHistory`
  one ordered fact history for a world object
- `AnchorLineage`
  one supersession or replacement relation between anchors
- `ProvenanceBundle`
  one explanation bundle for why an anchor or relation is current
- `BranchPresence`
  one object presence record scoped to a branch
- `GraphWalk`
  one bounded traversal query product
- `NeighborSet`
  one relation-neighbor query product
- `GraphReductionCursor`
  one reducer progress record
- `DerivedGraphFact`
  one graph-owned derived fact prepared for idempotent spine publication

## Components

The component families are:

- identity
- source fact
- object
- anchor
- relation
- lineage
- provenance
- branch
- traversal
- runtime

## Systems

The core graph systems are:

- graph catch-up
  read source events after reducer cursor
- fact admission
  admit graph-readable events
- fact lowering
  turn spine facts into graph-relevant intents
- traversal fact recording
  persist graph-readable source facts
- object indexing
  maintain object membership and object history
- relation indexing
  maintain incoming and outgoing relation indexes
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
- reducer cursor commit
  advance graph replay boundary after durable writes
- recovery and rebuild
  rebuild graph projections from spine history

## Role In The Set

`graph` does shared structural work once for all consumers.

`belief`, `causation`, `regime`, `planner`, and `agent` consume graph outputs instead of rebuilding graph structure per perspective.

## Read With

- [Graph](README.md)
- [Graph Entities](entities.md)
- [Graph Components](components.md)
- [Graph Systems](systems.md)
- [Graph Requirements](requirements.md)
- [World Model Domain](../README.md)
