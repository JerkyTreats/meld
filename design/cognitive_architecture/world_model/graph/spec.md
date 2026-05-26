# Graph Spec

Date: 2026-05-26
Status: active
Scope: domain specification for `world_model/graph` as a reducer-heavy substrate

`graph` is the most pipeline-heavy and least perspective-sensitive world model domain.
Its job is to reduce shared facts into current anchors, lineage, provenance, adjacency, and traversal surfaces.
Public reads remain graph-shaped queries; internal state is reduction and projection state.

`graph` does shared structural work once for all consumers.
`belief`, `causation`, `regime`, `planner`, and `agent` consume graph outputs instead of rebuilding graph structure per perspective.

## Domain Types

Graph domain types are stable identities for replayed facts, world objects, current anchors, relation edges, lineage, provenance, branch presence, traversal queries, and reducer state.

Graph domain types do structural work.
They do not carry belief confidence, causal authority, regime identity, planner policy, or task dispatch semantics.

### Type Categories

| Category | Types | Persistence expectation |
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

### Identity Rules

| Type | Identity rule | Required parent | Owning authority |
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

### `TraversalFact`

One graph-readable spine fact lowered into traversal state.

Purpose:
preserves event sequence, source fact id, object refs, relation refs, and event type for graph replay and hydration.

Required state:

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

### `WorldObject`

One graph object keyed by `DomainObjectRef`.

Purpose:
provides shared object identity for anchors, relations, history, branch presence, and traversal.

Required state:

- `ObjectIdentity`
- `ObjectPresence`
- `ObjectHistoryIndex`

Creation:
created when a traversal fact mentions a domain object.

Lifecycle:
present, absent in branch, archived by retention policy.

Dependencies:
`DomainObjectRef`, traversal fact membership index, branch presence records.

### `Anchor`

One selected current pointer for a subject and perspective.

Purpose:
records which target is current for an anchor ref under a perspective and how that current pointer superseded prior anchors.

Required state:

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

### `RelationEdge`

One typed adjacency between world objects.

Purpose:
indexes relation direction, relation type, source and target refs, source fact id, and sequence for neighbor and walk queries.

Required state:

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

### `ObjectHistory`

One ordered fact history for a world object.

Purpose:
supports facts mentioning an object after a sequence, freshness checks, coverage checks, and evidence hydration.

Required state:

- `ObjectHistoryIdentity`
- `HistoryWindow`
- `HistoryFactRefs`

Creation:
maintained by object fact indexing.

Lifecycle:
current projection, rebuildable from traversal facts.

Dependencies:
object ref, traversal fact index, source cursor.

### `AnchorLineage`

One supersession relation between anchors.

Purpose:
records which anchor replaced another anchor and preserves the sequence and source fact behind replacement.

Required state:

- `LineageIdentity`
- `LineageLink`
- `LineageSource`

Creation:
created when a new anchor supersedes the current anchor for the same anchor ref.

Lifecycle:
append-only.

Dependencies:
prior anchor, new anchor, supersession source fact.

### `ProvenanceBundle`

One explanation bundle for graph state.

Purpose:
explains which source facts, derived facts, objects, and relations made an anchor or traversal fact visible.

Required state:

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

### `BranchPresence`

One object or anchor presence record scoped to a branch.

Purpose:
preserves branch-local existence and allows branch-annotated federated graph reads.

Required state:

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

### `GraphWalk`

One bounded traversal query product.

Purpose:
captures visited objects, visited facts, traversed relations, and the source boundary for a walk request.

Required state:

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

### `NeighborSet`

One relation-neighbor query product.

Purpose:
captures adjacent objects for an object, direction, relation filter, and current-only setting.

Required state:

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

### `GraphReductionCursor`

One reducer progress record.

Purpose:
records the last source sequence reduced into graph state and gates idempotent catch-up.

Required state:

- `ReducerIdentity`
- `ReducerCursor`
- `ReducerState`

Creation:
created by graph runtime initialization.

Lifecycle:
active, rebuilding, failed, recovered.

Dependencies:
event spine sequence, traversal store, graph reducer version.

### `DerivedGraphFact`

One graph-owned derived fact prepared for idempotent spine publication.

Purpose:
publishes anchor selected and anchor superseded events back to the spine without making graph state depend on unpublished worker memory.

Required state:

- `DerivedFactIdentity`
- `DerivedFactPayload`
- `DerivedFactSource`
- `DerivedFactPublication`

Creation:
created by anchor selection and supersession processes.

Lifecycle:
prepared, published, skipped as duplicate.

Dependencies:
anchor record, supersession record, idempotent spine append.

### Domain Type Rules

- Graph domain types are replayable from spine facts and graph reducer version.
- `TraversalFact` is the graph source projection over spine history.
- `Anchor` records current selection, not trust.
- `RelationEdge` records reachability, not causal mechanism.
- `ProvenanceBundle` explains graph state, not belief authority.
- `GraphWalk` and `NeighborSet` are query products, not new source authority.
- `DerivedGraphFact` is idempotent publication, not manual mutation.

## Data Model

Graph state records are typed data contracts attached to graph domain types.

They carry object identity, fact source refs, anchor scope, relation endpoints, lineage, provenance, branch state, traversal specs, query results, reducer cursors, and derived publication state.

### Data Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable graph record addressing | fact lowering and graph reducers | all graph processes |
| Source fact | spine fact identity, sequence, objects, relations | fact lowering | indexes, anchors, provenance |
| Object | domain object identity and membership | object indexing | traversal, history, branch reads |
| Anchor | current pointer state by subject and perspective | anchor selection | belief, planner, query routes |
| Relation | typed adjacency and direction indexes | relation indexing | neighbor and walk queries |
| Lineage | supersession chains | anchor selection | belief, audit, provenance |
| Provenance | source and derived fact explanation | provenance projection | all consumers |
| Branch | branch-local presence and federation state | branch federation | branch-scoped reads |
| Traversal | walk and neighbor query specs and results | traversal projection | public graph routes |
| Runtime | reducer cursor and derived publication state | graph runtime | catch-up and recovery |

### Shape Rules

- State records carry typed refs and cursors before presentation text.
- State records do not carry confidence, causal effect, regime state, or action policy.
- Source-derived state records preserve source fact ids.
- Query result state records name their source cursor boundary.
- Runtime state records are durable enough for replay and recovery.

### Identity Data

| State | Attached to | Purpose |
|---|---|---|
| `TraversalFactIdentity` | `TraversalFact` | stable graph fact id |
| `ObjectIdentity` | `WorldObject` | stable object ref |
| `AnchorIdentity` | `Anchor` | stable anchor id |
| `RelationIdentity` | `RelationEdge` | stable relation index key |

#### `TraversalFactIdentity`

Purpose:
stable identity for one graph-readable spine fact.

Dependencies:
source spine fact id, source sequence, graph reducer version.

```rust
struct TraversalFactIdentity {
    fact_id: TraversalFactId,
    source_spine_fact_id: SourceFactId,
    fact_ref: DomainObjectRef,
    reducer_version: SchemaVersion,
}
```

#### `ObjectIdentity`

Purpose:
stable identity for one graph object.

Dependencies:
`DomainObjectRef`.

```rust
struct ObjectIdentity {
    object_ref: DomainObjectRef,
    object_key: ObjectIndexKey,
}
```

#### `AnchorIdentity`

Purpose:
stable identity for one anchor selection record.

Dependencies:
anchor ref, source sequence, graph reducer version.

```rust
struct AnchorIdentity {
    anchor_id: AnchorId,
    anchor_ref: DomainObjectRef,
    reducer_version: SchemaVersion,
}
```

#### `RelationIdentity`

Purpose:
stable identity for one indexed relation occurrence.

Dependencies:
source object, relation type, target object, source fact id, source sequence.

```rust
struct RelationIdentity {
    relation_id: RelationId,
    relation_type: RelationType,
    source_fact_id: TraversalFactId,
    seq: SourceSeq,
}
```

### Source Fact Data

| State | Attached to | Purpose |
|---|---|---|
| `TraversalFactSource` | `TraversalFact` | source event metadata |
| `TraversalFactObjects` | `TraversalFact` | mentioned object refs |
| `TraversalFactRelations` | `TraversalFact` | emitted relation refs |
| `TraversalFactCursor` | `TraversalFact` | source sequence boundary |

#### `TraversalFactSource`

Purpose:
preserves event metadata used by graph replay.

Dependencies:
spine event record.

```rust
struct TraversalFactSource {
    source_spine_fact_id: SourceFactId,
    event_type: EventType,
    domain_id: DomainId,
    stream_id: StreamId,
    content_hash: Option<ContentHash>,
}
```

#### `TraversalFactObjects`

Purpose:
records all graph objects mentioned by a fact.

Dependencies:
spine event object refs.

```rust
struct TraversalFactObjects {
    objects: Vec<DomainObjectRef>,
}
```

#### `TraversalFactRelations`

Purpose:
records all graph relations emitted by a fact.

Dependencies:
spine event relation refs.

```rust
struct TraversalFactRelations {
    relations: Vec<EventRelation>,
}
```

#### `TraversalFactCursor`

Purpose:
records source order for graph replay and object history.

Dependencies:
spine sequence.

```rust
struct TraversalFactCursor {
    seq: SourceSeq,
    source_cursor: SourceCursor,
}
```

### Object Data

| State | Attached to | Purpose |
|---|---|---|
| `ObjectPresence` | `WorldObject` | graph-visible presence state |
| `ObjectHistoryIndex` | `WorldObject` | fact history membership |

#### `ObjectPresence`

Purpose:
records whether an object is visible in graph projection and branch scope.

Dependencies:
traversal facts, branch presence records.

```rust
struct ObjectPresence {
    presence_status: ObjectPresenceStatus,
    first_seen_cursor: SourceCursor,
    last_seen_cursor: SourceCursor,
    branch_refs: Vec<BranchRef>,
}
```

#### `ObjectHistoryIndex`

Purpose:
records fact ids that mention an object.

Dependencies:
traversal fact object membership.

```rust
struct ObjectHistoryIndex {
    object_ref: DomainObjectRef,
    fact_ids: Vec<TraversalFactId>,
    high_water_cursor: SourceCursor,
}
```

### Anchor Data

| State | Attached to | Purpose |
|---|---|---|
| `AnchorScope` | `Anchor` | subject and perspective |
| `AnchorTarget` | `Anchor` | selected target |
| `AnchorCurrentState` | `Anchor` | current or ended state |
| `AnchorSourceFacts` | `Anchor` | source facts for selection |

#### `AnchorScope`

Purpose:
defines the subject and perspective of an anchor.

Dependencies:
anchor selection input, perspective key.

```rust
struct AnchorScope {
    subject: DomainObjectRef,
    perspective: PerspectiveKey,
}
```

#### `AnchorTarget`

Purpose:
records the selected current target.

Dependencies:
anchor selection input.

```rust
struct AnchorTarget {
    target: DomainObjectRef,
}
```

#### `AnchorCurrentState`

Purpose:
records whether an anchor is current, ended, or superseded.

Dependencies:
anchor selection and anchor end intents.

```rust
struct AnchorCurrentState {
    selected_at_seq: SourceSeq,
    ended_at_seq: Option<SourceSeq>,
    ended_by_anchor_id: Option<AnchorId>,
    ended_by_fact_id: Option<DerivedFactId>,
}
```

#### `AnchorSourceFacts`

Purpose:
records source facts that selected or ended the anchor.

Dependencies:
traversal facts and derived graph facts.

```rust
struct AnchorSourceFacts {
    source_fact_ids: Vec<SourceFactId>,
    created_by_fact_id: DerivedFactId,
}
```

### Relation Data

| State | Attached to | Purpose |
|---|---|---|
| `RelationEndpoints` | `RelationEdge` | source and target refs |
| `RelationSource` | `RelationEdge` | fact and sequence source |
| `RelationCurrentState` | `RelationEdge` | current-only visibility |

#### `RelationEndpoints`

Purpose:
records relation direction and endpoints.

Dependencies:
event relation record.

```rust
struct RelationEndpoints {
    src: DomainObjectRef,
    dst: DomainObjectRef,
    relation_type: RelationType,
}
```

#### `RelationSource`

Purpose:
records the fact that emitted the relation.

Dependencies:
traversal fact record.

```rust
struct RelationSource {
    fact_id: TraversalFactId,
    source_spine_fact_id: SourceFactId,
    seq: SourceSeq,
}
```

#### `RelationCurrentState`

Purpose:
records whether current-only traversal may include this relation.

Dependencies:
current anchor state and relation visibility policy.

```rust
struct RelationCurrentState {
    visible_in_current_reads: bool,
    visibility_reason: RelationVisibilityReason,
}
```

### Lineage Data

| State | Attached to | Purpose |
|---|---|---|
| `LineageIdentity` | `AnchorLineage` | stable lineage id |
| `LineageLink` | `AnchorLineage` | prior and superseding anchor ids |
| `LineageSource` | `AnchorLineage` | source fact and sequence |

#### `LineageIdentity`

Purpose:
stable identity for one anchor supersession link.

Dependencies:
prior anchor id and superseding anchor id.

```rust
struct LineageIdentity {
    lineage_id: LineageId,
    anchor_id: AnchorId,
    superseded_by_anchor_id: AnchorId,
}
```

#### `LineageLink`

Purpose:
records anchor replacement.

Dependencies:
anchor selection state.

```rust
struct LineageLink {
    anchor_id: AnchorId,
    superseded_by_anchor_id: AnchorId,
}
```

#### `LineageSource`

Purpose:
records when and why supersession happened.

Dependencies:
source fact and derived supersession fact.

```rust
struct LineageSource {
    source_fact_id: SourceFactId,
    derived_fact_id: DerivedFactId,
    seq: SourceSeq,
}
```

### Provenance Data

| State | Attached to | Purpose |
|---|---|---|
| `ProvenanceIdentity` | `ProvenanceBundle` | stable provenance id |
| `ProvenanceFacts` | `ProvenanceBundle` | source and derived fact refs |
| `ProvenanceObjects` | `ProvenanceBundle` | object refs |
| `ProvenanceRelations` | `ProvenanceBundle` | relation refs |

#### `ProvenanceIdentity`

Purpose:
stable identity for one provenance bundle.

Dependencies:
anchor id or traversal fact id.

```rust
struct ProvenanceIdentity {
    provenance_id: ProvenanceId,
    subject_ref: GraphRecordRef,
    provenance_version: SchemaVersion,
}
```

#### `ProvenanceFacts`

Purpose:
records source and derived facts that explain graph state.

Dependencies:
traversal facts and derived graph facts.

```rust
struct ProvenanceFacts {
    source_fact_ids: Vec<SourceFactId>,
    derived_fact_ids: Vec<DerivedFactId>,
}
```

#### `ProvenanceObjects`

Purpose:
records objects that participated in graph state.

Dependencies:
traversal fact object refs.

```rust
struct ProvenanceObjects {
    objects: Vec<DomainObjectRef>,
}
```

#### `ProvenanceRelations`

Purpose:
records relations that participated in graph state.

Dependencies:
traversal fact relation refs.

```rust
struct ProvenanceRelations {
    relations: Vec<EventRelation>,
}
```

### Branch Data

| State | Attached to | Purpose |
|---|---|---|
| `BranchScope` | `BranchPresence` | branch identity |
| `BranchPresenceState` | `BranchPresence` | branch-local presence |
| `BranchSource` | `BranchPresence` | source refs |

#### `BranchScope`

Purpose:
defines the branch where a graph object is present or absent.

Dependencies:
branch ref contract.

```rust
struct BranchScope {
    branch_ref: BranchRef,
    federation_scope: FederationScope,
}
```

#### `BranchPresenceState`

Purpose:
records branch-local object presence.

Dependencies:
branch-aware facts or federation adapter.

```rust
struct BranchPresenceState {
    object_ref: DomainObjectRef,
    presence_status: BranchPresenceStatus,
    inherited_from: Option<BranchRef>,
}
```

#### `BranchSource`

Purpose:
records source boundary for branch presence.

Dependencies:
branch source facts.

```rust
struct BranchSource {
    source_refs: Vec<SourceFactId>,
    source_cursor: SourceCursor,
}
```

### Traversal Data

| State | Attached to | Purpose |
|---|---|---|
| `WalkIdentity` | `GraphWalk` | stable walk cache key |
| `WalkSpec` | `GraphWalk` | traversal request |
| `WalkResult` | `GraphWalk` | traversal result |
| `WalkBoundary` | `GraphWalk` | source boundary |
| `NeighborSetIdentity` | `NeighborSet` | stable neighbor cache key |
| `NeighborSpec` | `NeighborSet` | neighbor request |
| `NeighborResult` | `NeighborSet` | neighbor result |
| `NeighborBoundary` | `NeighborSet` | source boundary |

#### `WalkIdentity`

Purpose:
stable identity for one bounded graph walk result.

Dependencies:
start object, walk spec, source cursor.

```rust
struct WalkIdentity {
    walk_id: GraphWalkId,
    start: DomainObjectRef,
    spec_hash: ContentHash,
}
```

#### `WalkSpec`

Purpose:
records bounded graph walk parameters.

Dependencies:
public graph query contract.

```rust
struct WalkSpec {
    direction: TraversalDirection,
    relation_types: Option<Vec<RelationType>>,
    max_depth: usize,
    current_only: bool,
    include_facts: bool,
}
```

#### `WalkResult`

Purpose:
records visited objects, facts, and relations for a bounded walk.

Dependencies:
relation indexes and traversal fact indexes.

```rust
struct WalkResult {
    visited_objects: Vec<DomainObjectRef>,
    visited_facts: Vec<TraversalFactId>,
    traversed_relations: Vec<EventRelation>,
}
```

#### `WalkBoundary`

Purpose:
records the source boundary for a walk result.

Dependencies:
graph reduction cursor.

```rust
struct WalkBoundary {
    source_cursor: SourceCursor,
    reducer_version: SchemaVersion,
}
```

#### `NeighborSetIdentity`

Purpose:
stable identity for one neighbor result.

Dependencies:
object ref, neighbor spec, source cursor.

```rust
struct NeighborSetIdentity {
    neighbor_set_id: NeighborSetId,
    object_ref: DomainObjectRef,
    spec_hash: ContentHash,
}
```

#### `NeighborSpec`

Purpose:
records neighbor query parameters.

Dependencies:
public graph query contract.

```rust
struct NeighborSpec {
    direction: TraversalDirection,
    relation_types: Option<Vec<RelationType>>,
    current_only: bool,
}
```

#### `NeighborResult`

Purpose:
records adjacent object refs.

Dependencies:
incoming and outgoing relation indexes.

```rust
struct NeighborResult {
    neighbors: Vec<DomainObjectRef>,
}
```

#### `NeighborBoundary`

Purpose:
records the source boundary for a neighbor result.

Dependencies:
graph reduction cursor.

```rust
struct NeighborBoundary {
    source_cursor: SourceCursor,
    reducer_version: SchemaVersion,
}
```

### Runtime Data

| State | Attached to | Purpose |
|---|---|---|
| `ReducerIdentity` | `GraphReductionCursor` | reducer id |
| `ReducerCursor` | `GraphReductionCursor` | last reduced source sequence |
| `ReducerState` | `GraphReductionCursor` | runtime status |
| `DerivedFactIdentity` | `DerivedGraphFact` | idempotent derived fact id |
| `DerivedFactPayload` | `DerivedGraphFact` | derived graph event payload |
| `DerivedFactSource` | `DerivedGraphFact` | source graph record |
| `DerivedFactPublication` | `DerivedGraphFact` | publication status |

#### `ReducerIdentity`

Purpose:
identifies one graph reducer stream.

Dependencies:
graph runtime configuration.

```rust
struct ReducerIdentity {
    reducer_id: GraphReducerId,
    reducer_version: SchemaVersion,
}
```

#### `ReducerCursor`

Purpose:
records graph catch-up progress.

Dependencies:
event spine sequence.

```rust
struct ReducerCursor {
    last_reduced_seq: SourceSeq,
    source_cursor: SourceCursor,
}
```

#### `ReducerState`

Purpose:
records reducer lifecycle and recovery state.

Dependencies:
graph runtime.

```rust
struct ReducerState {
    status: ReducerStatus,
    last_error: Option<ReasonCode>,
}
```

#### `DerivedFactIdentity`

Purpose:
stable identity for one graph-derived spine fact.

Dependencies:
source anchor or lineage record.

```rust
struct DerivedFactIdentity {
    derived_fact_id: DerivedFactId,
    event_type: EventType,
    idempotency_key: ContentHash,
}
```

#### `DerivedFactPayload`

Purpose:
records data prepared for spine publication.

Dependencies:
anchor selection or supersession record.

```rust
struct DerivedFactPayload {
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
    payload_ref: PayloadRef,
}
```

#### `DerivedFactSource`

Purpose:
links a derived fact back to graph source state.

Dependencies:
anchor or lineage record.

```rust
struct DerivedFactSource {
    source_graph_record: GraphRecordRef,
    source_fact_ids: Vec<SourceFactId>,
}
```

#### `DerivedFactPublication`

Purpose:
records idempotent publication outcome.

Dependencies:
spine append contract.

```rust
struct DerivedFactPublication {
    publication_status: PublicationStatus,
    published_seq: Option<SourceSeq>,
}
```

### Shared Data Shapes

#### `SourceCursor`

Purpose:
shared graph source boundary.

Dependencies:
event spine sequence.

```rust
struct SourceCursor {
    last_seen_seq: SourceSeq,
}
```

#### `GraphRecordRef`

Purpose:
shared ref to graph-owned records.

Dependencies:
graph domain type ids.

```rust
struct GraphRecordRef {
    record_kind: GraphRecordKind,
    record_id: GraphRecordId,
}
```

### Data Model Rules

- Anchor data names current selection and supersession, not trust.
- Relation data names adjacency, not mechanism.
- Provenance data names source and derived facts, not belief authority.
- Traversal result data names source cursor boundaries.
- Runtime data persists reducer progress before graph catch-up returns.

## Pipelines

Graph pipeline stages reduce spine facts into current anchors, lineage, provenance, relation indexes, object history, branch presence, traversal products, and idempotent derived graph facts.

They are deterministic over event order, reducer version, source intents, and storage state.
They do not infer belief, causal effect, regime identity, or action policy.

### Pipeline Overview

| Phase | Stage | Required input | Required output |
|---|---|---|---|
| 1 | graph catch-up | spine event store, reducer cursor | event batch |
| 2 | fact admission | event batch | graph-readable events |
| 3 | traversal fact recording | graph-readable events | `TraversalFact` |
| 4 | object indexing | traversal facts | `WorldObject`, object history index |
| 5 | relation indexing | traversal facts | `RelationEdge` |
| 6 | source intent lowering | graph-readable events | anchor selection or end intents |
| 7 | anchor selection | anchor intents | `Anchor` current state |
| 8 | lineage update | superseded anchors | `AnchorLineage` |
| 9 | provenance projection | facts, anchors, relations | `ProvenanceBundle` |
| 10 | branch federation | branch-aware facts | `BranchPresence` |
| 11 | traversal projection | query specs, indexes | `GraphWalk`, `NeighborSet` |
| 12 | derived graph publication | anchor and lineage changes | `DerivedGraphFact` |
| 13 | reducer cursor commit | successful reduction and publication | `GraphReductionCursor` |
| 14 | recovery and rebuild | spine facts and reducer cursor | rebuilt graph projections |

Replay invariant:
same spine events, reducer version, source intent rules, and starting cursor produce the same graph records, derived fact ids, and query results.

### Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| spine replay | events | graph catch-up |
| spine idempotent append | events | derived graph publication |
| domain object refs | events and source domains | object indexing |
| event relations | events and source domains | relation indexing |
| branch refs | branch domain or graph facts | branch federation |
| traversal store | graph | current anchor and traversal reads |

### Graph Catch-Up

Input:
event spine and graph reduction cursor.

Output:
ordered event batch after last reduced sequence.

Rules:

- Read events after the last reduced sequence.
- Hold one catch-up guard per graph runtime.
- Preserve event sequence order.
- Advance reducer cursor only after graph records and derived publications are durable.

### Fact Admission

Input:
event batch.

Output:
graph-readable events.

Rules:

- Admit events from domains that publish graph-readable objects or relations.
- Preserve event type, domain id, stream id, source sequence, object refs, and relation refs.
- Reject events without graph-readable payload by returning no graph intents.
- Do not interpret trust, confidence, or action semantics.

### Traversal Fact Recording

Input:
graph-readable events.

Output:
`TraversalFact`.

Rules:

- Create one traversal fact per admitted source event.
- Record source spine fact id and source sequence.
- Record objects and relations exactly as published.
- Index source fact id to traversal fact id.
- Index sequence to traversal fact id.

### Object Indexing

Input:
traversal facts.

Output:
`WorldObject` records and object history indexes.

Rules:

- Create object identity for every object ref mentioned by a traversal fact.
- Index fact membership by object.
- Preserve facts mentioning object after sequence queries.
- Maintain first and last seen source cursor.

### Relation Indexing

Input:
traversal facts with relation refs.

Output:
`RelationEdge`.

Rules:

- Index outgoing relation by source object, relation type, target object, sequence, and fact id.
- Index incoming relation by target object, relation type, source object, sequence, and fact id.
- Preserve relation source fact and sequence.
- Do not derive causal relation from reachability.

### Source Intent Lowering

Input:
graph-readable event and source fact id.

Output:
anchor selection or anchor end intents.

Rules:

- Lower workspace snapshot selection into snapshot current anchor intent.
- Lower context head selection into frame head anchor intent.
- Lower context head tombstone into anchor end intent.
- Lower task artifact emission into artifact slot anchor intent.
- Return no intent when event data lacks required object refs.
- Keep source intent rules versioned.

### Anchor Selection

Input:
anchor selection and end intents.

Output:
`Anchor` records and current anchor index.

Rules:

- Select current target for one anchor ref, subject, and perspective.
- Use source sequence in anchor id.
- Ignore duplicate anchor selection when the same anchor is already current.
- Ignore older selection when current anchor has a later sequence.
- End current anchor before selecting a newer replacement.
- Clear current anchor when an end intent applies.

### Lineage Update

Input:
prior current anchor and new anchor.

Output:
`AnchorLineage`.

Rules:

- Record prior anchor id and superseding anchor id.
- Record supersession sequence.
- Preserve supersession derived fact id.
- Keep lineage append-only.

### Provenance Projection

Input:
traversal facts, anchors, relations, lineage, and derived graph facts.

Output:
`ProvenanceBundle`.

Rules:

- Attach source fact ids to anchor records.
- Attach derived fact ids to anchor selected and anchor superseded publication.
- Preserve object and relation refs that explain graph state.
- Provide provenance by anchor id.

### Branch Federation

Input:
branch-aware graph facts and federated traversal stores.

Output:
`BranchPresence`.

Rules:

- Preserve object presence per branch.
- Preserve branch-local anchor state.
- Preserve inherited presence separately from local presence.
- Keep branch-scoped reads from merging branch facts into one global authority.

### Traversal Projection

Input:
object indexes, relation indexes, current anchor state, and query specs.

Output:
`GraphWalk` and `NeighborSet`.

Rules:

- Support outgoing, incoming, and both-direction traversal.
- Support relation type filters.
- Require positive walk depth.
- Support current-only selection.
- Include facts only when requested.
- Preserve source cursor in query products.

### Derived Graph Publication

Input:
anchor selection and lineage changes.

Output:
`DerivedGraphFact` and spine append.

Rules:

- Publish anchor selected facts with idempotent record ids.
- Publish anchor superseded facts with idempotent record ids.
- Include graph object refs in derived envelopes.
- Append through the spine idempotent append contract.
- Treat duplicate derived publication as success.

### Reducer Cursor Commit

Input:
successful graph records and derived publication results.

Output:
updated `GraphReductionCursor`.

Rules:

- Track max source sequence observed.
- Include derived publication sequence when it exceeds source sequence.
- Flush spine and graph store before cursor update completes.
- Never advance cursor past unpersisted graph state.

### Recovery And Rebuild

Input:
spine event history and graph reduction cursor.

Output:
rebuilt graph projections.

Rules:

- Rebuild graph state from spine replay.
- Recreate traversal facts, object indexes, relation indexes, anchors, lineage, and provenance.
- Recreate derived publication ids deterministically.
- Preserve idempotence on derived fact append.
- Recover from reducer failure by replaying after the durable cursor.

### Failure Semantics

| Condition | Graph behavior |
|---|---|
| event lacks graph-readable refs | no graph intent |
| event relation has invalid object ref | route or reducer error |
| duplicate anchor selected fact | idempotent success |
| older anchor selection arrives after newer current anchor | ignore older selection |
| current anchor is tombstoned | clear current anchor |
| walk depth is zero | query validation error |
| derived fact append duplicates existing record | idempotent success |
| reducer fails before cursor commit | replay from previous cursor |

### Test Matrix

| Test | Expected proof |
|---|---|
| replay determinism | same spine facts produce same anchors and indexes |
| current anchor selection | latest valid anchor is current |
| older selection guard | older selection does not replace newer current anchor |
| anchor end | tombstone clears current anchor |
| lineage preservation | superseded anchor points to replacement |
| provenance preservation | anchor provenance names source facts |
| relation direction | incoming and outgoing neighbor reads differ correctly |
| bounded walk | max depth and relation filters are enforced |
| current-only walk | current-only traversal respects current state |
| idempotent publication | repeated catch-up does not duplicate derived graph facts |
| recovery | reducer rebuilds from prior durable cursor |
| branch read | branch-local presence remains branch scoped |

### Pipeline Rules

- Graph pipeline stages are deterministic over event order and reducer version.
- Graph pipeline stages mutate graph-owned projection records only.
- Graph pipeline stages publish derived graph facts through the spine.
- Graph pipeline stages preserve source fact refs and sequence boundaries.
- Graph pipeline stages expose traversal and current state without belief settlement.

## Read With

- [Graph](README.md)
- [Graph Requirements](requirements.md)
- [World Model Domain](../README.md)
