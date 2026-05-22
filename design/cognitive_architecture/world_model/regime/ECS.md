# Regime ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/regime` as a structural uncertainty domain

## Thesis

`regime` is the domain that decides whether new evidence still belongs to the same structural world.

It is system-heavy because it compares continuation and break explanations, manages active segments, and projects mixture predictions.
It consumes signals from `graph`, `belief`, and `causation` rather than duplicating their logic.

Detailed regime entity, component, system, and requirement definitions live in:

- [Regime Entities](entities.md)
- [Regime Components](components.md)
- [Regime Systems](systems.md)
- [Regime Requirements](requirements.md)

## Entities

The core regime entities are:

- `Regime`
  one recurring operating mode
- `ActiveSegment`
  one currently active local segment
- `ChangepointCandidate`
  one possible break point
- `ContinuationModel`
  one explanation that current evidence still fits the active regime
- `BreakModel`
  one explanation that a new segment is more plausible
- `RegimePosterior`
  one probability mass over candidate regimes
- `RunLengthBelief`
  one belief over time since last break
- `RegimeLibraryEntry`
  one archived reusable regime
- `RegimeEntryPrior`
  one prior used when a new segment begins
- `MixturePrediction`
  one unresolved multi-regime forecast
- `RegimeSensitivity`
  one structural fragility summary
- `StressScenario`
  one coherent structural stress case
- `StressResult`
  one evaluated stress scenario result
- `RegimeSummary`
  one planner-facing regime projection

## Components

The component families are:

- identity
- segment
- signal
- changepoint
- model comparison
- posterior
- archive and priors
- mixture
- sensitivity
- stress
- provenance

## Systems

The core regime systems are:

- signal ingestion
- changepoint detection
- continuation modeling
- break modeling
- continuation versus break comparison
- posterior update
- run length update
- active segment rollover
- regime archive management
- entry prior selection
- mixture projection
- sensitivity projection
- stress evaluation
- summary projection
- recovery and replay

## Role In The Set

`regime` consumes surprise, contradiction clusters, cadence shifts, calibration drift, and effectiveness shifts.

`belief` may carry break pressure.
`regime` owns the structural decision about whether that pressure implies a new segment or unresolved mixture state.

`agent` consumes regime sensitivity through a lens, not through raw regime worker state.

## Read With

- [Regime Layer](README.md)
- [Regime Entities](entities.md)
- [Regime Components](components.md)
- [Regime Systems](systems.md)
- [Regime Requirements](requirements.md)
- [Belief ECS](../belief/ECS.md)
- [Causal ECS](../causation/ECS.md)
