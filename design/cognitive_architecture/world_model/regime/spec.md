# Regime Spec

`regime` is the domain that decides whether new evidence still belongs to the same structural world. It is pipeline-heavy because it compares continuation and break explanations, manages active segments, and projects mixture predictions. It consumes signals from `graph`, `belief`, and `causation` rather than duplicating their logic.

## Domain Types

### Type Categories

| Category | Types | Persistence expectation |
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

### Identity Rules

| Type | Identity rule | Required parent | Owning authority |
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

### `Regime`

One recurring operating mode.

Purpose:
names a structural context whose priors, mechanisms, observation cadence, calibration behavior, and action effectiveness may recur.

Required state:

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

### `ActiveSegment`

One currently active local segment.

Purpose:
records the current structural interval being evaluated for continuation, break, posterior state, and mixture prediction.

Required state:

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

### `ChangepointCandidate`

One possible structural break.

Purpose:
records candidate break location, evidence window, break pressure, and affected signals.

Required state:

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

### `ContinuationModel`

One explanation that current evidence still fits the active regime.

Purpose:
records how new evidence can be absorbed without starting a new segment.

Required state:

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

### `BreakModel`

One explanation that a new segment is more plausible.

Purpose:
records why current evidence is better explained as a structural break.

Required state:

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

### `RegimePosterior`

One probability mass over candidate regimes.

Purpose:
records unresolved or resolved belief about which regime is active.

Required state:

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

### `RunLengthBelief`

One belief over time since last break.

Purpose:
records segment age uncertainty and break timing uncertainty.

Required state:

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

### `RegimeLibraryEntry`

One archived reusable regime.

Purpose:
preserves recurring structural context for reuse, audit, smoothing, and prior selection.

Required state:

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

### `RegimeEntryPrior`

One prior used when a new segment begins.

Purpose:
initializes belief, causal, and goal-curation priors under a known or novel regime.

Required state:

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

### `MixturePrediction`

One unresolved multi-regime forecast.

Purpose:
predicts under uncertainty across candidate regimes instead of forcing premature regime identity.

Required state:

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

### `RegimeSensitivity`

One structural fragility summary.

Purpose:
records which beliefs, effects, risks, or planner outputs flip or weaken under regime uncertainty.

Required state:

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

### `StressScenario`

One coherent structural stress case.

Purpose:
perturbs linked evidence channels, mechanisms, priors, or cadence assumptions to test brittleness.

Required state:

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

### `StressResult`

One evaluated stress scenario result.

Purpose:
records flips, threshold crossings, recovery time, cascade depth, and blast radius.

Required state:

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

### `RegimeSummary`

One planner-facing regime projection.

Purpose:
exposes posterior, changepoint state, run length, continuation and break scores, mixture prediction, sensitivity, stress metrics, and active segment status.

Required state:

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

### Domain Type Rules

- `ActiveSegment` is the current structural context root.
- `Regime` and `RegimeLibraryEntry` preserve recurring structure without erasing old priors.
- `ChangepointCandidate` records possible break evidence, not final break authority by itself.
- `ContinuationModel` and `BreakModel` are compared explicitly.
- `MixturePrediction` preserves unresolved identity instead of forcing a winner.
- `StressScenario` tests brittleness, not direct likelihood.
- `RegimeSummary` is the public consumer view.

## Data Model

### Data Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable regime record addressing | regime processes | all regime processes |
| Segment | active structural interval state | segment management | all regime processes |
| Signal | graph, belief, causal, and calibration inputs | signal ingestion | changepoint and comparison |
| Changepoint | candidate break windows and scores | changepoint detection | comparison and rollover |
| Model comparison | continuation and break explanations | comparison processes | posterior update |
| Posterior | regime weights and run length | posterior update | mixture and summary |
| Archive and priors | reusable regime state and entry priors | archive management | posterior and agent priors |
| Mixture | unresolved multi-regime forecasts | mixture projection | planner and agent |
| Sensitivity | affected outputs and fragility | sensitivity projection | planner |
| Stress | scenario perturbations and metrics | stress evaluation | planner and operator |
| Provenance | source refs and hydration | every regime process | replay and explanation |

### Shape Rules

- Data carries typed structural state, not action policy.
- Signal data preserves graph, belief, causal, execution, and calibration refs.
- Posterior data keeps unresolved mixture state explicit.
- Archive data preserves old priors instead of overwriting them.
- Stress data records perturbation assumptions separately from posterior likelihood.
- Summary data excludes worker internals.

### Identity Data

| Data | Attached to | Purpose |
|---|---|---|
| `RegimeIdentity` | `Regime` | stable regime id |
| `SegmentIdentity` | `ActiveSegment` | stable segment id |
| `ChangepointIdentity` | `ChangepointCandidate` | stable changepoint id |
| `StressIdentity` | `StressScenario` | stable stress scenario id |

#### `RegimeIdentity`

Purpose:
stable identity for one recurring operating mode.

Dependencies:
regime signature, archive version, regime family.

```rust
struct RegimeIdentity {
    regime_id: RegimeId,
    regime_family: RegimeFamily,
    signature_hash: ContentHash,
    archive_version: SchemaVersion,
}
```

#### `SegmentIdentity`

Purpose:
stable identity for one active structural segment.

Dependencies:
subject scope, branch scope, segment start cursor.

```rust
struct SegmentIdentity {
    segment_id: ActiveSegmentId,
    subject_scope: SubjectScope,
    branch_scope: BranchScope,
    start_cursor: SourceCursor,
}
```

#### `ChangepointIdentity`

Purpose:
stable identity for one possible break point.

Dependencies:
active segment, candidate break cursor, detector version.

```rust
struct ChangepointIdentity {
    changepoint_id: ChangepointId,
    segment_id: ActiveSegmentId,
    candidate_break_cursor: SourceCursor,
    detector_version: SchemaVersion,
}
```

#### `StressIdentity`

Purpose:
stable identity for one structural stress scenario.

Dependencies:
scenario family, perturbation set, scope.

```rust
struct StressIdentity {
    stress_scenario_id: StressScenarioId,
    scenario_family: StressScenarioFamily,
    scope_hash: ContentHash,
    scenario_version: SchemaVersion,
}
```

### Segment Data

| Data | Attached to | Purpose |
|---|---|---|
| `SegmentScope` | `ActiveSegment` | scope of structural context |
| `SegmentBoundary` | `ActiveSegment` | start and end boundary |
| `SegmentStatus` | `ActiveSegment` | active segment lifecycle |

#### `SegmentScope`

Purpose:
records what the active segment covers.

Dependencies:
subject scope, branch scope, perspective policy.

```rust
struct SegmentScope {
    subject_scope: SubjectScope,
    branch_scope: BranchScope,
    perspective_scope: Option<PerspectiveScope>,
}
```

#### `SegmentBoundary`

Purpose:
records the structural interval boundary.

Dependencies:
source cursors and changepoint decisions.

```rust
struct SegmentBoundary {
    start_cursor: SourceCursor,
    end_cursor: Option<SourceCursor>,
    start_time: ReferenceTime,
    end_time: Option<ReferenceTime>,
}
```

#### `SegmentStatus`

Purpose:
records current segment lifecycle and uncertainty state.

Dependencies:
posterior update, changepoint state, rollover policy.

```rust
struct SegmentStatus {
    status: ActiveSegmentStatus,
    uncertainty_status: SegmentUncertaintyStatus,
    closing_reason: Option<ReasonCode>,
}
```

### Signal Data

| Data | Attached to | Purpose |
|---|---|---|
| `RegimeSignalSet` | `ActiveSegment`, `ChangepointCandidate` | input signal bundle |
| `SurpriseSignal` | `RegimeSignalSet` owner | belief surprise |
| `ContradictionClusterSignal` | `RegimeSignalSet` owner | repeated local contradiction |
| `CadenceShiftSignal` | `RegimeSignalSet` owner | observation cadence shift |
| `CalibrationDriftSignal` | `RegimeSignalSet` owner | outcome calibration drift |
| `EffectivenessShiftSignal` | `RegimeSignalSet` owner | causal effect shift |
| `RelationStabilitySignal` | `RegimeSignalSet` owner | graph relation stability shift |

#### `RegimeSignalSet`

Purpose:
collects structural signals for an evidence window.

Dependencies:
graph summaries, belief summaries, causal summaries, calibration records.

```rust
struct RegimeSignalSet {
    signal_window: EvidenceWindow,
    signal_refs: Vec<SourceRef>,
    signal_version: SchemaVersion,
}
```

#### `SurpriseSignal`

Purpose:
records prior to posterior surprise relevant to structural change.

Dependencies:
belief revisions and posterior summaries.

```rust
struct SurpriseSignal {
    affected_belief_keys: Vec<BeliefKey>,
    surprise_score: Score,
    source_refs: Vec<SourceRef>,
}
```

#### `ContradictionClusterSignal`

Purpose:
records clustered contradiction pressure across belief families.

Dependencies:
belief conflict summaries.

```rust
struct ContradictionClusterSignal {
    cluster_id: ContradictionClusterId,
    affected_belief_keys: Vec<BeliefKey>,
    cluster_score: Score,
}
```

#### `CadenceShiftSignal`

Purpose:
records shift in observation cadence.

Dependencies:
graph object history and evidence timestamps.

```rust
struct CadenceShiftSignal {
    observed_channel: EvidenceChannelRef,
    cadence_change: CadenceChange,
    severity: Score,
}
```

#### `CalibrationDriftSignal`

Purpose:
records drift between belief predictions and later outcomes.

Dependencies:
belief calibration records and execution outcomes.

```rust
struct CalibrationDriftSignal {
    affected_comparator_engines: Vec<ComparatorEngineId>,
    drift_score: Score,
    source_refs: Vec<SourceRef>,
}
```

#### `EffectivenessShiftSignal`

Purpose:
records shift in action or mechanism effectiveness.

Dependencies:
causal effect estimates and execution outcomes.

```rust
struct EffectivenessShiftSignal {
    affected_effects: Vec<EffectEstimateId>,
    shift_score: Score,
    source_refs: Vec<SourceRef>,
}
```

#### `RelationStabilitySignal`

Purpose:
records graph relation churn or disappearance.

Dependencies:
graph relation indexes and object history.

```rust
struct RelationStabilitySignal {
    relation_types: Vec<RelationType>,
    churn_score: Score,
    disappeared_relations: Vec<GraphRecordRef>,
}
```

### Changepoint Data

| Data | Attached to | Purpose |
|---|---|---|
| `ChangepointWindow` | `ChangepointCandidate` | candidate break interval |
| `ChangepointEvidence` | `ChangepointCandidate` | break evidence |
| `ChangepointScore` | `ChangepointCandidate` | break likelihood pressure |

#### `ChangepointWindow`

Purpose:
records the evidence window around a possible break.

Dependencies:
active segment boundary and source cursors.

```rust
struct ChangepointWindow {
    pre_break_window: EvidenceWindow,
    post_break_window: EvidenceWindow,
    candidate_break_time: ReferenceTime,
}
```

#### `ChangepointEvidence`

Purpose:
records signals supporting or weakening a break.

Dependencies:
regime signal set.

```rust
struct ChangepointEvidence {
    supporting_signals: Vec<SourceRef>,
    weakening_signals: Vec<SourceRef>,
    affected_outputs: Vec<WorldModelOutputRef>,
}
```

#### `ChangepointScore`

Purpose:
records break pressure for one candidate.

Dependencies:
changepoint detector method.

```rust
struct ChangepointScore {
    break_probability: Score,
    detector_version: SchemaVersion,
    reason_codes: Vec<ReasonCode>,
}
```

### Model Comparison Data

| Data | Attached to | Purpose |
|---|---|---|
| `ContinuationEvidence` | `ContinuationModel` | evidence that current regime still fits |
| `ContinuationScore` | `ContinuationModel` | continuation support |
| `BreakEvidence` | `BreakModel` | evidence for new segment |
| `BreakScore` | `BreakModel` | break support |

#### `ContinuationEvidence`

Purpose:
records evidence absorbed by the active regime.

Dependencies:
regime signal set and active segment.

```rust
struct ContinuationEvidence {
    evidence_window: EvidenceWindow,
    absorbed_signal_refs: Vec<SourceRef>,
    unresolved_signal_refs: Vec<SourceRef>,
}
```

#### `ContinuationScore`

Purpose:
records support for remaining in the current regime.

Dependencies:
continuation model method.

```rust
struct ContinuationScore {
    score: Score,
    model_version: SchemaVersion,
    reason_codes: Vec<ReasonCode>,
}
```

#### `BreakEvidence`

Purpose:
records evidence better explained by a new segment.

Dependencies:
changepoint candidate and regime signal set.

```rust
struct BreakEvidence {
    evidence_window: EvidenceWindow,
    break_signal_refs: Vec<SourceRef>,
    affected_belief_keys: Vec<BeliefKey>,
    affected_effects: Vec<EffectEstimateId>,
}
```

#### `BreakScore`

Purpose:
records support for starting a new segment.

Dependencies:
break model method.

```rust
struct BreakScore {
    score: Score,
    model_version: SchemaVersion,
    reason_codes: Vec<ReasonCode>,
}
```

### Posterior Data

| Data | Attached to | Purpose |
|---|---|---|
| `PosteriorScope` | `RegimePosterior` | active segment and candidate set |
| `CandidateRegimeSet` | `RegimePosterior` | candidate regimes |
| `PosteriorWeights` | `RegimePosterior` | probability mass |
| `PosteriorUncertainty` | `RegimePosterior` | unresolved identity state |
| `RunLengthDistribution` | `RunLengthBelief` | time since break belief |

#### `PosteriorScope`

Purpose:
records the segment and evidence window covered by posterior state.

Dependencies:
active segment and evidence window.

```rust
struct PosteriorScope {
    segment_id: ActiveSegmentId,
    evidence_window: EvidenceWindow,
}
```

#### `CandidateRegimeSet`

Purpose:
records candidate regimes being compared.

Dependencies:
regime library and break model.

```rust
struct CandidateRegimeSet {
    candidates: Vec<RegimeId>,
    includes_novel_regime: bool,
}
```

#### `PosteriorWeights`

Purpose:
records posterior mass over candidate regimes.

Dependencies:
continuation score, break score, prior state.

```rust
struct PosteriorWeights {
    weights: Vec<RegimeWeight>,
    normalization: WeightNormalization,
    method_version: SchemaVersion,
}
```

#### `PosteriorUncertainty`

Purpose:
records unresolved regime identity.

Dependencies:
posterior weights and thresholds.

```rust
struct PosteriorUncertainty {
    entropy: Score,
    unresolved_mixture: bool,
    dominant_regime: Option<RegimeId>,
}
```

#### `RunLengthDistribution`

Purpose:
records belief over time since the last break.

Dependencies:
segment boundary and changepoint candidates.

```rust
struct RunLengthDistribution {
    distribution: ProbabilityDistribution,
    expected_run_length: DurationEstimate,
    uncertainty: UncertaintySummary,
}
```

### Archive And Prior Data

| Data | Attached to | Purpose |
|---|---|---|
| `RegimeSignature` | `Regime`, `RegimeLibraryEntry` | recurring regime fingerprint |
| `ArchivedPriors` | `RegimeLibraryEntry` | reusable priors |
| `ArchivedCalibration` | `RegimeLibraryEntry` | calibration memory |
| `EntryPriorValues` | `RegimeEntryPrior` | priors for new segment |
| `EntryPriorUncertainty` | `RegimeEntryPrior` | widened or known prior uncertainty |

#### `RegimeSignature`

Purpose:
records structural fingerprint used for recurrence.

Dependencies:
belief, causal, graph, and calibration signals.

```rust
struct RegimeSignature {
    signature_hash: ContentHash,
    signal_features: Vec<RegimeFeature>,
    source_refs: Vec<SourceRef>,
}
```

#### `ArchivedPriors`

Purpose:
preserves priors learned under an archived regime.

Dependencies:
closed segment posterior, calibration records.

```rust
struct ArchivedPriors {
    belief_priors: Vec<BeliefPriorRef>,
    causal_priors: Vec<CausalPriorRef>,
    goal_value_priors: Vec<ValuePriorRef>,
}
```

#### `ArchivedCalibration`

Purpose:
preserves calibration memory for reuse.

Dependencies:
belief and causal calibration records.

```rust
struct ArchivedCalibration {
    calibration_refs: Vec<CalibrationRef>,
    reliability_summary: ReliabilitySummary,
}
```

#### `EntryPriorValues`

Purpose:
records priors selected when a segment begins.

Dependencies:
regime library or novel regime policy.

```rust
struct EntryPriorValues {
    priors: Vec<PriorRef>,
    source_regime_id: Option<RegimeId>,
}
```

#### `EntryPriorUncertainty`

Purpose:
records uncertainty on segment entry priors.

Dependencies:
known regime match or novel regime policy.

```rust
struct EntryPriorUncertainty {
    uncertainty: UncertaintySummary,
    widened_for_novel_regime: bool,
}
```

### Mixture Data

| Data | Attached to | Purpose |
|---|---|---|
| `MixtureScope` | `MixturePrediction` | forecast horizon and segment |
| `MixtureCandidates` | `MixturePrediction` | candidate regimes |
| `MixtureWeights` | `MixturePrediction` | posterior weights used for forecast |
| `MixtureForecast` | `MixturePrediction` | forecast output |

#### `MixtureScope`

Purpose:
records forecast scope under unresolved regime identity.

Dependencies:
active segment and decision horizon.

```rust
struct MixtureScope {
    segment_id: ActiveSegmentId,
    horizon: DecisionHorizon,
}
```

#### `MixtureCandidates`

Purpose:
records regimes included in the mixture.

Dependencies:
regime posterior.

```rust
struct MixtureCandidates {
    regime_ids: Vec<RegimeId>,
    includes_novel_regime: bool,
}
```

#### `MixtureWeights`

Purpose:
records weights used in mixture forecast.

Dependencies:
posterior weights.

```rust
struct MixtureWeights {
    weights: Vec<RegimeWeight>,
}
```

#### `MixtureForecast`

Purpose:
records forecast under unresolved regime identity.

Dependencies:
candidate regimes, priors, forecast method.

```rust
struct MixtureForecast {
    forecast_values: Vec<ForecastValue>,
    uncertainty: UncertaintySummary,
    expires_at: Option<ReferenceTime>,
}
```

### Sensitivity Data

| Data | Attached to | Purpose |
|---|---|---|
| `SensitivityScope` | `RegimeSensitivity` | affected context |
| `AffectedOutputSet` | `RegimeSensitivity`, `StressResult` | affected outputs |
| `SensitivityCondition` | `RegimeSensitivity` | flip or weakening condition |
| `SensitivitySeverity` | `RegimeSensitivity` | severity |

#### `SensitivityScope`

Purpose:
records the context whose outputs are regime-sensitive.

Dependencies:
active segment, planner context, affected outputs.

```rust
struct SensitivityScope {
    segment_id: ActiveSegmentId,
    context_ref: Option<PlannerContextRef>,
}
```

#### `AffectedOutputSet`

Purpose:
records beliefs, effects, risks, or planner outputs affected by regime uncertainty.

Dependencies:
belief views, causal summaries, planner outputs.

```rust
struct AffectedOutputSet {
    belief_keys: Vec<BeliefKey>,
    effect_estimate_ids: Vec<EffectEstimateId>,
    planner_output_refs: Vec<PlannerOutputRef>,
}
```

#### `SensitivityCondition`

Purpose:
records lower-layer changes that flip or weaken outputs.

Dependencies:
posterior weights and mixture prediction.

```rust
struct SensitivityCondition {
    condition_kind: SensitivityConditionKind,
    threshold: Score,
    affected_output: WorldModelOutputRef,
}
```

#### `SensitivitySeverity`

Purpose:
records severity of structural fragility.

Dependencies:
sensitivity condition and output importance.

```rust
struct SensitivitySeverity {
    severity: Score,
    blocking_for_planner: bool,
}
```

### Stress Data

| Data | Attached to | Purpose |
|---|---|---|
| `StressScope` | `StressScenario` | scenario scope |
| `StressPerturbationSet` | `StressScenario` | coordinated perturbations |
| `StressPolicy` | `StressScenario` | evaluation policy |
| `StressMetricSet` | `StressResult` | stress metrics |
| `StressAffectedOutputs` | `StressResult` | outputs changed by stress |

#### `StressScope`

Purpose:
records the structural context under stress.

Dependencies:
subject scope, branch scope, affected outputs.

```rust
struct StressScope {
    subject_scope: SubjectScope,
    branch_scope: BranchScope,
    affected_outputs: Vec<WorldModelOutputRef>,
}
```

#### `StressPerturbationSet`

Purpose:
records linked perturbations applied by the scenario.

Dependencies:
evidence channels, priors, mechanisms, cadence assumptions.

```rust
struct StressPerturbationSet {
    perturbations: Vec<StressPerturbation>,
}
```

#### `StressPolicy`

Purpose:
records how stress is evaluated.

Dependencies:
stress scenario registry.

```rust
struct StressPolicy {
    evaluation_method: StressEvaluationMethod,
    method_version: SchemaVersion,
}
```

#### `StressMetricSet`

Purpose:
records stress evaluation metrics.

Dependencies:
stress evaluation output.

```rust
struct StressMetricSet {
    posterior_flips: Vec<WorldModelOutputRef>,
    threshold_crossings: Vec<ThresholdCrossing>,
    recovery_time: Option<DurationEstimate>,
    cascade_depth: u32,
    blast_radius: Score,
}
```

#### `StressAffectedOutputs`

Purpose:
records outputs changed by stress.

Dependencies:
stress metric set and affected output registry.

```rust
struct StressAffectedOutputs {
    affected_outputs: Vec<WorldModelOutputRef>,
}
```

### Summary Data

| Data | Attached to | Purpose |
|---|---|---|
| `RegimePosteriorView` | `RegimeSummary` | planner-facing posterior |
| `ChangepointView` | `RegimeSummary` | planner-facing break state |
| `MixtureView` | `RegimeSummary` | planner-facing mixture state |
| `RegimeRiskView` | `RegimeSummary` | planner-facing structural risk |

#### `RegimePosteriorView`

Purpose:
exposes active regime posterior to consumers.

Dependencies:
regime posterior and active segment.

```rust
struct RegimePosteriorView {
    active_segment_id: ActiveSegmentId,
    candidate_weights: Vec<RegimeWeight>,
    dominant_regime: Option<RegimeId>,
    unresolved_mixture: bool,
}
```

#### `ChangepointView`

Purpose:
exposes changepoint state to consumers.

Dependencies:
changepoint candidate and model comparison.

```rust
struct ChangepointView {
    changepoint_status: ChangepointStatus,
    break_score: Score,
    continuation_score: Score,
    candidate_break_time: Option<ReferenceTime>,
}
```

#### `MixtureView`

Purpose:
exposes mixture forecast state to consumers.

Dependencies:
mixture prediction.

```rust
struct MixtureView {
    mixture_prediction_id: Option<MixturePredictionId>,
    forecast_values: Vec<ForecastValue>,
    expires_at: Option<ReferenceTime>,
}
```

#### `RegimeRiskView`

Purpose:
exposes regime risk, sensitivity, and stress state to consumers.

Dependencies:
sensitivity summaries and stress results.

```rust
struct RegimeRiskView {
    sensitivity_refs: Vec<RegimeSensitivityId>,
    stress_result_refs: Vec<StressResultId>,
    risk_reason_codes: Vec<ReasonCode>,
}
```

### Provenance Data

| Data | Attached to | Purpose |
|---|---|---|
| `RegimeProvenance` | `Regime` | source refs for regime |
| `SegmentProvenance` | `ActiveSegment` | source refs for segment |
| `ChangepointProvenance` | `ChangepointCandidate` | source refs for changepoint |
| `PosteriorProvenance` | `RegimePosterior` | source refs for posterior |
| `MixtureProvenance` | `MixturePrediction` | source refs for mixture |
| `StressResultProvenance` | `StressResult` | source refs for stress |
| `RegimeSummaryProvenance` | `RegimeSummary` | hydration refs for summary |

#### `RegimeSummaryProvenance`

Purpose:
preserves source refs and hydration handles for summary output.

Dependencies:
posterior, active segment, mixture, sensitivity, stress.

```rust
struct RegimeSummaryProvenance {
    source_refs: Vec<SourceRef>,
    hydration_handles: Vec<HydrationHandle>,
}
```

### Shared Data Shapes

#### `SourceRef`

Purpose:
shared source ref for graph, belief, causal, execution, and regime records.

Dependencies:
source layer vocabulary and cursor contract.

```rust
struct SourceRef {
    source_layer: SourceLayer,
    source_kind: SourceKind,
    source_id: SourceId,
    source_cursor: SourceCursor,
    projection_role: ProjectionRole,
}
```

#### `EvidenceWindow`

Purpose:
shared source boundary for regime evidence.

Dependencies:
source refs and source cursors.

```rust
struct EvidenceWindow {
    low_cursor: SourceCursor,
    high_cursor: SourceCursor,
    source_refs: Vec<SourceRef>,
}
```

#### `Score`

Purpose:
shared score shape for posterior, break, continuation, sensitivity, and stress.

Dependencies:
scale registry and method version.

```rust
struct Score {
    value: f64,
    scale: ScoreScale,
    source_fields: Vec<SourceFieldRef>,
    method_version: SchemaVersion,
}
```

### Data Rules

- Segment data names structural context, not local truth.
- Signal data preserves lower-layer refs.
- Break and continuation data are compared explicitly.
- Posterior data preserves unresolved regime identity.
- Archive data preserves priors for future reuse.
- Stress data records brittleness, not direct regime likelihood.
- Summary data hides worker internals and preserves explanation refs.

## Pipelines

### Pipeline Overview

| Phase | Stage | Required input | Required output |
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

### Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| graph object history and relation stability | graph | cadence and relation stability signals |
| belief revisions and views | belief | surprise and contradiction signals |
| belief calibration records | belief | calibration drift signals |
| causal effect estimates | causation | effectiveness shift signals |
| execution outcomes | execution through spine | calibration and action effectiveness |
| planner outputs | planner | sensitivity and stress affected outputs |
| agent goal priors | agent | goal curation prior selection |

### Signal Ingestion

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

### Changepoint Detection

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

### Continuation Modeling

Input:
active segment and signal window.

Output:
`ContinuationModel`.

Rules:

- Score whether current evidence still fits active regime.
- Record absorbed and unresolved signals.
- Preserve continuation assumptions.
- Keep continuation explanation separate from local belief settlement.

### Break Modeling

Input:
changepoint candidate and signal window.

Output:
`BreakModel`.

Rules:

- Score whether current evidence is better explained by a new segment.
- Record affected belief keys and effect estimates.
- Preserve break assumptions.
- Keep break explanation separate from final segment rollover.

### Continuation Versus Break Comparison

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

### Posterior Update

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

### Run Length Update

Input:
active segment boundary and changepoint state.

Output:
`RunLengthBelief`.

Rules:

- Update distribution over time since last break.
- Preserve break timing uncertainty.
- Use candidate changepoints without rewriting segment history.
- Publish new run length belief when source evidence changes.

### Active Segment Rollover

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

### Regime Archive Management

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

### Entry Prior Selection

Input:
regime posterior, regime library, active segment rollover.

Output:
`RegimeEntryPrior`.

Rules:

- Select archived priors for known regime entry.
- Widen priors for novel regime entry.
- Preserve prior source regime when available.
- Expose uncertainty for goal curation and belief assessment.

### Mixture Projection

Input:
unresolved posterior and horizon.

Output:
`MixturePrediction`.

Rules:

- Forecast over candidate regimes using posterior weights.
- Include novel regime mass when present.
- Preserve expiry horizon.
- Avoid forcing a single regime when posterior is unresolved.

### Sensitivity Projection

Input:
regime posterior, mixture prediction, belief views, causal summaries, planner outputs.

Output:
`RegimeSensitivity`.

Rules:

- Identify outputs that flip under candidate regime changes.
- Identify outputs that weaken under unresolved mixture state.
- Mark planner-blocking sensitivity when threshold is crossed.
- Preserve affected belief keys, effect estimates, and planner outputs.

### Stress Evaluation

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

### Summary Projection

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

### Recovery And Replay

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

### Failure Semantics

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

### Test Matrix

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

### Pipeline Rules

- Stages are deterministic over explicit inputs and method versions.
- Stages preserve lower-layer source refs.
- Stages compare continuation and break explicitly.
- Stages preserve unresolved mixture state.
- Stages archive old regimes rather than erasing priors.
- Stages publish summaries, not planner actions.

## Role In The Set

`regime` consumes surprise, contradiction clusters, cadence shifts, calibration drift, and effectiveness shifts.

`belief` may carry break pressure.
`regime` owns the structural decision about whether that pressure implies a new segment or unresolved mixture state.

`agent` consumes regime sensitivity through a lens, not through raw regime worker state.
