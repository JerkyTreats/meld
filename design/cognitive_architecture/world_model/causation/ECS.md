# Causal ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/causation` as a mechanism and intervention domain

## Thesis

`causation` is a structured interpretation domain over graph and belief outputs.

It is system-heavy because it links interventions, outcomes, selection paths, and confounders into effect estimates.
It consumes settled evidence and emits mechanism-aware summaries.

Detailed causal entity, component, system, and requirement definitions live in:

- [Causal Entities](entities.md)
- [Causal Components](components.md)
- [Causal Systems](systems.md)
- [Causal Requirements](requirements.md)

## Entities

The core causal entities are:

- `CausalVariable`
  one modeled variable in the causal layer
- `MechanismVersion`
  one regime-scoped mechanism candidate
- `InterventionRecord`
  one attempted intervention context
- `OutcomeLink`
  one evidence link from intervention to measured outcome
- `ConfounderHypothesis`
  one hidden-cause explanation candidate
- `IdentificationAssessment`
  one assessment of whether an effect is identifiable
- `EffectEstimate`
  one posterior effect summary
- `CounterfactualCase`
  one alternative-world evaluation
- `CausalAssumptionSet`
  one explicit assumption set for an estimate or counterfactual
- `CausalSummary`
  one planner-facing causal projection

## Components

The component families are:

- identity
- variable
- mechanism
- intervention
- outcome
- selection
- confounding
- identification
- effect
- counterfactual
- assumption
- provenance

## Systems

The core causal systems are:

- variable registration
- intervention lowering
- outcome linking
- measurement path interpretation
- mechanism selection
- confounder discovery
- confounder scoring
- identification assessment
- effect estimation
- counterfactual evaluation
- assumption projection
- causal summary projection
- recovery and replay

## Role In The Set

`causation` consumes graph structure and belief uncertainty, then emits effect summaries for `planner` and perspective consumers.

`agent` consumes causal summaries through an agent-specific lens.

## Read With

- [Causal Layer](README.md)
- [Causal Entities](entities.md)
- [Causal Components](components.md)
- [Causal Systems](systems.md)
- [Causal Requirements](requirements.md)
- [Graph ECS](../graph/ECS.md)
- [Belief ECS](../belief/ECS.md)
