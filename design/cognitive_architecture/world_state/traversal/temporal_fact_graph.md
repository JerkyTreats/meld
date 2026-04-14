# Temporal Fact Graph

Date: 2026-04-12
Status: active
Scope: canonical graph model for `world_state/traversal`, built downstream of the shared event spine

## Thesis

The canonical graph for `world_state/traversal` should be a temporal fact graph.

This means:

- the spine is the durable history of semantic fact commits
- the graph is the current traversal surface
- the graph is materialized from spine history
- the graph can emit new curation facts into the spine, but it does not rewrite history directly

This is the clean split between:

- how current anchors became available
- what is currently reachable

## Core Position

Do not make the graph and the spine the same storage role.

The spine should own:

- durable append
- total commit order
- replay
- cross-domain attachment

Traversal should own:

- current anchors
- current relations
- supersession state
- provenance views
- planner-facing and operator-facing projections

The traversal surface must always be derivable from the spine.

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

The graph must stop assuming filesystem identity is universal.

The cross-domain anchor should be `DomainObjectRef`.

Each durable object reference should carry:

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

`world_state` may also maintain an internal entity identifier for live materialization, but the public durable anchor should remain `DomainObjectRef`.

## Fact Shape

Each spine fact that is relevant to `world_state` should be reducible into a typed fact record.

Minimum durable fact shape:

```rust
struct TemporalFactRecord {
    fact_id: String,
    seq: u64,
    domain_id: String,
    stream_id: String,
    event_type: String,
    occurred_at: String,
    recorded_at: String,
    content_hash: Option<String>,
    objects: Vec<DomainObjectRef>,
    relations: Vec<FactRelation>,
    payload: serde_json::Value,
}

struct FactRelation {
    relation_type: String,
    src: DomainObjectRef,
    dst: DomainObjectRef,
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
- evidence links where available
- provenance links
- current head selection
- current lineage state

This layer answers what is current and how to reach it.
It does not by itself answer whether current is still credible.
That is the job of `world_state/belief`.

## Authority Model

Authority should flow one way:

- spine to graph is authoritative

Feedback still exists:

- reducers may publish new curation facts back into the spine

The safe rule is:

- no direct graph mutation is canonical unless it is represented as a spine fact

So the loop is:

1. `workspace_fs`, `execution`, or later `sensory` publishes semantic facts into the spine
2. `world_state/traversal` reducers consume those facts
3. reducers update current graph state
4. reducers may emit new `world_state.*` traversal facts
5. later replay can rebuild the same traversal graph

This preserves both provenance and replay discipline.

## Reducer Model

The graph should be materialized by traversal reducers, not by arbitrary writers.

Reducers should be partitionable by:

- object identity
- relation family
- stream
- projection family

This is the path to high parallelism.

The first reducer families should be:

- observation attachment
  attach promoted observations and execution outcomes to durable objects
- anchor selection
  select the current reachable artifact, frame, or head for one object and one perspective
- provenance threading
  preserve why one current anchor is selected

Each reducer should consume the spine, maintain one materialized view, and expose replay from the last applied `seq`.

## Graph Indexes

Do not answer graph questions by scanning the whole spine.

The graph needs explicit indexes.

Minimum traversal indexes:

- `fact_id -> fact`
- `object_ref -> fact_ids`
- `fact_id -> object_refs`
- `object_ref + relation_type -> current neighbors`
- `claim_id -> supporting evidence`
- `claim_id -> contradicting evidence`
- `claim_id -> supersession chain`
- `stream_id -> fact_ids`
- `domain_id -> fact_ids`
- `seq -> fact_id`

Minimum current state indexes:

- `object_ref -> current anchors`
- `object_ref -> current relations`
- `planner view key -> current settled value`
- `inspection view key -> provenance bundle`

These indexes are the real traversal engine.
The spine remains the durable source.

## Traversal Modes

A full temporal graph needs more than one traversal mode.

At minimum the system should support:

- temporal traversal by `seq`
- object history traversal by `DomainObjectRef`
- current relation traversal by settled graph adjacency
- provenance traversal from anchor to supporting facts
- lineage traversal from current anchor to prior anchor chain

The first spine refactor already gives temporal traversal.
`world_state/traversal` must add object and relation traversal on top.

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

`world_state` should define typed facts for:

- `world_state.entity_attached`
- `world_state.claim_added`
- `world_state.claim_revised`
- `world_state.claim_superseded`
- `world_state.evidence_attached`
- `world_state.calibration_recorded`

Each of these facts should attach to explicit `DomainObjectRef` values and explicit relation types.

## First Query Surfaces

The first graph should answer these queries cheaply:

- what do we currently believe about object X
- why do we believe claim Y
- what superseded claim Y
- which execution outcome changed belief about object X
- what planner-facing settled view exists for object X

If the first graph cannot answer those cleanly, it is not yet the right substrate.

## First Implementation Slice

The first `world_state` landing should do five things:

1. Define `DomainObjectRef` and typed relation records in the spine contract
2. Define `TemporalFactRecord` and one typed decoding layer for relevant spine facts
3. Build one object history index and one current claim index
4. Build one claim settlement reducer that consumes promoted facts and emits `world_state.claim_*` facts
5. Expose one planner-facing settled belief projection and one operator-facing provenance projection

This is enough to prove the model without forcing a full graph runtime rewrite.

## Non Goals

- raw sensory stream transport
- generic graph database integration
- distributed sequencer work
- arbitrary graph query language
- universal ECS adoption

## Read With

- [World State Domain](../README.md)
- [Traversal](README.md)
- [Belief](../belief/README.md)
- [Knowledge Graph ECS Decision Memo](../belief/knowledge_graph_ecs_decision_memo.md)
- [Spine Concern](../../spine/README.md)
- [Multi-Domain Spine](../../events/multi_domain_spine.md)
