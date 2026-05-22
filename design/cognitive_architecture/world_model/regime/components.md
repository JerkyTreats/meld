# Regime Components

Date: 2026-05-16
Status: active
Scope: ECS components for `world_model/regime`

## Thesis

Regime components are typed data contracts attached to regime entities.

They carry segment boundaries, structural signals, continuation and break evidence, posterior weights, run length beliefs, archived priors, mixture forecasts, sensitivity state, stress metrics, and provenance.

## Component Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable regime record addressing | regime systems | all regime systems |
| Segment | active structural interval state | segment management | all regime systems |
| Signal | graph, belief, causal, and calibration inputs | signal ingestion | changepoint and comparison |
| Changepoint | candidate break windows and scores | changepoint detection | comparison and rollover |
| Model comparison | continuation and break explanations | comparison systems | posterior update |
| Posterior | regime weights and run length | posterior update | mixture and summary |
| Archive and priors | reusable regime state and entry priors | archive management | posterior and agent priors |
| Mixture | unresolved multi-regime forecasts | mixture projection | planner and agent |
| Sensitivity | affected outputs and fragility | sensitivity projection | planner |
| Stress | scenario perturbations and metrics | stress evaluation | planner and operator |
| Provenance | source refs and hydration | every regime system | replay and explanation |

## Shape Rules

- Components carry typed structural state, not action policy.
- Signal data preserves graph, belief, causal, execution, and calibration refs.
- Posterior data keeps unresolved mixture state explicit.
- Archive data preserves old priors instead of overwriting them.
- Stress data records perturbation assumptions separately from posterior likelihood.
- Summary data excludes worker internals.

## Identity Components

| Component | Attached to | Purpose |
|---|---|---|
| `RegimeIdentity` | `Regime` | stable regime id |
| `SegmentIdentity` | `ActiveSegment` | stable segment id |
| `ChangepointIdentity` | `ChangepointCandidate` | stable changepoint id |
| `StressIdentity` | `StressScenario` | stable stress scenario id |

### `RegimeIdentity`

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

### `SegmentIdentity`

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

### `ChangepointIdentity`

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

### `StressIdentity`

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

## Segment Components

| Component | Attached to | Purpose |
|---|---|---|
| `SegmentScope` | `ActiveSegment` | scope of structural context |
| `SegmentBoundary` | `ActiveSegment` | start and end boundary |
| `SegmentStatus` | `ActiveSegment` | active segment lifecycle |

### `SegmentScope`

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

### `SegmentBoundary`

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

### `SegmentStatus`

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

## Signal Components

| Component | Attached to | Purpose |
|---|---|---|
| `RegimeSignalSet` | `ActiveSegment`, `ChangepointCandidate` | input signal bundle |
| `SurpriseSignal` | `RegimeSignalSet` owner | belief surprise |
| `ContradictionClusterSignal` | `RegimeSignalSet` owner | repeated local contradiction |
| `CadenceShiftSignal` | `RegimeSignalSet` owner | observation cadence shift |
| `CalibrationDriftSignal` | `RegimeSignalSet` owner | outcome calibration drift |
| `EffectivenessShiftSignal` | `RegimeSignalSet` owner | causal effect shift |
| `RelationStabilitySignal` | `RegimeSignalSet` owner | graph relation stability shift |

### `RegimeSignalSet`

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

### `SurpriseSignal`

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

### `ContradictionClusterSignal`

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

### `CadenceShiftSignal`

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

### `CalibrationDriftSignal`

Purpose:
records drift between belief predictions and later outcomes.

Dependencies:
belief calibration records and execution outcomes.

```rust
struct CalibrationDriftSignal {
    affected_comparators: Vec<ComparatorKind>,
    drift_score: Score,
    source_refs: Vec<SourceRef>,
}
```

### `EffectivenessShiftSignal`

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

### `RelationStabilitySignal`

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

## Changepoint Components

| Component | Attached to | Purpose |
|---|---|---|
| `ChangepointWindow` | `ChangepointCandidate` | candidate break interval |
| `ChangepointEvidence` | `ChangepointCandidate` | break evidence |
| `ChangepointScore` | `ChangepointCandidate` | break likelihood pressure |

### `ChangepointWindow`

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

### `ChangepointEvidence`

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

### `ChangepointScore`

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

## Model Comparison Components

| Component | Attached to | Purpose |
|---|---|---|
| `ContinuationEvidence` | `ContinuationModel` | evidence that current regime still fits |
| `ContinuationScore` | `ContinuationModel` | continuation support |
| `BreakEvidence` | `BreakModel` | evidence for new segment |
| `BreakScore` | `BreakModel` | break support |

### `ContinuationEvidence`

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

### `ContinuationScore`

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

### `BreakEvidence`

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

### `BreakScore`

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

## Posterior Components

| Component | Attached to | Purpose |
|---|---|---|
| `PosteriorScope` | `RegimePosterior` | active segment and candidate set |
| `CandidateRegimeSet` | `RegimePosterior` | candidate regimes |
| `PosteriorWeights` | `RegimePosterior` | probability mass |
| `PosteriorUncertainty` | `RegimePosterior` | unresolved identity state |
| `RunLengthDistribution` | `RunLengthBelief` | time since break belief |

### `PosteriorScope`

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

### `CandidateRegimeSet`

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

### `PosteriorWeights`

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

### `PosteriorUncertainty`

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

### `RunLengthDistribution`

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

## Archive And Prior Components

| Component | Attached to | Purpose |
|---|---|---|
| `RegimeSignature` | `Regime`, `RegimeLibraryEntry` | recurring regime fingerprint |
| `ArchivedPriors` | `RegimeLibraryEntry` | reusable priors |
| `ArchivedCalibration` | `RegimeLibraryEntry` | calibration memory |
| `EntryPriorValues` | `RegimeEntryPrior` | priors for new segment |
| `EntryPriorUncertainty` | `RegimeEntryPrior` | widened or known prior uncertainty |

### `RegimeSignature`

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

### `ArchivedPriors`

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

### `ArchivedCalibration`

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

### `EntryPriorValues`

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

### `EntryPriorUncertainty`

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

## Mixture Components

| Component | Attached to | Purpose |
|---|---|---|
| `MixtureScope` | `MixturePrediction` | forecast horizon and segment |
| `MixtureCandidates` | `MixturePrediction` | candidate regimes |
| `MixtureWeights` | `MixturePrediction` | posterior weights used for forecast |
| `MixtureForecast` | `MixturePrediction` | forecast output |

### `MixtureScope`

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

### `MixtureCandidates`

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

### `MixtureWeights`

Purpose:
records weights used in mixture forecast.

Dependencies:
posterior weights.

```rust
struct MixtureWeights {
    weights: Vec<RegimeWeight>,
}
```

### `MixtureForecast`

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

## Sensitivity Components

| Component | Attached to | Purpose |
|---|---|---|
| `SensitivityScope` | `RegimeSensitivity` | affected context |
| `AffectedOutputSet` | `RegimeSensitivity`, `StressResult` | affected outputs |
| `SensitivityCondition` | `RegimeSensitivity` | flip or weakening condition |
| `SensitivitySeverity` | `RegimeSensitivity` | severity |

### `SensitivityScope`

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

### `AffectedOutputSet`

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

### `SensitivityCondition`

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

### `SensitivitySeverity`

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

## Stress Components

| Component | Attached to | Purpose |
|---|---|---|
| `StressScope` | `StressScenario` | scenario scope |
| `StressPerturbationSet` | `StressScenario` | coordinated perturbations |
| `StressPolicy` | `StressScenario` | evaluation policy |
| `StressMetricSet` | `StressResult` | stress metrics |
| `StressAffectedOutputs` | `StressResult` | outputs changed by stress |

### `StressScope`

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

### `StressPerturbationSet`

Purpose:
records linked perturbations applied by the scenario.

Dependencies:
evidence channels, priors, mechanisms, cadence assumptions.

```rust
struct StressPerturbationSet {
    perturbations: Vec<StressPerturbation>,
}
```

### `StressPolicy`

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

### `StressMetricSet`

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

### `StressAffectedOutputs`

Purpose:
records outputs changed by stress.

Dependencies:
stress metric set and affected output registry.

```rust
struct StressAffectedOutputs {
    affected_outputs: Vec<WorldModelOutputRef>,
}
```

## Summary Components

| Component | Attached to | Purpose |
|---|---|---|
| `RegimePosteriorView` | `RegimeSummary` | planner-facing posterior |
| `ChangepointView` | `RegimeSummary` | planner-facing break state |
| `MixtureView` | `RegimeSummary` | planner-facing mixture state |
| `RegimeRiskView` | `RegimeSummary` | planner-facing structural risk |

### `RegimePosteriorView`

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

### `ChangepointView`

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

### `MixtureView`

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

### `RegimeRiskView`

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

## Provenance Components

| Component | Attached to | Purpose |
|---|---|---|
| `RegimeProvenance` | `Regime` | source refs for regime |
| `SegmentProvenance` | `ActiveSegment` | source refs for segment |
| `ChangepointProvenance` | `ChangepointCandidate` | source refs for changepoint |
| `PosteriorProvenance` | `RegimePosterior` | source refs for posterior |
| `MixtureProvenance` | `MixturePrediction` | source refs for mixture |
| `StressResultProvenance` | `StressResult` | source refs for stress |
| `RegimeSummaryProvenance` | `RegimeSummary` | hydration refs for summary |

### `RegimeSummaryProvenance`

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

## Shared Data Shapes

### `SourceRef`

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

### `EvidenceWindow`

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

### `Score`

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

## Component Rules

- Segment data names structural context, not local truth.
- Signal data preserves lower-layer refs.
- Break and continuation data are compared explicitly.
- Posterior data preserves unresolved regime identity.
- Archive data preserves priors for future reuse.
- Stress data records brittleness, not direct regime likelihood.
- Summary data hides worker internals and preserves explanation refs.

## Read With

- [Regime Entities](entities.md)
- [Regime Systems](systems.md)
- [Regime Requirements](requirements.md)
- [Regime Layer](README.md)
