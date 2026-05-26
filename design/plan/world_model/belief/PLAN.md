# World Model Belief Phased Implementation Plan

Date: 2026-05-24
Status: active
Scope: first runtime belief slice for `docs_freshness`

## Overview

This plan converts the Phase 3 belief assessment into one implementation order.

The first slice proves one replayable path from graph state and promoted facts to a planner-facing belief view. It loads one external runtime belief configuration named `docs_freshness`. The slice must produce a compact freshness confidence value that Phase 4 can project into `meld-lang::WorldState`.

The order is dependency driven:

- Define the first slice contract and module boundary
- Add belief identity and public record types
- Add evidence normalization for graph anchors and promoted outcome facts
- Load external belief content from runtime configuration
- Assign normalized evidence to the configured belief key
- Assess the key with one deterministic Bayesian comparator engine selected by configuration
- Commit an append-only belief revision
- Project the current belief view for planner consumption
- Add stale, missing evidence, and observation-needed state
- Add lease, recovery, and storm coalescing behavior for one belief key
- Prove replay and graph-to-view behavior through focused tests

Related specs:
- [World Model Domain](../../../cognitive_architecture/world_model/README.md)
- [World Model Belief](../../../cognitive_architecture/world_model/belief/README.md)
- [Belief Families](../../../cognitive_architecture/world_model/belief/belief_families.md)
- [Fact To Belief](../../../cognitive_architecture/world_model/belief/fact_to_belief.md)
- [Comparator Model](../../../cognitive_architecture/world_model/belief/comparator_model.md)
- [Belief Substrate](../../../cognitive_architecture/world_model/belief/substrate.md)
- [Belief Spec](../../../cognitive_architecture/world_model/belief/spec.md)
- [Belief Requirements](../../../cognitive_architecture/world_model/belief/requirements.md)
- [World Model Planner](../../../cognitive_architecture/world_model/planner/README.md)

Related plan docs:
- [Cognitive Architecture Implementation Plan](../../README.md)
- [Belief Assessment](assessment.md)
- [Graph Assessment](../graph/assessment.md)
- [Planner Projection Assessment](../planner/assessment.md)
- [Typed Loop Integration](../../integration/typed_loop.md)
- [Meld Lang Assessment](../../meld-lang/assessment.md)

## Guiding Rules

| Rule | Statement |
|------|-----------|
| One family | The loaded runtime family configuration is `docs_freshness` only. |
| Runtime content | Belief family content is external runtime configuration only. |
| No family code | Core code must not define `docs_freshness`, `ContentFreshness`, evidence names, priors, weights, thresholds, or source mappings as Rust modules, enum variants, or hardcoded match arms. |
| One question | The first question is whether documentation freshness confidence is below the planner threshold. |
| Graph first | Belief consumes graph anchors, lineage, provenance, and promoted facts. It does not reread source-domain internals. |
| View boundary | Planner and agent code consume `BeliefView`, not raw evidence, reducer state, leases, or comparator drafts. |
| Replay | Same graph state, source facts, policies, comparator version, and cursor range produce the same revision and view. |
| Append only | Evidence and revisions are appended. Revision head movement supersedes prior state without deleting it. |
| Perspective explicit | The first perspective may be `default`, but it must be carried in keys and views. |
| No dispatch | Belief may emit observation opportunity records. It must not create goals, dispatch tasks, or invoke capabilities. |
| Modern modules | Use `belief.rs` and `belief/*.rs`. Do not add `mod.rs`. |

## First Slice Contract

| Contract | First value |
|----------|-------------|
| Family configuration id | `docs_freshness` |
| Dimension | Runtime configured dimension id for content freshness |
| Subject | `DomainObjectRef` for a documented workspace node |
| Perspective | Explicit `default` perspective unless a caller provides one |
| Branch scope | Explicit branch scope with `main` as initial default |
| Posterior shape | scalar stale-docs probability from `0.0` through `1.0` |
| Planner field | docs freshness confidence derived from stale-docs probability |
| Planner threshold | freshness confidence below `0.7` blocks the goal condition used by the next phase |
| Minimum status set | `settled`, `stale`, `needs_observation`, `needs_assessment`, `assessment_pending`, `invalid` |
| Required evidence path | graph anchor plus provenance into normalized evidence |
| Required comparator | Typed Bayesian comparator engine selected by runtime configuration |
| Required output | current `BeliefView` with confidence, uncertainty, freshness, observation state, and provenance summary |

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 0 | Scope lock and contract inventory | Graph and events assessments | Not started |
| 1 | Module scaffold and public boundary | Phase 0 | Not started |
| 2 | Identity, runtime configuration, and record contracts | Phase 1 | Not started |
| 3 | Evidence normalization | Phase 2 and graph query surface | Not started |
| 4 | Belief key assignment | Phase 3 | Not started |
| 5 | Comparator engine selection and configured assessment | Phase 4 | Not started |
| 6 | Revision commit and current head | Phase 5 | Not started |
| 7 | Belief view projection | Phase 6 | Not started |
| 8 | Freshness, contradiction, and observation-needed state | Phase 7 | Not started |
| 9 | Lease, recovery, and storm coalescing | Phase 8 | Not started |
| 10 | End-to-end replay and typed-loop handoff tests | Phase 9 | Not started |

---

## Phase 0 -- Scope Lock And Contract Inventory

| Field | Value |
|-------|-------|
| Goal | Freeze the exact runtime surface for the `docs_freshness` belief slice. |
| Dependencies | `events` complete, `world_model/graph` complete, `meld-lang` complete |
| Docs | [Belief Assessment](assessment.md), [Belief Families](../../../cognitive_architecture/world_model/belief/belief_families.md), [Planner Projection Assessment](../planner/assessment.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Confirm the graph query used as first input for a docs node subject and current documentation anchor. | Not started |
| 2 | Confirm the source fact and anchor provenance fields carried into first evidence records. | Not started |
| 3 | Define the exact compact `BeliefView` fields Phase 4 will consume. | Not started |
| 4 | Define default perspective and branch scope names in one place. | Not started |
| 5 | Define where runtime belief configuration is loaded, validated, versioned, and snapshotted for replay. | Not started |
| 6 | Record all deferred belief requirements in this plan before code starts. | Not started |

Test Suite Expansion:
- Add a graph contract test that names the first docs-node query shape and locks the returned subject, anchor, and provenance fields.
- Add a planner handoff fixture test that asserts the compact belief fields required by Phase 4 can be represented without private belief state.
- Add a runtime configuration fixture test that loads the first belief configuration without a Rust module, enum variant, or hardcoded family branch.
- Add a documentation parity check in the test fixture comments or test names so future changes can see which assessment open question each test closes.

| Exit Criterion | Status |
|----------------|--------|
| The first graph-to-evidence input is named as a concrete query or adapter contract. | Not started |
| The first planner-facing fields are concrete enough for Phase 4 to implement projection without reading belief internals. | Not started |
| The first belief content is an external runtime configuration artifact, not code-owned family content. | Not started |
| Deferred concerns are explicit and do not leak into first-slice tasks. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Removes the two open assessment questions for evidence fields and compact planner fields. | Not started |

Verification:
- `cargo test -p meld-world-model world_model_queries`
- `cargo test --test integration_tests world_state_graph`

Key files:
- `design/plan/world_model/belief/PLAN.md`
- `design/plan/world_model/belief/assessment.md`
- `crates/meld-world-model/src/world_state/query.rs`
- `crates/meld-world-model/src/world_state/graph.rs`

---

## Phase 1 -- Module Scaffold And Public Boundary

| Field | Value |
|-------|-------|
| Goal | Add a belief domain under `meld-world-model` without disturbing graph compatibility paths. |
| Dependencies | Phase 0 |
| Docs | [World Model Domain](../../../cognitive_architecture/world_model/README.md), [Belief Requirements](../../../cognitive_architecture/world_model/belief/requirements.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add `crates/meld-world-model/src/belief.rs` as the public domain entry. | Not started |
| 2 | Add `crates/meld-world-model/src/belief/contracts.rs` for serializable public record types. | Not started |
| 3 | Add `crates/meld-world-model/src/belief/evidence.rs` for normalization and assignment types. | Not started |
| 4 | Add `crates/meld-world-model/src/belief/comparator.rs` for comparator contracts and first comparator implementation. | Not started |
| 5 | Add `crates/meld-world-model/src/belief/store.rs` for evidence, revision, and view storage. | Not started |
| 6 | Add `crates/meld-world-model/src/belief/runtime.rs` for scheduling, leases, and replay orchestration. | Not started |
| 7 | Re-export only the public belief contracts from `crates/meld-world-model/src/lib.rs`. | Not started |

Test Suite Expansion:
- Add a compile-facing smoke test that imports the public belief module through `meld_world_model` and proves the intended re-export path.
- Add a module-boundary test that existing graph and world-state imports still compile after belief is introduced.
- Add a repository structure check that fails if `crates/meld-world-model/src/belief/mod.rs` exists.

| Exit Criterion | Status |
|----------------|--------|
| `cargo check -p meld-world-model` succeeds with empty or minimal belief modules. | Not started |
| No `mod.rs` file is added. | Not started |
| Existing graph and world-state public imports keep compiling. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Establishes a domain-first home for belief while preserving compatibility naming around `world_state`. | Not started |

Verification:
- `cargo check -p meld-world-model`
- `cargo test -p meld-world-model`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/belief.rs`
- `crates/meld-world-model/src/belief/contracts.rs`
- `crates/meld-world-model/src/belief/evidence.rs`
- `crates/meld-world-model/src/belief/comparator.rs`
- `crates/meld-world-model/src/belief/store.rs`
- `crates/meld-world-model/src/belief/runtime.rs`

---

## Phase 2 -- Identity, Runtime Configuration, And Record Contracts

| Field | Value |
|-------|-------|
| Goal | Implement stable belief records plus runtime-loaded belief configuration contracts for the first slice. |
| Dependencies | Phase 1 |
| Docs | [Belief Requirements](../../../cognitive_architecture/world_model/belief/requirements.md), [Fact To Belief](../../../cognitive_architecture/world_model/belief/fact_to_belief.md), [Belief Substrate](../../../cognitive_architecture/world_model/belief/substrate.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Define `BeliefKey` with subject, runtime dimension id, runtime predicate id, perspective, branch scope, and evidence policy id fields. | Not started |
| 2 | Define `BeliefFamilyConfig` with family id, dimension id, evidence schemas, source mappings, comparator engine, comparator config, default prior, freshness policy, planner projection, and config version. | Not started |
| 3 | Define generic `EvidenceItem` with id, key candidate fields, source fact ids, graph anchor ids, source cursor range, role, runtime evidence schema id, typed value payload, reliability, precision, and provenance summary. | Not started |
| 4 | Define validation contracts for runtime evidence payloads. Do not add family-specific `EvidenceValue` enum variants. | Not started |
| 5 | Define `PosteriorSummary` with scalar probability plus runtime posterior meaning from configuration. | Not started |
| 6 | Define `BeliefRevision` with prior revision link, comparator metadata, config id or snapshot hash, evidence ids, posterior, confidence, uncertainty, precision, freshness, contradiction, status, and source cursor range. | Not started |
| 7 | Define `BeliefView` with key, perspective, current revision id, status, posterior, projected planner fields, confidence, uncertainty, freshness, contradiction, observation state, assessment state, advisory posture, and provenance summary. | Not started |
| 8 | Define `AssessmentLease` with key, epoch, owner id, input cursor range, start time, expiry time, comparator engine, config id or snapshot hash, and lease status. | Not started |
| 9 | Add serde round-trip tests for every public record and runtime configuration record. | Not started |

Test Suite Expansion:
- Add serde round-trip tests for every public belief record.
- Add equality and stable identity tests for `BeliefKey`, including perspective, branch scope, predicate, and evidence policy changes.
- Add runtime configuration loading and validation tests for the first belief configuration fixture.
- Add tests that fail if core contracts require a Rust enum variant for the first family, dimension, or evidence schema.
- Add constructor or validation tests that reject invalid scalar probabilities, invalid cursor ranges, missing required ids, and empty subject fields.
- Add view boundary tests that prove `BeliefView` carries hydration handles and does not carry raw evidence payloads or lease internals.

| Exit Criterion | Status |
|----------------|--------|
| Records are serializable, cloneable, comparable where meaningful, and stable enough for storage. | Not started |
| `BeliefKey` carries perspective and branch scope even when defaults are used. | Not started |
| Runtime belief configuration validates before evidence assignment or assessment can run. | Not started |
| `BeliefView` contains all fields Phase 4 needs without exposing evidence payload internals. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Provides the shared contracts consumed by normalization, assessment, revision, projection, and planner projection. | Not started |
| Prevents belief content from entering core code as family-specific Rust vocabulary. | Not started |

Verification:
- `cargo test -p meld-world-model belief_contracts`
- `cargo clippy -p meld-world-model -- -D warnings`
- `cargo fmt -p meld-world-model -- --check`

Key files:
- `crates/meld-world-model/src/belief/contracts.rs`
- `crates/meld-world-model/src/belief.rs`

---

## Phase 3 -- Evidence Normalization

| Field | Value |
|-------|-------|
| Goal | Convert graph anchors and promoted facts into belief-normalized evidence records. |
| Dependencies | Phase 2 and graph query surface |
| Docs | [Fact To Belief](../../../cognitive_architecture/world_model/belief/fact_to_belief.md), [Belief Spec](../../../cognitive_architecture/world_model/belief/spec.md), [Graph Assessment](../graph/assessment.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add a `BeliefEvidenceNormalizer` that accepts graph anchor records, anchor provenance, promoted outcome facts, and runtime evidence source mappings. | Not started |
| 2 | Preserve source fact ids, anchor ids, object refs, relation refs, source sequence range, reference time, transaction time, and content hash where available. | Not started |
| 3 | Normalize graph and promoted facts into configured evidence schemas when graph data is available. | Not started |
| 4 | Normalize configured source mappings when their promoted shapes are available. | Not started |
| 5 | Mark unsupported or unrecognized candidates as rejected evidence with an explicit reason when audit visibility is required. | Not started |
| 6 | Add tests proving one source fact can produce many evidence candidates and one evidence item preserves graph provenance. | Not started |

Test Suite Expansion:
- Add normalizer tests for each first-slice evidence schema loaded from runtime configuration.
- Add provenance preservation tests that start with a graph anchor plus anchor provenance and assert the evidence record retains source fact ids, anchor ids, object refs, relation refs, and cursor range.
- Add one-to-many mapping tests that prove one promoted fact can produce evidence candidates for multiple belief keys where policy allows it.
- Add rejection tests for unsupported candidates that assert the rejection reason is durable and inspectable when audit visibility is required.
- Add determinism tests that run normalization twice over the same graph input and compare the resulting evidence records.
- Add negative tests proving a missing runtime mapping produces rejected evidence or no assignment, not an implicit family-specific fallback.

| Exit Criterion | Status |
|----------------|--------|
| A docs node graph anchor plus provenance and external configuration can produce a deterministic `EvidenceItem`. | Not started |
| Normalized evidence includes source refs and cursor data needed for replay. | Not started |
| Raw source-domain internals are not required by the normalizer. | Not started |
| Family-specific source mappings are not hardcoded in the normalizer. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Provides the promotion and normalization stages for the fact-to-belief transition. | Not started |

Verification:
- `cargo test -p meld-world-model belief_evidence`
- `cargo test -p meld-world-model world_model_queries`
- `cargo test --test integration_tests world_state_graph`

Key files:
- `crates/meld-world-model/src/belief/evidence.rs`
- `crates/meld-world-model/src/world_state/graph.rs`
- `crates/meld-world-model/src/world_state/query.rs`

---

## Phase 4 -- Belief Key Assignment

| Field | Value |
|-------|-------|
| Goal | Attach normalized evidence to the configured first-slice belief key for a subject. |
| Dependencies | Phase 3 |
| Docs | [Belief Families](../../../cognitive_architecture/world_model/belief/belief_families.md), [Belief Spec](../../../cognitive_architecture/world_model/belief/spec.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement assignment through runtime evidence-to-belief mapping rules. | Not started |
| 2 | Build `BeliefKey` from evidence subject, runtime dimension id, runtime predicate id, perspective, branch scope, and evidence policy id. | Not started |
| 3 | Support default perspective and branch scope through explicit values, not hidden process state. | Not started |
| 4 | Store evidence-to-key assignments with source cursor and role. | Not started |
| 5 | Mark a belief dirty when assigned evidence exceeds the current revision high-water mark. | Not started |
| 6 | Add tests for deterministic key construction and dirty marking. | Not started |

Test Suite Expansion:
- Add assignment tests that construct a first-slice key from normalized evidence and runtime configuration, then assert subject, dimension id, predicate id, perspective, branch scope, and evidence policy id are all present.
- Add defaulting tests that prove `default` perspective and `main` branch scope are explicit field values, not hidden runtime state.
- Add dirty marking tests for evidence newer than the current revision high-water mark.
- Add idempotence tests that assigning the same evidence twice does not create duplicate assignments.
- Add rejection tests for evidence that cannot be assigned to the first-slice family.
- Add tests proving assignment fails closed when the runtime family configuration is absent.

| Exit Criterion | Status |
|----------------|--------|
| Every first-slice evidence item either assigns through runtime configuration or records a rejection reason. | Not started |
| The same evidence always produces the same key under the same perspective and branch scope. | Not started |
| No assignment branch depends on a family-specific Rust type or match arm. | Not started |
| Dirty cursor state is available for assessment scheduling. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Provides the scheduling unit used by comparator execution and leases. | Not started |

Verification:
- `cargo test -p meld-world-model belief_assignment`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/belief/evidence.rs`
- `crates/meld-world-model/src/belief/store.rs`

---

## Phase 5 -- Comparator Engine Selection And Configured Assessment

| Field | Value |
|-------|-------|
| Goal | Assess one belief key with a deterministic typed Bayesian comparator engine driven entirely by runtime configuration. |
| Dependencies | Phase 4 |
| Docs | [Comparator Model](../../../cognitive_architecture/world_model/belief/comparator_model.md), [Belief Families](../../../cognitive_architecture/world_model/belief/belief_families.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Define comparator input with belief key, prior revision, evidence window, source cursor range, comparator engine id, runtime config snapshot or hash, and provenance bundle. | Not started |
| 2 | Define comparator output with configured posterior, configured planner projection fields, uncertainty, precision, status, contributing evidence ids, contradicted evidence ids, observation need, and explanation summary. | Not started |
| 3 | Add comparator engine selection from runtime family configuration. | Not started |
| 4 | Implement a generic weighted Bayesian comparator engine. Do not implement a `DocsFreshnessBayesianComparator`. | Not started |
| 5 | Load prior, factor names, evidence schema bindings, weights, normalizations, polarity rules, and projection formula from runtime configuration. | Not started |
| 6 | Treat contradictory evidence according to configured polarity rules and projection formula. | Not started |
| 7 | Emit `needs_observation` when required evidence is absent and uncertainty remains material. | Not started |
| 8 | Emit `needs_assessment` through missing comparator state for unsupported dimensions. | Not started |
| 9 | Add deterministic replay tests for same evidence, same prior, and same comparator version. | Not started |

Test Suite Expansion:
- Add factor extraction tests for configured factor names with boundary values at zero, midpoint, and cap.
- Add posterior tests that lock the weighted calculation from runtime prior and config snapshot to configured posterior and planner projection fields.
- Add contradictory evidence tests driven by configured polarity rules.
- Add missing evidence tests that produce `needs_observation` rather than a guessed settled value.
- Add missing comparator tests for unsupported dimensions that produce `needs_assessment`.
- Add deterministic replay tests that compare comparator output for identical evidence, prior revision, config, and comparator version.
- Add negative tests proving changing the runtime config version changes the config hash and stale state rather than silently changing replay.

| Exit Criterion | Status |
|----------------|--------|
| The comparator engine emits a configured scalar posterior and configured planner projection fields for the first runtime family. | Not started |
| A docs freshness confidence below `0.7` can be produced for the typed loop scenario. | Not started |
| Missing comparator state is explicit and never guessed. | Not started |
| Same inputs produce byte-stable serialized output where ordering is deterministic. | Not started |
| No comparator type or Rust module is named after `docs_freshness`. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Produces the proposed belief revision that later phases commit and project. | Not started |

Verification:
- `cargo test -p meld-world-model configured_bayesian_comparator`
- `cargo test -p meld-world-model comparator_replay`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/belief/comparator.rs`
- `crates/meld-world-model/src/belief/contracts.rs`

---

## Phase 6 -- Revision Commit And Current Head

| Field | Value |
|-------|-------|
| Goal | Append belief revisions and move the current revision head without editing prior revisions. |
| Dependencies | Phase 5 |
| Docs | [Belief Requirements](../../../cognitive_architecture/world_model/belief/requirements.md), [Belief Spec](../../../cognitive_architecture/world_model/belief/spec.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add storage for evidence records, revision records, revision head by key, and source cursor indexes. | Not started |
| 2 | Validate that proposed revision evidence ids exist and fall within the lease input cursor range. | Not started |
| 3 | Append a new `BeliefRevision` with prior revision link when one exists. | Not started |
| 4 | Move the current revision head for the key to the new revision. | Not started |
| 5 | Preserve previous revisions for provenance and replay. | Not started |
| 6 | Add queries for current revision, revision history, evidence by revision, and provenance by revision. | Not started |
| 7 | Add tests for append, supersession by head movement, and provenance lookup. | Not started |

Test Suite Expansion:
- Add append tests that prove a committed revision is immutable after write.
- Add current-head tests that prove a later revision moves the head without deleting the prior revision.
- Add validation tests that reject proposed revisions with unknown evidence ids or cursor ranges outside the lease input window.
- Add provenance lookup tests from revision id to evidence ids, source fact ids, graph anchor ids, and prior revision id.
- Add replay-order tests that rebuild revision history from stored records and recover the same current head.

| Exit Criterion | Status |
|----------------|--------|
| Revision commit never deletes or rewrites old revisions. | Not started |
| Current revision query returns the latest committed revision for a key. | Not started |
| Evidence and provenance can be hydrated from a revision id. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Creates durable belief settlement state for view projection and recovery. | Not started |

Verification:
- `cargo test -p meld-world-model belief_revision_store`
- `cargo test -p meld-world-model belief_provenance`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/belief/store.rs`
- `crates/meld-world-model/src/belief/contracts.rs`

---

## Phase 7 -- Belief View Projection

| Field | Value |
|-------|-------|
| Goal | Project the current revision into a compact planner-facing `BeliefView`. |
| Dependencies | Phase 6 |
| Docs | [Belief Substrate](../../../cognitive_architecture/world_model/belief/substrate.md), [Fact To Belief](../../../cognitive_architecture/world_model/belief/fact_to_belief.md), [Planner Projection Assessment](../planner/assessment.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add projection from current revision plus freshness, contradiction, and observation state into `BeliefView`. | Not started |
| 2 | Include key, perspective, current revision id, status, posterior, confidence, uncertainty, precision, freshness, contradiction, observation state, assessment state, and provenance summary. | Not started |
| 3 | Attach hydration handles for evidence ids, source fact ids, graph anchor ids, and revision id. | Not started |
| 4 | Exclude raw spine payloads, active lease internals, comparator drafts, and unpublished evidence churn. | Not started |
| 5 | Add query by belief key. | Not started |
| 6 | Add query by subject and perspective. | Not started |
| 7 | Add tests proving planner-facing projection can read a confidence value without reaching into evidence or comparator modules. | Not started |

Test Suite Expansion:
- Add projection tests from a committed revision into `BeliefView` for settled, stale, needs-observation, and needs-assessment statuses.
- Add query tests by belief key and by subject plus perspective.
- Add boundary tests that verify planner-facing code reads confidence and status only from `BeliefView`.
- Add hydration handle tests that confirm source facts, evidence, revisions, and graph anchors can be referenced without embedding raw payloads.
- Add rebuild tests that drop any cached view and reconstruct the same view from committed revision state.

| Exit Criterion | Status |
|----------------|--------|
| Current `BeliefView` is rebuildable from committed revision state. | Not started |
| Planner projection can consume the view as a public contract. | Not started |
| The view does not expose raw fact payloads or active lease internals. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Unblocks Phase 4 planner projection into `meld-lang::WorldState`. | Not started |

Verification:
- `cargo test -p meld-world-model belief_view_projection`
- `cargo test --test integration_tests execution_projection`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/belief/runtime.rs`
- `crates/meld-world-model/src/belief/store.rs`
- `crates/meld-world-model/src/belief/contracts.rs`

---

## Phase 8 -- Freshness, Contradiction, And Observation Needed State

| Field | Value |
|-------|-------|
| Goal | Make stale state, conflict state, and missing evidence visible in the belief view. |
| Dependencies | Phase 7 |
| Docs | [Belief Requirements](../../../cognitive_architecture/world_model/belief/requirements.md), [Belief Substrate](../../../cognitive_architecture/world_model/belief/substrate.md), [Belief Spec](../../../cognitive_architecture/world_model/belief/spec.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Mark stale when assigned evidence exceeds the current revision high-water mark. | Not started |
| 2 | Mark stale when graph anchor evidence used by a revision is superseded. | Not started |
| 3 | Mark stale when comparator configuration or evidence policy changes. | Not started |
| 4 | Preserve supporting and contradicting evidence ids separately. | Not started |
| 5 | Distinguish weak coverage, counterevidence, supersession, invalidation, and missing comparator where the first slice can observe them. | Not started |
| 6 | Emit observation opportunity records for missing evidence, stale state, unresolved conflict, or missing comparator. | Not started |
| 7 | Ensure observation opportunity records remain world-model outputs and are not execution commands. | Not started |
| 8 | Add tests for stale view, missing evidence view, and contradiction state. | Not started |

Test Suite Expansion:
- Add stale-state tests for new evidence after revision high-water mark, superseded graph anchor, comparator config change, and evidence policy change.
- Add contradiction tests that keep supporting and contradicting evidence ids separate.
- Add conflict-kind tests for weak coverage, counterevidence, supersession, invalidation, and missing comparator when represented in the first slice.
- Add observation opportunity tests for missing evidence, stale state, unresolved conflict, and missing comparator.
- Add non-dispatch tests that prove observation opportunities contain no task, capability invocation, provider, or execution command.
- Add planner indeterminate-readiness tests that expose enough view state for the next phase to map uncertainty to `Indeterminate`.

| Exit Criterion | Status |
|----------------|--------|
| A stale belief view exposes stale reason without requiring planner to inspect evidence. | Not started |
| Missing evidence and missing comparator produce observation-needed or needs-assessment state. | Not started |
| Contradiction is not collapsed into a generic low confidence value. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Gives planner projection enough state to choose `Indeterminate` later when freshness or observation state blocks certainty. | Not started |

Verification:
- `cargo test -p meld-world-model belief_freshness`
- `cargo test -p meld-world-model belief_observation_opportunity`
- `cargo test -p meld-world-model belief_contradiction`

Key files:
- `crates/meld-world-model/src/belief/contracts.rs`
- `crates/meld-world-model/src/belief/runtime.rs`
- `crates/meld-world-model/src/belief/store.rs`

---

## Phase 9 -- Lease, Recovery, And Storm Coalescing

| Field | Value |
|-------|-------|
| Goal | Serialize assessment for one belief key while allowing replay recovery and evidence coalescing. |
| Dependencies | Phase 8 |
| Docs | [Belief Substrate](../../../cognitive_architecture/world_model/belief/substrate.md), [Belief Spec](../../../cognitive_architecture/world_model/belief/spec.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement one active `AssessmentLease` per `BeliefKey`. | Not started |
| 2 | Record queued, leased, completed, expired, and abandoned lease states. | Not started |
| 3 | Select comparator and input cursor range when acquiring a lease. | Not started |
| 4 | Keep incoming evidence appended while a lease is active. | Not started |
| 5 | Mark `dirty_since_seq` for new evidence that arrives during active assessment. | Not started |
| 6 | Complete the active lease and reschedule the key when dirty evidence remains. | Not started |
| 7 | Add recovery scan that finds expired leases, marks them abandoned, and reschedules dirty keys from durable evidence. | Not started |
| 8 | Add tests for one active lease, expired lease recovery, and storm coalescing behind one key. | Not started |

Test Suite Expansion:
- Add lease acquisition tests that permit only one active lease per `BeliefKey`.
- Add overlapping-worker tests that prove a stale lease owner cannot commit over a newer valid lease.
- Add lease expiry tests that mark expired work abandoned and reschedule from durable evidence.
- Add storm coalescing tests that append incoming evidence behind an active lease and set `dirty_since_seq`.
- Add recovery tests that rebuild pending assessment state from evidence high-water marks, revision high-water marks, and lease records.
- Add bounded-work tests that prove a burst of evidence for one key produces one active comparator run plus a rescheduled dirty pass.

| Exit Criterion | Status |
|----------------|--------|
| Two workers cannot commit overlapping revisions for the same key. | Not started |
| Recovery does not depend on hidden worker memory. | Not started |
| A burst of evidence for one key results in bounded comparator work and a deterministic final view. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Makes the first belief runtime operational enough for replay and repeated event ingress. | Not started |

Verification:
- `cargo test -p meld-world-model belief_leases`
- `cargo test -p meld-world-model belief_recovery`
- `cargo test -p meld-world-model belief_storm_coalescing`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/belief/runtime.rs`
- `crates/meld-world-model/src/belief/store.rs`

---

## Phase 10 -- End To End Replay And Typed Loop Handoff Tests

| Field | Value |
|-------|-------|
| Goal | Prove that graph state becomes a current belief view suitable for planner projection. |
| Dependencies | Phase 9 |
| Docs | [Typed Loop Integration](../../integration/typed_loop.md), [Planner Projection Assessment](../planner/assessment.md), [Cognitive Architecture Implementation Plan](../../README.md) |
| Status | Not started |

| Order | Task | Status |
|-------|------|--------|
| 1 | Build an integration test fixture with a docs node subject, graph anchor, provenance, first-slice runtime configuration, and configured freshness evidence. | Not started |
| 2 | Run configuration load, normalization, assignment, comparator assessment, revision commit, and view projection. | Not started |
| 3 | Assert the view contains docs freshness confidence below `0.7` for the typed loop scenario. | Not started |
| 4 | Assert replay from durable evidence and revisions rebuilds the same current view. | Not started |
| 5 | Assert planner-facing code can consume only `BeliefView` and does not import evidence, comparator, or lease internals. | Not started |
| 6 | Add a stale case where newer evidence marks the prior view stale before reassessment completes. | Not started |
| 7 | Add a content-written case where follow-up evidence lowers the stale-docs posterior. | Not started |

Test Suite Expansion:
- Add an end-to-end belief integration test that starts from external runtime configuration plus a docs node graph anchor and provenance, normalizes evidence, assigns a key, assesses it, commits a revision, and projects a `BeliefView`.
- Add replay integration tests that rebuild evidence, revisions, and current view from durable records and assert equality with the original view.
- Add typed-loop handoff tests that assert Phase 4 can consume only `BeliefView` to obtain docs freshness confidence below `0.7`.
- Add stale-before-reassessment integration tests that publish newer evidence while the prior view remains current and assert the view reports stale state.
- Add content-written flywheel tests that follow a stale docs view with content-written evidence and assert the stale-docs posterior falls.
- Add privacy tests that fail if integration code imports comparator, evidence store internals, or lease internals to satisfy planner-facing assertions.
- Add no-family-code tests that fail if the first slice requires a `docs_freshness` Rust module, `ContentFreshness` enum variant, or family-specific comparator type.

| Exit Criterion | Status |
|----------------|--------|
| One externally configured `docs_freshness` flywheel segment is proven from graph input to planner-facing belief view. | Not started |
| Replay rebuilds the same revision and view under the same inputs. | Not started |
| Phase 4 can proceed without any belief-internal dependencies. | Not started |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Closes Phase 3 and unblocks planner projection. | Not started |

Verification:
- `cargo test -p meld-world-model`
- `cargo test --test integration_tests world_state_graph`
- `cargo test --test integration_tests execution_projection`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --all -- --check`

Key files:
- `crates/meld-world-model/tests/world_model_queries.rs`
- `tests/integration/world_state_graph.rs`
- `tests/integration/execution_projection.rs`
- `crates/meld-world-model/src/belief.rs`
- `crates/meld-world-model/src/belief/runtime.rs`

## Deferred Scope

The first implementation must not include these items:

- Rust modules named after runtime belief families
- Rust enum variants named after runtime belief dimensions
- Rust enum variants named after runtime evidence schemas
- Family-specific comparator types
- Family-specific source mapping match arms
- Full prior and posterior distribution records beyond compact scalar probability
- Full comparator catalog
- Rule comparator beyond contract shape unless implementation needs it for missing-state tests
- Semantic settlement adapter beyond contract shape unless implementation needs it for provisional-state tests
- Message passing inference
- Predictive residual inference
- Multi-belief inference epochs
- Hypothesis sets over hidden causes
- Regime-conditioned prior selection
- Outcome-driven calibration that changes future priors
- Multi-agent divergence beyond explicit default perspective fields
- Cost-benefit goal curation
- Goal creation
- Task dispatch
- Capability invocation
- Causal effect summaries
- Regime sensitivity summaries
- Broad risk envelopes
- Plan diffing
- Graph mutation acceptance

## Phase Completion Definition

Phase 3 is complete when:

- external runtime configuration for `docs_freshness` can be loaded, validated, versioned, and snapshotted
- configured evidence can be normalized from graph-backed inputs
- normalized evidence assigns to a stable belief key using configured dimension and predicate ids
- the generic typed Bayesian comparator engine emits deterministic configured posterior and planner projection values
- a belief revision is appended and current head moves to it
- a current `BeliefView` exposes confidence, uncertainty, freshness, observation state, status, and provenance summary
- stale and missing-evidence states are explicit
- lease recovery and storm coalescing work for one belief key
- replay rebuilds the same view
- the planner projection phase can consume the view without depending on belief internals

## Validation Rules

Plan and implementation updates must keep these invariants:

- Belief code stays under `crates/meld-world-model/src/belief.rs` and `crates/meld-world-model/src/belief/*.rs`.
- No `mod.rs` files are introduced.
- Belief family content lives in external runtime configuration, not in Rust family modules.
- Belief core code defines generic runtime ids, schemas, validators, comparator engines, stores, and views only.
- The string `docs_freshness` may appear in tests, fixtures, and loaded configuration, but not as a Rust module name or comparator type.
- Graph remains the owner of anchors, lineage, provenance, and traversal.
- Events remain the owner of append, replay, sequence, and durable fact identity.
- Belief remains the owner of evidence normalization, belief assessment, revision, freshness, contradiction, observation opportunity, and belief view projection.
- Planner projection reads belief views and graph-derived public surfaces only.
- Agent owns goal policy.
- Execution owns planning runtime, task construction, dispatch, side effects, and outcome publication.
