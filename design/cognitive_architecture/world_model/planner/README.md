# World Model Planner

Date: 2026-04-30
Status: active
Scope: planner-facing world model projection for action-relevant reads

## Thesis

`world_model/planner` is the planner-facing projection of the modeled world.

Its job is to expose the current action-relevant view of belief, uncertainty, freshness, contradiction, causal effect, and regime sensitivity in a form that downstream planning can consume.

This area belongs to `world_model` because it is still an epistemic concern. It answers what appears to hold, what is unclear, what is risky, and what observation would most reduce uncertainty for a decision.

It does not own task decomposition, dispatch, continuation, repair, or live runtime coordination. Those concerns belong to `execution`.

## Boundary

`world_model/planner` owns:

- action-relevant world model reads
- belief summaries for decision making
- uncertainty and freshness summaries
- contradiction and conflict summaries
- causal effect summaries where justified
- regime sensitivity summaries where material
- observation opportunity summaries
- abstention grounds
- execution precondition summaries as world-facing conditions

`world_model/planner` does not own:

- task graphs
- task network state
- dispatch rules
- continuation state
- observation wait mechanics
- repair flow
- retry policy
- execution control semantics
- provider or worker orchestration

## Relationship To Execution

`world_model/planner` is upstream of `execution`.

It tells `execution` what the current modeled world supports, blocks, or leaves uncertain.

`execution` decides how to operationalize that information through planning, control, dispatch, waiting, repair, and completion.

The boundary matters because these crates serve different authorities:

- `world_model` owns epistemic judgment
- `execution` owns operational commitment

This area may describe action relevance, but it must not drift into execution policy.

## Relationship To Belief, Causation, And Regime

`world_model/planner` is not a separate inference system.

It is the projection layer over:

- belief for confidence, uncertainty, contradiction, and freshness
- causation for intervention and effect summaries
- regime for structural context and sensitivity

Its responsibility is to present those concerns in decision-relevant form without exposing their internal machinery.

## Durable Concepts

- `WorldModelView`
- `ActionableBeliefView`
- `ObservationPolicy`
- `ExpectedInformationGain`
- `DecisionRelevance`
- `AbstentionState`
- `CausalEffectSummary`
- `RiskEnvelope`
- `ExecutionPreconditions`

## Non Goals

This area should not become the home of execution planning.

It should not define task structure, runtime suspension, control graphs, or repair semantics.

It should not require downstream consumers to understand raw graph traversal, event replay, or belief revision internals.

## Read With

- [Planner ECS](ECS.md)
- [World Model Belief](../belief/README.md)
- [Causal Layer](../causation/README.md)
- [Regime Layer](../regime/README.md)
- [Execution Domain](../../execution/README.md)
