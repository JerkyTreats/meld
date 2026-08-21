# Causation Spec

Scope: domain specification for `world_model/causation`

`causation` is a structured interpretation domain over graph and belief outputs. It links interventions, outcomes, selection paths, and confounders into effect estimates. It consumes settled evidence and emits mechanism-aware summaries for `planner` and perspective consumers.

## Domain Types

Causal domain types are stable identities for variables, mechanisms, interventions, outcome links, confounders, identification state, effect estimates, counterfactuals, assumptions, and planner-facing summaries. They separate temporal evidence from mechanism claims. They prevent graph reachability, anchor selection, and execution success from becoming implicit causal proof.

### Type Categories

| Category | Domain type | Persistence expectation |
|---|---|---|
| Variable identity | `CausalVariable` | durable |
| Mechanism | `MechanismVersion` | durable when proposed or selected |
| Intervention | `InterventionRecord` | durable |
| Outcome evidence | `OutcomeLink` | durable |
| Confounding | `ConfounderHypothesis` | durable when material |
| Identification | `IdentificationAssessment` | materialized projection |
| Effect | `EffectEstimate` | durable revision or materialized projection |
| Counterfactual | `CounterfactualCase` | materialized projection |
| Assumption | `CausalAssumptionSet` | durable with estimate or case |
| Consumer view | `CausalSummary` | materialized projection |

### Identity Rules

| Domain type | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `CausalVariable` | variable family plus subject scope plus dimension plus perspective or regime scope | none | causation |
| `MechanismVersion` | mechanism family plus variable set plus regime condition plus version | one or more `CausalVariable` values | causation |
| `InterventionRecord` | execution action ref plus target variable plus intervention kind | `CausalVariable` target | causation |
| `OutcomeLink` | intervention record plus outcome variable plus measurement evidence id | `InterventionRecord` | causation |
| `ConfounderHypothesis` | candidate hidden variable plus affected intervention and outcome variables | variable pair or effect query | causation |
| `IdentificationAssessment` | intervention variable plus outcome variable plus evidence window plus method version | effect query | causation |
| `EffectEstimate` | intervention variable plus outcome variable plus evidence window plus mechanism version | effect query | causation |
| `CounterfactualCase` | factual intervention plus alternative intervention plus outcome variable plus mechanism version | effect query | causation |
| `CausalAssumptionSet` | effect estimate or counterfactual case plus assumption version | estimate or case | causation |
| `CausalSummary` | effect estimate plus planner context plus summary version | estimate | causation |

### `CausalVariable`

One modeled variable in the causal layer.

Purpose:
names a state, intervention, outcome, selection, regime, or confounder variable.

Required state:

- `VariableIdentity`
- `VariableScope`
- `VariableKind`
- `VariableMeasurement`
- `VariableProvenance`

Creation:
created by causal variable registration, intervention lowering, outcome linking, or mechanism definition.

Lifecycle:
active, deprecated, merged, archived.

Dependencies:
graph object refs, belief dimensions, outcome semantics, variable registry.

### `MechanismVersion`

One regime-conditioned mechanism candidate.

Purpose:
defines a candidate causal structure or update rule used to identify and estimate effects.

Required state:

- `MechanismIdentity`
- `MechanismScope`
- `MechanismStructure`
- `RegimeCondition`
- `MechanismProvenance`

Creation:
created by mechanism registration, mechanism selection, or regime-conditioned model update.

Lifecycle:
candidate, selected, superseded, archived.

Dependencies:
causal variables, graph relations, belief summaries, regime state, mechanism registry.

### `InterventionRecord`

One attempted intervention context.

Purpose:
records what action was attempted, intended target, actual target, scope, timing, and execution evidence.

Required state:

- `InterventionIdentity`
- `InterventionTarget`
- `InterventionSemantics`
- `InterventionTiming`
- `InterventionProvenance`

Creation:
created by lowering execution action and task outcome facts into causal intervention records.

Lifecycle:
candidate, applied, partially applied, failed, compensated.

Dependencies:
execution action facts, task run refs, graph object refs, belief evidence refs.

### `OutcomeLink`

One evidence link from intervention to measured outcome.

Purpose:
connects an intervention record to an observed outcome variable through explicit measurement evidence.

Required state:

- `OutcomeLinkIdentity`
- `OutcomeMeasurement`
- `OutcomeTiming`
- `OutcomeAttribution`
- `OutcomeProvenance`

Creation:
created by outcome linking over execution outcomes, graph state changes, and belief revisions.

Lifecycle:
candidate, accepted, rejected, superseded.

Dependencies:
intervention record, outcome variable, evidence item, measurement path.

### `ConfounderHypothesis`

One hidden-cause explanation candidate.

Purpose:
records a variable or context that may influence both intervention selection and outcome.

Required state:

- `ConfounderIdentity`
- `ConfounderScope`
- `ConfounderEvidence`
- `ConfounderRisk`
- `ConfounderProvenance`

Creation:
created by confounder discovery, selection-path interpretation, failed identification, or regime mismatch.

Lifecycle:
candidate, material, controlled, dismissed, archived.

Dependencies:
belief uncertainty, graph relations, selection paths, regime state, outcome links.

### `IdentificationAssessment`

One assessment of whether an effect is identifiable.

Purpose:
states whether available evidence and assumptions can support effect estimation for an intervention and outcome pair.

Required state:

- `IdentificationScope`
- `IdentificationStatus`
- `AdjustmentSet`
- `IdentificationBlockers`
- `IdentificationProvenance`

Creation:
created by identification assessment before or during effect estimation.

Lifecycle:
valid, blocked, provisional, expired.

Dependencies:
causal variables, confounder hypotheses, measurement paths, selection warnings, assumptions.

### `EffectEstimate`

One posterior effect summary.

Purpose:
records effect direction, magnitude, uncertainty, evidence window, identification status, and provenance.

Required state:

- `EffectIdentity`
- `EffectScope`
- `EffectPosterior`
- `EffectUncertainty`
- `EffectProvenance`

Creation:
created by effect estimation after identification assessment.

Lifecycle:
current, superseded, invalidated, archived.

Dependencies:
intervention records, outcome links, mechanism version, identification assessment, assumptions.

### `CounterfactualCase`

One alternative-world evaluation.

Purpose:
answers what outcome is expected under a different action, target, timing, or context.

Required state:

- `CounterfactualIdentity`
- `FactualCase`
- `AlternativeCase`
- `CounterfactualOutcome`
- `CounterfactualProvenance`

Creation:
created by counterfactual evaluation from an effect estimate and mechanism version.

Lifecycle:
current, superseded, invalidated, archived.

Dependencies:
effect estimate, mechanism version, assumptions, factual intervention, alternative intervention.

### `CausalAssumptionSet`

One explicit assumption set for an estimate or counterfactual.

Purpose:
records intervention, measurement, adjustment, selection, regime, and hidden-context assumptions.

Required state:

- `AssumptionScope`
- `CausalAssumptions`
- `AssumptionValidity`
- `AssumptionViolationEffect`
- `AssumptionProvenance`

Creation:
created with identification assessment, effect estimate, or counterfactual case.

Lifecycle:
active, violated, expired, superseded.

Dependencies:
mechanism version, identification assessment, evidence policy, regime context.

### `CausalSummary`

One planner-facing causal projection.

Purpose:
exposes supported effect, weak support, blocked identification, confounding, selection warnings, assumptions, and counterfactual state without exposing raw causal worker internals.

Required state:

- `CausalSummaryIdentity`
- `CausalEffectView`
- `CausalRiskView`
- `CausalAssumptionView`
- `CausalSummaryProvenance`

Creation:
created by causal summary projection.

Lifecycle:
current, stale, superseded, invalid.

Dependencies:
effect estimate, identification assessment, confounder hypotheses, assumption set, counterfactual case.

### Domain Type Rules

- `CausalVariable` is the identity root for mechanism reasoning.
- `InterventionRecord` records attempted action semantics, not proof of effect.
- `OutcomeLink` records measured outcome evidence, not attribution by itself.
- `IdentificationAssessment` gates effect estimation.
- `EffectEstimate` preserves identification status and uncertainty.
- `CounterfactualCase` depends on explicit mechanism and assumption state.
- `CausalSummary` is the public consumer view.

## Data Model

Causal data are typed contracts carried by causal domain types. They carry variable scope, intervention semantics, outcome measurements, selection paths, confounder state, identification status, effect posterior, counterfactual state, assumptions, summaries, and provenance.

### Data Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable causal record addressing | variable and estimate processes | all causal processes |
| Variable | variable kind, scope, and measurement shape | variable registration | intervention, outcome, mechanism |
| Mechanism | causal structure and regime condition | mechanism selection | identification and estimation |
| Intervention | action semantics and timing | intervention lowering | outcome linking and estimation |
| Outcome | measured effects and timing | outcome linking | identification and estimation |
| Selection | measurement and selection path warnings | selection interpretation | confounder and risk processes |
| Confounding | hidden-cause candidates and risk | confounder scoring | identification |
| Identification | adjustment set and blockers | identification assessment | effect estimation and summary |
| Effect | posterior effect and uncertainty | effect estimation | planner summary |
| Counterfactual | factual and alternative case state | counterfactual evaluation | planner summary |
| Assumption | explicit dependency set | identification and estimation | summary and planner |
| Provenance | source refs and hydration | every causal process | replay and explanation |

### Shape Rules

- Data contain typed causal data, not prose authority.
- Source-derived data carry graph, belief, spine, or execution refs.
- Derived data carry method version and evidence window.
- Identification fields remain separate from generic uncertainty.
- Confounder risk remains separate from effect uncertainty.
- Selection warnings remain visible to planner-facing summaries.

### Identity Data

| Data | Attached to | Purpose |
|---|---|---|
| `VariableIdentity` | `CausalVariable` | stable variable id |
| `MechanismIdentity` | `MechanismVersion` | stable mechanism version id |
| `InterventionIdentity` | `InterventionRecord` | stable intervention id |
| `EffectIdentity` | `EffectEstimate` | stable estimate id |

#### `VariableIdentity`

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

#### `MechanismIdentity`

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

#### `InterventionIdentity`

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

#### `EffectIdentity`

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

### Variable Data

| Data | Attached to | Purpose |
|---|---|---|
| `VariableScope` | `CausalVariable` | subject and context scope |
| `VariableKind` | `CausalVariable` | state, intervention, outcome, selection, regime, or confounder kind |
| `VariableMeasurement` | `CausalVariable` | evidence and measurement shape |

#### `VariableScope`

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

#### `VariableKind`

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

#### `VariableMeasurement`

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

### Mechanism Data

| Data | Attached to | Purpose |
|---|---|---|
| `MechanismScope` | `MechanismVersion` | variables and context |
| `MechanismStructure` | `MechanismVersion` | causal structure or model form |
| `RegimeCondition` | `MechanismVersion` | regime dependency |

#### `MechanismScope`

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

#### `MechanismStructure`

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

#### `RegimeCondition`

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

### Intervention Data

| Data | Attached to | Purpose |
|---|---|---|
| `InterventionTarget` | `InterventionRecord` | intended and actual target |
| `InterventionSemantics` | `InterventionRecord` | action meaning |
| `InterventionTiming` | `InterventionRecord` | application and delay window |

#### `InterventionTarget`

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

#### `InterventionSemantics`

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

#### `InterventionTiming`

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

### Outcome Data

| Data | Attached to | Purpose |
|---|---|---|
| `OutcomeMeasurement` | `OutcomeLink` | measured outcome value |
| `OutcomeTiming` | `OutcomeLink` | outcome observation timing |
| `OutcomeAttribution` | `OutcomeLink` | candidate attribution state |

#### `OutcomeMeasurement`

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

#### `OutcomeTiming`

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

#### `OutcomeAttribution`

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

### Selection Data

| Data | Attached to | Purpose |
|---|---|---|
| `MeasurementPath` | `OutcomeLink`, `CausalVariable` | how outcome became visible |
| `SelectionWarning` | `OutcomeLink`, `EffectEstimate`, `CausalSummary` | selection-shaped evidence warning |

#### `MeasurementPath`

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

#### `SelectionWarning`

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

### Confounding Data

| Data | Attached to | Purpose |
|---|---|---|
| `ConfounderScope` | `ConfounderHypothesis` | affected variables |
| `ConfounderEvidence` | `ConfounderHypothesis` | evidence supporting hidden cause |
| `ConfounderRisk` | `ConfounderHypothesis`, `IdentificationAssessment`, `EffectEstimate` | risk to identification |

#### `ConfounderScope`

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

#### `ConfounderEvidence`

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

#### `ConfounderRisk`

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

### Identification Data

| Data | Attached to | Purpose |
|---|---|---|
| `IdentificationScope` | `IdentificationAssessment` | intervention and outcome query |
| `IdentificationStatus` | `IdentificationAssessment`, `EffectEstimate` | identifiability state |
| `AdjustmentSet` | `IdentificationAssessment` | assumed adjustment variables |
| `IdentificationBlockers` | `IdentificationAssessment` | blockers and missing evidence |

#### `IdentificationScope`

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

#### `IdentificationStatus`

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

#### `AdjustmentSet`

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

#### `IdentificationBlockers`

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

### Effect Data

| Data | Attached to | Purpose |
|---|---|---|
| `EffectScope` | `EffectEstimate` | intervention and outcome variables |
| `EffectPosterior` | `EffectEstimate` | effect direction and magnitude |
| `EffectUncertainty` | `EffectEstimate` | effect uncertainty |

#### `EffectScope`

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

#### `EffectPosterior`

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

#### `EffectUncertainty`

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

### Counterfactual Data

| Data | Attached to | Purpose |
|---|---|---|
| `FactualCase` | `CounterfactualCase` | observed case |
| `AlternativeCase` | `CounterfactualCase` | alternative intervention |
| `CounterfactualOutcome` | `CounterfactualCase` | predicted alternative outcome |

#### `FactualCase`

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

#### `AlternativeCase`

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

#### `CounterfactualOutcome`

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

### Assumption Data

| Data | Attached to | Purpose |
|---|---|---|
| `AssumptionScope` | `CausalAssumptionSet` | owning estimate or case |
| `CausalAssumptions` | `CausalAssumptionSet` | assumption list |
| `AssumptionValidity` | `CausalAssumptionSet` | validity state |
| `AssumptionViolationEffect` | `CausalAssumptionSet` | effect of violation |

#### `AssumptionScope`

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

#### `CausalAssumptions`

Purpose:
records explicit causal, intervention, measurement, adjustment, and regime assumptions.

Dependencies:
mechanism version, evidence policy, regime condition.

```rust
struct CausalAssumptions {
    assumptions: Vec<CausalAssumption>,
}
```

#### `AssumptionValidity`

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

#### `AssumptionViolationEffect`

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

### Summary Data

| Data | Attached to | Purpose |
|---|---|---|
| `CausalEffectView` | `CausalSummary` | planner-facing effect support |
| `CausalRiskView` | `CausalSummary` | planner-facing risk state |
| `CausalAssumptionView` | `CausalSummary` | planner-facing assumptions |

#### `CausalEffectView`

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

#### `CausalRiskView`

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

#### `CausalAssumptionView`

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

### Provenance Data

| Data | Attached to | Purpose |
|---|---|---|
| `VariableProvenance` | `CausalVariable` | source refs for variable |
| `MechanismProvenance` | `MechanismVersion` | source refs for mechanism |
| `InterventionProvenance` | `InterventionRecord` | source refs for intervention |
| `OutcomeProvenance` | `OutcomeLink` | source refs for outcome link |
| `EffectProvenance` | `EffectEstimate` | source refs for effect |
| `CounterfactualProvenance` | `CounterfactualCase` | source refs for counterfactual |
| `CausalSummaryProvenance` | `CausalSummary` | hydration refs for summary |

#### `VariableProvenance`

Purpose:
preserves source refs for causal variable creation.

Dependencies:
graph refs, belief refs, variable registry.

```rust
struct VariableProvenance {
    source_refs: Vec<SourceRef>,
}
```

#### `MechanismProvenance`

Purpose:
preserves source refs for mechanism state.

Dependencies:
graph relations, belief summaries, regime refs.

```rust
struct MechanismProvenance {
    source_refs: Vec<SourceRef>,
}
```

#### `InterventionProvenance`

Purpose:
preserves source refs for intervention lowering.

Dependencies:
execution facts and graph refs.

```rust
struct InterventionProvenance {
    source_refs: Vec<SourceRef>,
}
```

#### `OutcomeProvenance`

Purpose:
preserves source refs for outcome linking.

Dependencies:
evidence refs, graph refs, belief refs.

```rust
struct OutcomeProvenance {
    source_refs: Vec<SourceRef>,
}
```

#### `EffectProvenance`

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

#### `CounterfactualProvenance`

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

#### `CausalSummaryProvenance`

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

### Shared Data Shapes

#### `SourceRef`

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

#### `Score`

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

#### `EvidenceWindow`

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

### Data Rules

- Variable data names modeled causal quantities.
- Intervention data names action semantics, not proof of effect.
- Outcome data names measured state, not attribution by itself.
- Identification data gates effect estimation.
- Effect data preserves identification and confounding state.
- Counterfactual data depends on explicit assumptions.
- Summary data hides worker internals and preserves explanation refs.

## Pipelines

Causal pipelines transform graph, belief, regime, and execution evidence into intervention records, outcome links, mechanism state, identification assessments, effect estimates, counterfactual cases, assumptions, and planner-facing summaries. They separate evidence of temporal change from justified causal claims. They do not settle belief, decide regime identity, rank planner policy, or dispatch tasks.

### Pipeline Overview

| Phase | Stage | Required input | Required output |
|---|---|---|---|
| 1 | variable registration | graph refs, belief dimensions, outcome semantics | `CausalVariable` |
| 2 | intervention lowering | execution action and task facts | `InterventionRecord` |
| 3 | outcome linking | intervention records, outcome evidence | `OutcomeLink` |
| 4 | measurement path interpretation | graph provenance, evidence channels | measurement paths and selection warnings |
| 5 | mechanism selection | variables, graph relations, belief state, regime context | `MechanismVersion` |
| 6 | confounder discovery | graph paths, belief uncertainty, selection state | `ConfounderHypothesis` |
| 7 | confounder scoring | confounder evidence, adjustment candidates | confounder risk |
| 8 | identification assessment | mechanism, confounders, measurement paths | `IdentificationAssessment` |
| 9 | effect estimation | identified evidence, mechanism, assumptions | `EffectEstimate` |
| 10 | counterfactual evaluation | effect estimate, factual case, alternative case | `CounterfactualCase` |
| 11 | assumption projection | identification, estimate, counterfactual | `CausalAssumptionSet` |
| 12 | causal summary projection | effect, risks, assumptions, counterfactual | `CausalSummary` |
| 13 | recovery and replay | source refs, method versions, cursors | rebuilt causal projections |

Replay invariant:
same graph records, belief views, execution facts, regime context, method versions, and source cursors produce the same causal outputs.

### Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| graph current state and walks | graph | variable scope, measurement paths, selection paths |
| graph provenance and lineage | graph | anchor selection warnings and supersession context |
| belief views | belief | uncertainty, freshness, evidence quality, outcome state |
| execution action and outcome facts | execution through spine | intervention lowering and outcome linking |
| regime summaries | regime | mechanism condition and assumption state |
| planner context | planner or query route | scoped causal summaries |

### Variable Registration

Input:
graph refs, belief dimensions, outcome semantics, intervention semantics.

Output:
`CausalVariable`.

Rules:

- Register state variables for persistent world conditions.
- Register intervention variables for deliberate actions and control moves.
- Register outcome variables for measured quality, utility, success, or downstream effect.
- Register selection variables when visibility or attachment policy affects observed data.
- Register regime variables only as mechanism conditions, not as regime authority.
- Preserve source refs for every variable.

### Intervention Lowering

Input:
execution action facts, task lifecycle facts, capability results, graph refs.

Output:
`InterventionRecord`.

Rules:

- Record intended target and actual target when known.
- Record intervention kind and application status.
- Record timing and expected delay window.
- Record rollback or compensation when available.
- Treat every intervention as candidate causal evidence until identification supports effect claims.

### Outcome Linking

Input:
intervention records, graph state changes, belief revisions, execution outcomes, measurement evidence.

Output:
`OutcomeLink`.

Rules:

- Link outcomes to interventions through explicit evidence refs.
- Preserve measurement channel and measurement quality.
- Preserve observed time and lag from intervention.
- Mark attribution as candidate until identification assessment.
- Reject outcome links with no compatible outcome variable or measurement path.

### Measurement Path Interpretation

Input:
graph provenance, anchor lineage, evidence channels, belief origin state.

Output:
measurement paths and selection warnings.

Rules:

- Record how the outcome became visible.
- Identify selection-shaped evidence when anchor choice may explain observed outcome.
- Identify measurement-shaped evidence when channel policy may explain observed outcome.
- Preserve graph paths and evidence refs.
- Keep selection warnings visible in effect summaries.

### Mechanism Selection

Input:
causal variables, graph relations, belief summaries, regime context.

Output:
`MechanismVersion`.

Rules:

- Select a mechanism only for scoped variables and applicable regime context.
- Preserve parent links and selection links.
- Preserve method version.
- Mark candidate mechanism when evidence is insufficient.
- Do not infer mechanism from temporal order alone.

### Confounder Discovery

Input:
graph paths, belief uncertainty, regime state, selection paths, outcome links.

Output:
`ConfounderHypothesis`.

Rules:

- Identify hidden or observed variables that may influence intervention and outcome.
- Preserve supporting and weakening evidence refs.
- Emit confounder hypotheses when identification depends on controlling them.
- Keep confounders separate from generic uncertainty.

### Confounder Scoring

Input:
confounder hypotheses, evidence quality, adjustment candidates.

Output:
confounder risk.

Rules:

- Score confounder severity.
- Record whether adjustment controls the confounder.
- Preserve risk reason codes.
- Feed unresolved confounders into identification blockers.

### Identification Assessment

Input:
mechanism version, intervention variable, outcome variable, confounder risks, measurement paths, assumptions.

Output:
`IdentificationAssessment`.

Rules:

- Determine whether effect estimation is supported, blocked, provisional, or expired.
- Preserve adjustment set and assumption refs.
- Preserve missing evidence and blocker list.
- Treat material selection warning as identification risk.
- Block effect estimation when required confounders are uncontrolled.

### Effect Estimation

Input:
identified evidence, outcome links, mechanism version, assumptions.

Output:
`EffectEstimate`.

Rules:

- Estimate effect direction and magnitude only after identification assessment.
- Preserve effect uncertainty and precision.
- Preserve identification status on the estimate.
- Preserve confounder risk and selection warning on the estimate.
- Mark estimate weak or blocked when identification is weak or blocked.

### Counterfactual Evaluation

Input:
effect estimate, factual intervention, observed outcome, alternative intervention or context, assumptions.

Output:
`CounterfactualCase`.

Rules:

- State factual intervention and observed outcome.
- State alternative action, target, timing, context, or regime.
- Use mechanism version and assumptions explicitly.
- Preserve uncertainty and confidence.
- Mark counterfactual invalid when mechanism or assumptions expire.

### Assumption Projection

Input:
identification assessment, effect estimate, counterfactual case, mechanism version, regime context.

Output:
`CausalAssumptionSet`.

Rules:

- Record causal structure assumptions.
- Record intervention application assumptions.
- Record measurement and selection assumptions.
- Record adjustment set assumptions.
- Record regime and hidden-context assumptions.
- Record violation effect and expiry.

### Causal Summary Projection

Input:
effect estimate, identification assessment, confounder risks, selection warnings, assumptions, counterfactual cases.

Output:
`CausalSummary`.

Rules:

- Expose effect direction, magnitude, uncertainty, and identification state.
- Expose confounder risk and selection warning.
- Expose assumption refs and validity state.
- Expose counterfactual summary when available.
- Exclude raw worker internals and uncommitted drafts.

### Recovery And Replay

Input:
source refs, causal records, method versions, source cursors.

Output:
rebuilt causal projections.

Rules:

- Rebuild intervention records from execution facts.
- Rebuild outcome links from outcome evidence and graph state.
- Rebuild identification and effect estimates from method versions and source refs.
- Preserve old estimates when method version changes by publishing new estimates.
- Do not rewrite old causal results.

### Failure Semantics

| Condition | Causal behavior |
|---|---|
| intervention lacks target | reject intervention lowering with reason |
| outcome lacks measurement path | keep outcome unlinked or rejected with reason |
| selection warning is material | mark identification provisional or blocked |
| uncontrolled confounder is material | block identification |
| mechanism is unavailable | emit missing mechanism status |
| regime condition is invalid | expire mechanism-dependent estimate |
| counterfactual lacks assumptions | mark counterfactual invalid |
| source evidence changes | publish new estimate instead of editing old result |

### Test Matrix

| Test | Expected proof |
|---|---|
| intervention lowering | execution fact becomes intervention record |
| outcome linking | outcome evidence links through explicit measurement path |
| anchor selection guard | selected anchor alone does not imply effect |
| confounder block | uncontrolled confounder blocks identification |
| selection warning | selection-shaped evidence remains visible |
| identification gate | blocked identification prevents strong effect estimate |
| effect replay | same inputs and method version produce same estimate |
| counterfactual assumptions | missing or expired assumption invalidates case |
| summary boundary | causal summary excludes worker internals |
| estimate supersession | source evidence change creates new estimate |

### Pipeline Rules

- Stages are deterministic over explicit inputs and method versions.
- Stages preserve graph, belief, execution, regime, and causal source refs.
- Stages emit weak support or blocked identification instead of fabricating effect support.
- Stages keep intervention evidence separate from outcome attribution.
- Stages keep selection warnings and confounder risks explicit.
