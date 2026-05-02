# Causal ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/causation` as a mechanism and intervention domain

## Thesis

`causation` is a structured interpretation domain over graph and belief outputs.

It is system-heavy because it links interventions, outcomes, selection paths, and confounders into effect estimates.
It should remain narrower than `belief`.
It consumes settled evidence and emits mechanism-aware summaries.

## Entities

The core causal entities should be:

- `CausalVariable`
  one modeled variable in the causal layer
- `MechanismVersion`
  one regime-scoped mechanism candidate
- `InterventionRecord`
  one attempted intervention context
- `OutcomeLink`
  one evidence link from intervention to measured outcome
- `EffectEstimate`
  one posterior effect summary
- `CounterfactualCase`
  one alternative-world evaluation
- `ConfounderHypothesis`
  one hidden-cause explanation candidate

## Components

The most useful causal components are:

- variable kind
- parent refs
- intervention target
- intervention kind
- measurement path
- selection semantics
- confounder refs
- regime condition
- identification status
- effect posterior
- uncertainty
- provenance refs

## Systems

The core causal systems should be:

- intervention lowering
- outcome linking
- selection-path interpretation
- confounder scoring
- mechanism selection
- effect estimation
- counterfactual evaluation
- causal summary projection

## Role In The Set

`causation` should consume graph structure and belief uncertainty, then emit effect summaries for `planner` and perspective consumers.

`agent` should not re-run causal estimation.
It should consume causal summaries through an Agent-specific lens.

## Read With

- [Causal Layer](README.md)
- [Graph ECS](../graph/ECS.md)
- [Belief ECS](../belief/ECS.md)
