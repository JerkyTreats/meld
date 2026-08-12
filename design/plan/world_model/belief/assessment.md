# Belief Readiness Assessment

Status: first slice implemented
Depends on: `design/plan/events/assessment.md`, `design/plan/world_model/graph/assessment.md`
Design source: `design/cognitive_architecture/world_model/belief/README.md`, `design/cognitive_architecture/world_model/belief/belief_families.md`, `design/cognitive_architecture/world_model/belief/fact_to_belief.md`, `design/cognitive_architecture/world_model/belief/comparator_model.md`, `design/cognitive_architecture/world_model/belief/substrate.md`, `design/cognitive_architecture/world_model/belief/curation.md`, `design/cognitive_architecture/world_model/belief/microarchitecture.md`, `design/cognitive_architecture/world_model/belief/spec.md`
Evidence date: 2026-07-10

Note 2026-08-12: belief outcome interpretation has since gained generic array-element selectors driven by theory data, commit 4894b73. This assessment stands as evidence for the earlier slice.

## Verdict Summary

Belief design has landed its first externally configured belief family path and one comparator engine path.

The first loaded belief family configuration is `docs_freshness`.

The implementation lives in `crates/meld-world-model/src/belief.rs` and `crates/meld-world-model/src/belief/*.rs`.

## Conceptual Correctness

Belief solves credibility, uncertainty, freshness, contradiction, and observation need over graph state.

Graph says what is current. Belief says how credible and action-relevant that current state is.

## Completeness

The landed slice covers one externally configured `docs_freshness` belief key, graph-anchor evidence normalization, promoted outcome evidence normalization, one generic comparator engine path, append-only revisions, and compact planner-facing belief views.

Belief produces values that world model planner converts into `WorldState` propositions.

Belief does not create `Goal`, `Method`, `Composition`, or task-network work.

## Boundary Clarity

Belief owns belief identity, runtime family configuration loading, evidence normalization, priors, posteriors, uncertainty, precision, freshness, contradiction, supersession, hypothesis sets, calibration, observation opportunities, and planner-facing belief values.

Belief does not own graph anchors, event append, causal claims, regime identity, goal lifecycle, task dispatch, or execution side effects.

## Dependency Readiness

The event mechanics and graph queries required by the implemented belief slice are ready.
Production authority cutover remains part of the active event foundation closeout.

Belief can consume current anchors, provenance, object history, graph walks, and promoted outcome facts.

## First-Slice Feasibility

Belief supports the typed-loop design path by loading `docs_freshness` runtime configuration and producing a configured confidence value below `0.7`.

World model planner projects that value into `Proposition::Holds` inside `WorldState`.

## Current Implementation Evidence

- `crates/meld-world-model/src/belief.rs`
- `crates/meld-world-model/src/belief/contracts.rs`
- `crates/meld-world-model/src/belief/config.rs`
- `crates/meld-world-model/src/belief/evidence.rs`
- `crates/meld-world-model/src/belief/comparator.rs`
- `crates/meld-world-model/src/belief/store.rs`
- `crates/meld-world-model/src/belief/runtime.rs`
- `crates/meld-world-model/src/belief/query.rs`
- `crates/meld-world-model/tests/belief.rs`
- `design/plan/world_model/belief/PLAN.md`
- `design/cognitive_architecture/world_model/belief/README.md`
- `design/cognitive_architecture/world_model/belief/belief_families.md`
- `design/cognitive_architecture/world_model/belief/fact_to_belief.md`
- `design/cognitive_architecture/world_model/belief/comparator_model.md`
- `design/cognitive_architecture/world_model/belief/spec.md`

## Gaps

- Full prior and posterior record shape remains outside the first slice.
- Broad comparator catalog remains outside the first slice.
- Runtime family configuration loading and replay snapshotting are implemented for the first slice only.
- Outcome-driven calibration requires the outcome publication bridge.
- Multi-agent divergence requires explicit perspective handling beyond the first key.
- Planner projection into `meld-lang::WorldState` is not implemented here.
- Semantic settlement and rule comparator paths remain deferred.

## Open Questions

- Which public world-model route should expose the first belief-backed planner projection.
- How execution outcomes become promoted belief evidence after outcome publication is specified.

## Recommendation

Proceed to planner projection and typed-loop integration on top of the landed `docs_freshness` belief slice.

## Evidence Notes 2026-08-12

Evidence note 2026-08-12: the belief design documents previously carried landed-first-slice and deferred sections. Their non-duplicated substance is recorded here as implementation evidence.

From the belief README landed-slice section: the slice also defined explicit perspective identity and branch scope, observation-needed state with target evidence, lease-based assessment, durable evidence, assignment, revision, view, lease, rejection, config snapshot, runtime metadata, and dirty-key stores, dirty-key rescheduling and storm coalescing for one belief key, and stale detection for newer evidence, superseded anchors, config snapshot changes, and evidence policy changes. Research-direction items listed deferred: explicit prior and posterior pair records for every revision, prior-to-posterior divergence or surprise as a required field, origin and coverage fields distinguishing direct observation, predictive persistence, smoothing, and never-observed state, retrospective smoothing over hidden transition timing, and structured multi-belief inference epochs across connected belief keys. Planner projection into `meld-lang::WorldState` was also listed deferred there, but the planner assessment records it as implemented, so that entry was dropped rather than moved.

From the microarchitecture landed-boundary section: the slice runs in one binary while behaving as if the belief and agent loops were separate processes. It defined `BeliefView` as the only planner input, a belief view polling contract through `BeliefQuery`, a task construction hydration path from belief provenance, and promoted evidence normalization for later outcome publication. It explicitly deferred full smoothing over hidden past transitions, global message passing across connected belief families, regime identity and changepoint authority, execution posture commitment, belief view subscriptions, and belief revision publication back to the spine.

From the fact-to-belief first-slice section: the starting shape is one externally configured belief family whose subject is a `DomainObjectRef`, whose runtime predicate id names the planner question, whose perspective is explicit or intentionally defaulted, whose evidence comes from graph anchors and execution outcomes, and whose settlement state reaches agent policy through the belief view.

From the comparator-model landed section: the slice supports one generic typed Bayesian comparator engine driven by runtime family configuration, missing comparator state, posterior, uncertainty, and observation-needed fields in comparator output, and replay tests proving that the same evidence and config snapshot yield the same revision. Deferred comparator work: deterministic rule comparator, semantic settlement adapter with provisional output, broader comparator catalog.

From the substrate first-slice checklist, the slice should prove: replay from spine to evidence, runtime family configuration loading, validation, and snapshotting, evidence to belief key assignment, lease acquisition and expiry, generic comparator engine output to revision, belief view projection, observation-needed projection for unresolved beliefs, storm coalescing for one belief key, and recovery after interrupted assessment.

From the belief requirements first-slice section: implement `BeliefKey`, `EvidenceItem`, `BeliefRevision`, `BeliefView`, and `AssessmentLease`, runtime family configuration loading, validation, and replay snapshotting, evidence normalization from graph anchors and promoted outcome evidence, belief key assignment, one generic typed Bayesian comparator engine driven by runtime configuration, missing comparator state, revision commit, belief view projection, observation-needed projection, stale detection, lease expiry and recovery, storm coalescing for one belief key, and replay tests. Deferred requirements: deterministic rule comparator, semantic settlement with provisional status, full multi-belief message passing, predictive residual inference over continuous state, retrospective smoothing over hidden transition timing, global hypothesis graph across all belief families, regime-conditioned prior selection, and automated capability synthesis for missing evidence channels.
