# World State Domain

Date: 2026-04-12
Status: active
Scope: canonical belief ownership, knowledge graph projection, and temporal provenance over shared observations

## Thesis

`world_state` owns the system's current belief about the world.
Within that domain, `curation` is the primary merge activity.
It turns observations and task outcomes into current belief.

The durable world model is a temporal knowledge graph with:

- thesis nodes for claims
- evidence edges for support and contradiction
- belief strength that can change over time
- provenance that preserves when and why a belief changed

## Boundary

`world_state` owns:

- the knowledge graph object model
- belief update and conflict resolution rules
- calibration and reflection over prediction quality
- projection of spine facts into current world-model state
- the canonical current-belief view used by planning

`sensory` owns observation production.
`execution` owns planning and side effects.
`spine` owns ordering, replay, and cross-domain attachment primitives.

## Current Anchors

- the prior reducer is already a narrow belief projection fed by historical outcomes
- the frame chain already acts like belief revision over time
- the multi-domain spine already reserves `knowledge_graph` as a future attachment domain

## Curation

- [Curation In World State](curation.md)
  living world-model engine, materialized belief, and ECS as a candidate internal substrate

## Required First Slice

- define thesis, evidence, supersession, and provenance records
- define reducer inputs from spine facts into knowledge graph projections
- anchor beliefs to `DomainObjectRef` so the model can extend beyond filesystem nodes
- define calibration records so predictor quality can improve from outcomes

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Curation In World State](curation.md)
- [Sensory Domain](../sensory/README.md)
- [Spine Concern](../spine/README.md)
- [Multi-Domain Spine](../events/multi_domain_spine.md)
- [Bayesian Evaluation Example](../execution/examples/bayesian_evaluation.md)
