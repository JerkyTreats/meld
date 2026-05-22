# Graph Entities

Date: 2026-05-15
Status: active
Scope: ECS entities for `world_model/graph`

## Thesis

Graph entities are stable identities for replayed facts, world objects, current anchors, relation edges, lineage, provenance, branch presence, traversal queries, and reducer state.

Graph entities do structural work.
They do not carry belief confidence, causal authority, regime identity, planner policy, or task dispatch semantics.

## Entity Categories

| Category | Entities | Persistence expectation |
|---|---|---|
| Source fact | `TraversalFact` | durable projection record |
| Object identity | `WorldObject` | durable projection record |
| Current state | `Anchor` | durable projection record |
| Relation index | `RelationEdge` | durable projection record |
| History | `ObjectHistory`, `AnchorLineage` | durable projection records |
| Explanation | `ProvenanceBundle` | durable projection record |
| Branch scope | `BranchPresence` | durable projection record |
| Query product | `GraphWalk`, `NeighborSet` | materialized or ephemeral projection |
| Runtime state | `GraphReductionCursor`, `DerivedGraphFact` | durable enough for replay and idempotence |

## Identity Rules

| Entity | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `TraversalFact` | source spine fact id plus graph reducer version | none | graph |
| `WorldObject` | canonical `DomainObjectRef` index key | none | graph |
| `Anchor` | anchor ref plus source sequence | `WorldObject` subject | graph |
| `RelationEdge` | source object plus relation type plus target object plus source fact id plus sequence | `TraversalFact` | graph |
| `ObjectHistory` | object ref plus source cursor range | `WorldObject` | graph |
| `AnchorLineage` | anchor id plus superseding anchor id | `Anchor` | graph |
| `ProvenanceBundle` | anchor id or traversal fact id plus provenance version | `Anchor` or `TraversalFact` | graph |
| `BranchPresence` | branch ref plus object ref plus source cursor | `WorldObject` | graph |
| `GraphWalk` | walk spec hash plus start object plus source cursor | `WorldObject` start | graph |
| `NeighborSet` | object ref plus direction plus relation filter plus source cursor | `WorldObject` | graph |
| `GraphReductionCursor` | reducer id | none | graph |
| `DerivedGraphFact` | derived fact id from graph event kind plus source record id | source graph record | graph and spine |

## `TraversalFact`

One graph-readable spine fact lowered into traversal state.

Purpose:
preserves event sequence, source fact id, object refs, relation refs, and event type for graph replay and hydration.

Required components:

- `TraversalFactIdentity`
- `TraversalFactSource`
- `TraversalFactObjects`
- `TraversalFactRelations`
- `TraversalFactCursor`

Creation:
created by fact lowering from spine events that publish graph-readable objects or relations.

Lifecycle:
recorded, indexed, replayed.

Dependencies:
event spine, `DomainObjectRef`, `EventRelation`, graph reducer version.

## `WorldObject`

One graph object keyed by `DomainObjectRef`.

Purpose:
provides shared object identity for anchors, relations, history, branch presence, and traversal.

Required components:

- `ObjectIdentity`
- `ObjectPresence`
- `ObjectHistoryIndex`

Creation:
created when a traversal fact mentions a domain object.

Lifecycle:
present, absent in branch, archived by retention policy.

Dependencies:
`DomainObjectRef`, traversal fact membership index, branch presence records.

## `Anchor`

One selected current pointer for a subject and perspective.

Purpose:
records which target is current for an anchor ref under a perspective and how that current pointer superseded prior anchors.

Required components:

- `AnchorIdentity`
- `AnchorScope`
- `AnchorTarget`
- `AnchorCurrentState`
- `AnchorSourceFacts`

Creation:
created by anchor selection from graph source intents.

Lifecycle:
current, ended, superseded.

Dependencies:
source fact, anchor ref, subject ref, perspective key, target ref.

## `RelationEdge`

One typed adjacency between world objects.

Purpose:
indexes relation direction, relation type, source and target refs, source fact id, and sequence for neighbor and walk queries.

Required components:

- `RelationIdentity`
- `RelationEndpoints`
- `RelationSource`
- `RelationCurrentState`

Creation:
created from every `EventRelation` carried by graph-readable facts.

Lifecycle:
recorded, visible in current-only reads, hidden by source state when current-only filters apply.

Dependencies:
source traversal fact, relation type vocabulary, object refs.

## `ObjectHistory`

One ordered fact history for a world object.

Purpose:
supports facts mentioning an object after a sequence, freshness checks, coverage checks, and evidence hydration.

Required components:

- `ObjectHistoryIdentity`
- `HistoryWindow`
- `HistoryFactRefs`

Creation:
maintained by object fact indexing.

Lifecycle:
current projection, rebuildable from traversal facts.

Dependencies:
object ref, traversal fact index, source cursor.

## `AnchorLineage`

One supersession relation between anchors.

Purpose:
records which anchor replaced another anchor and preserves the sequence and source fact behind replacement.

Required components:

- `LineageIdentity`
- `LineageLink`
- `LineageSource`

Creation:
created when a new anchor supersedes the current anchor for the same anchor ref.

Lifecycle:
append-only.

Dependencies:
prior anchor, new anchor, supersession source fact.

## `ProvenanceBundle`

One explanation bundle for graph state.

Purpose:
explains which source facts, derived facts, objects, and relations made an anchor or traversal fact visible.

Required components:

- `ProvenanceIdentity`
- `ProvenanceFacts`
- `ProvenanceObjects`
- `ProvenanceRelations`

Creation:
created during anchor selection, traversal fact indexing, and derived graph publication.

Lifecycle:
append-only projection record.

Dependencies:
source facts, derived graph facts, object refs, relation refs.

## `BranchPresence`

One object or anchor presence record scoped to a branch.

Purpose:
preserves branch-local existence and allows branch-annotated federated graph reads.

Required components:

- `BranchPresenceIdentity`
- `BranchScope`
- `BranchPresenceState`
- `BranchSource`

Creation:
created from branch-aware graph facts or branch federation adapters.

Lifecycle:
present, absent, tombstoned, inherited.

Dependencies:
branch ref, object ref, source cursor, federation policy.

## `GraphWalk`

One bounded traversal query product.

Purpose:
captures visited objects, visited facts, traversed relations, and the source boundary for a walk request.

Required components:

- `WalkIdentity`
- `WalkSpec`
- `WalkResult`
- `WalkBoundary`

Creation:
created by traversal projection when a walk is materialized or returned.

Lifecycle:
ephemeral, cacheable by source cursor.

Dependencies:
start object, relation indexes, walk spec, current anchor state.

## `NeighborSet`

One relation-neighbor query product.

Purpose:
captures adjacent objects for an object, direction, relation filter, and current-only setting.

Required components:

- `NeighborSetIdentity`
- `NeighborSpec`
- `NeighborResult`
- `NeighborBoundary`

Creation:
created by adjacency projection when neighbor reads are materialized or returned.

Lifecycle:
ephemeral, cacheable by source cursor.

Dependencies:
object ref, relation indexes, source cursor.

## `GraphReductionCursor`

One reducer progress record.

Purpose:
records the last source sequence reduced into graph state and gates idempotent catch-up.

Required components:

- `ReducerIdentity`
- `ReducerCursor`
- `ReducerState`

Creation:
created by graph runtime initialization.

Lifecycle:
active, rebuilding, failed, recovered.

Dependencies:
event spine sequence, traversal store, graph reducer version.

## `DerivedGraphFact`

One graph-owned derived fact prepared for idempotent spine publication.

Purpose:
publishes anchor selected and anchor superseded events back to the spine without making graph state depend on unpublished worker memory.

Required components:

- `DerivedFactIdentity`
- `DerivedFactPayload`
- `DerivedFactSource`
- `DerivedFactPublication`

Creation:
created by anchor selection and supersession systems.

Lifecycle:
prepared, published, skipped as duplicate.

Dependencies:
anchor record, supersession record, idempotent spine append.

## Entity Rules

- Graph entities are replayable from spine facts and graph reducer version.
- `TraversalFact` is the graph source projection over spine history.
- `Anchor` records current selection, not trust.
- `RelationEdge` records reachability, not causal mechanism.
- `ProvenanceBundle` explains graph state, not belief authority.
- `GraphWalk` and `NeighborSet` are query products, not new source authority.
- `DerivedGraphFact` is idempotent publication, not manual mutation.

## Read With

- [Graph Components](components.md)
- [Graph Systems](systems.md)
- [Graph Requirements](requirements.md)
- [Graph](README.md)
