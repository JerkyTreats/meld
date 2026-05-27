# World Model Planner Phased Implementation Plan

Date: 2026-05-27
Status: ready to execute
Scope: first planner projection slice from `BeliefView` to `meld-lang::WorldState`

## Overview

This plan converts the Phase 4 planner projection assessment into one implementation order.

The first slice proves the next cognitive flywheel handoff after belief. It takes public world model state from graph and belief, then emits a ground `meld-lang::WorldState` that execution can evaluate mechanically.

The word planner here means planner-facing world model projection. It does not mean execution planning.

The order is dependency driven:

- Define the first slice contract and module boundary
- Add a planner domain entry under `meld-world-model`
- Define compact projection contracts for the first slice
- Read current belief views through the public belief query facade
- Map belief confidence into a ground `Proposition::Holds`
- Map stale and observation-needed belief state into typed world-state propositions
- Add optional graph scope propositions only from public graph reads
- Build a ground `WorldState`
- Preserve source refs and hydration handles for later explanation
- Prove the handoff with focused tests against `meld-lang::evaluate`

Related specs:
- [World Model Domain](../../../cognitive_architecture/world_model/README.md)
- [World Model Planner](../../../cognitive_architecture/world_model/planner/README.md)
- [Planner Spec](../../../cognitive_architecture/world_model/planner/spec.md)
- [World Model Public Interface](../../../cognitive_architecture/world_model/public_interface.md)
- [World Model Belief](../../../cognitive_architecture/world_model/belief/README.md)
- [World Model Graph](../../../cognitive_architecture/world_model/graph/README.md)
- [Meld Lang World State](../../../cognitive_architecture/meld-lang/world_state.md)

Related plan docs:
- [Cognitive Architecture Implementation Plan](../../README.md)
- [Planner Projection Assessment](assessment.md)
- [Belief Assessment](../belief/assessment.md)
- [Graph Assessment](../graph/assessment.md)
- [Typed Loop Integration](../../integration/typed_loop.md)
- [Meld Lang Assessment](../../meld-lang/assessment.md)

## Implementation Status

Evidence date: 2026-05-27

The first planner projection slice is not implemented.

Available inputs:

- `meld-lang` value types and pure operations
- Graph subject, anchor, provenance, and traversal query surfaces
- Belief current views, subject views, revision history, evidence hydration, provenance, observation opportunities, and dirty-key query surfaces
- First externally configured belief slice with confidence, uncertainty, freshness, observation state, and provenance
- Existing `meld-world-model` test harness using temp sled stores, read facades, serde round trips, proptest, fuzz targets, source scans, and replay assertions

Missing:

- Planner module under `meld-world-model`
- Planner first-slice contracts
- Projection from `BeliefView` into `WorldState`
- Projection tests against `meld-lang::evaluate`
- Public route or query facade for the first `WorldState`
- Planner property tests and fuzz target

Harness additions expected:

- Add `crates/meld-world-model/tests/planner.rs` for first-slice integration and property tests.
- Add `crates/meld-world-model/fuzz/fuzz_targets/fuzz_planner_projection_contract.rs` if planner contracts accept serialized input.
- Add a planner entry to `crates/meld-world-model/fuzz/Cargo.toml` when the fuzz target lands.

Deferred:

- Broad `WorldModelView` runtime
- Causal effect summaries
- Regime sensitivity summaries
- Risk envelopes
- Abstention scoring
- Agent goal curation
- Goal storage
- Method selection
- Composition preparation
- Task dispatch
- Capability invocation
- Outcome publication

## Guiding Rules

| Rule | Statement |
|------|-----------|
| Epistemic only | Planner projection says what the modeled world appears to support, block, or leave uncertain. |
| No execution planning | Planner projection must not select methods, validate compositions, create tasks, dispatch work, wait, retry, or repair. |
| Public inputs only | Projection consumes public graph and belief read surfaces only. |
| Ground output | Every emitted `WorldState` proposition must be ground. |
| Deterministic projection | Same graph state, belief view, perspective, branch, and projection version produce the same `WorldState`. |
| No belief internals | Projection reads `BeliefView` and query facades, not evidence store internals, leases, comparator drafts, or reducer state. |
| No semantic family code | Core projection must not require Rust modules or enum variants named after runtime belief families. |
| No hardcoded family ids | Core projection must read dimension ids and planner field names from `BeliefView` and projection configuration. |
| Thin first slice | Build only what the next agent curation phase needs. |
| Traceable output | Every proposition must trace to graph, belief, or projection rule input. |
| Modern modules | Use `planner.rs` and `planner/*.rs`. Do not add `mod.rs`. |

## Quality Bar

Planner implementation must match the current `meld-world-model` crate bar.

Required verification families:

- Contract tests for serde round trip, validation, and source ref preservation.
- Boundary tests proving planner imports no execution crate and creates no execution planning values.
- Source scans proving planner core contains no hardcoded runtime belief family id or family-specific Rust identifier.
- Grounding tests proving every emitted proposition passes `WorldState::new`.
- Determinism tests proving identical inputs produce equal propositions, source refs, hydration handles, and projection warnings in stable order.
- Query facade tests using temp sled graph and belief stores seeded through existing graph and belief runtime helpers.
- Missing input tests proving absent belief yields typed absence or `Indeterminate`, never fabricated support.
- Reopen tests when persisted lower-layer stores are involved, proving projection after reopening graph and belief stores equals projection before reopening.
- Proptest coverage for confidence values, dimension ids, freshness booleans, observation booleans, duplicate inputs, and invalid field ids.
- Fuzz coverage for projection contract JSON and malformed dimension strings.
- Mutation testing for the planner projection surface during the handoff phase.
- Full crate and workspace verification before the first slice is considered complete.

## Boundary Statement

`world_model/planner` owns typed projection from world model knowledge into execution-readable world state.

It owns:

- decision context for projection
- projection version
- current world-state assembly
- proposition mapping from belief views
- graph scope proposition mapping
- source refs and hydration handles
- typed absence for missing or unresolved input

It does not own:

- goal creation
- goal lifecycle
- method matching
- composition validation
- task graph construction
- task readiness
- provider or capability choice
- dispatch
- retry
- repair
- outcome publication

## First Slice Contract

| Contract | First value |
|----------|-------------|
| Input subject | `DomainObjectRef` for a documented workspace node |
| Input belief | Current `BeliefView` for the configured first belief dimension |
| Input graph | Public graph query surface for subject scope where needed |
| Perspective | Explicit `default` perspective unless caller provides one |
| Branch scope | Explicit belief branch scope with `main` as initial default |
| Projection version | Static first-slice planner projection version |
| Required output | Ground `meld-lang::WorldState` |
| Confidence proposition | `Proposition::Holds` with dimension from the belief view and numeric confidence value |
| Freshness proposition | Ground proposition derived from a generic freshness projection rule |
| Observation proposition | Ground proposition derived from a generic observation-needed projection rule or explicit omission rule |
| Scope proposition | `Proposition::Accessible` for the docs subject when graph scope is available |
| Related proposition | Deferred unless the first execution method requires relation preconditions |
| Missing belief behavior | Empty or indeterminate world state input that lets `meld-lang::evaluate` return `Indeterminate` |
| Output consumer | Agent curation and execution evaluation through `meld-lang` only |

## First Projection Shape

The first confidence projection should produce this required proposition:

```rust
Proposition::Holds {
    subject: Term::Object(subject),
    dimension: Term::Dimension(view.key.dimension_id.clone()),
    condition: Condition::Equals(Term::Literal(Literal::Number(view.confidence))),
}
```

The first freshness projection should prefer a separate proposition so execution can evaluate it mechanically without unpacking a structured value. The dimension name must be derived by a generic projection rule, not by hardcoding a belief family:

```rust
let stale_dimension = projection_config.freshness_dimension(&view.key.dimension_id);

Proposition::Holds {
    subject: Term::Object(subject),
    dimension: Term::Dimension(stale_dimension),
    condition: Condition::Equals(Term::Literal(Literal::Bool(view.freshness.stale))),
}
```

Observation-needed state should be explicit when the belief view carries an open observation opportunity:

```rust
let observation_dimension = projection_config.observation_dimension(&view.key.dimension_id);
let open = view.observation.as_ref().is_some_and(|observation| observation.open);

Proposition::Holds {
    subject: Term::Object(subject),
    dimension: Term::Dimension(observation_dimension),
    condition: Condition::Equals(Term::Literal(Literal::Bool(open))),
}
```

This keeps the first slice compatible with the existing `meld-lang` proposition grammar.

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 0 | Scope lock and boundary audit | Belief, graph, and meld-lang assessments | Pending |
| 1 | Module scaffold and public boundary | Phase 0 | Pending |
| 2 | First-slice projection contracts | Phase 1 | Pending |
| 3 | Belief view to proposition mapping | Phase 2 | Pending |
| 4 | Graph scope projection | Phase 2 | Pending |
| 5 | WorldState assembly and grounding checks | Phases 3 and 4 | Pending |
| 6 | Query facade and public route shape | Phase 5 | Pending |
| 7 | Typed-loop handoff tests | Phase 6 | Pending |

---

## Phase 0 -- Scope Lock And Boundary Audit

| Field | Value |
|-------|-------|
| Goal | Freeze the exact first projection surface and execution boundary. |
| Dependencies | `world_model/graph` complete, `world_model/belief` complete, `meld-lang` complete |
| Docs | [Planner Projection Assessment](assessment.md), [World Model Planner](../../../cognitive_architecture/world_model/planner/README.md), [Planner Spec](../../../cognitive_architecture/world_model/planner/spec.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Confirm that first implementation is `BeliefView` to `WorldState`, not broad `WorldModelView`. | Pending |
| 2 | Confirm that goal curation remains outside planner projection. | Pending |
| 3 | Confirm that execution method selection remains outside planner projection. | Pending |
| 4 | Select generic projection rules for confidence, stale state, and observation-needed dimensions. | Pending |
| 5 | Decide whether `Accessible` is emitted from graph input in the first implementation. | Pending |
| 6 | Record all deferred planner concepts before code starts. | Pending |

Test Suite Expansion:
- Add a boundary test that planner code does not import execution crates.
- Add a boundary test that planner projection creates no `Goal`, `Method`, `Composition`, `Operator`, `Effect`, task, or capability value.
- Add a grounding test fixture that proves emitted propositions use configured dimension ids.
- Add a source scan that covers every `src/planner*.rs` path and rejects family-specific identifiers.

| Exit Criterion | Status |
|----------------|--------|
| First projection fields are named and mechanically evaluable by `meld-lang`. | Pending |
| Deferred `WorldModelView` concepts are explicit and cannot leak into first-slice tasks. | Pending |
| Planner and execution planning ownership is documented in tests or module docs. | Pending |
| Boundary and source-scan tests fail on execution imports or family-specific Rust vocabulary. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Resolves the planner assessment question about confidence plus freshness state. | Pending |
| Resolves the first public route shape enough for code. | Pending |

Verification:
- `cargo test -p meld-world-model planner_boundary`
- `cargo test -p meld-world-model planner_projection_shape`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `design/plan/world_model/planner/PLAN.md`
- `design/plan/world_model/planner/assessment.md`
- `design/cognitive_architecture/world_model/planner/README.md`
- `design/cognitive_architecture/world_model/planner/spec.md`

---

## Phase 1 -- Module Scaffold And Public Boundary

| Field | Value |
|-------|-------|
| Goal | Add a planner projection domain under `meld-world-model` without creating execution planning code. |
| Dependencies | Phase 0 |
| Docs | [World Model Crate](../../../cognitive_architecture/world_model/CRATE.md), [World Model Planner](../../../cognitive_architecture/world_model/planner/README.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add `crates/meld-world-model/src/planner.rs` as the public domain entry. | Pending |
| 2 | Add `crates/meld-world-model/src/planner/contracts.rs` for first-slice projection records. | Pending |
| 3 | Add `crates/meld-world-model/src/planner/projection.rs` for deterministic proposition mapping. | Pending |
| 4 | Add `crates/meld-world-model/src/planner/query.rs` for read-only projection facade. | Pending |
| 5 | Re-export only first-slice planner contracts from `crates/meld-world-model/src/lib.rs`. | Pending |
| 6 | Add a repository structure check that fails if `crates/meld-world-model/src/planner/mod.rs` exists. | Pending |

Test Suite Expansion:
- Add compile smoke tests for public planner re-exports.
- Add module-boundary tests that planner can use `meld-lang` and world-model public domains without importing `meld-execution`.
- Add no `mod.rs` test for planner.
- Add source-layout test covering `planner.rs` plus every file under `src/planner/`.

| Exit Criterion | Status |
|----------------|--------|
| `cargo check -p meld-world-model` succeeds with planner module scaffold. | Pending |
| Planner module has no dependency on execution crates. | Pending |
| No `mod.rs` file is added. | Pending |
| Planner source scan coverage includes all planner domain files. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Establishes a domain-first home for planner-facing world model projection. | Pending |

Verification:
- `cargo check -p meld-world-model`
- `cargo test -p meld-world-model planner_module_boundary`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/planner.rs`
- `crates/meld-world-model/src/planner/contracts.rs`
- `crates/meld-world-model/src/planner/projection.rs`
- `crates/meld-world-model/src/planner/query.rs`

---

## Phase 2 -- First-Slice Projection Contracts

| Field | Value |
|-------|-------|
| Goal | Define the minimal contracts needed to project one belief-backed world state. |
| Dependencies | Phase 1 |
| Docs | [Planner Spec](../../../cognitive_architecture/world_model/planner/spec.md), [Meld Lang World State](../../../cognitive_architecture/meld-lang/world_state.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Define `PlannerProjectionContext` with subject, perspective, branch scope, and projection version. | Pending |
| 2 | Define `PlannerProjectionInput` with subject, optional graph scope state, and current `BeliefView`. | Pending |
| 3 | Define `PlannerProjectionOutput` with `WorldState`, projection version, source refs, and hydration handles. | Pending |
| 4 | Define typed projection source refs for belief revision id, evidence ids, source fact ids, and graph anchor ids. | Pending |
| 5 | Define first-slice dimension constructor functions that derive field ids from projection configuration. | Pending |
| 6 | Validate that output propositions are ground before `WorldState` construction. | Pending |

Test Suite Expansion:
- Add serde round-trip tests for planner first-slice contracts.
- Add projection context validation tests.
- Add source-ref preservation tests from `BeliefView` hydration into projection output.
- Add proptest cases for non-empty dimension ids, projection field suffixes, and invalid field ids.
- Add fuzz target for planner projection contract JSON when contracts are serializable.

| Exit Criterion | Status |
|----------------|--------|
| Contracts are sufficient to build `WorldState` without broad `WorldModelView`. | Pending |
| Contracts expose provenance without raw belief internals. | Pending |
| Contracts carry projection version for replay. | Pending |
| Contract validation rejects empty or malformed projection field ids. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Makes planner projection concrete enough for implementation and tests. | Pending |

Verification:
- `cargo test -p meld-world-model planner_contracts`
- `cargo test -p meld-world-model planner_contract_proptest`
- `cargo fuzz run fuzz_planner_projection_contract`

Key files:
- `crates/meld-world-model/src/planner/contracts.rs`
- `crates/meld-world-model/src/planner.rs`

---

## Phase 3 -- Belief View To Proposition Mapping

| Field | Value |
|-------|-------|
| Goal | Convert current `BeliefView` fields into ground `meld-lang` propositions. |
| Dependencies | Phase 2 |
| Docs | [Belief Assessment](../belief/assessment.md), [Meld Lang World State](../../../cognitive_architecture/meld-lang/world_state.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Map `BeliefView.confidence` to numeric `Holds` using `view.key.dimension_id`. | Pending |
| 2 | Map `BeliefView.freshness.stale` to boolean `Holds` using generic freshness field projection. | Pending |
| 3 | Map open observation opportunity presence to boolean `Holds` using generic observation field projection. | Pending |
| 4 | Treat missing current belief as no confidence proposition so evaluation can become `Indeterminate`. | Pending |
| 5 | Reject or omit invalid belief views through typed projection warnings. | Pending |
| 6 | Preserve belief revision and evidence hydration refs. | Pending |

Test Suite Expansion:
- Add confidence projection tests below, equal to, and above `0.7`.
- Add stale and non-stale projection tests.
- Add observation-needed projection tests.
- Add no-hardcoded-family test for planner projection source.
- Add missing-belief indeterminate handoff test using `meld-lang::evaluate`.
- Add proptest cases for finite confidence values from `0.0` through `1.0`.
- Add proptest cases for arbitrary valid dimension ids to prove family-agnostic projection.
- Add source-ref preservation tests for revision id, evidence ids, source fact ids, and graph anchor ids.

| Exit Criterion | Status |
|----------------|--------|
| A `BeliefView` with confidence below `0.7` produces an unsatisfied goal target when evaluated by `meld-lang`. | Pending |
| A missing belief produces no fabricated support. | Pending |
| Stale and observation-needed state are mechanically visible. | Pending |
| Arbitrary valid dimension ids project without hardcoded family names. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Provides the core world-model-to-language projection for the cognitive flywheel. | Pending |

Verification:
- `cargo test -p meld-world-model planner_belief_projection`
- `cargo test -p meld-world-model planner_indeterminate_projection`
- `cargo test -p meld-world-model planner_projection_proptest`
- `cargo clippy -p meld-world-model -- -D warnings`

Key files:
- `crates/meld-world-model/src/planner/projection.rs`
- `crates/meld-world-model/src/planner/contracts.rs`
- `crates/meld-world-model/tests/planner.rs`

---

## Phase 4 -- Graph Scope Projection

| Field | Value |
|-------|-------|
| Goal | Add graph-derived scope propositions without turning graph state into belief confidence. |
| Dependencies | Phase 2 |
| Docs | [Graph Assessment](../graph/assessment.md), [World Model Graph](../../../cognitive_architecture/world_model/graph/README.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Use public graph query surfaces to confirm the subject is in scope where available. | Pending |
| 2 | Emit `Proposition::Accessible` for the docs subject when graph scope is available. | Pending |
| 3 | Preserve graph anchor and source fact refs as hydration handles. | Pending |
| 4 | Do not emit confidence, freshness, causation, or regime propositions from graph alone. | Pending |
| 5 | Defer `Related` unless the first execution method requires relation preconditions. | Pending |

Test Suite Expansion:
- Add accessible projection test for a subject with current graph anchor.
- Add graph boundary test proving graph-only input cannot emit docs freshness confidence.
- Add source-ref preservation tests for anchor and provenance hydration handles.
- Add temp sled query facade test that seeds graph through `TraversalStore` and reads through public query surfaces.
- Add missing graph scope test proving absence is explicit and does not fabricate `Accessible`.

| Exit Criterion | Status |
|----------------|--------|
| Graph input can make a subject accessible for execution evaluation. | Pending |
| Graph input cannot fabricate belief support. | Pending |
| Graph scope projection is read-only and uses public graph reads. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Supplies scope propositions needed by the existing typed-loop method precondition. | Pending |

Verification:
- `cargo test -p meld-world-model planner_graph_projection`
- `cargo test -p meld-world-model planner_graph_query_facade`

Key files:
- `crates/meld-world-model/src/planner/projection.rs`
- `crates/meld-world-model/src/planner/query.rs`
- `crates/meld-world-model/tests/planner.rs`

---

## Phase 5 -- WorldState Assembly And Grounding Checks

| Field | Value |
|-------|-------|
| Goal | Assemble projected propositions into a valid ground `WorldState`. |
| Dependencies | Phases 3 and 4 |
| Docs | [Meld Lang Assessment](../../meld-lang/assessment.md), [Typed Loop Integration](../../integration/typed_loop.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Build `WorldState` from projected propositions. | Pending |
| 2 | Surface `GroundingError` as planner projection error. | Pending |
| 3 | Ensure projection never emits `Term::Variable` or `Term::Derived`. | Pending |
| 4 | Deduplicate equivalent propositions deterministically. | Pending |
| 5 | Preserve deterministic proposition order for replay tests. | Pending |

Test Suite Expansion:
- Add grounding failure tests for malformed projection input.
- Add deterministic ordering tests.
- Add duplicate projection tests.
- Add direct `meld-lang::evaluate` tests against projected `WorldState`.
- Add property tests for duplicate and shuffled source refs to prove deterministic output ordering.
- Add finite number validation tests so non-finite confidence cannot enter `WorldState`.

| Exit Criterion | Status |
|----------------|--------|
| Every projected `WorldState` is accepted by `WorldState::new`. | Pending |
| Same input produces equal `WorldState` propositions in stable order. | Pending |
| Evaluation of the docs freshness goal is mechanical and semantic-free. | Pending |
| Malformed projection input fails closed with a typed projection error. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Turns world model projection into the concrete execution-readable contract. | Pending |

Verification:
- `cargo test -p meld-world-model planner_world_state`
- `cargo test -p meld-world-model planner_world_state_proptest`

Key files:
- `crates/meld-world-model/src/planner/projection.rs`
- `crates/meld-world-model/tests/planner.rs`

---

## Phase 6 -- Query Facade And Public Route Shape

| Field | Value |
|-------|-------|
| Goal | Provide one read-only projection facade for the first slice. |
| Dependencies | Phase 5 |
| Docs | [World Model Public Interface](../../../cognitive_architecture/world_model/public_interface.md), [Planner Projection Assessment](assessment.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add a `PlannerQuery` facade over belief and graph stores. | Pending |
| 2 | Add a method that projects current world state for one subject and perspective. | Pending |
| 3 | Keep route behavior read-only and deterministic. | Pending |
| 4 | Return projection warnings for missing belief or unavailable graph scope. | Pending |
| 5 | Preserve source refs and hydration handles in the output envelope. | Pending |

Test Suite Expansion:
- Add query facade test from seeded graph plus belief runtime to projected `WorldState`.
- Add missing belief route test.
- Add read-only behavior test that no belief revisions, graph anchors, goals, or execution records are written.
- Add reopen test that projects once, reopens graph and belief stores, projects again, and asserts equality.
- Add warning preservation tests for missing belief, stale belief, observation-needed belief, and missing graph scope.

| Exit Criterion | Status |
|----------------|--------|
| Caller can request the first projected `WorldState` without importing belief internals. | Pending |
| Projection route does not mutate graph, belief, agent, or execution state. | Pending |
| Reopened lower-layer stores produce the same projection output. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Defines the first public world model planner route shape. | Pending |

Verification:
- `cargo test -p meld-world-model planner_query`
- `cargo test -p meld-world-model planner_query_reopen`

Key files:
- `crates/meld-world-model/src/planner/query.rs`
- `crates/meld-world-model/src/planner/contracts.rs`
- `crates/meld-world-model/tests/planner.rs`

---

## Phase 7 -- Typed-Loop Handoff Tests

| Field | Value |
|-------|-------|
| Goal | Prove the world model can hand execution a ground world state from graph plus belief. |
| Dependencies | Phase 6 |
| Docs | [Typed Loop Integration](../../integration/typed_loop.md), [Execution Planning Assessment](../../execution/planning/assessment.md) |
| Status | Pending |

| Order | Task | Status |
|-------|------|--------|
| 1 | Seed graph state for the docs subject. | Pending |
| 2 | Run belief assessment for the configured first belief family fixture. | Pending |
| 3 | Project planner world state for the subject. | Pending |
| 4 | Evaluate a ground docs freshness goal against the projected `WorldState`. | Pending |
| 5 | Assert confidence below `0.7` yields `Unsatisfied`. | Pending |
| 6 | Assert missing belief yields `Indeterminate` rather than false support. | Pending |
| 7 | Assert no execution planning types are required to produce the `WorldState`. | Pending |

Test Suite Expansion:
- Add end-to-end planner projection test in `meld-world-model`.
- Add cross-crate typed-loop handoff test if needed by workspace test layout.
- Add boundary tests for no goal creation and no method selection.
- Add source scan test for no hardcoded runtime family id in planner core.
- Add workspace-level test command to keep planner projection compatible with existing graph, belief, and meld-lang tests.
- Add a targeted `cargo-mutants` pass over the planner module and planner tests.

| Exit Criterion | Status |
|----------------|--------|
| Graph plus belief produces a ground `WorldState`. | Pending |
| Execution can evaluate the freshness goal mechanically. | Pending |
| The next agent curation phase can consume the projected world state. | Pending |
| Targeted mutation testing for planner projection is clean or every survivor is documented as equivalent. | Pending |
| Full crate and workspace verification pass. | Pending |

| Dependency Closure Solved | Status |
|---------------------------|--------|
| Completes the world-model planner segment of the cognitive flywheel. | Pending |

Verification:
- `cargo test -p meld-world-model planner_typed_loop_handoff`
- `cargo test -p meld-lang evaluation_loop`
- `cargo test -p meld-world-model`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo mutants --package meld-world-model --file crates/meld-world-model/src/planner.rs --file crates/meld-world-model/src/planner --test-tool cargo --timeout 120`

Key files:
- `crates/meld-world-model/tests/planner.rs`
- `crates/meld-lang/tests/evaluation_loop.rs`

## Explicit Non-Goals

Do not implement in this slice:

- broad `WorldModelView` runtime
- `ActionableBeliefView`
- `RiskEnvelope`
- `AbstentionState`
- `CausalEffectSummary`
- `SensitivitySummary`
- `AssumptionSet`
- execution planning input ports
- agent goal curation
- goal set storage
- method library loading
- method matching
- composition substitution
- composition validation
- task network mutation
- task dispatch
- observation dispatch
- provider selection
- outcome publication

## First Slice Completion Checklist

- Planner code lives under `crates/meld-world-model/src/planner.rs` and `crates/meld-world-model/src/planner/*.rs`.
- No `mod.rs` is introduced.
- Projection consumes public graph and belief reads only.
- Projection creates no goals, methods, compositions, tasks, capabilities, or effects.
- Confidence, stale state, and observation-needed state are projected as ground propositions using configured dimension ids and generic field projection.
- Planner core contains no hardcoded runtime belief family id.
- `WorldState::new` accepts every projected world state.
- `meld-lang::evaluate` can evaluate the first docs freshness goal against the projected world state.
- Targeted `cargo-mutants` run over planner projection has no unexplained survivors.
- Missing belief state becomes typed absence or indeterminate evaluation, not fabricated support.
- Source refs and hydration handles preserve graph and belief provenance.
- The agent curation phase can proceed without importing belief internals.
