# Graph Components

Date: 2026-05-15
Status: active
Scope: ECS components for `world_model/graph`

## Thesis

Graph components are typed data contracts attached to graph entities.

They carry object identity, fact source refs, anchor scope, relation endpoints, lineage, provenance, branch state, traversal specs, query results, reducer cursors, and derived publication state.

## Component Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable graph record addressing | fact lowering and graph reducers | all graph systems |
| Source fact | spine fact identity, sequence, objects, relations | fact lowering | indexes, anchors, provenance |
| Object | domain object identity and membership | object indexing | traversal, history, branch reads |
| Anchor | current pointer state by subject and perspective | anchor selection | belief, planner, query routes |
| Relation | typed adjacency and direction indexes | relation indexing | neighbor and walk queries |
| Lineage | supersession chains | anchor selection | belief, audit, provenance |
| Provenance | source and derived fact explanation | provenance projection | all consumers |
| Branch | branch-local presence and federation state | branch federation | branch-scoped reads |
| Traversal | walk and neighbor query specs and results | traversal projection | public graph routes |
| Runtime | reducer cursor and derived publication state | graph runtime | catch-up and recovery |

## Shape Rules

- Components carry typed refs and cursors before presentation text.
- Components do not carry confidence, causal effect, regime state, or action policy.
- Source-derived components preserve source fact ids.
- Query result components name their source cursor boundary.
- Runtime components are durable enough for replay and recovery.

## Identity Components

| Component | Attached to | Purpose |
|---|---|---|
| `TraversalFactIdentity` | `TraversalFact` | stable graph fact id |
| `ObjectIdentity` | `WorldObject` | stable object ref |
| `AnchorIdentity` | `Anchor` | stable anchor id |
| `RelationIdentity` | `RelationEdge` | stable relation index key |

### `TraversalFactIdentity`

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

### `ObjectIdentity`

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

### `AnchorIdentity`

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

### `RelationIdentity`

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

## Source Fact Components

| Component | Attached to | Purpose |
|---|---|---|
| `TraversalFactSource` | `TraversalFact` | source event metadata |
| `TraversalFactObjects` | `TraversalFact` | mentioned object refs |
| `TraversalFactRelations` | `TraversalFact` | emitted relation refs |
| `TraversalFactCursor` | `TraversalFact` | source sequence boundary |

### `TraversalFactSource`

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

### `TraversalFactObjects`

Purpose:
records all graph objects mentioned by a fact.

Dependencies:
spine event object refs.

```rust
struct TraversalFactObjects {
    objects: Vec<DomainObjectRef>,
}
```

### `TraversalFactRelations`

Purpose:
records all graph relations emitted by a fact.

Dependencies:
spine event relation refs.

```rust
struct TraversalFactRelations {
    relations: Vec<EventRelation>,
}
```

### `TraversalFactCursor`

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

## Object Components

| Component | Attached to | Purpose |
|---|---|---|
| `ObjectPresence` | `WorldObject` | graph-visible presence state |
| `ObjectHistoryIndex` | `WorldObject` | fact history membership |

### `ObjectPresence`

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

### `ObjectHistoryIndex`

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

## Anchor Components

| Component | Attached to | Purpose |
|---|---|---|
| `AnchorScope` | `Anchor` | subject and perspective |
| `AnchorTarget` | `Anchor` | selected target |
| `AnchorCurrentState` | `Anchor` | current or ended state |
| `AnchorSourceFacts` | `Anchor` | source facts for selection |

### `AnchorScope`

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

### `AnchorTarget`

Purpose:
records the selected current target.

Dependencies:
anchor selection input.

```rust
struct AnchorTarget {
    target: DomainObjectRef,
}
```

### `AnchorCurrentState`

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

### `AnchorSourceFacts`

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

## Relation Components

| Component | Attached to | Purpose |
|---|---|---|
| `RelationEndpoints` | `RelationEdge` | source and target refs |
| `RelationSource` | `RelationEdge` | fact and sequence source |
| `RelationCurrentState` | `RelationEdge` | current-only visibility |

### `RelationEndpoints`

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

### `RelationSource`

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

### `RelationCurrentState`

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

## Lineage Components

| Component | Attached to | Purpose |
|---|---|---|
| `LineageIdentity` | `AnchorLineage` | stable lineage id |
| `LineageLink` | `AnchorLineage` | prior and superseding anchor ids |
| `LineageSource` | `AnchorLineage` | source fact and sequence |

### `LineageIdentity`

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

### `LineageLink`

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

### `LineageSource`

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

## Provenance Components

| Component | Attached to | Purpose |
|---|---|---|
| `ProvenanceIdentity` | `ProvenanceBundle` | stable provenance id |
| `ProvenanceFacts` | `ProvenanceBundle` | source and derived fact refs |
| `ProvenanceObjects` | `ProvenanceBundle` | object refs |
| `ProvenanceRelations` | `ProvenanceBundle` | relation refs |

### `ProvenanceIdentity`

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

### `ProvenanceFacts`

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

### `ProvenanceObjects`

Purpose:
records objects that participated in graph state.

Dependencies:
traversal fact object refs.

```rust
struct ProvenanceObjects {
    objects: Vec<DomainObjectRef>,
}
```

### `ProvenanceRelations`

Purpose:
records relations that participated in graph state.

Dependencies:
traversal fact relation refs.

```rust
struct ProvenanceRelations {
    relations: Vec<EventRelation>,
}
```

## Branch Components

| Component | Attached to | Purpose |
|---|---|---|
| `BranchScope` | `BranchPresence` | branch identity |
| `BranchPresenceState` | `BranchPresence` | branch-local presence |
| `BranchSource` | `BranchPresence` | source refs |

### `BranchScope`

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

### `BranchPresenceState`

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

### `BranchSource`

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

## Traversal Components

| Component | Attached to | Purpose |
|---|---|---|
| `WalkIdentity` | `GraphWalk` | stable walk cache key |
| `WalkSpec` | `GraphWalk` | traversal request |
| `WalkResult` | `GraphWalk` | traversal result |
| `WalkBoundary` | `GraphWalk` | source boundary |
| `NeighborSetIdentity` | `NeighborSet` | stable neighbor cache key |
| `NeighborSpec` | `NeighborSet` | neighbor request |
| `NeighborResult` | `NeighborSet` | neighbor result |
| `NeighborBoundary` | `NeighborSet` | source boundary |

### `WalkIdentity`

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

### `WalkSpec`

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

### `WalkResult`

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

### `WalkBoundary`

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

### `NeighborSetIdentity`

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

### `NeighborSpec`

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

### `NeighborResult`

Purpose:
records adjacent object refs.

Dependencies:
incoming and outgoing relation indexes.

```rust
struct NeighborResult {
    neighbors: Vec<DomainObjectRef>,
}
```

### `NeighborBoundary`

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

## Runtime Components

| Component | Attached to | Purpose |
|---|---|---|
| `ReducerIdentity` | `GraphReductionCursor` | reducer id |
| `ReducerCursor` | `GraphReductionCursor` | last reduced source sequence |
| `ReducerState` | `GraphReductionCursor` | runtime status |
| `DerivedFactIdentity` | `DerivedGraphFact` | idempotent derived fact id |
| `DerivedFactPayload` | `DerivedGraphFact` | derived graph event payload |
| `DerivedFactSource` | `DerivedGraphFact` | source graph record |
| `DerivedFactPublication` | `DerivedGraphFact` | publication status |

### `ReducerIdentity`

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

### `ReducerCursor`

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

### `ReducerState`

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

### `DerivedFactIdentity`

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

### `DerivedFactPayload`

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

### `DerivedFactSource`

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

### `DerivedFactPublication`

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

## Shared Data Shapes

### `SourceCursor`

Purpose:
shared graph source boundary.

Dependencies:
event spine sequence.

```rust
struct SourceCursor {
    last_seen_seq: SourceSeq,
}
```

### `GraphRecordRef`

Purpose:
shared ref to graph-owned records.

Dependencies:
graph entity ids.

```rust
struct GraphRecordRef {
    record_kind: GraphRecordKind,
    record_id: GraphRecordId,
}
```

## Component Rules

- Anchor data names current selection and supersession, not trust.
- Relation data names adjacency, not mechanism.
- Provenance data names source and derived facts, not belief authority.
- Traversal result data names source cursor boundaries.
- Runtime data persists reducer progress before graph catch-up returns.

## Read With

- [Graph Entities](entities.md)
- [Graph Systems](systems.md)
- [Graph Requirements](requirements.md)
- [Graph](README.md)
