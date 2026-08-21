# Regime Requirements

Scope: canonical requirements for `world_model/regime`

## Thesis

Regime proves a replayable path from graph, belief, causal, execution, and calibration signals to changepoints, continuation and break explanations, posterior state, run length, segment rollover, archived priors, mixture predictions, sensitivity summaries, stress results, and planner-facing summaries.

Regime protects the system from silently treating structural change as ordinary local evidence.

## Functional Requirements

### Signals

- Consume graph relation stability, object history, and observation cadence signals.
- Consume belief surprise, contradiction clusters, freshness shifts, and calibration drift.
- Consume causal effectiveness shifts and selection warnings.
- Consume execution outcomes through calibration records.
- Preserve source refs and evidence windows for every signal.

### Changepoints

- Detect candidate break points from structural signal pressure.
- Preserve pre-break and post-break windows.
- Preserve supporting and weakening signals.
- Avoid treating one residual as decisive by default.

### Continuation And Break Models

- Produce a continuation explanation for evidence that still fits the active regime.
- Produce a break explanation for evidence better explained by a new segment.
- Compare continuation and break scores explicitly.
- Preserve reason codes and method versions.

### Posterior And Run Length

- Maintain posterior mass over known and novel regime candidates.
- Preserve unresolved mixture state.
- Track dominant regime when threshold is met.
- Maintain run length belief over time since last break.
- Preserve break timing uncertainty.

### Segment Rollover

- Keep one active segment per scoped structural context.
- Close current segment when break model wins.
- Archive current segment before opening the next segment.
- Open new segment at selected break boundary.
- Preserve segment history.

### Archive And Priors

- Archive regime signatures, priors, calibration memory, and source refs.
- Reuse archived priors when a known regime recurs.
- Widen priors when a novel regime begins.
- Preserve old priors rather than overwriting them.

### Mixture Prediction

- Publish mixture prediction when posterior identity is unresolved.
- Weight forecasts by posterior regime mass.
- Include novel regime mass when present.
- Preserve expiry horizon.

### Sensitivity

- Identify beliefs, effects, risks, and planner outputs that flip or weaken under regime uncertainty.
- Preserve affected outputs and thresholds.
- Mark planner-blocking sensitivity when applicable.

### Stress

- Define stress scenarios over linked evidence channels, priors, mechanisms, or cadence assumptions.
- Evaluate posterior flips, threshold crossings, recovery time, cascade depth, and blast radius.
- Keep stress results separate from posterior likelihood.

### Summaries

- Publish `RegimeSummary` for planner and perspective consumers.
- Include posterior, changepoint state, run length, continuation score, break score, mixture prediction, sensitivity, stress metrics, and active segment status.
- Exclude raw worker internals and uncommitted drafts.

## Public Interface Requirements

- Query active regime summary by subject scope and branch scope.
- Query changepoint state for a recent window.
- Query run length belief for an active segment.
- Query mixture prediction for a decision horizon.
- Query regime sensitivity for a planner context.
- Query stress results for a scenario and active segment.
- Query archived regime priors.
- Query regime library entries by signature.

## Boundary Requirements

- Regime consumes graph, belief, causation, execution, and calibration signals.
- Regime owns changepoints, segments, regime identity, mixture prediction, archived priors, and structural stress.
- Regime does not settle belief.
- Regime does not estimate causal effects.
- Regime does not choose planner actions.
- Regime does not dispatch tasks.
- Planner consumes regime summaries, not raw regime worker state.

## Nonfunctional Requirements

### Replay

- Every regime output is rebuildable from source refs, priors, method versions, and source cursors.
- Segment rollover is replayable from changepoint decisions.
- Archive entries preserve source segment refs.

### Determinism

- Same source signals and method versions produce same changepoints, posterior state, mixture predictions, and summaries.
- Method changes publish new outputs instead of editing old outputs.

### Audit

- Source refs survive signal ingestion, changepoint detection, comparison, posterior update, archive, mixture, sensitivity, stress, and summary projection.
- Break, continuation, mixture, and stress reasons remain inspectable.

### Stability

- One residual is insufficient for break authority by default.
- Unresolved posterior stays unresolved through mixture prediction.
- Old regime priors are archived before new priors are selected.

## Read With

- [Regime Spec](spec.md)
- [Regime Layer](README.md)
