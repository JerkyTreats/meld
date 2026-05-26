# Belief Spec

Date: 2026-05-26
Status: active
Scope: domain specification for `world_model/belief` as a curation-heavy inference domain

`belief` is the heaviest mutable curation domain in `world_model`. It normalizes evidence, schedules assessment, updates posterior state, and projects belief views. It is more perspective-sensitive than `graph` because one shared substrate may produce many scoped belief views.

`belief` is where graph evidence becomes posterior state. `agent` consumes belief outputs through lenses and scoped views. `planner` consumes belief summaries rather than raw belief worker state. `regime` consumes contradiction, surprise, drift, and calibration signals without taking over belief revision.

## Domain Types

Domain types are stable identities for assessed questions, normalized evidence, revision history, inference work, latent alternatives, observation needs, and shaped consumer views.

Domain types are generic over runtime family configuration. No identity rule may require a Rust type, enum variant, or module for a specific belief family.

### Type Categories

| Category | Types | Persistence expectation |
|---|---|---|
| Belief identity | `Belief` | durable |
| Evidence | `EvidenceItem` | durable |
| Assessment history | `BeliefRevision` | durable append-only |
| Work coordination | `AssessmentLease`, `InferenceEpoch` | durable enough for recovery |
| Latent alternatives | `HypothesisSet` | durable when material |
| Observation need | `ObservationOpportunity` | materialized projection |
| Consumer view | `BeliefView` | materialized projection |
| Calibration | `CalibrationRecord` | durable when produced from outcomes |

### Identity Rules

| Type | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `Belief` | canonical hash of subject, runtime dimension id, perspective key, branch scope, and evidence policy key | none | belief |
| `EvidenceItem` | source fact id plus evidence role plus belief key plus normalizer version | `Belief` when assigned | belief |
| `BeliefRevision` | belief key plus assessment epoch plus evidence window plus comparator engine version plus runtime config snapshot hash | `Belief` | belief |
| `AssessmentLease` | belief key plus assessment epoch | `Belief` | belief |
| `InferenceEpoch` | epoch id from scoped belief keys, topology hash, method, and source cursor | one or more `Belief` values | belief |
| `HypothesisSet` | belief key plus hypothesis family plus revision id | `Belief` | belief |
| `ObservationOpportunity` | belief key plus evidence target plus channel ref plus source revision id | `Belief` | belief |
| `BeliefView` | belief key plus perspective key plus branch scope plus current revision id plus view version | `Belief` | belief |
| `CalibrationRecord` | belief key plus outcome evidence id plus calibration target | `Belief` | belief |

### `Belief`

One stable assessed question.

Purpose:
anchors evidence, revisions, leases, hypotheses, calibration, and views for a subject and runtime dimension id under a scope.

Required state:

- `BeliefIdentity`
- `BeliefScope`
- `BeliefPolicy`
- `RevisionHead`
- `BeliefLifecycle`

Creation:
created by key registration, evidence assignment, or bootstrap over discovered graph subjects.

Lifecycle:
unassessed, active, stale, invalid, archived.

Dependencies:
`DomainObjectRef`, `PerspectiveKey`, `BranchScope`, runtime dimension id, evidence policy contract, runtime family configuration.

### `EvidenceItem`

One normalized belief input.

Purpose:
turns spine facts, graph anchors, outcomes, corrections, or measurements into belief-readable evidence.

Required state:

- `EvidenceIdentity`
- `EvidenceSource`
- `EvidenceMeaning`
- `EvidenceQuality`
- `EvidenceProvenance`

Creation:
created by evidence normalization.

Lifecycle:
normalized, assigned, superseded, rejected, calibration-only.

Dependencies:
spine facts, graph anchor records, execution outcome facts, normalizer version, loaded evidence schema.

### `BeliefRevision`

One append-only settlement over a bounded evidence window.

Purpose:
records the assessment result for one belief key at one assessment epoch.

Required state:

- `RevisionIdentity`
- `EvidenceWindow`
- `ComparatorRun`
- `PosteriorState`
- `UncertaintyState`
- `FreshnessState`
- `ContradictionState`
- `RevisionStatus`
- `RevisionProvenance`

Creation:
created by revision commit after comparator or inference output passes validation.

Lifecycle:
current, superseded, invalidated, archived.

Dependencies:
assigned evidence items, previous revision when available, comparator engine contract, runtime config snapshot.

### `AssessmentLease`

One durable coordination record for assessment work.

Purpose:
serializes assessment for one belief key while allowing independent belief keys to run in parallel.

Required state:

- `LeaseIdentity`
- `LeaseWindow`
- `LeaseOwner`
- `LeaseState`
- `ComparatorSelection`

Creation:
created by assessment scheduler.

Lifecycle:
queued, leased, completed, expired, abandoned.

Dependencies:
belief key, assessment epoch, evidence high-water mark, worker identity.

### `InferenceEpoch`

One bounded multi-belief inference pass.

Purpose:
freezes enough local topology and source boundary to make message passing, predictive residual inference, or hypothesis scoring replayable.

Required state:

- `InferenceBoundary`

Creation:
created by inference scheduler when a belief update spans multiple belief keys or topology.

Lifecycle:
open, converged, expired, abandoned, committed.

Dependencies:
belief keys, graph or evidence high-water mark, method version, topology hash.

### `HypothesisSet`

One competing set of latent explanations or state alternatives.

Purpose:
keeps hidden causes and alternatives explicit instead of burying them in one confidence value.

Required state:

- `HypothesisCandidate`
- `HypothesisWeight`
- `HypothesisEvidence`

Creation:
created by comparator output or inference epoch when alternatives materially affect posterior state.

Lifecycle:
active, pruned, superseded, archived.

Dependencies:
belief revision, evidence refs, inference method.

### `ObservationOpportunity`

One information need produced by unresolved belief state.

Purpose:
states what evidence would reduce uncertainty, conflict, staleness, or missing-comparator pressure.

Required state:

- `ObservationNeed`
- `ObservationValue`
- `ObservationTarget`
- `ObservationExpiry`
- `ObservationProvenance`

Creation:
created by observation opportunity projection.

Lifecycle:
open, superseded, satisfied, expired, dismissed.

Dependencies:
belief view, comparator output, evidence channel registry, cost estimate when available.

### `BeliefView`

One shaped projection for planners, agents, operators, and upper world-model layers.

Purpose:
exposes current belief state without leaking raw facts, active leases, comparator drafts, or worker internals.

Required state:

- `BeliefViewIdentity`
- `BeliefViewSummary`
- `BeliefViewFreshness`
- `BeliefViewConflict`
- `BeliefViewObservation`
- `BeliefViewProvenance`

Creation:
created by belief view projection after revision commit, stale detection, recovery, or observation projection.

Lifecycle:
current, stale, invalid, superseded.

Dependencies:
current revision, assigned evidence refs, observation opportunities, freshness policy.

### `CalibrationRecord`

One outcome-based calibration update.

Purpose:
compares prior belief outputs to later outcomes so reliability, precision, comparator trust, and priors can improve.

Required state:

- `CalibrationTarget`
- `CalibrationOutcome`
- `CalibrationAdjustment`
- `CalibrationProvenance`

Creation:
created by calibration ingestion from execution outcomes, observations, or later corrections.

Lifecycle:
pending, applied, rejected, archived.

Dependencies:
outcome evidence, prior revision, comparator kind, calibration policy.

### Domain Type Rules

- `Belief` is the scheduling and ownership root.
- `BeliefRevision` is append-only and never edited in place.
- `EvidenceItem` is normalized once and then assigned by refs.
- `BeliefView` is a projection, not durable source authority.
- `AssessmentLease` and `InferenceEpoch` are durable enough for recovery.
- `HypothesisSet` is material only when alternatives affect uncertainty, observation, regime signals, or planner views.
- `ObservationOpportunity` is not an action command.
- `CalibrationRecord` changes future reliability and prior selection, not old revision history.

## Data Model

Belief data are typed contracts attached to belief domain types. They carry identity, scope, evidence meaning, posterior state, work coordination, projection state, and provenance. Pipelines use these data to turn durable facts into replayable belief revisions and shaped views.

Belief family content is not a data family in Rust. Family ids, dimension ids, evidence schema ids, source mappings, comparator parameters, priors, thresholds, and planner projection fields are loaded runtime configuration. Data definitions may store ids and config refs, but they must not encode a specific family as a Rust type or enum variant.

### Data Families

| Family | Purpose | Written by | Read by |
|---|---|---|---|
| Identity | stable addressing for belief records | key assignment and commit stages | all stages |
| Scope and policy | subject, perspective, branch, runtime dimension id, evidence policy id | key assignment and public routes | ingestion, scheduling, projection |
| Evidence | normalized source facts and their belief meaning | evidence normalizer | assignment, comparator, provenance |
| Assessment | leases, windows, comparator selection, epochs | scheduler and workers | recovery, comparator, commit |
| Posterior state | prior, posterior, uncertainty, precision, freshness | comparator and inference stages | view projection, planner input, regime |
| Conflict and hypotheses | contradiction state and latent alternatives | comparator and inference stages | observation, view projection, regime |
| Observation | unresolved evidence need and information value | opportunity projector | planner and execution bridge |
| View | shaped consumer state | view projector | public interface and planner |
| Calibration | outcome comparison and reliability adjustment | calibration stage | comparator selection and priors |
| Provenance | source refs, cursors, explanation refs | every mutating stage | replay, audit, hydration |

### Shape Rules

- Data carry typed fields before presentation text.
- Source-derived data carry `SourceRef` values.
- Derived data carry projection, comparator engine version, and config snapshot refs when material.
- Work coordination data carry enough state for recovery.
- Score fields use `Score`.
- Status fields use typed enums.
- Public view data exclude active lease internals and comparator drafts.

### Identity Data

| Data | Attached to | Purpose |
|---|---|---|
| `BeliefIdentity` | `Belief` | stable belief key and object ref |
| `EvidenceIdentity` | `EvidenceItem` | stable evidence id |
| `RevisionIdentity` | `BeliefRevision` | stable revision id |
| `BeliefViewIdentity` | `BeliefView` | stable view id |

#### `BeliefIdentity`

Purpose:
stable identity for one assessed belief question.

Dependencies:
subject ref, runtime dimension id, perspective key, branch scope, evidence policy id.

```rust
struct BeliefIdentity {
    belief_key: BeliefKey,
    belief_ref: DomainObjectRef,
    key_hash: ContentHash,
    key_version: SchemaVersion,
}
```

#### `EvidenceIdentity`

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

#### `RevisionIdentity`

Purpose:
stable identity for one append-only belief settlement.

Dependencies:
belief key, assessment epoch, evidence window, comparator engine version, and runtime config snapshot hash.

```rust
struct RevisionIdentity {
    revision_id: BeliefRevisionId,
    revision_ref: DomainObjectRef,
    belief_key: BeliefKey,
    assessment_epoch: AssessmentEpoch,
    revision_version: SchemaVersion,
}
```

#### `BeliefViewIdentity`

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

### Scope And Policy Data

| Data | Attached to | Purpose |
|---|---|---|
| `BeliefScope` | `Belief` | subject and dimension scope |
| `BeliefPolicy` | `Belief` | evidence policy and comparator preference |
| `BeliefLifecycle` | `Belief` | lifecycle state |
| `RevisionHead` | `Belief` | current revision pointer |

#### `BeliefScope`

Purpose:
defines what the belief is about.

Dependencies:
domain object ref contract, loaded dimension id, perspective model.

```rust
struct BeliefScope {
    subject: DomainObjectRef,
    dimension_id: BeliefDimensionId,
    predicate: BeliefPredicate,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}
```

#### `BeliefPolicy`

Purpose:
defines how evidence is admitted and how assessment is selected.

Dependencies:
runtime family configuration, evidence policy configuration, comparator engine catalog, freshness policy configuration.

```rust
struct BeliefPolicy {
    evidence_policy_key: EvidencePolicyKey,
    family_config_ref: BeliefFamilyConfigRef,
    comparator_preference: ComparatorPreference,
    freshness_policy: FreshnessPolicy,
    calibration_policy: CalibrationPolicy,
    missing_comparator_policy: MissingComparatorPolicy,
}
```

#### `BeliefLifecycle`

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

#### `RevisionHead`

Purpose:
points from a belief to its latest visible revision.

Dependencies:
revision commit process, supersession rules.

```rust
struct RevisionHead {
    current_revision_id: Option<BeliefRevisionId>,
    previous_revision_id: Option<BeliefRevisionId>,
    revision_high_water_mark: SourceCursor,
    dirty_since_cursor: Option<SourceCursor>,
}
```

### Evidence Data

| Data | Attached to | Purpose |
|---|---|---|
| `EvidenceSource` | `EvidenceItem` | source record refs and cursor |
| `EvidenceMeaning` | `EvidenceItem` | belief-facing claim shape |
| `EvidenceQuality` | `EvidenceItem` | reliability and precision |
| `EvidenceAssignment` | `EvidenceItem` | belief keys and roles |

#### `EvidenceSource`

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

#### `EvidenceMeaning`

Purpose:
normalizes source data into belief-readable semantics.

Dependencies:
evidence normalizer, loaded evidence schema, value encoding contract.

```rust
struct EvidenceMeaning {
    subject: DomainObjectRef,
    predicate_id: BeliefPredicateId,
    evidence_schema_id: EvidenceSchemaId,
    value: EvidenceValue,
    polarity: EvidencePolarity,
    evidence_role: EvidenceRole,
    effective_range: EffectiveSequenceRange,
}
```

#### `EvidenceQuality`

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

#### `EvidenceAssignment`

Purpose:
connects one evidence item to one or more belief keys.

Dependencies:
belief key assignment process, evidence role vocabulary.

```rust
struct EvidenceAssignment {
    assignments: Vec<BeliefEvidenceAssignment>,
}
```

### Assessment Data

| Data | Attached to | Purpose |
|---|---|---|
| `LeaseIdentity` | `AssessmentLease` | stable lease identity |
| `LeaseWindow` | `AssessmentLease` | evidence input range |
| `LeaseOwner` | `AssessmentLease` | worker ownership |
| `LeaseState` | `AssessmentLease` | lease lifecycle |
| `ComparatorSelection` | `AssessmentLease` | selected comparator for assessment |
| `ComparatorRun` | `BeliefRevision` | comparator method and config |
| `InferenceBoundary` | `InferenceEpoch` | multi-belief inference boundary |

#### `LeaseIdentity`

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

#### `LeaseWindow`

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

#### `LeaseOwner`

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

#### `LeaseState`

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

#### `ComparatorSelection`

Purpose:
records the comparator selected for one assessment lease.

Dependencies:
belief policy, comparator engine catalog, runtime family configuration, evidence window.

```rust
struct ComparatorSelection {
    comparator_engine: ComparatorEngineId,
    comparator_version: SchemaVersion,
    config_ref: ComparatorConfigRef,
    config_snapshot_hash: ContentHash,
    selected_reason: ReasonCode,
}
```

#### `ComparatorRun`

Purpose:
records the assessment method that produced a revision.

Dependencies:
comparator engine catalog, runtime comparator config, inference method version.

```rust
struct ComparatorRun {
    comparator_engine: ComparatorEngineId,
    comparator_version: SchemaVersion,
    config_ref: ComparatorConfigRef,
    config_snapshot_hash: ContentHash,
    input_evidence_ids: Vec<EvidenceId>,
    contradicted_evidence_ids: Vec<EvidenceId>,
}
```

#### `InferenceBoundary`

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

### Posterior State Data

| Data | Attached to | Purpose |
|---|---|---|
| `EvidenceWindow` | `BeliefRevision` | assessed evidence range |
| `PosteriorState` | `BeliefRevision` | prior and posterior summary |
| `UncertaintyState` | `BeliefRevision` | uncertainty and precision |
| `FreshnessState` | `BeliefRevision`, `BeliefView` | stale and decay state |
| `RevisionStatus` | `BeliefRevision` | settlement status |

#### `EvidenceWindow`

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

#### `PosteriorState`

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

#### `UncertaintyState`

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

#### `FreshnessState`

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

#### `RevisionStatus`

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

### Conflict And Hypothesis Data

| Data | Attached to | Purpose |
|---|---|---|
| `ContradictionState` | `BeliefRevision`, `BeliefView` | unresolved conflict state |
| `HypothesisCandidate` | `HypothesisSet` | latent alternatives |
| `HypothesisWeight` | `HypothesisSet` | relative support |
| `HypothesisEvidence` | `HypothesisSet` | evidence links |

#### `ContradictionState`

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

#### `HypothesisCandidate`

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

#### `HypothesisWeight`

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

#### `HypothesisEvidence`

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

### Observation Data

| Data | Attached to | Purpose |
|---|---|---|
| `ObservationNeed` | `ObservationOpportunity` | missing or ambiguous evidence |
| `ObservationValue` | `ObservationOpportunity` | expected information value |
| `ObservationTarget` | `ObservationOpportunity` | evidence channel and target |
| `ObservationExpiry` | `ObservationOpportunity` | validity horizon |

#### `ObservationNeed`

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

#### `ObservationValue`

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

#### `ObservationTarget`

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

#### `ObservationExpiry`

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

### View Data

| Data | Attached to | Purpose |
|---|---|---|
| `BeliefViewSummary` | `BeliefView` | planner-facing belief state |
| `BeliefViewFreshness` | `BeliefView` | planner-facing freshness state |
| `BeliefViewConflict` | `BeliefView` | planner-facing conflict state |
| `BeliefViewObservation` | `BeliefView` | planner-facing observation state |

#### `BeliefViewSummary`

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

#### `BeliefViewFreshness`

Purpose:
exposes stale and expiry state to consumers.

Dependencies:
freshness state, stale detection process.

```rust
struct BeliefViewFreshness {
    freshness_status: FreshnessStatus,
    last_evidence_time: Option<ReferenceTime>,
    expires_at: Option<ReferenceTime>,
    stale_reason: Option<ReasonCode>,
}
```

#### `BeliefViewConflict`

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

#### `BeliefViewObservation`

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

### Calibration Data

| Data | Attached to | Purpose |
|---|---|---|
| `CalibrationTarget` | `CalibrationRecord` | belief output being evaluated |
| `CalibrationOutcome` | `CalibrationRecord` | later outcome used for calibration |
| `CalibrationAdjustment` | `CalibrationRecord` | reliability or prior adjustment |

#### `CalibrationTarget`

Purpose:
identifies the prior belief output being calibrated.

Dependencies:
belief revision id, comparator engine, posterior state.

```rust
struct CalibrationTarget {
    belief_key: BeliefKey,
    revision_id: BeliefRevisionId,
    comparator_engine: ComparatorEngineId,
    predicted_value: PosteriorSummary,
}
```

#### `CalibrationOutcome`

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

#### `CalibrationAdjustment`

Purpose:
records adjustment data for future comparator and prior selection.

Dependencies:
calibration policy, comparator engine catalog, prior configuration.

```rust
struct CalibrationAdjustment {
    reliability_adjustment: Option<Score>,
    precision_adjustment: Option<Score>,
    prior_adjustment_ref: Option<PriorAdjustmentRef>,
    applied: bool,
}
```

### Provenance Data

| Data | Attached to | Purpose |
|---|---|---|
| `EvidenceProvenance` | `EvidenceItem` | source refs for evidence |
| `RevisionProvenance` | `BeliefRevision` | source refs for revision |
| `BeliefViewProvenance` | `BeliefView` | source refs for public view |
| `ObservationProvenance` | `ObservationOpportunity` | source refs for observation need |
| `CalibrationProvenance` | `CalibrationRecord` | source refs for calibration |

#### `EvidenceProvenance`

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

#### `RevisionProvenance`

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

#### `BeliefViewProvenance`

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

#### `ObservationProvenance`

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

#### `CalibrationProvenance`

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

### Shared Data Shapes

#### `SourceRef`

Purpose:
shared reference shape for lower-layer and spine records used by belief data.

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

#### `Score`

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

### Data Rules

- Data attached to durable types are replayable from source refs.
- Data attached to projections name the revision or source state they summarize.
- Assessment data are never exposed as planner-facing belief state.
- Posterior state data never hide contradiction, missing comparator, or stale state.
- Observation data describe evidence needs, not execution commands.
- Calibration data affect future assessment behavior, not past revision history.

## Pipelines

Pipeline stages transform durable facts and graph state into evidence, revisions, observation opportunities, calibration records, and shaped belief views.

They are deterministic over explicit inputs, source cursors, method versions, policy records, and runtime family configuration snapshots. They do not dispatch tasks, decide goals, estimate causal effects, or decide regime identity.

### Pipeline Overview

| Phase | Stage | Required input | Required output |
|---|---|---|---|
| 1 | fact promotion | spine facts, graph anchors, execution outcomes | belief-relevant fact candidates |
| 2 | evidence normalization | fact candidates, runtime source mappings | `EvidenceItem` |
| 3 | belief key assignment | evidence items, runtime family configuration | `Belief`, evidence assignments |
| 4 | dirty marking | assigned evidence, revision head | dirty belief keys |
| 5 | assessment scheduling | dirty belief keys, lease state | `AssessmentLease` |
| 6 | comparator execution | lease, evidence window, prior revision, runtime comparator config snapshot | proposed revision output |
| 7 | inference epoch execution | scoped belief set, topology boundary | proposed multi-belief outputs |
| 8 | revision commit | proposed output, validation rules | `BeliefRevision`, revision head update |
| 9 | stale detection | revision heads, policies, graph supersession | stale state |
| 10 | contradiction handling | evidence roles, comparator output | contradiction state |
| 11 | observation opportunity projection | unresolved state, evidence channels | `ObservationOpportunity` |
| 12 | belief view projection | revision, freshness, conflict, observation | `BeliefView` |
| 13 | calibration ingestion | outcomes, prior revisions | `CalibrationRecord` |
| 14 | recovery scan | leases, high-water marks, views | rescheduled work and rebuilt views |
| 15 | storm coalescing | active leases, incoming evidence | compacted windows and pending metadata |

Replay invariant:
same source facts, graph state, policies, comparator engine versions, runtime family configuration snapshots, and source cursors produce the same revisions and belief views.

### Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| spine replay and subscription | events | fact promotion and recovery |
| graph anchor reads | graph | anchor-derived evidence |
| graph provenance reads | graph | evidence provenance |
| graph lineage reads | graph | supersession and stale detection |
| execution outcome reads | execution through spine | calibration evidence |
| runtime family configuration store | belief configuration loader | evidence normalization, assignment, assessment scheduling |
| evidence channel configuration | belief and execution bridge | observation opportunity projection |
| comparator engine catalog | belief | assessment scheduling |
| perspective and evidence policy reads | agent or belief policy store | scoped belief views |

### Fact Promotion

Input:
spine facts, graph anchors, execution outcomes, corrections, and derived facts.

Output:
belief-relevant fact candidates.

Rules:

- Admit only facts with graph-readable or belief-readable subject refs.
- Preserve source sequence, event type, object refs, relation refs, and content hash.
- Preserve graph anchor refs when a fact becomes evidence through current graph state.
- Preserve execution outcome refs when a fact calibrates belief or supports outcome state.
- Reject raw sensory churn unless promoted into semantic facts.

### Evidence Normalization

Input:
belief-relevant fact candidates and runtime source mappings.

Output:
`EvidenceItem` with source, meaning, quality, and provenance data.

Rules:

- Normalize evidence into subject, runtime predicate id, runtime schema id, value payload, polarity, and evidence role.
- Record support, contradiction, context, calibration, and supersession roles explicitly.
- Attach source cursor and effective sequence range.
- Attach reliability and precision when available.
- Mark rejected evidence with a reason instead of silently dropping it when audit requires visibility.
- Do not embed family-specific source mappings in normalizer code.

### Belief Key Assignment

Input:
evidence items and runtime family configuration.

Output:
`Belief` types and evidence assignments.

Rules:

- Assign evidence to every belief key it can materially affect according to loaded configuration.
- Create a belief key when loaded configuration and registration policy allow creation from evidence.
- Include subject, runtime dimension id, runtime predicate id, perspective, branch scope, and evidence policy id in the key.
- Preserve one fact to many beliefs and many facts to one belief mappings.
- Leave unassigned evidence inspectable when assignment policy cannot classify it.
- Fail closed when required runtime family configuration is absent.

### Dirty Marking

Input:
evidence assignments and revision heads.

Output:
dirty belief keys and dirty cursor state.

Rules:

- Mark belief dirty when assigned evidence is newer than the current revision high-water mark.
- Mark belief dirty when graph anchor supersession changes evidence meaning.
- Mark belief dirty when evidence policy changes.
- Mark belief dirty when comparator engine configuration or runtime family configuration changes.
- Preserve dirty state while an assessment lease is active.

### Assessment Scheduling

Input:
dirty belief keys, revision heads, runtime family configuration, comparator engine catalog, and lease state.

Output:
`AssessmentLease`.

Rules:

- Use `BeliefKey` as the scheduling unit.
- Maintain one active lease per belief key.
- Select comparator engine from loaded family configuration, belief policy, and comparator availability.
- Use `MissingComparator` when no comparator exists.
- Coalesce incoming evidence behind active leases.
- Record input low and high cursor on the lease.
- Record config id or snapshot hash on the lease.

### Comparator Execution

Input:
assessment lease, evidence window, prior revision, and runtime comparator configuration snapshot.

Output:
proposed belief revision.

Rules:

- Run the selected generic comparator engine deterministically over explicit inputs.
- Emit posterior summary, confidence, uncertainty, precision, status, and provenance.
- Emit contradiction state when evidence conflicts.
- Emit observation need when uncertainty remains material.
- Mark semantic settlement as provisional unless policy grants settled status.
- Emit needs-assessment or needs-observation for missing comparator state.
- Do not implement comparator types named after runtime belief families.

### Inference Epoch Execution

Input:
belief keys, local topology, evidence windows, method configuration.

Output:
proposed revisions, hypothesis sets, or posterior updates.

Rules:

- Freeze enough scope to make the inference pass replayable.
- Record topology hash when topology affects output.
- Record damping or convergence policy for iterative methods.
- Publish revisions or posterior summaries, not raw worker memory.
- Expire or abandon epochs that exceed their validity boundary.

### Revision Commit

Input:
proposed revision output and validation rules.

Output:
`BeliefRevision` and revision head update.

Rules:

- Append a new revision instead of editing prior revisions.
- Validate evidence ids and source cursor range.
- Link prior revision when available.
- Supersede the previous revision by moving the revision head.
- Preserve comparator run and provenance.
- Publish derived revision facts through the spine when required by integration policy.

### Stale Detection

Input:
revision heads, freshness policy, graph lineage, evidence high-water marks.

Output:
freshness state updates and stale belief views.

Rules:

- Mark stale when new assigned evidence exceeds revision high-water mark.
- Mark stale when semantic settlement expires.
- Mark stale when confidence decay crosses a policy threshold.
- Mark stale when a graph anchor used by evidence is superseded.
- Mark stale when perspective evidence policy changes.
- Mark stale when required comparator is missing.

### Contradiction Handling

Input:
evidence roles, comparator output, supersession state.

Output:
contradiction state and conflict metadata.

Rules:

- Preserve supporting and contradicting evidence ids separately.
- Distinguish counterevidence, supersession, invalidation, weak coverage, and competing hypothesis.
- Do not collapse contradiction into low confidence.
- Mark unresolved conflict when no comparator can settle the conflict.

### Observation Opportunity Projection

Input:
uncertainty, freshness, contradiction, missing comparator, and evidence channels.

Output:
`ObservationOpportunity`.

Rules:

- Emit opportunity when missing or ambiguous evidence is material.
- Include target belief key, missing evidence type, evidence channel, and suggested artifact type.
- Include expected information gain when available.
- Include cost, delay, and expiry when available.
- Do not dispatch observation.

### Belief View Projection

Input:
current revision, freshness state, contradiction state, observation opportunities, and provenance.

Output:
`BeliefView`.

Rules:

- Expose belief key, perspective, current revision, status, posterior summary, confidence, uncertainty, precision, freshness, contradiction, observation-needed state, assessment state, advisory posture, and provenance summary.
- Exclude raw spine payloads.
- Exclude active lease internals.
- Exclude unpublished comparator drafts.
- Attach hydration handles for source facts, evidence, revisions, and graph anchors.

### Calibration Ingestion

Input:
execution outcomes, later observations, corrections, and prior revisions.

Output:
`CalibrationRecord` and future comparator adjustment state.

Rules:

- Compare prior posterior summaries with later observed outcomes.
- Record comparator kind and belief revision being calibrated.
- Adjust future reliability, precision, prior selection, or comparator trust.
- Do not rewrite old revisions.
- Publish calibration provenance.

### Recovery Scan

Input:
lease state, evidence high-water marks, revision heads, belief views.

Output:
abandoned leases, rescheduled assessment, rebuilt views.

Rules:

- Expire old leases.
- Mark abandoned work.
- Reschedule dirty belief keys.
- Rebuild belief views from revisions.
- Compare evidence high-water mark to revision high-water mark.
- Depend only on durable state.

### Storm Coalescing

Input:
incoming evidence, active leases, dirty cursor state.

Output:
compacted assessment windows and pending assessment metadata.

Rules:

- Keep one active lease per belief key.
- Append incoming evidence normally.
- Update dirty cursor while assessment is active.
- Debounce expensive comparator work.
- Publish previous settled view plus pending assessment metadata.

### Failure Semantics

| Condition | Belief behavior |
|---|---|
| invalid source fact | reject evidence with reason |
| unassignable evidence | keep unassigned evidence inspectable |
| missing belief key | create key or mark assignment unresolved by policy |
| missing comparator | publish needs assessment or needs observation |
| comparator failure | abandon lease, preserve error reason, reschedule or mark invalid by policy |
| expired lease | recovery scanner reschedules from durable evidence |
| evidence storm | coalesce behind one active lease |
| stale semantic settlement | publish stale view |
| contradiction unresolved | publish conflict state and observation opportunity |
| calibration mismatch | adjust future reliability or prior policy |

### Test Matrix

| Test | Expected proof |
|---|---|
| replay determinism | same facts and policies produce same evidence, revision, and view |
| evidence mapping | one fact can affect many beliefs and one belief can use many facts |
| lease serialization | one active lease exists per belief key |
| recovery | expired lease reschedules and rebuilds view from durable state |
| stale detection | new evidence and expired settlement mark belief stale |
| contradiction preservation | counterevidence remains visible outside confidence |
| missing comparator | view reports needs assessment or needs observation |
| storm coalescing | incoming evidence does not spawn unbounded comparator work |
| public boundary | belief view excludes lease internals and comparator drafts |
| calibration | later outcome changes future reliability without rewriting old revisions |

### Pipeline Rules

- Stages are deterministic over explicit inputs and method versions.
- Stages mutate belief-owned records only.
- Stages publish shaped views and optional derived facts, not action commands.
- Stages preserve source refs and cursor boundaries.
- Stages expose absence, staleness, conflict, and missing comparator state explicitly.
- Stages keep `BeliefKey` as the default scheduling unit.
