# Planner ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/planner` as a projection-heavy assembly domain

## Thesis

`planner` is a projection domain.

It is less about deep inference and more about assembling action-relevant world-model views from `belief`, `causation`, and `regime`.
It is still inside `world_model`, so it remains epistemic rather than operational.

## Entities

The core planner entities should be:

- `WorldModelView`
  one assembled action-relevant world-model projection
- `ActionableBeliefView`
  one belief summary with decision relevance
- `ObservationPolicy`
  one candidate information-gathering policy
- `RiskEnvelope`
  one compact risk summary
- `ExecutionPreconditions`
  one compact precondition set
- `AbstentionState`
  one explicit no-commit state

## Components

The most useful planner components are:

- decision context
- belief refs
- uncertainty summary
- freshness summary
- contradiction summary
- causal summary
- regime sensitivity summary
- observation policy fields
- abstention reason
- risk summary
- hydration handles

## Systems

The core planner systems should be:

- planner-view assembly
- decision relevance scoring
- observation policy projection
- abstention projection
- risk-envelope projection
- execution-precondition projection
- hydration-handle projection

## Role In The Set

`planner` should consume shaped outputs from the lower world model domains and emit one action-relevant view.

`agent` should consume that view as part of its perspective assembly.
`execution` should consume the shaped result, not internal planner ECS state.

## Read With

- [World Model Planner](README.md)
- [Belief ECS](../belief/ECS.md)
- [Causal ECS](../causation/ECS.md)
- [Regime ECS](../regime/ECS.md)
