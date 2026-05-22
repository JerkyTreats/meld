# Belief Components

Date: 2026-05-15
Status: active
Scope: ECS components for `world_model/belief`

## Thesis

Belief components are typed data contracts attached to belief entities.

They carry identity, scope, evidence meaning, posterior state, work coordination, projection state, and provenance.
Systems use these components to turn durable facts into replayable belief revisions and shaped views.

## Component Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable addressing for belief records | key assignment and commit systems | all systems |
| Scope and policy | subject, perspective, branch, dimension, evidence policy | key assignment and public routes | ingestion, scheduling, projection |
| Evidence | normalized source facts and their belief meaning | evidence normalizer | assignment, comparator, provenance |
| Assessment | leases, windows, comparator selection, epochs | scheduler and workers | recovery, comparator, commit |
| Posterior state | prior, posterior, uncertainty, precision, freshness | comparator and inference systems | view projection, planner input, regime |
| Conflict and hypotheses | contradiction state and latent alternatives | comparator and inference systems | observation, view projection, regime |
| Observation | unresolved evidence need and information value | opportunity projector | planner and execution bridge |
| View | shaped consumer state | view projector | public interface and planner |
| Calibration | outcome comparison and reliability adjustment | calibration system | comparator selection and priors |
| Provenance | source refs, cursors, explanation refs | every mutating system | replay, audit, hydration |

## Shape Rules

- Components carry typed fields before presentation text.
- Source-derived components carry `SourceRef` values.
- Derived components carry projection or comparator version.
- Work coordination components carry enough state for recovery.
- Score fields use `Score`.
- Status fields use typed enums.
- Public view components exclude active lease internals and comparator drafts.

## Identity Components

| Component | Attached to | Purpose |
|---|---|---|
| `BeliefIdentity` | `Belief` | stable belief key and object ref |
| `EvidenceIdentity` | `EvidenceItem` | stable evidence id |
| `RevisionIdentity` | `BeliefRevision` | stable revision id |
| `BeliefViewIdentity` | `BeliefView` | stable view id |

### `BeliefIdentity`

Purpose:
stable identity for one assessed belief question.

Dependencies:
subject ref, dimension, perspective key, branch scope, evidence policy key.

```rust
struct BeliefIdentity {
    belief_key: BeliefKey,
    belief_ref: DomainObjectRef,
    key_hash: ContentHash,
    key_version: SchemaVersion,
}
```

### `EvidenceIdentity`

Purpose:
stable identity for one normalized evidence record.

Dependencies:
source fact id, evidence role, belief key when assigned, normalizer version.

```rust
struct EvidenceIdentity {
    evidence_id: EvidenceId,
    evidence_ref: DomainObjectRef,
    normalizer_version: SchemaVersion,
    content_hash: ContentHash,
}
```

### `RevisionIdentity`

Purpose:
stable identity for one append-only belief settlement.

Dependencies:
belief key, assessment epoch, evidence window, comparator version.

```rust
struct RevisionIdentity {
    revision_id: BeliefRevisionId,
    revision_ref: DomainObjectRef,
    belief_key: BeliefKey,
    assessment_epoch: AssessmentEpoch,
    revision_version: SchemaVersion,
}
```

### `BeliefViewIdentity`

Purpose:
stable identity for one current shaped belief projection.

Dependencies:
belief key, perspective key, branch scope, current revision id, view version.

```rust
struct BeliefViewIdentity {
    belief_view_id: BeliefViewId,
    belief_key: BeliefKey,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
    current_revision_id: Option<BeliefRevisionId>,
    view_version: SchemaVersion,
}
```

## Scope And Policy Components

| Component | Attached to | Purpose |
|---|---|---|
| `BeliefScope` | `Belief` | subject and dimension scope |
| `BeliefPolicy` | `Belief` | evidence policy and comparator preference |
| `BeliefLifecycle` | `Belief` | lifecycle state |
| `RevisionHead` | `Belief` | current revision pointer |

### `BeliefScope`

Purpose:
defines what the belief is about.

Dependencies:
domain object ref contract, dimension registry, perspective model.

```rust
struct BeliefScope {
    subject: DomainObjectRef,
    dimension: BeliefDimension,
    predicate: BeliefPredicate,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}
```

### `BeliefPolicy`

Purpose:
defines how evidence is admitted and how assessment is selected.

Dependencies:
evidence policy registry, comparator registry, freshness policy registry.

```rust
struct BeliefPolicy {
    evidence_policy_key: EvidencePolicyKey,
    comparator_preference: ComparatorPreference,
    freshness_policy: FreshnessPolicy,
    calibration_policy: CalibrationPolicy,
    missing_comparator_policy: MissingComparatorPolicy,
}
```

### `BeliefLifecycle`

Purpose:
records lifecycle state for belief maintenance.

Dependencies:
revision head, stale detection, archival policy.

```rust
struct BeliefLifecycle {
    lifecycle_status: BeliefLifecycleStatus,
    created_at_cursor: SourceCursor,
    archived_at_cursor: Option<SourceCursor>,
    invalid_reason: Option<ReasonCode>,
}
```

### `RevisionHead`

Purpose:
points from a belief to its latest visible revision.

Dependencies:
revision commit system, supersession rules.

```rust
struct RevisionHead {
    current_revision_id: Option<BeliefRevisionId>,
    previous_revision_id: Option<BeliefRevisionId>,
    revision_high_water_mark: SourceCursor,
    dirty_since_cursor: Option<SourceCursor>,
}
```

## Evidence Components

| Component | Attached to | Purpose |
|---|---|---|
| `EvidenceSource` | `EvidenceItem` | source record refs and cursor |
| `EvidenceMeaning` | `EvidenceItem` | belief-facing claim shape |
| `EvidenceQuality` | `EvidenceItem` | reliability and precision |
| `EvidenceAssignment` | `EvidenceItem` | belief keys and roles |

### `EvidenceSource`

Purpose:
preserves where the evidence came from.

Dependencies:
spine fact ids, graph anchor ids, execution outcome ids, source cursor contract.

```rust
struct EvidenceSource {
    source_refs: Vec<SourceRef>,
    source_cursor: SourceCursor,
    reference_time: ReferenceTime,
    transaction_time: TransactionTime,
}
```

### `EvidenceMeaning`

Purpose:
normalizes source data into belief-readable semantics.

Dependencies:
evidence normalizer, belief dimension registry, value encoding contract.

```rust
struct EvidenceMeaning {
    subject: DomainObjectRef,
    predicate: BeliefPredicate,
    value: EvidenceValue,
    polarity: EvidencePolarity,
    evidence_role: EvidenceRole,
    effective_range: EffectiveSequenceRange,
}
```

### `EvidenceQuality`

Purpose:
records reliability, precision, and source quality for the evidence item.

Dependencies:
source reliability registry, measurement metadata, calibration records.

```rust
struct EvidenceQuality {
    reliability: Score,
    precision: Option<Score>,
    measurement_quality: Option<MeasurementQuality>,
    trust_downgrade: Option<TrustDowngrade>,
    quality_reason_codes: Vec<ReasonCode>,
}
```

### `EvidenceAssignment`

Purpose:
connects one evidence item to one or more belief keys.

Dependencies:
belief key assignment system, evidence role vocabulary.

```rust
struct EvidenceAssignment {
    assignments: Vec<BeliefEvidenceAssignment>,
}
```

## Assessment Components

| Component | Attached to | Purpose |
|---|---|---|
| `LeaseIdentity` | `AssessmentLease` | stable lease identity |
| `LeaseWindow` | `AssessmentLease` | evidence input range |
| `LeaseOwner` | `AssessmentLease` | worker ownership |
| `LeaseState` | `AssessmentLease` | lease lifecycle |
| `ComparatorSelection` | `AssessmentLease` | selected comparator for assessment |
| `ComparatorRun` | `BeliefRevision` | comparator method and config |
| `InferenceBoundary` | `InferenceEpoch` | multi-belief inference boundary |

### `LeaseIdentity`

Purpose:
identifies one assessment work item.

Dependencies:
belief key, assessment epoch.

```rust
struct LeaseIdentity {
    lease_id: AssessmentLeaseId,
    belief_key: BeliefKey,
    assessment_epoch: AssessmentEpoch,
}
```

### `LeaseWindow`

Purpose:
defines the evidence window under assessment.

Dependencies:
evidence store high-water marks, prior revision cursor.

```rust
struct LeaseWindow {
    input_low_cursor: SourceCursor,
    input_high_cursor: SourceCursor,
    prior_revision_id: Option<BeliefRevisionId>,
}
```

### `LeaseOwner`

Purpose:
records worker ownership for recoverable assessment.

Dependencies:
worker identity, lease clock.

```rust
struct LeaseOwner {
    owner_id: WorkerId,
    leased_at: TransactionTime,
    expires_at: TransactionTime,
}
```

### `LeaseState`

Purpose:
records assessment lease state.

Dependencies:
scheduler, recovery scanner, comparator worker.

```rust
struct LeaseState {
    state: LeaseStatus,
    state_reason: Option<ReasonCode>,
    completed_revision_id: Option<BeliefRevisionId>,
}
```

### `ComparatorSelection`

Purpose:
records the comparator selected for one assessment lease.

Dependencies:
belief policy, comparator registry, evidence window.

```rust
struct ComparatorSelection {
    comparator_kind: ComparatorKind,
    comparator_version: SchemaVersion,
    config_ref: ComparatorConfigRef,
    selected_reason: ReasonCode,
}
```

### `ComparatorRun`

Purpose:
records the assessment method that produced a revision.

Dependencies:
comparator registry, comparator config, inference method version.

```rust
struct ComparatorRun {
    comparator_kind: ComparatorKind,
    comparator_version: SchemaVersion,
    config_ref: ComparatorConfigRef,
    input_evidence_ids: Vec<EvidenceId>,
    contradicted_evidence_ids: Vec<EvidenceId>,
}
```

### `InferenceBoundary`

Purpose:
records a bounded multi-belief inference pass.

Dependencies:
belief keys, graph topology hash, source cursor, method config.

```rust
struct InferenceBoundary {
    epoch_id: InferenceEpochId,
    belief_keys: Vec<BeliefKey>,
    input_topology_hash: Option<ContentHash>,
    method: InferenceMethod,
    boundary_cursor: SourceCursor,
    state: InferenceEpochState,
}
```

## Posterior State Components

| Component | Attached to | Purpose |
|---|---|---|
| `EvidenceWindow` | `BeliefRevision` | assessed evidence range |
| `PosteriorState` | `BeliefRevision` | prior and posterior summary |
| `UncertaintyState` | `BeliefRevision` | uncertainty and precision |
| `FreshnessState` | `BeliefRevision`, `BeliefView` | stale and decay state |
| `RevisionStatus` | `BeliefRevision` | settlement status |

### `EvidenceWindow`

Purpose:
records the source range assessed by one revision.

Dependencies:
assigned evidence items, source cursor contract.

```rust
struct EvidenceWindow {
    evidence_ids: Vec<EvidenceId>,
    input_low_cursor: SourceCursor,
    input_high_cursor: SourceCursor,
    effective_range: EffectiveSequenceRange,
}
```

### `PosteriorState`

Purpose:
records prior and posterior state emitted by assessment.

Dependencies:
prior revision, evidence window, comparator output.

```rust
struct PosteriorState {
    prior_summary: Option<PriorSummary>,
    posterior_summary: PosteriorSummary,
    surprise: Option<Score>,
    advisory_posture: Option<BeliefPostureHint>,
}
```

### `UncertaintyState`

Purpose:
records uncertainty, precision, and confidence.

Dependencies:
comparator output, evidence quality, calibration state.

```rust
struct UncertaintyState {
    confidence: Score,
    uncertainty: UncertaintySummary,
    precision: PrecisionSummary,
    reliability_notes: Vec<ReasonCode>,
}
```

### `FreshnessState`

Purpose:
records freshness and stale-state semantics.

Dependencies:
freshness policy, evidence cursor, graph supersession, semantic expiry.

```rust
struct FreshnessState {
    last_evidence_cursor: SourceCursor,
    last_evidence_time: ReferenceTime,
    freshness_status: FreshnessStatus,
    expires_at: Option<ReferenceTime>,
    stale_reason: Option<ReasonCode>,
}
```

### `RevisionStatus`

Purpose:
records settlement state for one belief revision.

Dependencies:
comparator output, missing comparator policy, contradiction state.

```rust
struct RevisionStatus {
    status: BeliefStatus,
    settlement_hint: SettlementHint,
    provisional: bool,
    invalid_reason: Option<ReasonCode>,
}
```

## Conflict And Hypothesis Components

| Component | Attached to | Purpose |
|---|---|---|
| `ContradictionState` | `BeliefRevision`, `BeliefView` | unresolved conflict state |
| `HypothesisCandidate` | `HypothesisSet` | latent alternatives |
| `HypothesisWeight` | `HypothesisSet` | relative support |
| `HypothesisEvidence` | `HypothesisSet` | evidence links |

### `ContradictionState`

Purpose:
records contradiction state without hiding it inside confidence.

Dependencies:
evidence roles, comparator output, supersession rules.

```rust
struct ContradictionState {
    contradiction_status: ContradictionStatus,
    conflict_kind: Option<ConflictKind>,
    supporting_evidence_ids: Vec<EvidenceId>,
    contradicting_evidence_ids: Vec<EvidenceId>,
    unresolved: bool,
}
```

### `HypothesisCandidate`

Purpose:
records latent alternatives for a belief.

Dependencies:
comparator output, inference epoch, hypothesis family registry.

```rust
struct HypothesisCandidate {
    hypothesis_family: HypothesisFamily,
    candidates: Vec<HypothesisCandidate>,
}
```

### `HypothesisWeight`

Purpose:
records support assigned to each hypothesis candidate.

Dependencies:
evidence scoring, inference method, posterior state.

```rust
struct HypothesisWeight {
    weights: Vec<HypothesisWeight>,
    normalization: WeightNormalization,
    uncertainty: UncertaintySummary,
}
```

### `HypothesisEvidence`

Purpose:
links hypothesis candidates to evidence.

Dependencies:
evidence ids, source refs, inference epoch.

```rust
struct HypothesisEvidence {
    evidence_by_candidate: Vec<HypothesisEvidenceLink>,
    source_refs: Vec<SourceRef>,
}
```

## Observation Components

| Component | Attached to | Purpose |
|---|---|---|
| `ObservationNeed` | `ObservationOpportunity` | missing or ambiguous evidence |
| `ObservationValue` | `ObservationOpportunity` | expected information value |
| `ObservationTarget` | `ObservationOpportunity` | evidence channel and target |
| `ObservationExpiry` | `ObservationOpportunity` | validity horizon |

### `ObservationNeed`

Purpose:
records why more evidence is needed.

Dependencies:
belief revision, comparator output, freshness state, contradiction state.

```rust
struct ObservationNeed {
    target_belief_key: BeliefKey,
    need_kind: ObservationNeedKind,
    reason_codes: Vec<ReasonCode>,
    blocking_for_planner: bool,
}
```

### `ObservationValue`

Purpose:
records expected information gain and cost.

Dependencies:
uncertainty state, evidence channel registry, cost estimate contract.

```rust
struct ObservationValue {
    expected_information_gain: Option<Score>,
    observation_cost: Option<CostEstimate>,
    delay_estimate: Option<DurationEstimate>,
    opportunity_cost: Option<CostEstimate>,
}
```

### `ObservationTarget`

Purpose:
records the evidence target for observation.

Dependencies:
evidence channel registry, artifact type registry, comparator requirement.

```rust
struct ObservationTarget {
    missing_evidence_type: EvidenceType,
    evidence_channel: Option<EvidenceChannelRef>,
    suggested_artifact_type: Option<ArtifactTypeRef>,
    target_subject: DomainObjectRef,
}
```

### `ObservationExpiry`

Purpose:
records when the opportunity no longer matters.

Dependencies:
freshness policy, decision horizon, regime or source expiry when available.

```rust
struct ObservationExpiry {
    expires_at: Option<ReferenceTime>,
    expiry_reason: Option<ReasonCode>,
}
```

## View Components

| Component | Attached to | Purpose |
|---|---|---|
| `BeliefViewSummary` | `BeliefView` | planner-facing belief state |
| `BeliefViewFreshness` | `BeliefView` | planner-facing freshness state |
| `BeliefViewConflict` | `BeliefView` | planner-facing conflict state |
| `BeliefViewObservation` | `BeliefView` | planner-facing observation state |

### `BeliefViewSummary`

Purpose:
exposes current belief state to consumers.

Dependencies:
revision head, posterior state, uncertainty state, revision status.

```rust
struct BeliefViewSummary {
    belief_key: BeliefKey,
    perspective: PerspectiveKey,
    current_revision_id: Option<BeliefRevisionId>,
    status: BeliefStatus,
    posterior_summary: Option<PosteriorSummary>,
    confidence: Option<Score>,
    uncertainty: Option<UncertaintySummary>,
    precision: Option<PrecisionSummary>,
}
```

### `BeliefViewFreshness`

Purpose:
exposes stale and expiry state to consumers.

Dependencies:
freshness state, stale detection system.

```rust
struct BeliefViewFreshness {
    freshness_status: FreshnessStatus,
    last_evidence_time: Option<ReferenceTime>,
    expires_at: Option<ReferenceTime>,
    stale_reason: Option<ReasonCode>,
}
```

### `BeliefViewConflict`

Purpose:
exposes contradiction and unresolved conflict to consumers.

Dependencies:
contradiction state, conflict taxonomy.

```rust
struct BeliefViewConflict {
    contradiction_status: ContradictionStatus,
    conflict_kind: Option<ConflictKind>,
    unresolved: bool,
    evidence_refs: Vec<SourceRef>,
}
```

### `BeliefViewObservation`

Purpose:
exposes observation-needed state to consumers.

Dependencies:
observation opportunities, uncertainty state, freshness state, contradiction state.

```rust
struct BeliefViewObservation {
    observation_needed: bool,
    opportunities: Vec<ObservationOpportunityId>,
    advisory_posture: Option<BeliefPostureHint>,
}
```

## Calibration Components

| Component | Attached to | Purpose |
|---|---|---|
| `CalibrationTarget` | `CalibrationRecord` | belief output being evaluated |
| `CalibrationOutcome` | `CalibrationRecord` | later outcome used for calibration |
| `CalibrationAdjustment` | `CalibrationRecord` | reliability or prior adjustment |

### `CalibrationTarget`

Purpose:
identifies the prior belief output being calibrated.

Dependencies:
belief revision id, comparator kind, posterior state.

```rust
struct CalibrationTarget {
    belief_key: BeliefKey,
    revision_id: BeliefRevisionId,
    comparator_kind: ComparatorKind,
    predicted_value: PosteriorSummary,
}
```

### `CalibrationOutcome`

Purpose:
records the later outcome or observation compared against the belief.

Dependencies:
outcome evidence item, source cursor.

```rust
struct CalibrationOutcome {
    outcome_evidence_id: EvidenceId,
    observed_value: EvidenceValue,
    source_cursor: SourceCursor,
}
```

### `CalibrationAdjustment`

Purpose:
records adjustment data for future comparator and prior selection.

Dependencies:
calibration policy, comparator registry, prior registry.

```rust
struct CalibrationAdjustment {
    reliability_adjustment: Option<Score>,
    precision_adjustment: Option<Score>,
    prior_adjustment_ref: Option<PriorAdjustmentRef>,
    applied: bool,
}
```

## Provenance Components

| Component | Attached to | Purpose |
|---|---|---|
| `EvidenceProvenance` | `EvidenceItem` | source refs for evidence |
| `RevisionProvenance` | `BeliefRevision` | source refs for revision |
| `BeliefViewProvenance` | `BeliefView` | source refs for public view |
| `ObservationProvenance` | `ObservationOpportunity` | source refs for observation need |
| `CalibrationProvenance` | `CalibrationRecord` | source refs for calibration |

### `EvidenceProvenance`

Purpose:
preserves source refs for normalized evidence.

Dependencies:
source refs from spine, graph, execution, or derived facts.

```rust
struct EvidenceProvenance {
    source_refs: Vec<SourceRef>,
    provenance_summary: ProvenanceSummary,
}
```

### `RevisionProvenance`

Purpose:
preserves source refs and comparator lineage for one revision.

Dependencies:
evidence ids, comparator run, prior revision.

```rust
struct RevisionProvenance {
    evidence_ids: Vec<EvidenceId>,
    prior_revision_id: Option<BeliefRevisionId>,
    comparator_run_ref: ComparatorRunRef,
    source_refs: Vec<SourceRef>,
}
```

### `BeliefViewProvenance`

Purpose:
preserves hydration refs for public belief views.

Dependencies:
current revision, evidence provenance, observation opportunities.

```rust
struct BeliefViewProvenance {
    revision_id: Option<BeliefRevisionId>,
    evidence_refs: Vec<SourceRef>,
    hydration_handles: Vec<HydrationHandle>,
}
```

### `ObservationProvenance`

Purpose:
preserves source refs that caused an observation opportunity.

Dependencies:
belief revision, comparator output, freshness and contradiction state.

```rust
struct ObservationProvenance {
    belief_key: BeliefKey,
    revision_id: Option<BeliefRevisionId>,
    source_refs: Vec<SourceRef>,
}
```

### `CalibrationProvenance`

Purpose:
preserves source refs for calibration.

Dependencies:
calibration target, outcome evidence, calibration policy.

```rust
struct CalibrationProvenance {
    target_revision_id: BeliefRevisionId,
    outcome_evidence_id: EvidenceId,
    source_refs: Vec<SourceRef>,
}
```

## Shared Data Shapes

### `SourceRef`

Purpose:
shared reference shape for lower-layer and spine records used by belief components.

Dependencies:
source layer vocabulary, source cursor contract, domain object ref contract.

```rust
struct SourceRef {
    source_layer: SourceLayer,
    source_kind: SourceKind,
    source_id: SourceId,
    domain_object_ref: DomainObjectRef,
    source_cursor: SourceCursor,
    evidence_role: Option<EvidenceRole>,
}
```

### `Score`

Purpose:
shared score shape for confidence, reliability, precision, surprise, and information gain.

Dependencies:
score scale registry, source field refs, method version.

```rust
struct Score {
    value: f64,
    scale: ScoreScale,
    source_fields: Vec<SourceFieldRef>,
    method_version: SchemaVersion,
}
```

## Component Rules

- Components attached to durable entities are replayable from source refs.
- Components attached to projections name the revision or source state they summarize.
- Assessment components are never exposed as planner-facing belief state.
- Posterior state components never hide contradiction, missing comparator, or stale state.
- Observation components describe evidence needs, not execution commands.
- Calibration components affect future assessment behavior, not past revision history.

## Read With

- [Belief Entities](entities.md)
- [Belief Systems](systems.md)
- [Belief Requirements](requirements.md)
- [World Model Belief](README.md)
