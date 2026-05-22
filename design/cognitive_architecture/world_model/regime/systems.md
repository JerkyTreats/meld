# Regime Systems

Date: 2026-05-16
Status: active
Scope: ECS systems for `world_model/regime`

## Thesis

Regime systems transform graph, belief, causal, execution, and calibration signals into changepoints, continuation and break explanations, active segments, posterior state, mixture forecasts, archived priors, sensitivity summaries, stress results, and planner-facing regime summaries.

They decide structural context.
They do not settle belief, estimate causal effects, or choose actions.

## Pipeline Overview

| Phase | System | Required input | Required output |
|---|---|---|---|
| 1 | signal ingestion | graph, belief, causal, calibration signals | `RegimeSignalSet` |
| 2 | changepoint detection | signal window, active segment | `ChangepointCandidate` |
| 3 | continuation modeling | active segment, signal window | `ContinuationModel` |
| 4 | break modeling | changepoint candidate, signal window | `BreakModel` |
| 5 | continuation versus break comparison | continuation and break models | selected structural posture |
| 6 | posterior update | model scores, priors, library candidates | `RegimePosterior` |
| 7 | run length update | changepoint state, segment boundary | `RunLengthBelief` |
| 8 | active segment rollover | selected break, entry prior | `ActiveSegment` update |
| 9 | regime archive management | closed segment, posterior, calibration | `RegimeLibraryEntry` |
| 10 | entry prior selection | regime posterior, library match | `RegimeEntryPrior` |
| 11 | mixture projection | unresolved posterior, horizon | `MixturePrediction` |
| 12 | sensitivity projection | posterior, mixture, world-model outputs | `RegimeSensitivity` |
| 13 | stress evaluation | stress scenario, active segment | `StressResult` |
| 14 | summary projection | posterior, changepoint, mixture, stress | `RegimeSummary` |
| 15 | recovery and replay | source refs, method versions, cursors | rebuilt regime projections |

Replay invariant:
same graph, belief, causal, execution, calibration, prior, method version, and source cursor inputs produce the same regime outputs.

## Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| graph object history and relation stability | graph | cadence and relation stability signals |
| belief revisions and views | belief | surprise and contradiction signals |
| belief calibration records | belief | calibration drift signals |
| causal effect estimates | causation | effectiveness shift signals |
| execution outcomes | execution through spine | calibration and action effectiveness |
| planner outputs | planner | sensitivity and stress affected outputs |
| agent goal priors | agent | goal curation prior selection |

## Signal Ingestion

Input:
graph history, belief views, belief revisions, calibration records, causal estimates, execution outcomes.

Output:
`RegimeSignalSet`.

Rules:

- Preserve source refs and evidence window.
- Compute surprise signals from prior to posterior movement.
- Group repeated contradictions into contradiction clusters.
- Detect observation cadence shifts from graph and evidence history.
- Detect calibration drift from outcome mismatch.
- Detect action effectiveness shifts from causal estimates and execution outcomes.
- Detect relation stability shifts from graph relation churn.

## Changepoint Detection

Input:
signal set and active segment.

Output:
`ChangepointCandidate`.

Rules:

- Produce candidate break points when signal pressure crosses detector threshold.
- Preserve pre-break and post-break windows.
- Preserve supporting and weakening signals.
- Avoid treating one residual as decisive by default.
- Emit no candidate when evidence fits continuation within tolerance.

## Continuation Modeling

Input:
active segment and signal window.

Output:
`ContinuationModel`.

Rules:

- Score whether current evidence still fits active regime.
- Record absorbed and unresolved signals.
- Preserve continuation assumptions.
- Keep continuation explanation separate from local belief settlement.

## Break Modeling

Input:
changepoint candidate and signal window.

Output:
`BreakModel`.

Rules:

- Score whether current evidence is better explained by a new segment.
- Record affected belief keys and effect estimates.
- Preserve break assumptions.
- Keep break explanation separate from final segment rollover.

## Continuation Versus Break Comparison

Input:
continuation model and break model.

Output:
selected structural posture.

Rules:

- Compare continuation score and break score with explicit threshold basis.
- Select continuation when evidence still fits active regime.
- Select break when break model dominates.
- Select unresolved mixture when posterior remains distributed.
- Preserve reason codes for structural posture.

## Posterior Update

Input:
model comparison, prior state, regime library candidates.

Output:
`RegimePosterior`.

Rules:

- Update posterior mass over known and novel regime candidates.
- Preserve unresolved mixture state.
- Preserve dominant regime when threshold is met.
- Preserve posterior uncertainty.
- Include novel regime candidate when archive match is weak.

## Run Length Update

Input:
active segment boundary and changepoint state.

Output:
`RunLengthBelief`.

Rules:

- Update distribution over time since last break.
- Preserve break timing uncertainty.
- Use candidate changepoints without rewriting segment history.
- Publish new run length belief when source evidence changes.

## Active Segment Rollover

Input:
selected break, entry prior, active segment.

Output:
closed old segment and opened new active segment.

Rules:

- Close current segment when break model wins.
- Archive closing segment before starting new segment.
- Start new segment at selected break boundary.
- Use known regime priors when archive match is strong.
- Use widened priors for novel regime.

## Regime Archive Management

Input:
closed segment, posterior state, calibration state.

Output:
`RegimeLibraryEntry`.

Rules:

- Archive old regime priors.
- Archive calibration memory.
- Merge with existing regime when recurrence match is strong.
- Preserve old entries when superseded.
- Never erase priors silently.

## Entry Prior Selection

Input:
regime posterior, regime library, active segment rollover.

Output:
`RegimeEntryPrior`.

Rules:

- Select archived priors for known regime entry.
- Widen priors for novel regime entry.
- Preserve prior source regime when available.
- Expose uncertainty for goal curation and belief assessment.

## Mixture Projection

Input:
unresolved posterior and horizon.

Output:
`MixturePrediction`.

Rules:

- Forecast over candidate regimes using posterior weights.
- Include novel regime mass when present.
- Preserve expiry horizon.
- Avoid forcing a single regime when posterior is unresolved.

## Sensitivity Projection

Input:
regime posterior, mixture prediction, belief views, causal summaries, planner outputs.

Output:
`RegimeSensitivity`.

Rules:

- Identify outputs that flip under candidate regime changes.
- Identify outputs that weaken under unresolved mixture state.
- Mark planner-blocking sensitivity when threshold is crossed.
- Preserve affected belief keys, effect estimates, and planner outputs.

## Stress Evaluation

Input:
stress scenario, active segment, posterior, affected outputs.

Output:
`StressResult`.

Rules:

- Apply linked perturbations coherently.
- Record posterior flips.
- Record threshold crossings.
- Record recovery time when available.
- Record cascade depth and blast radius.
- Keep stress result separate from posterior likelihood.

## Summary Projection

Input:
active segment, posterior, changepoint state, run length, mixture, sensitivity, stress results.

Output:
`RegimeSummary`.

Rules:

- Expose regime posterior.
- Expose changepoint state.
- Expose run length belief.
- Expose continuation and break scores.
- Expose mixture prediction.
- Expose sensitivity and stress metrics.
- Expose active segment status.
- Exclude raw worker internals.

## Recovery And Replay

Input:
source refs, regime records, method versions, source cursors.

Output:
rebuilt regime projections.

Rules:

- Rebuild signal sets from lower-layer source refs.
- Rebuild changepoint candidates from detector version.
- Rebuild posterior state from model scores and priors.
- Preserve old posterior outputs when method version changes by publishing new outputs.
- Do not rewrite archived regime history.

## Failure Semantics

| Condition | Regime behavior |
|---|---|
| missing graph signal | emit incomplete signal set |
| missing belief calibration | widen posterior uncertainty |
| isolated residual | keep continuation unless threshold is crossed |
| break model dominates | close active segment and open new segment |
| posterior unresolved | publish mixture prediction |
| archive match weak | include novel regime candidate |
| stress scenario lacks affected outputs | mark stress invalid |
| source evidence changes | publish new posterior or summary |

## Test Matrix

| Test | Expected proof |
|---|---|
| signal ingestion | lower-layer refs survive into signal set |
| isolated residual | one residual does not force break |
| contradiction cluster | repeated contradictions raise break pressure |
| continuation win | active segment remains open |
| break win | old segment closes and new segment opens |
| posterior unresolved | mixture prediction is published |
| archive known regime | archived priors are selected |
| novel regime | priors widen |
| sensitivity | affected planner output is named |
| stress metrics | flips and threshold crossings are recorded |
| replay | same inputs and method versions produce same summary |

## System Rules

- Systems are deterministic over explicit inputs and method versions.
- Systems preserve lower-layer source refs.
- Systems compare continuation and break explicitly.
- Systems preserve unresolved mixture state.
- Systems archive old regimes rather than erasing priors.
- Systems publish summaries, not planner actions.

## Read With

- [Regime Entities](entities.md)
- [Regime Components](components.md)
- [Regime Requirements](requirements.md)
- [Regime Layer](README.md)
