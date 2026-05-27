# Planner Spec

`planner` is a projection domain. It assembles action-relevant world-model views from `belief`, `causation`, and `regime` rather than performing deep inference. It remains inside `world_model`, so it stays epistemic rather than operational.

## Domain Types

### Type Categories

| Category | Types | Persistence expectation |
|---|---|---|
| Root query type | `DecisionContext` | value object, cache key, optional audit record |
| Lens type | `PlannerAgentLens` | read from agent domain, not owned by planner |
| Source packet types | `PlannerGraphInput`, `PlannerBeliefInput`, `PlannerCausalInput`, `PlannerRegimeInput` | projection inputs, cacheable by source cursor set |
| View root type | `WorldModelView` | read model result, materializable for replay and explanation |
| View child types | `ActionableBeliefView`, `ObservationOpportunityView`, `PreconditionAssessment`, `CausalEffectSummary`, `RiskEnvelope`, `AbstentionState`, `ConflictSummary`, `SensitivitySummary`, `AssumptionSet` | child records under one view |
| Reference value | `HydrationHandle` | data value unless independently indexed |

### Identity Rules

| Type | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `DecisionContext` | canonical hash of perspective, subject scope, branch scope, times, desired predicate, candidate intervention, and horizon | none | planner query contract |
| `ViewSnapshot` | canonical hash of source cursor set, projection version, reference time, transaction time, and expiry horizon | `WorldModelView` | planner projection |
| `PlannerAgentLens` | agent id plus lens version | none | agent domain |
| `PlannerGraphInput` | context id plus graph cursor set plus graph input version | `DecisionContext` | graph domain |
| `PlannerBeliefInput` | context id plus belief key plus belief revision id plus belief input version | `DecisionContext` | belief domain |
| `PlannerCausalInput` | context id plus causal claim or effect estimate ref plus causal input version | `DecisionContext` | causation domain |
| `PlannerRegimeInput` | context id plus active segment or regime posterior ref plus regime input version | `DecisionContext` | regime domain |
| `WorldModelView` | context id plus lens id plus source cursor set plus projection version | `DecisionContext` | planner domain |
| `ActionableBeliefView` | world model view id plus belief key plus predicate ref | `WorldModelView` | planner domain |
| `ObservationOpportunityView` | world model view id plus target evidence ref plus channel ref | `WorldModelView` | planner domain |
| `PreconditionAssessment` | world model view id plus predicate ref plus candidate intervention ref when present | `WorldModelView` | planner domain |
| `CausalEffectSummary` | world model view id plus causal claim ref or effect estimate ref | `WorldModelView` | planner domain |
| `RiskEnvelope` | world model view id | `WorldModelView` | planner domain |
| `AbstentionState` | world model view id | `WorldModelView` | planner domain |
| `ConflictSummary` | world model view id plus affected belief key plus conflict kind | `WorldModelView` | planner domain |
| `SensitivitySummary` | world model view id plus affected output ref plus source layer | `WorldModelView` | planner domain |
| `AssumptionSet` | world model view id | `WorldModelView` | planner domain |
| `HydrationHandle` | source layer plus domain object ref plus source id plus explanation role | any output type | source layer plus planner projection |

### Context Types

#### `DecisionContext`

One scoped planning question.

Required fields:

- perspective
- subject scope
- branch scope
- reference time
- transaction time
- desired predicate
- candidate intervention
- decision horizon

Implementation requirement:
define canonical serialization before any cache, replay, or query route uses `DecisionContext`.

Open gap:
`SubjectScope`, `BranchScope`, `ReferenceTime`, `TransactionTime`, `BeliefPredicate`, `InterventionRef`, and `DecisionHorizon` need shared contracts.

#### `ViewSnapshot`

One replay and validity boundary for a completed view.

Required fields:

- event cursor
- source cursor set by layer
- reference time
- transaction time
- projection version
- expiry horizon
- input packet ids

Implementation requirement:
`ViewSnapshot` must be attached to every `WorldModelView` and must be sufficient to rerun projection from the same lower records.

Open gap:
source cursor shape is not yet shared across graph, belief, causation, and regime.

#### `PlannerAgentLens`

One perspective-specific admissibility lens.

Required fields:

- agent id
- lens version
- perspective
- trust profile
- evidence policy
- observation scope
- tolerated uncertainty
- regime sensitivity profile

Implementation requirement:
planner must receive the lens through an agent read port or by value in the query envelope.
Planner must not import agent internals.

Open gap:
agent lens contracts are not yet defined as typed public records.

### Input Packet Types

#### `PlannerGraphInput`

One graph packet scoped to a `DecisionContext`.

Carries:

- subject refs
- current anchor records
- lineage records
- relation neighborhood
- object history
- provenance bundles
- branch presence
- graph hydration handles
- graph source cursor

Implementation requirement:
graph input must be produced by a graph-owned projection or read adapter.
Planner may filter graph input for context, but planner must not derive truth, confidence, or causal support from graph input alone.

Open gap:
`ObjectHistory`, `BranchPresence`, and graph source cursor contracts are not fully exposed in current implementation.

#### `PlannerBeliefInput`

One belief packet scoped to a `DecisionContext`.

Carries:

- belief key
- subject ref
- dimension
- posterior summary
- uncertainty summary
- precision summary
- freshness summary
- contradiction state
- origin state
- coverage state
- evidence refs
- assessment state
- observation opportunities
- belief source cursor

Implementation requirement:
belief input must preserve belief revision refs and evidence refs through every planner output that depends on them.

Open gap:
belief layer needs typed public records for posterior, uncertainty, precision, freshness, contradiction, origin, coverage, and assessment state.

#### `PlannerCausalInput`

One causal packet scoped to a `DecisionContext`.

Carries:

- intervention variable
- target variable
- outcome variable
- effect estimate
- effect uncertainty
- identification status
- confounder risk
- selection warning
- counterfactual summary
- causal assumptions
- causal source cursor

Implementation requirement:
planner may summarize effect support, but must not estimate effects or choose adjustment sets.

Open gap:
causation layer needs read contracts for causal variables, effect estimates, identification status, confounder risk, selection warnings, and assumption refs.

#### `PlannerRegimeInput`

One regime packet scoped to a `DecisionContext`.

Carries:

- regime posterior
- changepoint state
- run length belief
- continuation score
- break score
- mixture prediction
- sensitivity set
- stress metrics
- active segment status
- regime source cursor

Implementation requirement:
planner may use regime state for sensitivity, expiry, risk, assumptions, and abstention.
Planner must not decide the active regime.

Open gap:
regime layer needs typed read contracts for active segment state, mixture prediction, stress metrics, and sensitivity sets.

### Output Types

#### `WorldModelView`

One assembled planner-facing read model for a scoped question.

Required children:

- `ViewSnapshot`
- zero or more `ActionableBeliefView`
- zero or more `ObservationOpportunityView`
- zero or more `PreconditionAssessment`
- zero or more `CausalEffectSummary`
- one `RiskEnvelope`
- one `AbstentionState`
- zero or more `ConflictSummary`
- zero or more `SensitivitySummary`
- one `AssumptionSet`
- hydration handles needed for explanation and execution handoff

Implementation requirement:
`WorldModelView` is the only root output from `query_world_model_view`.
Other planner routes may return child projections, but they must be derivable from the same projection pipeline.

Open gap:
the public interface has route names but no typed response envelope with errors, warnings, cursor metadata, and partial result semantics.

#### `ActionableBeliefView`

One belief projection for the scoped decision.

Required outcomes:

- supports
- blocks
- uncertain
- stale
- conflicted
- under-observed
- not assessed

Implementation requirement:
each view must name the predicate it affects and carry evidence refs, belief revision refs, decision relevance, and projection version.

Open gap:
decision relevance scoring needs an explicit scale, threshold basis, and tie-break rule.

#### `ObservationOpportunityView`

One projected information-gathering opportunity.

Required outcomes:

- target evidence
- evidence channel
- expected information gain
- cost
- expiry
- blocking status
- dependency on belief, conflict, freshness, or coverage gap

Implementation requirement:
planner ranks opportunities but does not dispatch observation or choose execution method.

Open gap:
observation cost and channel refs need a cross-domain contract with execution capabilities.

#### `PreconditionAssessment`

One assessment of world-facing conditions for a desired predicate or candidate intervention.

Required outcomes:

- supported
- blocked
- uncertain
- stale
- conflicted
- not assessed

Implementation requirement:
precondition assessment must stay world-facing.
Execution method readiness, resource readiness, retry rules, and dispatch policy remain outside planner.

Open gap:
the boundary between world-facing condition and execution method precondition needs a shared test fixture with execution.

#### `CausalEffectSummary`

One planner-facing causal effect projection.

Required outcomes:

- supported effect
- weak effect support
- blocked identification
- confounded
- selection-shaped
- counterfactual unavailable

Implementation requirement:
preserve causal claim refs, adjustment assumptions, selection warnings, and counterfactual refs.

Open gap:
causal input records are design-only and need implementation contracts.

#### `RiskEnvelope`

One composed typed risk record for the view.

Required risk dimensions:

- belief uncertainty
- low precision
- stale evidence
- contradiction
- weak coverage
- causal identification
- confounder risk
- selection warning
- regime uncertainty
- changepoint risk
- stress sensitivity

Implementation requirement:
risk dimensions stay typed and traceable.
The planner must not collapse them into one authority score.

Open gap:
risk severity scale, blocking threshold, and mitigation hint vocabulary are undefined.

#### `AbstentionState`

One explicit no-commit state.

Required outcomes:

- no abstention
- soft abstention
- hard abstention
- proceed under assumptions

Implementation requirement:
abstention must cite the source refs that caused it and must distinguish missing support from negative support.

Open gap:
execution needs a clear mapping from abstention state to planner handoff behavior.

#### `ConflictSummary`

One contradiction and conflict projection.

Required conflict kinds:

- direct counterevidence
- supersession
- invalidation
- weak coverage
- competing hypothesis

Implementation requirement:
conflict blocks a decision only when it affects the scoped predicate or candidate intervention.

Open gap:
belief contradiction taxonomy needs stable typed values.

#### `SensitivitySummary`

One structural fragility projection.

Required fields:

- affected output
- source layer
- flip condition
- weakening condition
- expiry rule
- stress scenario refs

Implementation requirement:
sensitivity must state what lower-layer change would materially alter the view.

Open gap:
regime sensitivity and stress metrics need a threshold vocabulary.

#### `AssumptionSet`

One explicit dependency set for the view.

Required assumption kinds:

- causal
- regime
- coverage
- freshness
- evidence policy
- scope

Implementation requirement:
every assumption must state validity scope, source refs, violation effect, and expiry rule.

Open gap:
assumption refs need shared formatting across causal, regime, belief, and planner records.

#### `HydrationHandle`

One stable reference to lower-layer source records.

Use as a data value by default.
Promote to an independent type only if handles become independently indexed, audited, or dereferenced outside the owning output.

Required fields:

- source layer
- domain object ref
- source id
- source cursor
- projection path
- explanation role

Implementation requirement:
hydration handles support explanation and replay.
They do not substitute for typed projection fields.

Open gap:
hydration handle dereference routes are not defined in the public interface.

### Type Rules

- Every planner output must be derivable from explicit lower-layer records and projection version.
- Every planner output must include `ViewSnapshot` reachability.
- Child types must carry parent `WorldModelView` identity.
- Input packet types must carry source cursor boundaries.
- Score concepts such as `ExpectedInformationGain` and `DecisionRelevance` are state fields, not independent types.
- `ObservationPolicy` may exist inside projection internals, but the public planner output is `ObservationOpportunityView`.
- `ExecutionPreconditions` should be expressed as `PreconditionAssessment`, because execution owns operational readiness.
- `HydrationHandle` is not a root type unless implementation needs independent indexing.

## Data Model

### Data Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable addressing for projection records | planner and source adapters | every pipeline stage |
| Context | scope, time, branch, horizon, and perspective | planner route | all stages |
| Lens | admissibility and tolerance settings | agent source or query envelope | normalization and scoring |
| Source refs | lower-layer object and revision references | source input projections | all output projections |
| Source summaries | lower-layer values normalized for planner use | source input projections | output projections |
| Derived outputs | planner-owned assessment, risk, conflict, assumption, and abstention state | planner stages | view assembly |
| Hydration | explanation and replay handles | source input and output stages | public route and execution handoff |

### Shape Rules

- Data contains typed values, not prose authority.
- Data carries source refs when its values derive from lower-layer records.
- Data carries projection version when its values are planner-derived.
- Nested structs are used for detail that has no independent identity.
- Reason codes and typed statuses come before presentation text.
- Data never replaces lower-layer authority records.

### Identity Data

| Data | Attached to | Purpose |
|---|---|---|
| `DecisionContextIdentity` | `DecisionContext` | stable context key |
| `ProjectionRun` | `WorldModelView` | projection version and source boundary |
| `ViewIdentity` | `WorldModelView` | root output identity |
| `InputPacketIdentity` | planner input packet types | source packet identity |
| `ChildOutputIdentity` | planner output child types | stable child identity |

#### `DecisionContextIdentity`

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

#### `ProjectionRun`

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

#### `ViewIdentity`

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

#### `InputPacketIdentity`

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

#### `ChildOutputIdentity`

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

### Context Data

| Data | Attached to | Purpose |
|---|---|---|
| `DecisionScope` | `DecisionContext` | scoped planning question |
| `ReplayBoundary` | `WorldModelView` | replay and validity boundary |
| `AgentLens` | `PlannerAgentLens` | perspective-specific admissibility |

#### `DecisionScope`

Purpose:
defines the scoped planning question that every pipeline stage answers.

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

#### `ReplayBoundary`

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

#### `AgentLens`

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

### Source Ref Data

| Data | Attached to | Purpose |
|---|---|---|
| `GraphInputRefs` | `PlannerGraphInput` | graph source refs and cursor |
| `BeliefInputRefs` | `PlannerBeliefInput` | belief source refs and cursor |
| `CausalInputRefs` | `PlannerCausalInput` | causal source refs and cursor |
| `RegimeInputRefs` | `PlannerRegimeInput` | regime source refs and cursor |

#### `GraphInputRefs`

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

#### `BeliefInputRefs`

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

#### `CausalInputRefs`

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

#### `RegimeInputRefs`

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

### Source Summary Data

| Data | Attached to | Purpose |
|---|---|---|
| `BeliefSummary` | `PlannerBeliefInput`, `ActionableBeliefView` | belief state normalized for planner use |
| `CausalSummary` | `PlannerCausalInput`, `CausalEffectSummary` | causal effect state normalized for planner use |
| `RegimeSummary` | `PlannerRegimeInput`, `SensitivitySummary`, `RiskEnvelope` | structural uncertainty normalized for planner use |

#### `BeliefSummary`

Purpose:
normalizes belief state into the planner data model.

Dependencies:
`BeliefInputRefs`, belief view contract, evidence refs, belief revision refs.

```rust
struct BeliefSummary {
    subject: DomainObjectRef,
    dimension_id: BeliefDimensionId,
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

#### `CausalSummary`

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

#### `RegimeSummary`

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

### Derived Output Data

| Data | Attached to | Purpose |
|---|---|---|
| `DecisionSupport` | `ActionableBeliefView` | predicate support and decision relevance |
| `ObservationValue` | `ObservationOpportunityView` | information gain, cost, and target evidence |
| `PreconditionEvaluation` | `PreconditionAssessment` | world-facing condition status |
| `RiskDimensions` | `RiskEnvelope` | typed risk dimensions |
| `ConflictImpact` | `ConflictSummary` | contradiction and conflict impact |
| `SensitivityConditions` | `SensitivitySummary` | lower-layer changes that flip or weaken the view |
| `Assumptions` | `AssumptionSet` | explicit dependencies |
| `AbstentionGrounds` | `AbstentionState` | no-commit or proceed-under-assumption state |

#### `DecisionSupport`

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

#### `ObservationValue`

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

#### `PreconditionEvaluation`

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

#### `RiskDimensions`

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

#### `ConflictImpact`

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

#### `SensitivityConditions`

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

#### `Assumptions`

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

#### `AbstentionGrounds`

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

### Hydration Data

| Data | Attached to | Purpose |
|---|---|---|
| `HydrationHandles` | every planner input and output that needs explanation or replay | stable lower-layer dereference handles |

#### `HydrationHandles`

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

### Shared Data Shapes

#### `SourceRef`

Purpose:
shared reference shape for lower-layer records used by planner data.

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

#### `Score`

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

#### `RiskDimension`

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

#### `PlannerAssumption`

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

### Data Rules

- Identity data defines stable keys for cache, replay, and child refs.
- Context and lens data defines admissibility before scoring.
- Source ref data preserves lower-layer authority boundaries.
- Source summary data is read-only from planner perspective.
- Derived output data cites source refs and projection version.
- Hydration data supports explanation and replay.
- Scores carry scale metadata and threshold basis.
- Expiry fields feed `ReplayBoundary`.

## Pipelines

### Pipeline Overview

| Phase | Stage | Required input | Required output |
|---|---|---|---|
| 1 | decision-context normalization | `DecisionContext`, optional query envelope | normalized context, context id |
| 2 | agent-lens acquisition | normalized context | `PlannerAgentLens` |
| 3 | lower-input acquisition | normalized context, lens | graph, belief, causal, regime input packets |
| 4 | source packet validation | input packets, source cursor set | accepted packets, warnings, missing packet markers |
| 5 | belief projection | belief packet, graph packet, context, lens | belief views, preconditions, conflicts, observation opportunities |
| 6 | causal projection | causal packet, context, lens | causal effect summaries, causal assumptions, causal risks |
| 7 | regime projection | regime packet, context, lens | regime risks, sensitivity, expiry, regime assumptions |
| 8 | risk and assumption composition | belief, causal, regime outputs | risk envelope, assumption set |
| 9 | abstention projection | risk, conflict, assumptions, source warnings | abstention state |
| 10 | hydration projection | all accepted source refs and outputs | hydration handle set |
| 11 | view snapshot projection | source cursor set, expiry, projection version | `ViewSnapshot` |
| 12 | planner-view assembly | all child outputs | `WorldModelView` |

Replay invariant:
same normalized context, same lens, same lower-layer source cursor set, and same projection version produce the same `WorldModelView`.

Pipeline stages are deterministic projection functions. They read lower-layer input packets and planner context, then write planner-facing types and data. They do not settle belief, estimate causal effects, detect regimes, curate goals, dispatch tasks, or repair execution.

### Required Read Ports

| Port | Owner | Required for | Gap |
|---|---|---|---|
| graph packet read | graph | identity, anchors, lineage, provenance, branch presence, hydration | object history and branch presence are not fully exposed |
| belief packet read | belief | posterior, uncertainty, freshness, contradiction, coverage, observation opportunities | first `BeliefView` contract available, planner packet adapter needed |
| causal packet read | causation | intervention effects, identification, confounders, selection warnings | causal public read contract needed |
| regime packet read | regime | regime uncertainty, changepoints, sensitivity, stress metrics | regime public read contract needed |
| agent lens read | agent | admissibility, trust, tolerance, observation scope | typed lens contract needed |
| source cursor read | each source layer | replay and view snapshot | shared cursor shape needed |

Pipeline stages must depend on these ports.
They must not import lower-layer internals.

### Public Route Mapping

| Route | Pipeline behavior |
|---|---|
| `query_world_model_view` | run full pipeline and return `WorldModelView` |
| `query_observation_opportunities` | run full pipeline, return opportunity children and shared snapshot metadata |
| `query_preconditions` | run full pipeline, return precondition children and shared snapshot metadata |

Route rule:
partial routes do not use separate projection logic.
They select from the same deterministic pipeline so behavior remains consistent.

### Projection Table

| Projection | Owning domain | Stage | Output |
|---|---|---|---|
| `DecisionContext -> normalized context` | planner | decision-context normalization | scoped admissibility basis |
| `normalized context -> PlannerAgentLens` | agent | agent-lens acquisition | `PlannerAgentLens` |
| `GraphInput -> PlannerGraphInput` | graph | graph-input projection | `PlannerGraphInput` |
| `BeliefView -> PlannerBeliefInput` | belief | belief-input projection | `PlannerBeliefInput` |
| `CausalOutput -> PlannerCausalInput` | causation | causal-input projection | `PlannerCausalInput` |
| `RegimeOutput -> PlannerRegimeInput` | regime | regime-input projection | `PlannerRegimeInput` |
| `PlannerBeliefInput + DecisionContext + PlannerAgentLens -> ActionableBeliefView` | planner | actionable-belief projection | `ActionableBeliefView` |
| `PlannerBeliefInput + PlannerGraphInput + DecisionContext -> PreconditionAssessment` | planner | precondition assessment projection | `PreconditionAssessment` |
| `PlannerBeliefInput + DecisionContext -> ObservationOpportunityView` | planner | observation opportunity projection | `ObservationOpportunityView` |
| `PlannerBeliefInput + DecisionContext -> ConflictSummary` | planner | conflict-summary projection | `ConflictSummary` |
| `PlannerCausalInput + DecisionContext -> CausalEffectSummary` | planner | causal-effect projection | `CausalEffectSummary` |
| `PlannerBeliefInput + PlannerCausalInput + PlannerRegimeInput + DecisionContext + PlannerAgentLens -> RiskEnvelope` | planner | risk-envelope projection | `RiskEnvelope` |
| `PlannerRegimeInput + planner outputs -> SensitivitySummary` | planner | sensitivity-summary projection | `SensitivitySummary` |
| `PlannerBeliefInput + PlannerCausalInput + PlannerRegimeInput + DecisionContext -> AssumptionSet` | planner | assumption-set projection | `AssumptionSet` |
| `RiskEnvelope + ConflictSummary + AssumptionSet + source warnings -> AbstentionState` | planner | abstention projection | `AbstentionState` |
| `all admitted source refs -> HydrationHandle set` | planner | hydration-handle projection | hydration handles |
| `source cursor set + expiry + projection version -> ViewSnapshot` | planner | view-snapshot projection | `ViewSnapshot` |
| `all planner projections -> WorldModelView` | planner | planner-view assembly | `WorldModelView` |
| `WorldModelView -> ExecutionPlanningInput` | execution | execution planning input projection | execution port input |
| `TaskLifecycleEvent + Artifacts + ExecutionContext -> OutcomeFact` | execution | outcome publication projection | `OutcomeFact` |
| `OutcomeFact -> EvidenceItem` | belief | outcome evidence projection | `EvidenceItem` |
| `BeliefRevision + PlannerAgentLens + ActiveGoals + CostBeliefs + ValueBeliefs + RegimeContext -> GoalMutation` | agent | goal curation projection | `GoalMutation` |

### Decision-Context Normalization

Input:
`DecisionContext`.

Output:
normalized context, context id, and route warnings.

Rules:

- Canonicalize subject scope.
- Canonicalize branch scope.
- Require reference time and transaction time.
- Validate that desired predicate and candidate intervention are compatible when both are present.
- Validate that broad survey routes are explicitly marked when no desired predicate exists.
- Produce deterministic context id.

Failure behavior:
invalid context returns a route error before lower-layer reads.

Implementation gap:
canonical serialization for `DecisionContext` is undefined.

### Agent-Lens Acquisition

Input:
normalized context.

Output:
`PlannerAgentLens`.

Rules:

- Resolve lens by perspective and agent id when agent id is present.
- Validate lens perspective against context perspective.
- Apply evidence policy before scoring decision relevance.
- Apply tolerated uncertainty only after source summaries are available.
- Carry lens id and lens version into output provenance.

Failure behavior:
missing required lens returns route error unless the route supplies an explicit default lens.

Implementation gap:
agent lens records and default lens policy are undefined.

### Lower-Input Acquisition

Input:
normalized context and lens.

Output:
planner input packets and missing packet markers.

Rules:

- Request graph input for subject scope, branch scope, and reference boundary.
- Request belief inputs for relevant subjects, dimensions, and predicate scope.
- Request causal inputs only when candidate intervention or causal predicate requires them.
- Request regime inputs for subjects and decision horizon.
- Request source cursors with every packet.

Failure behavior:
missing optional packet becomes explicit absence.
missing required packet becomes warning or abstention depending on route and context.

Implementation gap:
source domains do not yet expose all packet reads.

### Source Packet Validation

Input:
input packets and source cursor set.

Output:
accepted packets, warnings, and missing packet markers.

Rules:

- Reject packets outside subject scope.
- Reject packets outside branch scope.
- Reject packets outside reference time or transaction time.
- Reject packet versions unsupported by projection version.
- Preserve missing comparator, stale view, invalid view, unresolved mixture, blocked identification, and source cursor absence as typed warnings.

Failure behavior:
invalid source packet does not silently disappear.
It becomes route error, warning, or abstention input.

Implementation gap:
version negotiation between planner and lower source packets is not specified.

### Belief-Input Projection

Input:
belief views, belief revisions, evidence refs, assessment state, and observation opportunities.

Output:
`PlannerBeliefInput`.

Rules:

- Select belief views matching subject scope, perspective, branch scope, and decision horizon.
- Normalize posterior, uncertainty, precision, freshness, contradiction, origin, and coverage into planner data.
- Preserve evidence refs and belief revision refs as hydration handles.
- Keep `ObservationOpportunity` as source-layer output until planner filters it into `ObservationOpportunityView`.
- Mark missing comparator, provisional assessment, stale view, and invalid view explicitly.

Replay invariant:
same belief state and projection version produce the same belief input packet.

Implementation gap:
planner-specific belief packet adaptation is not yet available in code.

### Graph-Input Projection

Input:
graph anchors, lineage, relation walks, object history, provenance, branch presence, and source cursors.

Output:
`PlannerGraphInput`.

Rules:

- Select graph records in subject scope.
- Preserve lineage and provenance for every current anchor used by the view.
- Include relation neighborhood only to the depth and filters required by `DecisionContext`.
- Emit hydration handles for every graph record needed by execution or explanation.
- Do not infer confidence, truth status, causal effect, or regime status from graph state alone.

Replay invariant:
same graph state and as-of boundary produce the same graph input packet.

Implementation gap:
the graph implementation exposes anchors, walks, and provenance, but object history, branch presence, and source cursor envelopes need public route shape.

### Actionable-Belief Projection

Input:
`PlannerBeliefInput`, `DecisionContext`, and `PlannerAgentLens`.

Output:
`ActionableBeliefView`.

Rules:

- Map posterior summary to supported, unsupported, uncertain, mixed, stale, conflicted, under-observed, or not assessed.
- Downgrade support when freshness is stale for the decision horizon.
- Downgrade support when precision is below agent tolerance.
- Mark conflict when contradiction state affects the scoped predicate.
- Mark under-observed when coverage cannot license the predicate.
- Carry evidence refs, belief revision refs, decision relevance, and projection version.

Abstention handoff:
emit abstention input when critical uncertainty, stale evidence, unresolved contradiction, invalid assessment, or insufficient coverage blocks commitment.

Implementation gap:
decision relevance scoring needs scale and threshold rules.

### Precondition Assessment Projection

Input:
`PlannerBeliefInput`, `PlannerGraphInput`, and `DecisionContext`.

Output:
`PreconditionAssessment`.

Rules:

- Evaluate desired predicate or candidate intervention condition against scoped belief state.
- Use graph input only for identity, relation scope, and hydration.
- Return supported, blocked, uncertain, stale, conflicted, or not assessed.
- Treat contradiction as blocking when it targets the required condition.
- Treat weak coverage as uncertain unless a local completeness rule licenses absence.
- Preserve predicate refs and source belief refs.

Boundary:
planner assesses world-facing conditions.
execution assesses method readiness, resource readiness, retry rules, dispatch, and repair.

Implementation gap:
shared fixtures are needed to keep planner preconditions and execution readiness from overlapping.

### Observation Opportunity Projection

Input:
`PlannerBeliefInput` and `DecisionContext`.

Output:
`ObservationOpportunityView`.

Rules:

- Admit only opportunities linked to decision-relevant uncertainty, freshness, conflict, or coverage gaps.
- Carry expected information gain as a score field.
- Carry cost, expiry, target evidence, evidence channel, and blocking status.
- Rank opportunities by decision relevance and expected information gain after admissibility filtering.
- Do not dispatch observation or choose an execution method.

Implementation gap:
evidence channel refs and observation cost records need a shared contract with execution capabilities.

### Conflict-Summary Projection

Input:
`PlannerBeliefInput` and `DecisionContext`.

Output:
`ConflictSummary`.

Rules:

- Group contradictions by affected belief key and decision predicate.
- Distinguish direct counterevidence, supersession, invalidation, weak coverage, and competing hypothesis conflict.
- Mark conflict as blocking only when it affects the scoped decision.
- Preserve competing evidence refs and hydration handles.

Implementation gap:
belief contradiction taxonomy needs stable typed values.

### Causal-Input Projection

Input:
causal variables, effect estimates, intervention records, outcome links, counterfactual cases, confounder hypotheses, and assumptions.

Output:
`PlannerCausalInput`.

Rules:

- Select causal outputs relevant to candidate intervention, target variable, outcome variable, and decision horizon.
- Preserve identification status and confounder risk.
- Preserve selection and measurement warnings.
- Carry causal assumptions as refs, not prose authority.
- Do not re-estimate causal effects inside planner.

Replay invariant:
same causal source records and projection version produce the same causal input packet.

Implementation gap:
causal source records are design-only and need public read contracts.

### Causal-Effect Projection

Input:
`PlannerCausalInput` and `DecisionContext`.

Output:
`CausalEffectSummary`.

Rules:

- Project effect direction, effect magnitude, uncertainty, and identification status.
- Mark weak support when identification status is blocked or confounder risk is material.
- Mark selection-shaped evidence when measurement or anchor selection may explain the outcome.
- Preserve counterfactual summary when available.
- Preserve causal assumptions and source refs.

Implementation gap:
effect support categories and blocked identification semantics need typed values.

### Regime-Input Projection

Input:
regime posterior, changepoint state, run length belief, continuation score, break score, mixture prediction, sensitivity set, stress metrics, and active segment state.

Output:
`PlannerRegimeInput`.

Rules:

- Select regime outputs relevant to the perspective, subject scope, and decision horizon.
- Preserve unresolved mixture state.
- Preserve active segment status and changepoint uncertainty.
- Preserve stress metrics only when they affect the scoped decision.
- Do not decide regime identity inside planner.

Replay invariant:
same regime source records and projection version produce the same regime input packet.

Implementation gap:
regime source records are design-only and need public read contracts.

### Risk-Envelope Projection

Input:
`PlannerBeliefInput`, `PlannerCausalInput`, `PlannerRegimeInput`, `DecisionContext`, and `PlannerAgentLens`.

Output:
`RiskEnvelope`.

Rules:

- Create separate risk dimensions for belief uncertainty, low precision, staleness, contradiction, weak coverage, causal identification, confounder risk, selection warning, regime uncertainty, changepoint risk, and stress sensitivity.
- Preserve source layer and source refs for each risk dimension.
- Mark a risk as blocking only when it violates decision context or agent tolerance.
- Do not collapse typed risk dimensions into a single authority score.

Implementation gap:
risk severity scale and blocking thresholds are undefined.

### Sensitivity-Summary Projection

Input:
`PlannerRegimeInput` and relevant planner outputs.

Output:
`SensitivitySummary`.

Rules:

- Identify regime changes that would flip belief support, causal effect support, risk level, or abstention state.
- Include stress metrics that cross planner-relevant thresholds.
- Emit expiry horizon when regime uncertainty makes the view time-sensitive.
- Preserve regime refs and stress scenario refs.

Implementation gap:
flip and weakening condition vocabulary is undefined.

### Assumption-Set Projection

Input:
`PlannerCausalInput`, `PlannerRegimeInput`, `PlannerBeliefInput`, and `DecisionContext`.

Output:
`AssumptionSet`.

Rules:

- Record causal assumptions used by effect summaries.
- Record regime assumptions used by risk and sensitivity summaries.
- Record coverage and freshness assumptions used by belief and precondition projections.
- Record evidence policy assumptions from agent lens.
- Record scope assumptions from decision context.
- State violation effect for each assumption.

Implementation gap:
assumption refs and violation effects need typed records.

### Abstention Projection

Input:
`RiskEnvelope`, `ConflictSummary`, `AssumptionSet`, accepted source packet warnings, and missing packet markers.

Output:
`AbstentionState`.

Rules:

- Emit hard abstention when required belief is invalid, critical evidence is missing, contradiction blocks the decision, causal identification is blocked for a required intervention, or regime uncertainty invalidates the decision horizon.
- Emit soft abstention when action may proceed only under explicit assumptions.
- Emit no abstention when all blocking risks are absent or below tolerance.
- Preserve source refs that caused abstention.
- Distinguish missing support from negative support.

Implementation gap:
execution handoff semantics for hard and soft abstention are not yet specified.

### Hydration-Handle Projection

Input:
all source refs admitted into planner projection.

Output:
hydration handles attached to planner outputs.

Rules:

- Preserve domain object refs and source ids.
- Record source cursor and projection path.
- Record why the handle exists, such as evidence, provenance, explanation, execution hydration, or replay.
- Deduplicate handles by source identity and explanation role.
- Never use hydration handles as substitute authority for typed projection fields.

Implementation gap:
hydration dereference routes are not defined.

### View-Snapshot Projection

Input:
source cursors, expiry candidates, input packet ids, and projection version.

Output:
`ViewSnapshot`.

Rules:

- Record reference time and transaction time from `DecisionContext`.
- Record event cursor or source cursor set used by lower inputs.
- Record projection version.
- Record input packet ids.
- Record expiry horizon from freshness, observation opportunity, regime sensitivity, and causal assumption expiry.

Implementation gap:
source cursor set shape is undefined.

### Planner-View Assembly

Input:
all planner output types.

Output:
`WorldModelView`.

Rules:

- Assemble only outputs sharing the same normalized `DecisionContext`.
- Attach `ViewSnapshot`.
- Attach all hydration handles needed for explanation and execution handoff.
- Include abstention state even when empty.
- Include risk, conflict, sensitivity, and assumptions even when non-blocking.
- Preserve projection version and lower-layer source cursors.

Implementation gap:
typed response envelope is missing for partial views, warnings, and route metadata.

### Cross-Domain Projections

Some projections sit outside `world_model/planner` but complete the loop.
They are listed here for placement and dependency clarity.
Their detailed specs should live under the owning domain.

`WorldModelView -> ExecutionPlanningInput`
belongs to execution.
It defines the read port that execution planning may consume without importing world model internals.

`TaskLifecycleEvent + Artifacts + ExecutionContext -> OutcomeFact`
belongs to execution.
It turns task activity into world-model-legible outcome evidence.

`OutcomeFact -> EvidenceItem`
belongs to belief.
It normalizes execution outcomes back into belief evidence.

`BeliefRevision + PlannerAgentLens + ActiveGoals + CostBeliefs + ValueBeliefs + RegimeContext -> GoalMutation`
belongs to agent.
It turns belief revision and cost-benefit state into goal curation.

### Failure Semantics

| Condition | Planner behavior |
|---|---|
| invalid context | route error |
| missing required lens | route error or explicit default lens warning |
| missing graph identity input | route error for scoped view |
| missing belief input for desired predicate | hard abstention or not assessed precondition |
| stale belief input | stale output plus risk dimension |
| invalid belief view | hard abstention when required |
| missing causal input for candidate intervention | soft or hard abstention based on context requirement |
| blocked causal identification | causal risk plus abstention when intervention is required |
| unresolved regime mixture | regime risk plus possible expiry or abstention |
| missing source cursor | route warning at minimum, route error when replay is required |

### Test Matrix

| Test | Expected proof |
|---|---|
| replay determinism | same inputs and projection version produce same view id and child outputs |
| source ref preservation | every derived output traces to lower-layer refs or hydration handles |
| context filtering | out-of-scope records cannot affect outputs |
| branch filtering | records outside branch scope cannot affect outputs |
| stale belief handling | stale evidence downgrades support and feeds expiry |
| contradiction handling | scoped contradiction creates conflict and possible abstention |
| causal boundary | planner never estimates effect from graph or belief alone |
| regime boundary | planner never decides active regime |
| public route consistency | partial routes select from full pipeline outputs |
| execution boundary | planner preconditions do not include dispatch or readiness policy |

### Pipeline Rules

- Projection stages must be deterministic.
- Projection stages must be replayable from lower-layer records and projection version.
- Projection stages must not mutate lower-layer authority records.
- Projection stages must preserve provenance through source refs or hydration handles.
- Projection stages must emit abstention or uncertainty instead of fabricating missing support.
- Projection stages may rank or score views, but ranking is not dispatch policy.
- Public planner routes must share projection logic.
- Missing data must become typed absence, warning, route error, or abstention.

## Lower-Layer Inputs

`planner` consumes shaped packets from lower world-model domains.
The packets remain owned by their source domains.

- `PlannerGraphInput`
  anchors, lineage, graph walks, object history, provenance, branch presence, and hydration handles
- `PlannerBeliefInput`
  posterior summary, uncertainty, precision, freshness, contradiction, origin, coverage, evidence refs, assessment state, and observation opportunities
- `PlannerCausalInput`
  intervention variables, outcome variables, effect estimates, identification status, confounder risk, selection warnings, counterfactual summaries, and causal assumptions
- `PlannerRegimeInput`
  regime posterior, changepoint state, run length, continuation and break scores, mixture prediction, sensitivity sets, stress metrics, and active segment status
- `PlannerAgentLens`
  perspective, trust profile, evidence policy, observation scope, tolerated uncertainty, and regime sensitivity profile

These inputs are decomposed into planner-facing types through deterministic projection stages.

## Role In The Set

`planner` consumes shaped outputs from the lower world model domains and emits one action-relevant view.

`agent` consumes that view as part of its perspective assembly.
`execution` consumes the shaped result, not internal planner state.

## Implementation Gap Register

| Gap | Blocks | Needed owner |
|---|---|---|
| canonical `DecisionContext` serialization | cache keys, replay keys, route idempotence | planner |
| source cursor set format | deterministic replay across layers | graph, belief, causation, regime |
| typed agent lens contract | perspective-scoped admissibility | agent |
| typed belief view contract | belief input projection | belief |
| typed causal output contract | causal effect summary and abstention | causation |
| typed regime output contract | risk, sensitivity, expiry | regime |
| hydration dereference routes | explanation and execution handoff | public interface |
| decision relevance scale | ranking and blocking | planner |
| risk severity scale | abstention and execution handoff | planner |
| partial result semantics | robust query behavior during missing lower-layer data | public interface |
| context canonicalization | route idempotence and cache keys | planner |
| agent lens read port | admissibility filtering | agent |
| graph packet contract | graph input projection | graph |
| belief packet contract | belief projections | belief |
| causal packet contract | causal summaries | causation |
| regime packet contract | risk, sensitivity, expiry | regime |
| source cursor set | replay and snapshot | all source layers |
| score thresholds | ranking, blocking, abstention tests | planner |
| execution handoff contract | abstention and precondition consumption | execution, planner |
| partial response envelope | robust public routes | public interface |
