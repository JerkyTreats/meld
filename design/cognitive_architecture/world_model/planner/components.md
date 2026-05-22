# Planner Components

Date: 2026-05-14
Status: active
Scope: ECS components for `world_model/planner` projections

## Thesis

Planner components are typed data contracts attached to planner entities.

Entities provide identity.
Components provide state, refs, summaries, scores, and projection metadata.
Systems read and write components to produce deterministic planner views.

Planner uses one component per system contract.
Nested data types carry detail inside the component when the detail has no independent system lifecycle.

## Component Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable addressing for projection records | planner and source adapters | every planner system |
| Context | scope, time, branch, horizon, and perspective | planner route | all systems |
| Lens | admissibility and tolerance settings | agent source or query envelope | normalization and scoring |
| Source refs | lower-layer object and revision references | source input projections | all output projections |
| Source summaries | lower-layer values normalized for planner use | source input projections | output projections |
| Derived outputs | planner-owned assessment, risk, conflict, assumption, and abstention state | planner systems | view assembly |
| Hydration | explanation and replay handles | source input and output systems | public route and execution handoff |

## Shape Rules

- Components contain typed data, not prose authority.
- Components carry source refs when their data derives from lower-layer records.
- Components carry projection version when their data is planner-derived.
- Components use nested structs for detail that has no independent identity.
- Components expose reason codes and typed statuses before presentation text.
- Components never replace lower-layer authority records.

## Identity Components

| Component | Attached to | Purpose |
|---|---|---|
| `DecisionContextIdentity` | `DecisionContext` | stable context key |
| `ProjectionRun` | `WorldModelView` | projection version and source boundary |
| `ViewIdentity` | `WorldModelView` | root output identity |
| `InputPacketIdentity` | planner input packet entities | source packet identity |
| `ChildOutputIdentity` | planner output child entities | stable child identity |

### `DecisionContextIdentity`

Purpose:
stable identity for one canonical planning question.

Dependencies:
`DecisionContext`, canonical serialization version, context hash algorithm.

```rust
struct DecisionContextIdentity {
    context_id: DecisionContextId,
    canonical_serialization_version: SchemaVersion,
    context_hash: ContentHash,
}
```

### `ProjectionRun`

Purpose:
records the projection version and source boundary used for one planner view.

Dependencies:
`SourceCursorSet`, projection version registry, planner route invocation.

```rust
struct ProjectionRun {
    projection_version: ProjectionVersion,
    projection_run_id: ProjectionRunId,
    source_cursor_set: SourceCursorSet,
}
```

### `ViewIdentity`

Purpose:
stable root identity for one assembled `WorldModelView`.

Dependencies:
`DecisionContextIdentity`, `AgentLens`, `ProjectionRun`.

```rust
struct ViewIdentity {
    world_model_view_id: WorldModelViewId,
    context_id: DecisionContextId,
    lens_id: PlannerAgentLensId,
    source_cursor_set_hash: ContentHash,
}
```

### `InputPacketIdentity`

Purpose:
stable identity for one lower-layer packet admitted into planner projection.

Dependencies:
`DecisionContextIdentity`, source layer cursor, source packet schema version.

```rust
struct InputPacketIdentity {
    packet_id: InputPacketId,
    context_id: DecisionContextId,
    source_layer: SourceLayer,
    source_cursor_set: SourceCursorSet,
    input_version: SchemaVersion,
}
```

### `ChildOutputIdentity`

Purpose:
stable identity for one planner output child under a `WorldModelView`.

Dependencies:
`ViewIdentity`, output kind registry, local output key rule.

```rust
struct ChildOutputIdentity {
    child_id: PlannerOutputId,
    world_model_view_id: WorldModelViewId,
    output_kind: PlannerOutputKind,
    local_key: OutputLocalKey,
}
```

## Context Components

| Component | Attached to | Purpose |
|---|---|---|
| `DecisionScope` | `DecisionContext` | scoped planning question |
| `ReplayBoundary` | `WorldModelView` | replay and validity boundary |
| `AgentLens` | `PlannerAgentLens` | perspective-specific admissibility |

### `DecisionScope`

Purpose:
defines the scoped planning question that every planner system answers.

Dependencies:
public planner query envelope, perspective contract, branch contract, time contracts.

```rust
struct DecisionScope {
    perspective: Perspective,
    subject_scope: SubjectScope,
    branch_scope: BranchScope,
    reference_time: ReferenceTime,
    transaction_time: TransactionTime,
    desired_predicate: Option<BeliefPredicate>,
    candidate_intervention: Option<InterventionRef>,
    decision_horizon: DecisionHorizon,
}
```

### `ReplayBoundary`

Purpose:
captures replay and validity boundaries for an assembled planner view.

Dependencies:
`ProjectionRun`, `InputPacketIdentity`, source cursor contracts.

```rust
struct ReplayBoundary {
    event_cursor: EventCursor,
    source_cursor_set: SourceCursorSet,
    reference_time: ReferenceTime,
    transaction_time: TransactionTime,
    projection_version: ProjectionVersion,
    expiry_horizon: Option<ReferenceTime>,
    input_packet_ids: Vec<InputPacketId>,
}
```

### `AgentLens`

Purpose:
defines perspective-specific admissibility, trust, observation, and tolerance rules.

Dependencies:
agent domain lens record, perspective contract, evidence policy contract.

```rust
struct AgentLens {
    agent_id: AgentId,
    lens_id: PlannerAgentLensId,
    lens_version: SchemaVersion,
    perspective: Perspective,
    trust_profile: TrustProfile,
    evidence_policy: EvidencePolicy,
    observation_scope: ObservationScope,
    tolerated_uncertainty: ToleratedUncertainty,
    regime_sensitivity_profile: RegimeSensitivityProfile,
}
```

## Source Ref Components

| Component | Attached to | Purpose |
|---|---|---|
| `GraphInputRefs` | `PlannerGraphInput` | graph source refs and cursor |
| `BeliefInputRefs` | `PlannerBeliefInput` | belief source refs and cursor |
| `CausalInputRefs` | `PlannerCausalInput` | causal source refs and cursor |
| `RegimeInputRefs` | `PlannerRegimeInput` | regime source refs and cursor |

### `GraphInputRefs`

Purpose:
preserves graph refs and graph cursor state used by planner projection.

Dependencies:
graph packet read port, graph source cursor, graph provenance records.

```rust
struct GraphInputRefs {
    anchor_ids: Vec<AnchorId>,
    anchor_refs: Vec<DomainObjectRef>,
    lineage_ids: Vec<LineageId>,
    relation_walk_ids: Vec<GraphWalkId>,
    object_history_ids: Vec<ObjectHistoryId>,
    provenance_ids: Vec<ProvenanceId>,
    branch_refs: Vec<BranchRef>,
    source_cursor: SourceCursor,
}
```

### `BeliefInputRefs`

Purpose:
preserves belief refs and belief cursor state used by planner projection.

Dependencies:
belief packet read port, belief revision records, evidence records, comparator records.

```rust
struct BeliefInputRefs {
    belief_key: BeliefKey,
    belief_revision_ids: Vec<BeliefRevisionId>,
    evidence_refs: Vec<EvidenceRef>,
    comparator_refs: Vec<ComparatorRef>,
    assessment_epoch: AssessmentEpoch,
    source_cursor: SourceCursor,
}
```

### `CausalInputRefs`

Purpose:
preserves causal refs and causal cursor state used by planner projection.

Dependencies:
causal packet read port, causal claims, effect estimates, intervention and outcome links.

```rust
struct CausalInputRefs {
    causal_variable_refs: Vec<CausalVariableRef>,
    effect_estimate_refs: Vec<EffectEstimateRef>,
    intervention_refs: Vec<InterventionRef>,
    outcome_link_refs: Vec<OutcomeLinkRef>,
    confounder_refs: Vec<ConfounderRef>,
    causal_claim_refs: Vec<CausalClaimRef>,
    source_cursor: SourceCursor,
}
```

### `RegimeInputRefs`

Purpose:
preserves regime refs and regime cursor state used by planner projection.

Dependencies:
regime packet read port, active segment records, changepoint records, stress scenarios.

```rust
struct RegimeInputRefs {
    regime_ids: Vec<RegimeId>,
    active_segment_ids: Vec<ActiveSegmentId>,
    changepoint_ids: Vec<ChangepointId>,
    mixture_prediction_refs: Vec<MixturePredictionRef>,
    stress_scenario_refs: Vec<StressScenarioRef>,
    source_cursor: SourceCursor,
}
```

## Source Summary Components

| Component | Attached to | Purpose |
|---|---|---|
| `BeliefSummary` | `PlannerBeliefInput`, `ActionableBeliefView` | belief state normalized for planner use |
| `CausalSummary` | `PlannerCausalInput`, `CausalEffectSummary` | causal effect state normalized for planner use |
| `RegimeSummary` | `PlannerRegimeInput`, `SensitivitySummary`, `RiskEnvelope` | structural uncertainty normalized for planner use |

### `BeliefSummary`

Purpose:
normalizes belief state into the planner data model.

Dependencies:
`BeliefInputRefs`, belief view contract, evidence refs, belief revision refs.

```rust
struct BeliefSummary {
    subject: DomainObjectRef,
    dimension: BeliefDimension,
    posterior: PosteriorSummary,
    uncertainty: UncertaintySummary,
    precision: PrecisionSummary,
    freshness: FreshnessSummary,
    contradiction: ContradictionSummary,
    origin: OriginSummary,
    coverage: CoverageSummary,
    assessment: AssessmentState,
    source_refs: Vec<SourceRef>,
}
```

### `CausalSummary`

Purpose:
normalizes causal effect and intervention state into the planner data model.

Dependencies:
`CausalInputRefs`, causal output contract, effect estimate refs, assumption refs.

```rust
struct CausalSummary {
    intervention_variable: CausalVariableRef,
    target_variable: CausalVariableRef,
    outcome_variable: CausalVariableRef,
    effect_direction: EffectDirection,
    effect_size: EffectSize,
    effect_uncertainty: UncertaintySummary,
    identification: IdentificationSummary,
    selection_warning: Option<SelectionWarning>,
    counterfactual: Option<CounterfactualSummary>,
    assumption_refs: Vec<AssumptionRef>,
    source_refs: Vec<SourceRef>,
}
```

### `RegimeSummary`

Purpose:
normalizes structural uncertainty and regime sensitivity into the planner data model.

Dependencies:
`RegimeInputRefs`, regime output contract, stress metrics, sensitivity set.

```rust
struct RegimeSummary {
    regime_posterior: RegimePosterior,
    active_segment_status: ActiveSegmentStatus,
    changepoint_state: ChangepointState,
    run_length: RunLengthBelief,
    continuation_score: Score,
    break_score: Score,
    mixture_prediction: Option<MixturePrediction>,
    sensitivity_set: SensitivitySet,
    stress_metrics: Vec<StressMetric>,
    source_refs: Vec<SourceRef>,
}
```

## Derived Output Components

| Component | Attached to | Purpose |
|---|---|---|
| `DecisionSupport` | `ActionableBeliefView` | predicate support and decision relevance |
| `ObservationValue` | `ObservationOpportunityView` | information gain, cost, and target evidence |
| `PreconditionEvaluation` | `PreconditionAssessment` | world-facing condition status |
| `RiskDimensions` | `RiskEnvelope` | typed risk dimensions |
| `ConflictImpact` | `ConflictSummary` | contradiction and conflict impact |
| `SensitivityConditions` | `SensitivitySummary` | lower-layer changes that flip or weaken the view |
| `Assumptions` | `AssumptionSet` | explicit dependencies |
| `AbstentionGrounds` | `AbstentionState` | no-commit or proceed-under-assumption state |

### `DecisionSupport`

Purpose:
records how a belief summary supports, blocks, or leaves uncertain the scoped decision.

Dependencies:
`BeliefSummary`, `DecisionScope`, `AgentLens`, `Score`.

```rust
struct DecisionSupport {
    affected_predicate: BeliefPredicate,
    affected_intervention: Option<InterventionRef>,
    support_status: SupportStatus,
    support_level: Score,
    decision_relevance: Score,
    reason_codes: Vec<ReasonCode>,
    blocking: bool,
    source_refs: Vec<SourceRef>,
    projection_version: ProjectionVersion,
}
```

### `ObservationValue`

Purpose:
records the planner value of an information-gathering opportunity.

Dependencies:
`BeliefSummary`, source observation opportunity, evidence channel contract, cost estimate contract.

```rust
struct ObservationValue {
    target_belief: BeliefKey,
    target_evidence: EvidenceTarget,
    evidence_channel: EvidenceChannelRef,
    expected_information_gain: Score,
    observation_cost: CostEstimate,
    opportunity_cost: CostEstimate,
    expiry: Option<ReferenceTime>,
    blocking: bool,
    source_refs: Vec<SourceRef>,
    projection_version: ProjectionVersion,
}
```

### `PreconditionEvaluation`

Purpose:
records world-facing condition status for a desired predicate or candidate intervention.

Dependencies:
`DecisionScope`, `BeliefSummary`, `GraphInputRefs`, gate rules.

```rust
struct PreconditionEvaluation {
    predicate_ref: PredicateRef,
    assessment_status: PreconditionStatus,
    support_level: Score,
    blocker_reason: Option<ReasonCode>,
    freshness_gate: GateResult,
    confidence_gate: GateResult,
    source_refs: Vec<SourceRef>,
    projection_version: ProjectionVersion,
}
```

### `RiskDimensions`

Purpose:
collects typed risk dimensions for one assembled planner view.

Dependencies:
`BeliefSummary`, `CausalSummary`, `RegimeSummary`, `AgentLens`, risk threshold rules.

```rust
struct RiskDimensions {
    dimensions: Vec<RiskDimension>,
    blocking_dimensions: Vec<RiskKind>,
    aggregate_blocking: bool,
    source_refs: Vec<SourceRef>,
    projection_version: ProjectionVersion,
}
```

### `ConflictImpact`

Purpose:
records contradiction or conflict impact on the scoped decision.

Dependencies:
`BeliefSummary`, `DecisionScope`, contradiction taxonomy, evidence refs.

```rust
struct ConflictImpact {
    conflict_kind: ConflictKind,
    affected_belief_key: BeliefKey,
    competing_refs: Vec<SourceRef>,
    decision_impact: DecisionImpact,
    blocking: bool,
    required_observation_target: Option<EvidenceTarget>,
    projection_version: ProjectionVersion,
}
```

### `SensitivityConditions`

Purpose:
records lower-layer changes that would flip or weaken a planner output.

Dependencies:
`RegimeSummary`, planner child output refs, sensitivity condition rules.

```rust
struct SensitivityConditions {
    affected_output: PlannerOutputId,
    source_layer: SourceLayer,
    flip_condition: SensitivityCondition,
    weakening_condition: SensitivityCondition,
    expiry_rule: Option<ExpiryRule>,
    source_refs: Vec<SourceRef>,
    projection_version: ProjectionVersion,
}
```

### `Assumptions`

Purpose:
collects explicit assumptions that a planner view depends on.

Dependencies:
`BeliefSummary`, `CausalSummary`, `RegimeSummary`, `DecisionScope`.

```rust
struct Assumptions {
    assumptions: Vec<PlannerAssumption>,
    source_refs: Vec<SourceRef>,
    projection_version: ProjectionVersion,
}
```

### `AbstentionGrounds`

Purpose:
records no-commit state or proceed-under-assumption state for the planner view.

Dependencies:
`RiskDimensions`, `ConflictImpact`, `Assumptions`, source packet warnings.

```rust
struct AbstentionGrounds {
    abstention_kind: AbstentionKind,
    reason_codes: Vec<ReasonCode>,
    source_refs: Vec<SourceRef>,
    proceed_under_assumptions: bool,
    projection_version: ProjectionVersion,
}
```

## Hydration Components

| Component | Attached to | Purpose |
|---|---|---|
| `HydrationHandles` | every planner input and output that needs explanation or replay | stable lower-layer dereference handles |

### `HydrationHandles`

Purpose:
attaches dereference handles for explanation, replay, and execution handoff.

Dependencies:
source refs from admitted planner inputs and derived planner outputs.

```rust
struct HydrationHandles {
    handles: Vec<HydrationHandle>,
}
```

```rust
struct HydrationHandle {
    handle_kind: HydrationHandleKind,
    source_layer: SourceLayer,
    domain_object_ref: DomainObjectRef,
    source_id: SourceId,
    source_cursor: SourceCursor,
    projection_path: ProjectionPath,
    explanation_role: ExplanationRole,
}
```

## Shared Data Shapes

### `SourceRef`

Purpose:
shared reference shape for lower-layer records used by planner components.

Dependencies:
source layer id vocabulary, source cursor contract, domain object ref contract.

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
shared score shape for relevance, information gain, severity, support, and gates.

Dependencies:
score scale registry, threshold basis rules, projection version.

```rust
struct Score {
    value: f64,
    scale: ScoreScale,
    threshold_basis: ThresholdBasis,
    source_fields: Vec<SourceFieldRef>,
    projection_version: ProjectionVersion,
}
```

### `RiskDimension`

Purpose:
one typed risk entry inside a planner risk envelope.

Dependencies:
`Score`, `SourceRef`, risk kind vocabulary, mitigation hint vocabulary.

```rust
struct RiskDimension {
    risk_kind: RiskKind,
    severity: Score,
    source_layer: SourceLayer,
    source_refs: Vec<SourceRef>,
    mitigation_hint: Option<MitigationHint>,
    blocking: bool,
}
```

### `PlannerAssumption`

Purpose:
one explicit assumption used by a planner view.

Dependencies:
`SourceRef`, assumption kind vocabulary, validity scope contract, expiry rule contract.

```rust
struct PlannerAssumption {
    assumption_kind: AssumptionKind,
    source_refs: Vec<SourceRef>,
    validity_scope: ValidityScope,
    violation_effect: ViolationEffect,
    expiry_rule: Option<ExpiryRule>,
}
```

## Component Rules

- Identity components define stable keys for cache, replay, and child refs.
- Context and lens components define admissibility before scoring.
- Source ref components preserve lower-layer authority boundaries.
- Source summary components are read-only from planner perspective.
- Derived output components cite source refs and projection version.
- Hydration components support explanation and replay.
- Scores carry scale metadata and threshold basis.
- Expiry fields feed `ReplayBoundary`.

## Read With

- [Planner Entities](entities.md)
- [Planner Systems](systems.md)
- [World Model Planner](README.md)
