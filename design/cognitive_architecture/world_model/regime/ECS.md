# Regime ECS

Date: 2026-05-02
Status: active
Scope: ECS interpretation of `world_model/regime` as a structural uncertainty domain

## Thesis

`regime` is the domain that decides whether new evidence still belongs to the same structural world.

It is system-heavy because it compares continuation and break explanations, manages active segments, and projects mixture predictions.
It should consume signals from `graph`, `belief`, and `causation` rather than duplicating their logic.

## Entities

The core regime entities should be:

- `Regime`
  one recurring operating mode
- `ActiveSegment`
  one currently active local segment
- `ChangepointCandidate`
  one possible break point
- `RegimeLibraryEntry`
  one archived reusable regime
- `MixturePrediction`
  one unresolved multi-regime forecast
- `StressScenario`
  one coherent structural stress case

## Components

The most useful regime components are:

- regime id
- regime posterior
- run length posterior
- continuation score
- break score
- regime-entry prior
- archived prior ref
- active segment status
- sensitivity set
- stress metrics

## Systems

The core regime systems should be:

- changepoint detection
- continuation versus break comparison
- active segment rollover
- regime archive management
- mixture projection
- retrospective smoothing
- stress evaluation

## Role In The Set

`regime` should consume surprise, contradiction clusters, cadence shifts, calibration drift, and effectiveness shifts.

`belief` may carry break pressure.
`regime` owns the structural decision about whether that pressure implies a new segment or unresolved mixture state.

`agent` should consume regime sensitivity through a lens, not through raw regime worker state.

## Read With

- [Regime Layer](README.md)
- [Belief ECS](../belief/ECS.md)
- [Causal ECS](../causation/ECS.md)
