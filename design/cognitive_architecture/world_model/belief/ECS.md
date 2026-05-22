# Belief ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/belief` as a curation-heavy inference domain

## Thesis

`belief` is the heaviest mutable curation domain in `world_model`.

It is more system-heavy than `agent` because it normalizes evidence, schedules assessment, updates posterior state, and projects belief views.
It is more perspective-sensitive than `graph` because one shared substrate may produce many scoped belief views.

This is the clearest internal home for hybrid ECS.

Detailed belief entity, component, system, and requirement definitions live in:

- [Belief Entities](entities.md)
- [Belief Components](components.md)
- [Belief Systems](systems.md)
- [Belief Requirements](requirements.md)

## Entities

The core belief entities are:

- `Belief`
  one stable assessed question
- `EvidenceItem`
  one normalized belief input
- `BeliefRevision`
  one append-only settlement over an evidence window
- `AssessmentLease`
  one durable coordination record for assessment work
- `InferenceEpoch`
  one bounded multi-belief inference pass
- `HypothesisSet`
  one competing set of latent explanations
- `ObservationOpportunity`
  one decision-relevant information need
- `BeliefView`
  one current shaped projection for consumers
- `CalibrationRecord`
  one outcome-based calibration update

## Components

The component families are:

- identity
- scope and policy
- evidence
- assessment
- posterior state
- conflict and hypotheses
- observation
- view
- calibration
- provenance

## Systems

The core belief systems are:

- fact promotion
- evidence normalization
- belief key assignment
- dirty marking
- comparator scheduling
- comparator execution
- inference epoch execution
- revision commit
- stale detection
- contradiction handling
- belief view projection
- observation opportunity projection
- calibration ingestion
- recovery scan
- storm coalescing

## Role In The Set

`belief` is where graph evidence becomes posterior state.

`agent` consumes belief outputs through lenses and scoped views.
`planner` consumes belief summaries rather than raw belief worker state.
`regime` consumes contradiction, surprise, drift, and calibration signals without taking over belief revision.

## Read With

- [World Model Belief](README.md)
- [Belief Entities](entities.md)
- [Belief Components](components.md)
- [Belief Systems](systems.md)
- [Belief Requirements](requirements.md)
- [Belief Substrate](substrate.md)
- [Knowledge Graph ECS Decision Memo](knowledge_graph_ecs_decision_memo.md)
