# Belief Entities

Date: 2026-05-15
Status: active
Scope: ECS entities for `world_model/belief`

## Thesis

Belief entities are stable identities for assessed questions, normalized evidence, revision history, inference work, latent alternatives, observation needs, and shaped consumer views.

Entities provide continuity.
Components carry data.
Systems transform evidence into revisions and views.

## Entity Categories

| Category | Entities | Persistence expectation |
|---|---|---|
| Belief identity | `Belief` | durable |
| Evidence | `EvidenceItem` | durable |
| Assessment history | `BeliefRevision` | durable append-only |
| Work coordination | `AssessmentLease`, `InferenceEpoch` | durable enough for recovery |
| Latent alternatives | `HypothesisSet` | durable when material |
| Observation need | `ObservationOpportunity` | materialized projection |
| Consumer view | `BeliefView` | materialized projection |
| Calibration | `CalibrationRecord` | durable when produced from outcomes |

## Identity Rules

| Entity | Identity rule | Required parent | Owning authority |
|---|---|---|---|
| `Belief` | canonical hash of subject, dimension, perspective key, branch scope, and evidence policy key | none | belief |
| `EvidenceItem` | source fact id plus evidence role plus belief key plus normalizer version | `Belief` when assigned | belief |
| `BeliefRevision` | belief key plus assessment epoch plus evidence window plus comparator version | `Belief` | belief |
| `AssessmentLease` | belief key plus assessment epoch | `Belief` | belief |
| `InferenceEpoch` | epoch id from scoped belief keys, topology hash, method, and source cursor | one or more `Belief` values | belief |
| `HypothesisSet` | belief key plus hypothesis family plus revision id | `Belief` | belief |
| `ObservationOpportunity` | belief key plus evidence target plus channel ref plus source revision id | `Belief` | belief |
| `BeliefView` | belief key plus perspective key plus branch scope plus current revision id plus view version | `Belief` | belief |
| `CalibrationRecord` | belief key plus outcome evidence id plus calibration target | `Belief` | belief |

## `Belief`

One stable assessed question.

Purpose:
anchors evidence, revisions, leases, hypotheses, calibration, and views for a subject and dimension under a scope.

Required components:

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
`DomainObjectRef`, `PerspectiveKey`, `BranchScope`, `BeliefDimension`, evidence policy contract.

## `EvidenceItem`

One normalized belief input.

Purpose:
turns spine facts, graph anchors, outcomes, corrections, or measurements into belief-readable evidence.

Required components:

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
spine facts, graph anchor records, execution outcome facts, normalizer version.

## `BeliefRevision`

One append-only settlement over a bounded evidence window.

Purpose:
records the assessment result for one belief key at one assessment epoch.

Required components:

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
assigned evidence items, previous revision when available, comparator contract.

## `AssessmentLease`

One durable coordination record for assessment work.

Purpose:
serializes assessment for one belief key while allowing independent belief keys to run in parallel.

Required components:

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

## `InferenceEpoch`

One bounded multi-belief inference pass.

Purpose:
freezes enough local topology and source boundary to make message passing, predictive residual inference, or hypothesis scoring replayable.

Required components:

- `InferenceBoundary`

Creation:
created by inference scheduler when a belief update spans multiple belief keys or topology.

Lifecycle:
open, converged, expired, abandoned, committed.

Dependencies:
belief keys, graph or evidence high-water mark, method version, topology hash.

## `HypothesisSet`

One competing set of latent explanations or state alternatives.

Purpose:
keeps hidden causes and alternatives explicit instead of burying them in one confidence value.

Required components:

- `HypothesisCandidate`
- `HypothesisWeight`
- `HypothesisEvidence`

Creation:
created by comparator output or inference epoch when alternatives materially affect posterior state.

Lifecycle:
active, pruned, superseded, archived.

Dependencies:
belief revision, evidence refs, inference method.

## `ObservationOpportunity`

One information need produced by unresolved belief state.

Purpose:
states what evidence would reduce uncertainty, conflict, staleness, or missing-comparator pressure.

Required components:

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

## `BeliefView`

One shaped projection for planners, agents, operators, and upper world-model layers.

Purpose:
exposes current belief state without leaking raw facts, active leases, comparator drafts, or worker internals.

Required components:

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

## `CalibrationRecord`

One outcome-based calibration update.

Purpose:
compares prior belief outputs to later outcomes so reliability, precision, comparator trust, and priors can improve.

Required components:

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

## Entity Rules

- `Belief` is the scheduling and ownership root.
- `BeliefRevision` is append-only and never edited in place.
- `EvidenceItem` is normalized once and then assigned by refs.
- `BeliefView` is a projection, not durable source authority.
- `AssessmentLease` and `InferenceEpoch` are durable enough for recovery.
- `HypothesisSet` is material only when alternatives affect uncertainty, observation, regime signals, or planner views.
- `ObservationOpportunity` is not an action command.
- `CalibrationRecord` changes future reliability and prior selection, not old revision history.

## Read With

- [Belief Components](components.md)
- [Belief Systems](systems.md)
- [Belief Requirements](requirements.md)
- [World Model Belief](README.md)
