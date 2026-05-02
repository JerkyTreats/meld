# Causal Layer

Date: 2026-04-26
Status: active
Scope: mechanism, intervention, confounding, and counterfactual semantics above belief and below planner-facing policy

## Thesis

The causal layer explains why changes happen and what would happen under a different action or context.

The state graph records temporal evidence.
The belief layer records what is credible.
The causal layer records which variables and mechanisms could produce those observations.

Anchors are evidence of selection and state.
They are not by themselves causal proof.

## Boundary

The causal layer owns:

- explicit causal variable families
- intervention semantics for execution
- outcome semantics for evaluation
- selection and measurement semantics
- confounder hypotheses
- effect estimates and uncertainty
- counterfactual query contracts

The causal layer does not own:

- raw event append
- temporal truth materialization
- generic belief freshness
- planner policy ranking

## Variable Families

The first durable split should be:

- `StateVariable`
  world condition that may persist or evolve
- `InterventionVariable`
  deliberate action, task, or control move
- `OutcomeVariable`
  observed utility, quality, success, or downstream effect
- `SelectionVariable`
  why one object, frame, or report became visible, chosen, or attached
- `RegimeVariable`
  latent or explicit context that changes mechanisms
- `ConfounderHypothesis`
  hidden cause that may influence both action and outcome

## Core Design Rule

Do not infer causality from temporal precedence or anchor replacement alone.

A selected frame may reflect:

- a real improvement in state
- a measurement policy choice
- a reporting artifact
- hidden context that drove both action and outcome
- a delayed effect from an earlier action

The causal layer exists to keep those possibilities separate.

## Durable Concepts

- `CausalVariable`
  stable identity for one modeled variable
- `CausalClaim`
  current causal hypothesis with provenance and uncertainty
- `InterventionRecord`
  what action was attempted, with intended target and scope
- `InterventionOutcomeLink`
  evidence that connects an intervention to measured outcomes
- `EffectEstimate`
  posterior estimate of effect size, direction, and uncertainty
- `CounterfactualQuery`
  query for a plausible alternative world under a different action
- `MechanismVersion`
  one regime-conditioned mechanism used for effect estimation

## Execution Interface

Execution should publish intervention-shaped facts that support causal reasoning.

Useful semantics include:

- intended target
- actual target when known
- intervention kind
- success or partial application
- delay window
- rollback or compensation
- measurement channel used for evaluation

The causal layer should treat these as candidate interventions until assumptions or evidence justify stronger claims.

## Queries

The first durable causal query families should be:

- What is the posterior effect of intervention `X` on outcome `Y`
- Which confounders still block identification for this effect
- Would outcome `Y` likely still hold without intervention `X`
- Which measurements are selection-shaped rather than outcome-shaped
- Which regime assumptions this effect estimate depends on

## Relationship To Other Layers

- spine provides action and outcome history
- state graph provides temporal state and measurement structure
- belief provides uncertainty and evidence normalization
- regime layer conditions which mechanisms are currently plausible
- planner surface consumes effect summaries, not raw causal graphs

## First Design Slice

The first slice should model a narrow but explicit chain:

- intervention attempt
- post-intervention observable state
- outcome quality
- selection or measurement path
- regime context

That is enough to prevent the common failure where a selected anchor is mistaken for a proven effect.

## Read With

- [Causal ECS](ECS.md)
- [Graph](../graph/README.md)
- [Belief](../belief/README.md)
- [Regime Layer](../regime/README.md)
