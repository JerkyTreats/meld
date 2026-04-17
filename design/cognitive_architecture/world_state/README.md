# World State Domain

Date: 2026-04-12
Status: active
Scope: canonical belief ownership, knowledge graph projection, and temporal provenance over shared observations

## Thesis

`world_state` now splits into two internal concerns:

- `graph`
  what is current and how to reach it
- `belief`
  whether current should still be trusted

Within `belief`, `curation` is the primary merge activity.
It turns observations and task outcomes into current belief.

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
`spine` owns ordering, replay, and cross-domain attachment primitives.

## Current Anchors

- the prior reducer is already a narrow belief projection fed by historical outcomes
- the frame and head system already acts like a current-anchor graph over filesystem nodes and agent perspectives
- the multi-domain spine already reserves `knowledge_graph` as a future attachment domain

## Graph

- [Graph](graph/README.md)
  current anchor selection, lineage, provenance, and graph walk
- [Temporal Fact Graph](graph/temporal_fact_graph.md)
  canonical graph model for the graph layer and its spine contract
- [Graph Implementation Plan](graph/implementation_plan.md)
  phased delivery plan and branch scope for graph work
- [Workspace FS Graph Transition Requirements](graph/workspace_fs_transition_requirements.md)
  compatibility-led lift of `workspace_fs` into canonical graph inputs without breaking `NodeID` flows
- [Root Federation Runtime](graph/root_federation_runtime.md)
  runtime discovery, safe migration, and operator trigger flow for federated roots

## Belief

- [Belief](belief/README.md)
  confidence, revision, contradiction, and settlement
- [Curation In Belief](belief/curation.md)
  natural runtime for belief maintenance and materialized belief
- [Knowledge Graph ECS Decision Memo](belief/knowledge_graph_ecs_decision_memo.md)
  ECS evaluation for curation internals, migration cost, and recommendation

## Required First Slice

- define graph records for current anchor, lineage, and provenance
- define reducer inputs from spine facts into graph projections
- anchor graph walk to `DomainObjectRef` so the model can extend beyond filesystem nodes
- define a later belief layer for confidence, contradiction, and calibration

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Graph](graph/README.md)
- [Temporal Fact Graph](graph/temporal_fact_graph.md)
- [Graph Implementation Plan](graph/implementation_plan.md)
- [Workspace FS Graph Transition Requirements](graph/workspace_fs_transition_requirements.md)
- [Root Federation Runtime](graph/root_federation_runtime.md)
- [Belief](belief/README.md)
- [Curation In Belief](belief/curation.md)
- [Knowledge Graph ECS Decision Memo](belief/knowledge_graph_ecs_decision_memo.md)
- [Sensory Domain](../sensory/README.md)
- [Spine Concern](../spine/README.md)
- [Multi-Domain Spine](../events/multi_domain_spine.md)
- [Bayesian Evaluation Example](../execution/examples/bayesian_evaluation.md)
