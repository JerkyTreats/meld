# Belief Systems

Date: 2026-05-15
Status: active
Scope: ECS systems for `world_model/belief`

## Thesis

Belief systems transform durable facts and graph state into evidence, revisions, observation opportunities, calibration records, and shaped belief views.

They are deterministic over explicit inputs, source cursors, method versions, and policy records.
They do not dispatch tasks, decide goals, estimate causal effects, or decide regime identity.

## Pipeline Overview

| Phase | System | Required input | Required output |
|---|---|---|---|
| 1 | fact promotion | spine facts, graph anchors, execution outcomes | belief-relevant fact candidates |
| 2 | evidence normalization | fact candidates, normalizer policy | `EvidenceItem` |
| 3 | belief key assignment | evidence items, key registry | `Belief`, evidence assignments |
| 4 | dirty marking | assigned evidence, revision head | dirty belief keys |
| 5 | assessment scheduling | dirty belief keys, lease state | `AssessmentLease` |
| 6 | comparator execution | lease, evidence window, prior revision | proposed revision output |
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
same source facts, graph state, policies, comparator versions, and source cursors produce the same revisions and belief views.

## Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| spine replay and subscription | events | fact promotion and recovery |
| graph anchor reads | graph | anchor-derived evidence |
| graph provenance reads | graph | evidence provenance |
| graph lineage reads | graph | supersession and stale detection |
| execution outcome reads | execution through spine | calibration evidence |
| evidence channel registry | belief and execution bridge | observation opportunity projection |
| comparator registry | belief | assessment scheduling |
| perspective and evidence policy reads | agent or belief policy store | scoped belief views |

## Fact Promotion

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

## Evidence Normalization

Input:
belief-relevant fact candidates.

Output:
`EvidenceItem` with source, meaning, quality, and provenance components.

Rules:

- Normalize evidence into subject, predicate, value, polarity, and evidence role.
- Record support, contradiction, context, calibration, and supersession roles explicitly.
- Attach source cursor and effective sequence range.
- Attach reliability and precision when available.
- Mark rejected evidence with a reason instead of silently dropping it when audit requires visibility.

## Belief Key Assignment

Input:
evidence items and key registry.

Output:
`Belief` entities and evidence assignments.

Rules:

- Assign evidence to every belief key it can materially affect.
- Create a belief key when registration policy allows creation from evidence.
- Include subject, dimension, predicate, perspective, branch scope, and evidence policy in the key.
- Preserve one fact to many beliefs and many facts to one belief mappings.
- Leave unassigned evidence inspectable when assignment policy cannot classify it.

## Dirty Marking

Input:
evidence assignments and revision heads.

Output:
dirty belief keys and dirty cursor state.

Rules:

- Mark belief dirty when assigned evidence is newer than the current revision high-water mark.
- Mark belief dirty when graph anchor supersession changes evidence meaning.
- Mark belief dirty when evidence policy changes.
- Mark belief dirty when comparator configuration changes.
- Preserve dirty state while an assessment lease is active.

## Assessment Scheduling

Input:
dirty belief keys, revision heads, comparator registry, and lease state.

Output:
`AssessmentLease`.

Rules:

- Use `BeliefKey` as the scheduling unit.
- Maintain one active lease per belief key.
- Select comparator from belief policy and comparator availability.
- Use `MissingComparator` when no comparator exists.
- Coalesce incoming evidence behind active leases.
- Record input low and high cursor on the lease.

## Comparator Execution

Input:
assessment lease, evidence window, prior revision, comparator configuration.

Output:
proposed belief revision.

Rules:

- Run the selected comparator deterministically over explicit inputs.
- Emit posterior summary, confidence, uncertainty, precision, status, and provenance.
- Emit contradiction state when evidence conflicts.
- Emit observation need when uncertainty remains material.
- Mark semantic settlement as provisional unless policy grants settled status.
- Emit needs-assessment or needs-observation for missing comparator state.

## Inference Epoch Execution

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

## Revision Commit

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

## Stale Detection

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

## Contradiction Handling

Input:
evidence roles, comparator output, supersession state.

Output:
contradiction state and conflict metadata.

Rules:

- Preserve supporting and contradicting evidence ids separately.
- Distinguish counterevidence, supersession, invalidation, weak coverage, and competing hypothesis.
- Do not collapse contradiction into low confidence.
- Mark unresolved conflict when no comparator can settle the conflict.

## Observation Opportunity Projection

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

## Belief View Projection

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

## Calibration Ingestion

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

## Recovery Scan

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

## Storm Coalescing

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

## Failure Semantics

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

## Test Matrix

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

## System Rules

- Systems are deterministic over explicit inputs and method versions.
- Systems mutate belief-owned records only.
- Systems publish shaped views and optional derived facts, not action commands.
- Systems preserve source refs and cursor boundaries.
- Systems expose absence, staleness, conflict, and missing comparator state explicitly.
- Systems keep `BeliefKey` as the default scheduling unit.

## Read With

- [Belief Entities](entities.md)
- [Belief Components](components.md)
- [Belief Requirements](requirements.md)
- [World Model Belief](README.md)
