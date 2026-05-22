# Causal Components

Date: 2026-05-15
Status: active
Scope: ECS components for `world_model/causation`

## Thesis

Causal components are typed data contracts attached to causal entities.

They carry variable scope, intervention semantics, outcome measurements, selection paths, confounder state, identification status, effect posterior, counterfactual state, assumptions, summaries, and provenance.

## Component Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable causal record addressing | variable and estimate systems | all causal systems |
| Variable | variable kind, scope, and measurement shape | variable registration | intervention, outcome, mechanism |
| Mechanism | causal structure and regime condition | mechanism selection | identification and estimation |
| Intervention | action semantics and timing | intervention lowering | outcome linking and estimation |
| Outcome | measured effects and timing | outcome linking | identification and estimation |
| Selection | measurement and selection path warnings | selection interpretation | confounder and risk systems |
| Confounding | hidden-cause candidates and risk | confounder scoring | identification |
| Identification | adjustment set and blockers | identification assessment | effect estimation and summary |
| Effect | posterior effect and uncertainty | effect estimation | planner summary |
| Counterfactual | factual and alternative case state | counterfactual evaluation | planner summary |
| Assumption | explicit dependency set | identification and estimation | summary and planner |
| Provenance | source refs and hydration | every causal system | replay and explanation |

## Shape Rules

- Components contain typed causal data, not prose authority.
- Source-derived components carry graph, belief, spine, or execution refs.
- Derived components carry method version and evidence window.
- Identification fields remain separate from generic uncertainty.
- Confounder risk remains separate from effect uncertainty.
- Selection warnings remain visible to planner-facing summaries.

## Identity Components

| Component | Attached to | Purpose |
|---|---|---|
| `VariableIdentity` | `CausalVariable` | stable variable id |
| `MechanismIdentity` | `MechanismVersion` | stable mechanism version id |
| `InterventionIdentity` | `InterventionRecord` | stable intervention id |
| `EffectIdentity` | `EffectEstimate` | stable estimate id |

### `VariableIdentity`

Purpose:
stable identity for one causal variable.

Dependencies:
variable family, subject scope, dimension, perspective or regime scope.

```rust
struct VariableIdentity {
    variable_id: CausalVariableId,
    variable_ref: DomainObjectRef,
    key_hash: ContentHash,
    schema_version: SchemaVersion,
}
```

### `MechanismIdentity`

Purpose:
stable identity for one mechanism candidate or selected mechanism.

Dependencies:
mechanism family, variable set, regime condition, method version.

```rust
struct MechanismIdentity {
    mechanism_id: MechanismVersionId,
    mechanism_family: MechanismFamily,
    mechanism_version: SchemaVersion,
}
```

### `InterventionIdentity`

Purpose:
stable identity for one attempted intervention context.

Dependencies:
execution action ref, target variable, intervention kind.

```rust
struct InterventionIdentity {
    intervention_id: InterventionRecordId,
    action_ref: DomainObjectRef,
    intervention_version: SchemaVersion,
}
```

### `EffectIdentity`

Purpose:
stable identity for one effect estimate.

Dependencies:
intervention variable, outcome variable, evidence window, mechanism version.

```rust
struct EffectIdentity {
    effect_estimate_id: EffectEstimateId,
    intervention_variable: CausalVariableId,
    outcome_variable: CausalVariableId,
    estimate_version: SchemaVersion,
}
```

## Variable Components

| Component | Attached to | Purpose |
|---|---|---|
| `VariableScope` | `CausalVariable` | subject and context scope |
| `VariableKind` | `CausalVariable` | state, intervention, outcome, selection, regime, or confounder kind |
| `VariableMeasurement` | `CausalVariable` | evidence and measurement shape |

### `VariableScope`

Purpose:
defines what a causal variable ranges over.

Dependencies:
graph object refs, belief dimensions, branch scope, perspective scope.

```rust
struct VariableScope {
    subject_scope: SubjectScope,
    branch_scope: BranchScope,
    perspective: Option<PerspectiveKey>,
    regime_scope: Option<RegimeScope>,
}
```

### `VariableKind`

Purpose:
classifies causal variable role.

Dependencies:
variable registry.

```rust
struct VariableKind {
    kind: CausalVariableKind,
    dimension: CausalDimension,
}
```

### `VariableMeasurement`

Purpose:
defines how the variable is observed or derived.

Dependencies:
belief view contract, graph query contract, evidence channel registry.

```rust
struct VariableMeasurement {
    measurement_kind: MeasurementKind,
    evidence_channels: Vec<EvidenceChannelRef>,
    source_belief_keys: Vec<BeliefKey>,
    source_graph_refs: Vec<GraphRecordRef>,
}
```

## Mechanism Components

| Component | Attached to | Purpose |
|---|---|---|
| `MechanismScope` | `MechanismVersion` | variables and context |
| `MechanismStructure` | `MechanismVersion` | causal structure or model form |
| `RegimeCondition` | `MechanismVersion` | regime dependency |

### `MechanismScope`

Purpose:
records variables covered by one mechanism.

Dependencies:
causal variable registry.

```rust
struct MechanismScope {
    variables: Vec<CausalVariableId>,
    intervention_variables: Vec<CausalVariableId>,
    outcome_variables: Vec<CausalVariableId>,
}
```

### `MechanismStructure`

Purpose:
records the structural form used for identification and estimation.

Dependencies:
mechanism registry, graph relation evidence, belief summaries.

```rust
struct MechanismStructure {
    structure_kind: MechanismStructureKind,
    parent_links: Vec<CausalParentLink>,
    selection_links: Vec<SelectionPathRef>,
    method_version: SchemaVersion,
}
```

### `RegimeCondition`

Purpose:
records structural context required for a mechanism to apply.

Dependencies:
regime summaries and active segment state.

```rust
struct RegimeCondition {
    regime_refs: Vec<RegimeRef>,
    active_segment_ref: Option<ActiveSegmentRef>,
    condition_status: RegimeConditionStatus,
}
```

## Intervention Components

| Component | Attached to | Purpose |
|---|---|---|
| `InterventionTarget` | `InterventionRecord` | intended and actual target |
| `InterventionSemantics` | `InterventionRecord` | action meaning |
| `InterventionTiming` | `InterventionRecord` | application and delay window |

### `InterventionTarget`

Purpose:
records intended and actual target variables or objects.

Dependencies:
execution action facts, graph object refs, causal variable refs.

```rust
struct InterventionTarget {
    intended_target: DomainObjectRef,
    actual_target: Option<DomainObjectRef>,
    target_variable: CausalVariableId,
}
```

### `InterventionSemantics`

Purpose:
records action meaning for causal reasoning.

Dependencies:
execution action contract, task outcome facts.

```rust
struct InterventionSemantics {
    intervention_kind: InterventionKind,
    application_status: InterventionApplicationStatus,
    rollback_or_compensation: Option<CompensationRef>,
}
```

### `InterventionTiming`

Purpose:
records temporal window for intervention effects.

Dependencies:
execution fact time, graph reference time, outcome horizon policy.

```rust
struct InterventionTiming {
    started_at: ReferenceTime,
    completed_at: Option<ReferenceTime>,
    expected_delay_window: Option<TimeWindow>,
}
```

## Outcome Components

| Component | Attached to | Purpose |
|---|---|---|
| `OutcomeMeasurement` | `OutcomeLink` | measured outcome value |
| `OutcomeTiming` | `OutcomeLink` | outcome observation timing |
| `OutcomeAttribution` | `OutcomeLink` | candidate attribution state |

### `OutcomeMeasurement`

Purpose:
records the observed outcome variable value.

Dependencies:
belief revision, graph state, execution outcome, measurement evidence.

```rust
struct OutcomeMeasurement {
    outcome_variable: CausalVariableId,
    measured_value: OutcomeValue,
    measurement_channel: EvidenceChannelRef,
    measurement_quality: MeasurementQuality,
}
```

### `OutcomeTiming`

Purpose:
records timing between intervention and outcome observation.

Dependencies:
intervention timing and evidence time.

```rust
struct OutcomeTiming {
    observed_at: ReferenceTime,
    lag_from_intervention: Option<DurationEstimate>,
    within_expected_window: bool,
}
```

### `OutcomeAttribution`

Purpose:
records candidate link status between intervention and measured outcome.

Dependencies:
intervention record, selection path, identification state.

```rust
struct OutcomeAttribution {
    attribution_status: AttributionStatus,
    attribution_reason: ReasonCode,
    source_intervention_id: InterventionRecordId,
}
```

## Selection Components

| Component | Attached to | Purpose |
|---|---|---|
| `MeasurementPath` | `OutcomeLink`, `CausalVariable` | how outcome became visible |
| `SelectionWarning` | `OutcomeLink`, `EffectEstimate`, `CausalSummary` | selection-shaped evidence warning |

### `MeasurementPath`

Purpose:
records how state or outcome was measured.

Dependencies:
graph provenance, evidence channel, belief origin.

```rust
struct MeasurementPath {
    graph_path_refs: Vec<GraphRecordRef>,
    evidence_refs: Vec<EvidenceRef>,
    measurement_policy_ref: Option<MeasurementPolicyRef>,
}
```

### `SelectionWarning`

Purpose:
records when observed evidence may reflect selection or measurement policy rather than outcome change.

Dependencies:
graph anchor selection, measurement path, belief origin, confounder scoring.

```rust
struct SelectionWarning {
    warning_status: SelectionWarningStatus,
    selection_path: Option<SelectionPathRef>,
    reason_codes: Vec<ReasonCode>,
}
```

## Confounding Components

| Component | Attached to | Purpose |
|---|---|---|
| `ConfounderScope` | `ConfounderHypothesis` | affected variables |
| `ConfounderEvidence` | `ConfounderHypothesis` | evidence supporting hidden cause |
| `ConfounderRisk` | `ConfounderHypothesis`, `IdentificationAssessment`, `EffectEstimate` | risk to identification |

### `ConfounderScope`

Purpose:
records intervention and outcome variables affected by a possible hidden cause.

Dependencies:
causal variable refs, mechanism structure.

```rust
struct ConfounderScope {
    candidate_variable: CausalVariableId,
    affected_intervention: CausalVariableId,
    affected_outcome: CausalVariableId,
}
```

### `ConfounderEvidence`

Purpose:
records evidence that supports or weakens a confounder hypothesis.

Dependencies:
belief summaries, graph relations, regime signals.

```rust
struct ConfounderEvidence {
    supporting_refs: Vec<SourceRef>,
    weakening_refs: Vec<SourceRef>,
    evidence_window: EvidenceWindow,
}
```

### `ConfounderRisk`

Purpose:
records confounding risk separately from effect uncertainty.

Dependencies:
confounder evidence, adjustment set, identification method.

```rust
struct ConfounderRisk {
    severity: Score,
    controlled_by_adjustment: bool,
    risk_reason_codes: Vec<ReasonCode>,
}
```

## Identification Components

| Component | Attached to | Purpose |
|---|---|---|
| `IdentificationScope` | `IdentificationAssessment` | intervention and outcome query |
| `IdentificationStatus` | `IdentificationAssessment`, `EffectEstimate` | identifiability state |
| `AdjustmentSet` | `IdentificationAssessment` | assumed adjustment variables |
| `IdentificationBlockers` | `IdentificationAssessment` | blockers and missing evidence |

### `IdentificationScope`

Purpose:
records the effect query being identified.

Dependencies:
intervention variable, outcome variable, mechanism version.

```rust
struct IdentificationScope {
    intervention_variable: CausalVariableId,
    outcome_variable: CausalVariableId,
    mechanism_id: MechanismVersionId,
}
```

### `IdentificationStatus`

Purpose:
records whether evidence and assumptions support effect estimation.

Dependencies:
confounder risk, selection warnings, measurement paths, assumptions.

```rust
struct IdentificationStatus {
    status: IdentificationState,
    confidence: Score,
    reason_codes: Vec<ReasonCode>,
}
```

### `AdjustmentSet`

Purpose:
records variables assumed sufficient to control confounding.

Dependencies:
mechanism structure, confounder hypotheses.

```rust
struct AdjustmentSet {
    variables: Vec<CausalVariableId>,
    assumption_refs: Vec<CausalAssumptionRef>,
}
```

### `IdentificationBlockers`

Purpose:
records why an effect is not identified.

Dependencies:
missing evidence, confounder risk, selection warning, measurement path.

```rust
struct IdentificationBlockers {
    blockers: Vec<IdentificationBlocker>,
    missing_evidence: Vec<EvidenceTarget>,
}
```

## Effect Components

| Component | Attached to | Purpose |
|---|---|---|
| `EffectScope` | `EffectEstimate` | intervention and outcome variables |
| `EffectPosterior` | `EffectEstimate` | effect direction and magnitude |
| `EffectUncertainty` | `EffectEstimate` | effect uncertainty |

### `EffectScope`

Purpose:
records the variables and evidence window for an effect estimate.

Dependencies:
intervention variable, outcome variable, evidence window.

```rust
struct EffectScope {
    intervention_variable: CausalVariableId,
    outcome_variable: CausalVariableId,
    evidence_window: EvidenceWindow,
    mechanism_id: MechanismVersionId,
}
```

### `EffectPosterior`

Purpose:
records estimated effect direction and magnitude.

Dependencies:
identified evidence, mechanism version, estimation method.

```rust
struct EffectPosterior {
    direction: EffectDirection,
    magnitude: EffectMagnitude,
    posterior_summary: PosteriorSummary,
}
```

### `EffectUncertainty`

Purpose:
records effect uncertainty and estimate quality.

Dependencies:
identification status, confounder risk, evidence quality.

```rust
struct EffectUncertainty {
    uncertainty: UncertaintySummary,
    precision: PrecisionSummary,
    estimate_quality: EstimateQuality,
}
```

## Counterfactual Components

| Component | Attached to | Purpose |
|---|---|---|
| `FactualCase` | `CounterfactualCase` | observed case |
| `AlternativeCase` | `CounterfactualCase` | alternative intervention |
| `CounterfactualOutcome` | `CounterfactualCase` | predicted alternative outcome |

### `FactualCase`

Purpose:
records the observed intervention and outcome.

Dependencies:
intervention record, outcome link, effect estimate.

```rust
struct FactualCase {
    intervention_id: InterventionRecordId,
    outcome_link_id: OutcomeLinkId,
    observed_outcome: OutcomeValue,
}
```

### `AlternativeCase`

Purpose:
records the alternative action or context.

Dependencies:
counterfactual query and mechanism version.

```rust
struct AlternativeCase {
    alternative_intervention: Option<InterventionSpec>,
    alternative_context: Option<ContextSpec>,
    alternative_regime: Option<RegimeRef>,
}
```

### `CounterfactualOutcome`

Purpose:
records expected outcome under the alternative case.

Dependencies:
effect estimate, mechanism version, assumptions.

```rust
struct CounterfactualOutcome {
    expected_outcome: OutcomeValue,
    uncertainty: UncertaintySummary,
    confidence: Score,
}
```

## Assumption Components

| Component | Attached to | Purpose |
|---|---|---|
| `AssumptionScope` | `CausalAssumptionSet` | owning estimate or case |
| `CausalAssumptions` | `CausalAssumptionSet` | assumption list |
| `AssumptionValidity` | `CausalAssumptionSet` | validity state |
| `AssumptionViolationEffect` | `CausalAssumptionSet` | effect of violation |

### `AssumptionScope`

Purpose:
records the causal result that depends on the assumptions.

Dependencies:
effect estimate, identification assessment, counterfactual case.

```rust
struct AssumptionScope {
    effect_estimate_id: Option<EffectEstimateId>,
    identification_assessment_id: Option<IdentificationAssessmentId>,
    counterfactual_case_id: Option<CounterfactualCaseId>,
}
```

### `CausalAssumptions`

Purpose:
records explicit causal, intervention, measurement, adjustment, and regime assumptions.

Dependencies:
mechanism version, evidence policy, regime condition.

```rust
struct CausalAssumptions {
    assumptions: Vec<CausalAssumption>,
}
```

### `AssumptionValidity`

Purpose:
records whether assumptions remain valid.

Dependencies:
source evidence, regime state, measurement policy.

```rust
struct AssumptionValidity {
    validity_status: AssumptionValidityStatus,
    expires_at: Option<ReferenceTime>,
    reason_codes: Vec<ReasonCode>,
}
```

### `AssumptionViolationEffect`

Purpose:
records what changes when an assumption fails.

Dependencies:
effect estimate and identification assessment.

```rust
struct AssumptionViolationEffect {
    violation_effect: ViolationEffect,
    affected_outputs: Vec<CausalOutputRef>,
}
```

## Summary Components

| Component | Attached to | Purpose |
|---|---|---|
| `CausalEffectView` | `CausalSummary` | planner-facing effect support |
| `CausalRiskView` | `CausalSummary` | planner-facing risk state |
| `CausalAssumptionView` | `CausalSummary` | planner-facing assumptions |

### `CausalEffectView`

Purpose:
exposes effect direction, magnitude, uncertainty, and identification state.

Dependencies:
effect estimate and identification assessment.

```rust
struct CausalEffectView {
    intervention_variable: CausalVariableId,
    outcome_variable: CausalVariableId,
    effect_direction: EffectDirection,
    effect_magnitude: EffectMagnitude,
    uncertainty: UncertaintySummary,
    identification_status: IdentificationState,
}
```

### `CausalRiskView`

Purpose:
exposes confounding, selection, and measurement risks.

Dependencies:
confounder hypotheses, selection warnings, measurement paths.

```rust
struct CausalRiskView {
    confounder_risk: Option<ConfounderRisk>,
    selection_warning: Option<SelectionWarning>,
    measurement_warning: Option<MeasurementWarning>,
    risk_reason_codes: Vec<ReasonCode>,
}
```

### `CausalAssumptionView`

Purpose:
exposes assumption state for planner and explanation.

Dependencies:
causal assumption set.

```rust
struct CausalAssumptionView {
    assumption_refs: Vec<CausalAssumptionRef>,
    validity_status: AssumptionValidityStatus,
    expires_at: Option<ReferenceTime>,
}
```

## Provenance Components

| Component | Attached to | Purpose |
|---|---|---|
| `VariableProvenance` | `CausalVariable` | source refs for variable |
| `MechanismProvenance` | `MechanismVersion` | source refs for mechanism |
| `InterventionProvenance` | `InterventionRecord` | source refs for intervention |
| `OutcomeProvenance` | `OutcomeLink` | source refs for outcome link |
| `EffectProvenance` | `EffectEstimate` | source refs for effect |
| `CounterfactualProvenance` | `CounterfactualCase` | source refs for counterfactual |
| `CausalSummaryProvenance` | `CausalSummary` | hydration refs for summary |

### `VariableProvenance`

Purpose:
preserves source refs for causal variable creation.

Dependencies:
graph refs, belief refs, variable registry.

```rust
struct VariableProvenance {
    source_refs: Vec<SourceRef>,
}
```

### `MechanismProvenance`

Purpose:
preserves source refs for mechanism state.

Dependencies:
graph relations, belief summaries, regime refs.

```rust
struct MechanismProvenance {
    source_refs: Vec<SourceRef>,
}
```

### `InterventionProvenance`

Purpose:
preserves source refs for intervention lowering.

Dependencies:
execution facts and graph refs.

```rust
struct InterventionProvenance {
    source_refs: Vec<SourceRef>,
}
```

### `OutcomeProvenance`

Purpose:
preserves source refs for outcome linking.

Dependencies:
evidence refs, graph refs, belief refs.

```rust
struct OutcomeProvenance {
    source_refs: Vec<SourceRef>,
}
```

### `EffectProvenance`

Purpose:
preserves source refs and method version for effect estimates.

Dependencies:
interventions, outcomes, identification assessment, mechanism version.

```rust
struct EffectProvenance {
    source_refs: Vec<SourceRef>,
    method_version: SchemaVersion,
}
```

### `CounterfactualProvenance`

Purpose:
preserves source refs and method version for counterfactual evaluation.

Dependencies:
effect estimate and mechanism version.

```rust
struct CounterfactualProvenance {
    source_refs: Vec<SourceRef>,
    method_version: SchemaVersion,
}
```

### `CausalSummaryProvenance`

Purpose:
preserves hydration refs for planner-facing summaries.

Dependencies:
effect estimate, assumptions, counterfactual case, source refs.

```rust
struct CausalSummaryProvenance {
    source_refs: Vec<SourceRef>,
    hydration_handles: Vec<HydrationHandle>,
}
```

## Shared Data Shapes

### `SourceRef`

Purpose:
shared reference shape for graph, belief, spine, execution, regime, and causal records.

Dependencies:
source layer vocabulary and source cursor contract.

```rust
struct SourceRef {
    source_layer: SourceLayer,
    source_kind: SourceKind,
    source_id: SourceId,
    domain_object_ref: DomainObjectRef,
    source_cursor: SourceCursor,
    projection_role: ProjectionRole,
}
```

### `Score`

Purpose:
shared score shape for confidence, risk, identification, and effect quality.

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

### `EvidenceWindow`

Purpose:
shared assessed evidence boundary.

Dependencies:
source refs and source cursor.

```rust
struct EvidenceWindow {
    source_refs: Vec<SourceRef>,
    low_cursor: SourceCursor,
    high_cursor: SourceCursor,
}
```

## Component Rules

- Variable data names modeled causal quantities.
- Intervention data names action semantics, not proof of effect.
- Outcome data names measured state, not attribution by itself.
- Identification data gates effect estimation.
- Effect data preserves identification and confounding state.
- Counterfactual data depends on explicit assumptions.
- Summary data hides worker internals and preserves explanation refs.

## Read With

- [Causal Entities](entities.md)
- [Causal Systems](systems.md)
- [Causal Requirements](requirements.md)
- [Causal Layer](README.md)
