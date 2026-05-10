# Regime Layer

Date: 2026-04-26
Status: active
Scope: changepoints, recurring operating modes, mixture prediction, and structural stress above belief and across causal hypotheses

## Thesis

The regime layer explains when the world model should stop treating new evidence as more of the same.

A sharp update in belief does not always mean a structural break.
A structural break does not always mean the old history should be discarded.

The regime layer exists to preserve this distinction.

## Boundary

The regime layer owns:

- changepoint inference
- run length beliefs
- regime identity and recurrence
- continuation versus break comparison
- mixture prediction while regime identity is uncertain
- regime-conditioned archival priors
- structural stress scenarios

The regime layer does not own:

- raw contradiction handling
- temporal truth maintenance
- task orchestration
- generic planner goals

## Durable Concepts

- `RegimeId`
  stable identity for a recurring operating mode
- `RegimePosterior`
  current probability mass over candidate regimes
- `ChangepointState`
  current belief about whether a break has occurred
- `RunLengthBelief`
  belief over time since the last break
- `ContinuationModel`
  explanation that current evidence still fits the active regime
- `BreakModel`
  explanation that a new segment is more plausible
- `RegimeEntryPrior`
  prior used when a new segment begins
- `MixturePrediction`
  prediction averaged over several plausible regimes
- `RegimeLibrary`
  archived recurring regimes for reuse, audit, and smoothing
- `StressScenario`
  coherent severe counterfactual used to test brittleness

## Core Design Rule

Do not erase priors when the world changes.

Archive old regimes.
Start a new active segment when a break model wins.
Use mixture prediction while the active regime remains uncertain.

## Signals

The regime layer should integrate signals from:

- prior and posterior surprise
- repeated local contradictions
- outcome calibration drift
- shift in observation cadence
- shift in action effectiveness
- shift in relation stability
- correlated failures across several belief families

No single residual should be treated as decisive by default.

## Relationship To Belief

Belief answers whether a proposition is supported now.
Regime answers whether the statistical and causal structure that supported earlier revisions still applies now.

This means one belief may be:

- locally settled
- globally fragile under regime uncertainty

The planner-facing view must be able to expose that difference.

## Relationship To Goal Curation

Regime change directly affects goal generation. The world model agent's cost-benefit comparators use regime-scoped priors. When a regime shift is detected:

- **Known regime**: archived priors from the regime library provide immediate recalibration. The agent's goal generation decisions change without relearning from scratch.
- **Novel regime**: priors widen. The agent becomes exploratory, favoring observation goals over action goals until outcome data arrives under the new regime.
- **Regime ends**: current priors are archived for future reuse.

The practical effect: regime change reshapes which goals the agent generates. During incident response, stability goals pass the cost-benefit threshold easily while documentation goals do not — not because of a hardcoded policy, but because the regime-scoped value priors reflect learned experience from prior incidents.

See [Goal Curation](../agent/goal_curation.md) for the full cost-benefit evaluation mechanism.

## Queries

The first durable regime query families should be:

- What regime is most likely active now
- How likely is a changepoint in this recent window
- Which beliefs are most sensitive to regime uncertainty
- Which archived regime best explains this segment
- What prediction should be used under unresolved regime identity
- Which stress scenarios flip key beliefs or action choices

## Structural Stress

Stress scenarios should be part of the regime layer, not an afterthought.

A useful stress scenario perturbs several linked evidence channels together and reports:

- posterior flips
- threshold crossings
- recovery time
- cascade depth
- blast radius

## First Design Slice

The first slice should support:

- one active segment
- one continuation model
- one break model
- one recurring regime archive
- one mixture prediction path

That is enough to prevent stationary priors from becoming silent hidden assumptions.

## Read With

- [Regime ECS](ECS.md)
- [Belief](../belief/README.md)
- [Causal Layer](../causation/README.md)
- [World Model Planner](../planner/README.md)
- [Goal Curation](../agent/goal_curation.md)
