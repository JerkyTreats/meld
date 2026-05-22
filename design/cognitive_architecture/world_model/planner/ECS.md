# Planner ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/planner` as a projection-heavy assembly domain

## Thesis

`planner` is a projection domain.

It is less about deep inference and more about assembling action-relevant world-model views from `belief`, `causation`, and `regime`.
It is still inside `world_model`, so it remains epistemic rather than operational.

The detailed entity, component, and system definitions live in:

- [Planner Entities](entities.md)
- [Planner Components](components.md)
- [Planner Systems](systems.md)

## Entities

The core planner entities should be:

- `DecisionContext`
  one scoped planning question over a perspective, subject set, time boundary, desired predicate, and optional candidate intervention
- `WorldModelView`
  one assembled action-relevant world-model projection
- `ActionableBeliefView`
  one belief summary with decision relevance
- `ObservationOpportunityView`
  one projected information-gathering opportunity
- `RiskEnvelope`
  one compact risk summary
- `PreconditionAssessment`
  one compact assessment of world-facing conditions
- `AbstentionState`
  one explicit no-commit state
- `CausalEffectSummary`
  one planner-facing summary of intervention effect and causal uncertainty
- `ConflictSummary`
  one planner-facing summary of unresolved contradiction or conflict
- `SensitivitySummary`
  one summary of lower-layer changes that would flip or weaken the view
- `AssumptionSet`
  one explicit set of causal, regime, coverage, and freshness assumptions
- `ViewSnapshot`
  one replay and validity boundary for an assembled view
- `HydrationHandle`
  one stable reference back to lower-layer evidence, graph, belief, causal, or regime records

`ExpectedInformationGain` and `DecisionRelevance` should be components or fields, not entities.
They are scores carried on projected views.

## Components

The most useful planner components are:

- decision context
- view snapshot
- projection version
- graph input refs
- belief refs
- uncertainty summary
- freshness summary
- contradiction summary
- origin summary
- coverage summary
- causal summary
- causal assumption refs
- regime sensitivity summary
- observation opportunity fields
- expected information gain
- decision relevance score
- precondition assessment fields
- abstention reason
- risk summary
- conflict summary
- sensitivity summary
- assumption set
- hydration handles

## Systems

The core planner systems should be:

- decision-context normalization
- graph-input projection
- belief-input projection
- causal-input projection
- regime-input projection
- planner-view assembly
- decision relevance scoring
- observation opportunity projection
- abstention projection
- risk-envelope projection
- precondition assessment projection
- conflict-summary projection
- sensitivity-summary projection
- assumption-set projection
- hydration-handle projection
- view-snapshot projection

## Lower-Layer Inputs

`planner` consumes shaped packets from lower world-model domains.
The packets remain owned by their source domains.

- `PlannerGraphInput`
  anchors, lineage, graph walks, object history, provenance, branch presence, and hydration handles
- `PlannerBeliefInput`
  posterior summary, uncertainty, precision, freshness, contradiction, origin, coverage, evidence refs, assessment state, and observation opportunities
- `PlannerCausalInput`
  intervention variables, outcome variables, effect estimates, identification status, confounder risk, selection warnings, counterfactual summaries, and causal assumptions
- `PlannerRegimeInput`
  regime posterior, changepoint state, run length, continuation and break scores, mixture prediction, sensitivity sets, stress metrics, and active segment status
- `PlannerAgentLens`
  perspective, trust profile, evidence policy, observation scope, tolerated uncertainty, and regime sensitivity profile

These inputs are decomposed into planner-facing entities through deterministic projection systems.

## Role In The Set

`planner` should consume shaped outputs from the lower world model domains and emit one action-relevant view.

`agent` should consume that view as part of its perspective assembly.
`execution` should consume the shaped result, not internal planner ECS state.

## Read With

- [World Model Planner](README.md)
- [Planner Entities](entities.md)
- [Planner Components](components.md)
- [Planner Systems](systems.md)
- [Belief ECS](../belief/ECS.md)
- [Causal ECS](../causation/ECS.md)
- [Regime ECS](../regime/ECS.md)
