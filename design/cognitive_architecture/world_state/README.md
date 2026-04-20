# World State Domain

Date: 2026-04-20
Status: active
Scope: canonical belief ownership, knowledge graph projection, and temporal provenance over shared observations

## Thesis

`world_state` now has three visible strata:

- `graph`
  what is current and how to reach it
- legacy claim projection
  compatibility over execution outcomes and graph anchors
- `belief`
  whether current should still be trusted

`graph` and the shared `events` substrate are implemented.
They are no longer future assumptions.

The graph layer consumes canonical spine events through `DomainObjectRef` objects and `EventRelation` edges, materializes traversal indexes, selects current anchors, preserves anchor lineage, and emits durable derived anchor events back into the spine.

The older claim projection still exists as a compatibility surface.
It records generation and artifact availability claims, supports provenance and supersession queries, and now also has a traversal adapter for current frame-head style claims.

`belief` remains the future layer for confidence, contradiction, calibration, and curation.

The durable world model is a temporal knowledge graph with:

- thesis nodes for claims
- evidence edges for support and contradiction
- belief strength that can change over time
- provenance that preserves when and why a belief changed

## Boundary

`world_state` owns:

- the knowledge graph object model
- current anchor selection and traversal
- belief update and conflict resolution rules
- calibration and reflection over prediction quality
- projection of spine facts into current world-model state
- the canonical current-belief view used by planning

`sensory` owns observation production.
`execution` owns planning and side effects.
`events` owns ordering, replay, and cross-domain attachment primitives.
`telemetry` is now a downstream compatibility and reporting surface over that spine.

## Current Contract

- canonical spine events carry graph objects and relations
- graph materialization is replayable from spine history
- current anchors are selected by subject and perspective
- provenance is preserved through source facts, derived facts, and lineage
- traversal crosses workspace, context, execution, task, workflow, and branch scoped graph surfaces
- branch reads preserve branch presence and provenance
- belief remains a separate layer above traversal

## Graph

- [Graph](graph/README.md)
  current anchor selection, lineage, provenance, and graph walk
- [Temporal Fact Graph](graph/temporal_fact_graph.md)
  implemented graph model for the graph layer and its spine contract
- [Completed Graph Implementation History](../../completed/world_state/graph/README.md)
  completed delivery trackers, migration notes, and implementation closeout records

## Belief

- [Belief](belief/README.md)
  confidence, revision, contradiction, and settlement
- [Curation In Belief](belief/curation.md)
  natural runtime for belief maintenance and materialized belief
- [Knowledge Graph ECS Decision Memo](belief/knowledge_graph_ecs_decision_memo.md)
  ECS evaluation for curation internals, migration cost, and recommendation

## First Slice Boundary

The graph slice is now part of the baseline architecture.

The remaining world-state design work starts above that baseline:

- define belief records and settlement policy
- define curation replay and worker ownership
- decide how legacy claim projection retires into belief
- add confidence, contradiction, calibration, and revision semantics

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Graph](graph/README.md)
- [Temporal Fact Graph](graph/temporal_fact_graph.md)
- [Completed Graph Implementation History](../../completed/world_state/graph/README.md)
- [Belief](belief/README.md)
- [Curation In Belief](belief/curation.md)
- [Knowledge Graph ECS Decision Memo](belief/knowledge_graph_ecs_decision_memo.md)
- [Sensory Domain](../sensory/README.md)
- [Spine Concern](../spine/README.md)
- [Multi-Domain Spine](../events/multi_domain_spine.md)
- [Bayesian Evaluation Example](../execution/examples/bayesian_evaluation.md)
