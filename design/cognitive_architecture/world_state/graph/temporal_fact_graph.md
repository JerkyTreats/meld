# Temporal Fact Graph

Date: 2026-04-20
Status: active
Scope: canonical graph model for `world_state/graph`, built downstream of the shared event spine

## Thesis

The canonical graph for `world_state/graph` is a temporal fact graph.

This means:

- the spine is the durable history of semantic fact commits
- the graph is the current graph surface
- the graph is materialized from spine history
- the graph emits derived anchor facts into the spine, but it does not rewrite history directly

This is the clean split between:

- how current anchors became available
- what is currently reachable

## Core Position

Do not make the graph and the spine the same storage role.

The spine owns:

- durable append
- total commit order
- replay
- cross-domain attachment
- idempotent record lookup through `record_id`
- compatibility reads for legacy telemetry events

Graph materialization owns:

- current anchors
- current relations
- supersession state
- provenance views
- planner-facing and operator-facing projections

The graph surface must always be derivable from the spine.

## Canonical Model

The canonical model has four first-class primitives:

- object
  stable cross-domain identity
- fact
  one immutable semantic commit in time
- relation
  typed edge declared by a fact
- anchor
  current selected surface derived from facts and relations

This is not a property graph first.
This is not a belief calculus first design.
This is not a raw JSON event bucket.

It is a temporal fact ledger plus materialized graph views.

## Why This Model Fits The Architecture

The cognitive loop already implies a split:

- `sensory` promotes observations
- `execution` publishes action outcomes
- `world_state` settles current belief

That means `world_state` needs two things at once:

- durable temporal provenance from the spine
- fast current traversal for planning and inspection

A temporal fact graph gives both.

## Object Identity

The graph no longer assumes filesystem identity is universal.

The cross-domain anchor is `DomainObjectRef`.

Each durable object reference carries:

- `domain_id`
- `object_kind`
- `object_id`

Examples:

- workspace source
- workspace node
- frame
- task run
- workflow thread
- artifact
- world state entity
- claim

`world_state` may still add internal entity identifiers later, but the public durable anchor remains `DomainObjectRef`.

## Fact Shape

Each spine fact that is relevant to `world_state` is reducible into a typed traversal fact record.

The canonical event record carries:

```rust
pub struct EventRecord {
    pub ts: String,
    pub recorded_at: String,
    pub record_id: Option<String>,
    pub session: String,
    pub seq: u64,
    domain_id: String,
    stream_id: String,
    event_type: String,
    occurred_at: Option<String>,
    content_hash: Option<String>,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
    data: serde_json::Value,
}
```

Legacy telemetry events read through serde defaults and normalize to the canonical fields.

The traversal materialization record carries:

```rust
pub struct TraversalFactRecord {
    pub fact_id: TraversalFactId,
    pub source_spine_fact_id: String,
    pub seq: u64,
    pub event_type: String,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}
```

The important design rule is simple:

- payload is not enough
- object attachment and relations must be explicit

Without that rule the spine remains a log, not a graph substrate.

## Traversal Model

The graph is the result of selection and materialization over facts.

Traversal means:

- attach new objects
- preserve relation lineage
- expose current anchors
- expose current reachability
- preserve provenance

The traversal graph should carry:

- entity nodes
- anchor nodes
- typed relations
- source fact links where available
- provenance links
- current head selection
- current lineage state

This layer answers what is current and how to reach it.
It does not by itself answer whether current is still credible.
That is the job of `world_state/belief`.

## Authority Model

Authority flows one way:

- spine to graph is authoritative

Feedback still exists:

- reducers may publish derived anchor facts back into the spine

The safe rule is:

- no direct graph mutation is canonical unless it is represented as a spine fact

So the loop is:

1. `workspace_fs`, `context`, `control`, `task`, or `workflow` publishes semantic facts into the spine
2. `world_state/graph` reducers consume those facts
3. reducers update current graph state
4. `GraphRuntime` appends derived `world_state.*` traversal facts idempotently
5. later replay can rebuild the same traversal graph

This preserves both provenance and replay discipline.

## Reducer Model

The graph is materialized by traversal reducers, not by arbitrary writers.

Reducers are still designed to remain partitionable by:

- object identity
- relation family
- stream
- projection family

This is the path to high parallelism.

Reducer families are:

- anchor selection
  select the current reachable artifact, frame, or head for one object and one perspective
- provenance threading
  preserve why one current anchor is selected
- relation indexing
  preserve graph adjacency for object walk and neighbor lookup
- legacy claim compatibility
  expose older current-claim reads over graph anchors where needed

Each reducer consumes the spine, maintains materialized views, and exposes replay from the last applied `seq`.

## Graph Indexes

Do not answer graph questions by scanning the whole spine.

The graph needs explicit indexes.

Traversal index contract:

- `fact_id -> fact`
- `object_ref -> fact_ids`
- `fact_id -> object_refs`
- outgoing relation adjacency
- incoming relation adjacency
- `anchor_id -> anchor`
- `anchor_ref -> current anchor`
- `anchor_ref -> anchor history`
- `anchor_id -> superseded anchor ids`
- `source_spine_fact_id -> traversal fact id`
- `seq -> fact_id`
- `subject + perspective -> current anchor`

Legacy claim indexes also remain:

- `claim_id -> claim`
- `evidence_id -> evidence`
- `subject -> active claims`
- `subject -> claim history`
- `claim_id -> supersession chain`
- `source_spine_fact_id -> world state fact`

These indexes are the real traversal engine.
The spine remains the durable source.

## Traversal Modes

A full temporal graph needs more than one traversal mode.

The current system supports:

- temporal traversal by `seq`
- object history traversal by `DomainObjectRef`
- relation traversal by graph adjacency
- provenance traversal from anchor to supporting facts
- lineage traversal from current anchor to prior anchor chain through anchor history and supersession
- bounded graph walk with direction, relation filters, current-only selected-edge filtering, and optional fact inclusion

The spine gives temporal traversal.
`world_state/graph` adds object and relation traversal on top.

## What ECS Is And Is Not Here

ECS may still be useful as an internal live materialization substrate.

It is good for:

- sparse attached current state
- parallel system passes
- fast current lookup
- planner-facing caches

It is not the canonical durable graph model.

The durable model should remain:

- temporal facts in the spine
- graph indexes and settled views in `world_state`

If ECS is used later, it should sit behind this contract, not replace it.

## First Durable Record Families

`world_state` currently defines typed facts for:

- `world_state.claim_added`
- `world_state.claim_superseded`
- `world_state.evidence_attached`
- `world_state.anchor_selected`
- `world_state.anchor_superseded`

The implemented graph facts attach to explicit `DomainObjectRef` values.
Relations are carried by source domain facts and by graph-readable envelopes where needed.

Future belief facts may add claim revision, contradiction, calibration, and curation families.

## First Query Surfaces

The current graph answers these queries cheaply:

- what is the current anchor for object X and perspective Y
- what current snapshot represents workspace source X
- what current frame head represents workspace node X and frame type Y
- what current artifact represents task run X and artifact type Y
- which facts mention object X after sequence Y
- which objects neighbor object X by relation and direction
- what bounded subgraph is reachable from object X
- which facts and relations explain anchor X
- which prior anchors were replaced by anchor X

Legacy claim query still answers current claims, claim history, and claim provenance for the older settlement projection.

## Baseline Contract

The graph baseline requires:

1. `DomainObjectRef` and `EventRelation` are the graph attachment contract
2. Canonical events carry graph objects and relations
3. Traversal materialization keeps fact, object, relation, current anchor, history, lineage, source fact, and sequence indexes
4. Reducers consume graph-capable source facts through explicit domain contracts
5. Derived anchor facts are represented as spine events
6. Traversal queries and branch annotated federated reads preserve provenance

The remaining work is belief and curation, not first graph materialization.

## Non Goals

- raw sensory stream transport
- generic graph database integration
- distributed sequencer work
- arbitrary graph query language
- universal ECS adoption

## Read With

- [World State Domain](../README.md)
- [Graph](README.md)
- [Belief](../belief/README.md)
- [Knowledge Graph ECS Decision Memo](../belief/knowledge_graph_ecs_decision_memo.md)
- [Spine Concern](../../spine/README.md)
- [Multi-Domain Spine](../../events/multi_domain_spine.md)
