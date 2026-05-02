# Belief ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/belief` as a curation-heavy inference domain

## Thesis

`belief` is the heaviest mutable curation domain in `world_model`.

It is more system-heavy than `agent` because it normalizes evidence, schedules assessment, updates posterior state, and projects belief views.
It is more perspective-sensitive than `graph` because one shared substrate may produce many scoped belief views.

This is the clearest internal home for hybrid ECS.

## Entities

The core belief entities should be:

- `Belief`
  one stable assessed question
- `BeliefRevision`
  one append-only settlement over an evidence window
- `EvidenceItem`
  one normalized belief input
- `HypothesisSet`
  one competing set of latent explanations
- `ObservationOpportunity`
  one decision-relevant information need
- `BeliefView`
  one current shaped projection for consumers

## Components

The most useful belief components are:

- belief key
- perspective key
- prior state
- posterior state
- uncertainty
- precision
- freshness
- contradiction state
- origin state
- coverage state
- calibration state
- evidence refs
- comparator kind
- lease state
- assessment epoch
- provenance refs

## Systems

The core belief systems should be:

- evidence normalization
- belief key assignment
- comparator scheduling
- posterior update
- revision commit
- belief view projection
- observation opportunity projection
- recovery scan
- stale detection
- storm coalescing

## Role In The Set

`belief` is where graph evidence becomes posterior state.

`agent` should consume belief outputs through lenses and scoped views.
`planner` should consume belief summaries rather than raw belief worker state.
`regime` should consume contradiction, surprise, drift, and calibration signals without taking over belief revision.

## Read With

- [World Model Belief](README.md)
- [Belief Substrate](substrate.md)
- [Knowledge Graph ECS Decision Memo](knowledge_graph_ecs_decision_memo.md)
