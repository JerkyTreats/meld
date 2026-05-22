# World Model Planner

Date: 2026-04-30
Status: active
Scope: planner-facing world model projection for action-relevant reads

## Thesis

`world_model/planner` is the planner-facing projection of the modeled world.

Its job is to expose the current action-relevant view of belief, uncertainty, freshness, contradiction, causal effect, and regime sensitivity in a form that downstream planning can consume.

This area belongs to `world_model` because it is still an epistemic concern. It answers what appears to hold, what is unclear, what is risky, and what observation would most reduce uncertainty for a decision.

It does not own task decomposition, dispatch, continuation, repair, or live runtime coordination. Those concerns belong to `execution`.

## Boundary

`world_model/planner` owns:

- action-relevant world model reads
- belief summaries for decision making
- uncertainty and freshness summaries
- contradiction and conflict summaries
- causal effect summaries where justified
- regime sensitivity summaries where material
- observation opportunity summaries
- abstention grounds
- execution precondition summaries as world-facing conditions

`world_model/planner` does not own:

- task graphs
- task network state
- dispatch rules
- continuation state
- observation wait mechanics
- repair flow
- retry policy
- execution control semantics
- provider or worker orchestration

## Relationship To Execution

`world_model/planner` is upstream of `execution`.

It tells `execution` what the current modeled world supports, blocks, or leaves uncertain.

`execution` decides how to operationalize that information through planning, control, dispatch, waiting, repair, and completion.

The boundary matters because these crates serve different authorities:

- `world_model` owns epistemic judgment
- `execution` owns operational commitment

This area may describe action relevance, but it must not drift into execution policy.

### WorldState as the Downstream Contract

The planner-facing projection's downstream output is `WorldState` — a set of ground `Proposition` values in the shared language ([`meld-lang`](../../meld-lang/README.md)). This is the concrete contract that resolves the world-model-to-execution seam (Gap 3 in [Execution Gaps](../../execution/GAPS.md)).

The planner assembles `WorldModelView` from lower-layer inputs (belief, graph, causation, regime). The projection then translates this into `WorldState` for execution consumption:

```
WorldModelView (internal planner-facing richness)
    |
    | project_world_state(perspective)
    |
    v
WorldState (ground propositions in meld-lang)
    |
    | consumed by execution's planning loop
    v
evaluate(world_state, goal.target) → EvalResult
```

The `WorldModelView` types defined below retain their full richness for internal planner operations (precondition assessment, observation opportunity scoring, abstention evaluation). `WorldState` is the subset projected outward — it carries the belief states, freshness, artifact existence, scope accessibility, and relationships that execution can evaluate mechanically. See [World State and Evaluation](../../meld-lang/world_state.md) for the concrete construction example.

## Relationship To Belief, Causation, And Regime

`world_model/planner` is not a separate inference system.

It is the projection layer over:

- belief for confidence, uncertainty, contradiction, and freshness
- causation for intervention and effect summaries
- regime for structural context and sensitivity

Its responsibility is to present those concerns in decision-relevant form without exposing their internal machinery.

## Durable Concepts

- `WorldModelView`
- `ActionableBeliefView`
- `DecisionContext`
- `ViewSnapshot`
- `ObservationOpportunityView`
- `AbstentionState`
- `CausalEffectSummary`
- `RiskEnvelope`
- `PreconditionAssessment`
- `ConflictSummary`
- `SensitivitySummary`
- `AssumptionSet`
- `HydrationHandle`

`ExpectedInformationGain` and `DecisionRelevance` are not durable planner entities.
They are scoring fields carried by observation opportunities, belief summaries, causal summaries, regime summaries, and risk projections.

`ObservationPolicy` should be used only when a policy object is needed inside projection internals.
The planner-facing output should prefer `ObservationOpportunityView`, because execution decides how to operationalize observation.

`ExecutionPreconditions` should be expressed as `PreconditionAssessment`.
The planner assesses world-facing conditions.
Execution owns method applicability, task readiness, and dispatch.

## Deterministic Projection Rule

Planner concepts are deterministic, relational, replayable projections over lower world-model records.

They are not free-form semantic summaries.

An `ActionableBeliefView` is calculated from explicit belief state, evidence, perspective, decision context, and projection rules.
The same event spine, graph state, belief revisions, perspective, and projection version must produce the same view during replay.

LLM-backed capabilities may produce evidence.
Planner-facing concepts consume that evidence only after it has entered the world model as explicit records with provenance, versioning, and calibration state.

The authority shape is:

```text
BeliefRevision records
+ EvidenceItem records
+ Perspective
+ DecisionContext
+ projection rules
= ActionableBeliefView
```

The non-authority shape is:

```text
LLM reads context
+ writes free-form actionable belief
= planner input
```

## Planner Input Contract

`DecisionContext` is the root planner input.
It defines the question being projected and scopes every lower-layer record before assembly.

```rust
DecisionContext {
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

`ViewSnapshot` records the replay and validity boundary of the assembled view.

```rust
ViewSnapshot {
    event_cursor: EventCursor,
    reference_time: ReferenceTime,
    transaction_time: TransactionTime,
    projection_version: ProjectionVersion,
    expires_at: Option<ReferenceTime>,
}
```

## Lower-Layer Inputs

The planner consumes shaped outputs from lower world-model domains.
Each input packet remains owned by its source layer.
The planner assembles them into one decision-ready view.

### Graph Input

Graph answers what objects, anchors, relations, and provenance are in scope.

```rust
PlannerGraphInput {
    subjects: Vec<DomainObjectRef>,
    current_anchors: Vec<AnchorSelectionRecord>,
    anchor_lineage: Vec<LineageChain>,
    relation_neighborhood: GraphWalkResult,
    object_history: Vec<ObjectHistory>,
    provenance_bundles: Vec<ProvenanceBundle>,
    branch_presence: Vec<BranchPresence>,
    hydration_handles: Vec<HydrationHandle>,
}
```

Planner uses graph input for identity, scope, provenance, explanation, branch presence, and later hydration.
It must not infer belief confidence or causal effect from graph input alone.

### Belief Input

Belief answers what appears credible, stale, uncertain, contradicted, under-observed, or worth observing.

```rust
PlannerBeliefInput {
    belief_key: BeliefKey,
    subject: DomainObjectRef,
    dimension: BeliefDimension,
    posterior_summary: PosteriorSummary,
    uncertainty: UncertaintySummary,
    precision: PrecisionSummary,
    freshness: FreshnessSummary,
    contradiction_state: ContradictionState,
    origin_state: OriginState,
    coverage_state: CoverageState,
    evidence_refs: Vec<EvidenceRef>,
    assessment_state: AssessmentState,
    observation_opportunities: Vec<ObservationOpportunity>,
}
```

Planner maps belief input into `ActionableBeliefView`, `PreconditionAssessment`, `ConflictSummary`, `ObservationOpportunityView`, and belief portions of `RiskEnvelope`.

### Causal Input

Causation answers what intervention effects are justified and where causal claims are weak.

```rust
PlannerCausalInput {
    intervention_variable: CausalVariableRef,
    target_variable: CausalVariableRef,
    outcome_variable: CausalVariableRef,
    effect_estimate: EffectEstimate,
    effect_uncertainty: UncertaintySummary,
    identification_status: IdentificationStatus,
    confounder_risk: ConfounderRisk,
    selection_path_warning: Option<SelectionPathWarning>,
    counterfactual_summary: Option<CounterfactualSummary>,
    causal_assumptions: Vec<CausalAssumption>,
}
```

Planner maps causal input into `CausalEffectSummary`, causal portions of `RiskEnvelope`, `AssumptionSet`, action support, and abstention grounds.

### Regime Input

Regime answers whether the current structure is stable enough for prior beliefs, causal effects, and cost assumptions to remain useful.

```rust
PlannerRegimeInput {
    regime_posterior: RegimePosterior,
    changepoint_state: ChangepointState,
    run_length_belief: RunLengthBelief,
    continuation_score: ContinuationScore,
    break_score: BreakScore,
    mixture_prediction: Option<MixturePrediction>,
    sensitivity_set: SensitivitySet,
    stress_metrics: Vec<StressMetric>,
    active_segment_status: ActiveSegmentStatus,
}
```

Planner maps regime input into regime portions of `RiskEnvelope`, `SensitivitySummary`, `AssumptionSet`, view expiry, and abstention grounds.

### Agent Lens

Agent perspective is not lower inference output, but it is required to assemble the correct projection.

```rust
PlannerAgentLens {
    agent_id: AgentId,
    perspective: Perspective,
    trust_profile: TrustProfile,
    evidence_policy: EvidencePolicy,
    observation_scope: ObservationScope,
    tolerated_uncertainty: ToleratedUncertainty,
    regime_sensitivity_profile: RegimeSensitivityProfile,
}
```

The agent lens decides which lower-layer records are admissible and how relevance is scoped.

## Projection Flow

The planner transition is deterministic:

```text
DecisionContext
+ PlannerAgentLens
+ PlannerGraphInput
+ PlannerBeliefInput
+ PlannerCausalInput
+ PlannerRegimeInput
= WorldModelView
```

The projection systems map lower inputs into planner-facing outputs:

```text
PlannerBeliefInput -> ActionableBeliefView
PlannerBeliefInput + PlannerGraphInput -> PreconditionAssessment
PlannerBeliefInput -> ObservationOpportunityView
PlannerCausalInput -> CausalEffectSummary
PlannerBeliefInput + PlannerCausalInput + PlannerRegimeInput -> RiskEnvelope
PlannerBeliefInput + PlannerCausalInput + PlannerRegimeInput -> AbstentionState
PlannerGraphInput -> HydrationHandle set
PlannerRegimeInput -> SensitivitySummary
PlannerCausalInput + PlannerRegimeInput -> AssumptionSet
PlannerBeliefInput -> ConflictSummary
```

Every planner field must trace back to one or more lower-layer records.
Free text may be generated for presentation, but it is not authority.

## Non Goals

This area should not become the home of execution planning.

It should not define task structure, runtime suspension, control graphs, or repair semantics.

It should not require downstream consumers to understand raw graph traversal, event replay, or belief revision internals.

## Projection Structure

Planner projection design is split into ECS-shaped documents:

- [Planner Entities](entities.md)
  durable context, input packet, and output view entities
- [Planner Components](components.md)
  field groups and source refs that projection systems read and write
- [Planner Systems](systems.md)
  deterministic projection functions from lower-layer records into planner-facing views

## Read With

- [Planner ECS](ECS.md)
- [Planner Entities](entities.md)
- [Planner Components](components.md)
- [Planner Systems](systems.md)
- [World Model Belief](../belief/README.md)
- [Causal Layer](../causation/README.md)
- [Regime Layer](../regime/README.md)
- [Lang Domain](../../meld-lang/README.md)
- [Lang World State and Evaluation](../../meld-lang/world_state.md)
- [Lang Primitives](../../meld-lang/primitives.md)
- [Execution Domain](../../execution/README.md)
- [Execution Gaps](../../execution/GAPS.md)
