# Regime Entities

Date: 2026-05-16
Status: active
Scope: ECS entities for `world_model/regime`

## Thesis

Regime entities are stable identities for recurring operating modes, active segments, changepoints, continuation and break explanations, posterior state, archival priors, mixture forecasts, sensitivity summaries, and stress scenarios.

They preserve structural uncertainty.
They do not settle local belief, estimate causal effects, or choose planner actions.

## Entity Categories

| Category | Entities | Persistence expectation |
|---|---|---|
| Regime identity | `Regime` | durable when known or archived |
| Active context | `ActiveSegment` | durable current state |
| Break evidence | `ChangepointCandidate` | durable when material |
| Explanation models | `ContinuationModel`, `BreakModel` | materialized projection or durable revision |
| Posterior state | `RegimePosterior`, `RunLengthBelief` | materialized projection |
| Archive | `RegimeLibraryEntry`, `RegimeEntryPrior` | durable |
| Forecast | `MixturePrediction` | materialized projection |
| Sensitivity | `RegimeSensitivity` | materialized projection |
| Stress | `StressScenario`, `StressResult` | durable when evaluated |
| Consumer view | `RegimeSummary` | materialized projection |

## Identity Rules

| Entity | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `Regime` | regime family plus learned signature plus archive version | none | regime |
| `ActiveSegment` | subject scope plus branch scope plus segment start cursor | one or more regime candidates | regime |
| `ChangepointCandidate` | active segment plus candidate break cursor plus detector version | `ActiveSegment` | regime |
| `ContinuationModel` | active segment plus evidence window plus method version | `ActiveSegment` | regime |
| `BreakModel` | active segment plus candidate break cursor plus evidence window plus method version | `ChangepointCandidate` | regime |
| `RegimePosterior` | active segment plus candidate regime set plus evidence window plus method version | `ActiveSegment` | regime |
| `RunLengthBelief` | active segment plus evidence window plus method version | `ActiveSegment` | regime |
| `RegimeLibraryEntry` | regime id plus archive version | `Regime` | regime |
| `RegimeEntryPrior` | regime id plus prior version plus source segment | `RegimeLibraryEntry` | regime |
| `MixturePrediction` | active segment plus candidate regime set plus horizon plus method version | `ActiveSegment` | regime |
| `RegimeSensitivity` | active segment plus affected output plus sensitivity version | `ActiveSegment` | regime |
| `StressScenario` | scenario family plus perturbation set plus scope | none | regime |
| `StressResult` | stress scenario plus active segment plus method version | `StressScenario` | regime |
| `RegimeSummary` | active segment plus posterior version plus summary version | `ActiveSegment` | regime |

## `Regime`

One recurring operating mode.

Purpose:
names a structural context whose priors, mechanisms, observation cadence, calibration behavior, and action effectiveness may recur.

Required components:

- `RegimeIdentity`
- `RegimeSignature`
- `RegimePriorState`
- `RegimeArchiveState`
- `RegimeProvenance`

Creation:
created by regime archive management when an active segment is confirmed, archived, or matched to recurrence.

Lifecycle:
candidate, active, archived, deprecated, merged.

Dependencies:
belief drift, causal effectiveness shifts, graph stability signals, archive policy.

## `ActiveSegment`

One currently active local segment.

Purpose:
records the current structural interval being evaluated for continuation, break, posterior state, and mixture prediction.

Required components:

- `SegmentIdentity`
- `SegmentScope`
- `SegmentBoundary`
- `SegmentStatus`
- `SegmentProvenance`

Creation:
created at boot, after a confirmed break, or during replay from segment boundary records.

Lifecycle:
open, uncertain, closing, closed, archived.

Dependencies:
subject scope, branch scope, reference time, graph and belief source cursors.

## `ChangepointCandidate`

One possible structural break.

Purpose:
records candidate break location, evidence window, break pressure, and affected signals.

Required components:

- `ChangepointIdentity`
- `ChangepointWindow`
- `ChangepointEvidence`
- `ChangepointScore`
- `ChangepointProvenance`

Creation:
created by changepoint detection when evidence deviates from continuation expectations.

Lifecycle:
candidate, promoted, rejected, superseded, archived.

Dependencies:
belief surprise, contradiction clusters, graph cadence shifts, calibration drift, causal effect shifts.

## `ContinuationModel`

One explanation that current evidence still fits the active regime.

Purpose:
records how new evidence can be absorbed without starting a new segment.

Required components:

- `ContinuationScope`
- `ContinuationEvidence`
- `ContinuationScore`
- `ContinuationAssumptions`
- `ContinuationProvenance`

Creation:
created by continuation versus break comparison.

Lifecycle:
current, superseded, rejected, archived.

Dependencies:
active segment, evidence window, belief and graph signals, method version.

## `BreakModel`

One explanation that a new segment is more plausible.

Purpose:
records why current evidence is better explained as a structural break.

Required components:

- `BreakScope`
- `BreakEvidence`
- `BreakScore`
- `BreakAssumptions`
- `BreakProvenance`

Creation:
created by continuation versus break comparison for a changepoint candidate.

Lifecycle:
candidate, selected, rejected, archived.

Dependencies:
changepoint candidate, evidence window, belief drift, calibration drift, causal effectiveness shift.

## `RegimePosterior`

One probability mass over candidate regimes.

Purpose:
records unresolved or resolved belief about which regime is active.

Required components:

- `PosteriorScope`
- `CandidateRegimeSet`
- `PosteriorWeights`
- `PosteriorUncertainty`
- `PosteriorProvenance`

Creation:
created by posterior update after continuation and break comparison.

Lifecycle:
current, stale, superseded, archived.

Dependencies:
active segment, regime library, continuation model, break model, evidence window.

## `RunLengthBelief`

One belief over time since last break.

Purpose:
records segment age uncertainty and break timing uncertainty.

Required components:

- `RunLengthScope`
- `RunLengthDistribution`
- `RunLengthEvidence`
- `RunLengthProvenance`

Creation:
created by run length update during regime posterior update.

Lifecycle:
current, stale, superseded, archived.

Dependencies:
active segment boundary, changepoint candidates, evidence window.

## `RegimeLibraryEntry`

One archived reusable regime.

Purpose:
preserves recurring structural context for reuse, audit, smoothing, and prior selection.

Required components:

- `LibraryIdentity`
- `ArchivedSignature`
- `ArchivedPriors`
- `ArchivedCalibration`
- `LibraryProvenance`

Creation:
created by regime archive management when a segment closes or a recurrent regime is consolidated.

Lifecycle:
active archive, superseded, merged, deprecated.

Dependencies:
closed active segment, posterior state, calibration records, archive policy.

## `RegimeEntryPrior`

One prior used when a new segment begins.

Purpose:
initializes belief, causal, and goal-curation priors under a known or novel regime.

Required components:

- `EntryPriorScope`
- `EntryPriorValues`
- `EntryPriorUncertainty`
- `EntryPriorProvenance`

Creation:
created from regime library entry or widened for novel regime entry.

Lifecycle:
active, superseded, archived.

Dependencies:
regime library, active segment rollover, calibration state.

## `MixturePrediction`

One unresolved multi-regime forecast.

Purpose:
predicts under uncertainty across candidate regimes instead of forcing premature regime identity.

Required components:

- `MixtureScope`
- `MixtureCandidates`
- `MixtureWeights`
- `MixtureForecast`
- `MixtureProvenance`

Creation:
created when posterior mass remains distributed across regimes.

Lifecycle:
current, expired, resolved, superseded.

Dependencies:
regime posterior, run length belief, horizon, forecast method.

## `RegimeSensitivity`

One structural fragility summary.

Purpose:
records which beliefs, effects, risks, or planner outputs flip or weaken under regime uncertainty.

Required components:

- `SensitivityScope`
- `AffectedOutputSet`
- `SensitivityCondition`
- `SensitivitySeverity`
- `SensitivityProvenance`

Creation:
created by sensitivity projection from posterior and mixture state.

Lifecycle:
current, stale, superseded.

Dependencies:
regime posterior, mixture prediction, planner outputs, belief and causal summaries.

## `StressScenario`

One coherent structural stress case.

Purpose:
perturbs linked evidence channels, mechanisms, priors, or cadence assumptions to test brittleness.

Required components:

- `StressIdentity`
- `StressScope`
- `StressPerturbationSet`
- `StressPolicy`
- `StressProvenance`

Creation:
created by stress scenario registry, operator policy, or regime analysis.

Lifecycle:
available, active, deprecated, archived.

Dependencies:
evidence channels, belief families, causal effects, planner outputs.

## `StressResult`

One evaluated stress scenario result.

Purpose:
records flips, threshold crossings, recovery time, cascade depth, and blast radius.

Required components:

- `StressResultIdentity`
- `StressMetricSet`
- `StressAffectedOutputs`
- `StressResultProvenance`

Creation:
created by stress evaluation.

Lifecycle:
current, superseded, archived.

Dependencies:
stress scenario, active segment, regime posterior, affected outputs.

## `RegimeSummary`

One planner-facing regime projection.

Purpose:
exposes posterior, changepoint state, run length, continuation and break scores, mixture prediction, sensitivity, stress metrics, and active segment status.

Required components:

- `RegimeSummaryIdentity`
- `RegimePosteriorView`
- `ChangepointView`
- `MixtureView`
- `RegimeRiskView`
- `RegimeSummaryProvenance`

Creation:
created by regime summary projection.

Lifecycle:
current, stale, superseded, invalid.

Dependencies:
active segment, posterior, run length, mixture prediction, sensitivity, stress results.

## Entity Rules

- `ActiveSegment` is the current structural context root.
- `Regime` and `RegimeLibraryEntry` preserve recurring structure without erasing old priors.
- `ChangepointCandidate` records possible break evidence, not final break authority by itself.
- `ContinuationModel` and `BreakModel` are compared explicitly.
- `MixturePrediction` preserves unresolved identity instead of forcing a winner.
- `StressScenario` tests brittleness, not direct likelihood.
- `RegimeSummary` is the public consumer view.

## Read With

- [Regime Components](components.md)
- [Regime Systems](systems.md)
- [Regime Requirements](requirements.md)
- [Regime Layer](README.md)
