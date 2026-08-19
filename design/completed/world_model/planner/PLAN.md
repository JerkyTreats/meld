# World Model Planner Phased Implementation Plan

Date: 2026-05-27
Status: complete
Scope: first planner projection slice from `BeliefView` to `meld-lang::WorldState`

## Overview

This plan records the implemented Phase 4 planner projection slice and its verification bar.

The first slice proves the next cognitive flywheel handoff after belief. It takes public world model state from graph and belief, then emits a ground `meld-lang::WorldState` that execution can evaluate mechanically.

The word planner here means planner-facing world model projection. It does not mean execution planning.

The implemented order was dependency driven:

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

The first planner projection slice is implemented.

Implemented:

- `crates/meld-world-model/src/planner.rs`
- `crates/meld-world-model/src/planner/contracts.rs`
- `crates/meld-world-model/src/planner/projection.rs`
- `crates/meld-world-model/src/planner/query.rs`
- `PlannerProjectionContext`, `PlannerProjectionInput`, `PlannerProjectionOutput`, source refs, hydration refs, warnings, and typed errors
- `project_world_state` for pure projection into ground `meld-lang::WorldState`
- `PlannerQuery::project_current_world_state` over `BeliefQuery` and `TraversalQuery`
- confidence, stale state, observation-needed state, and graph accessibility propositions
- deterministic proposition, source ref, hydration ref, and warning ordering
- source ref and hydration handle preservation
- missing belief behavior that allows `meld-lang::evaluate` to return `Indeterminate`
- planner test harness, property tests, source scans, reopen tests, and fuzz target

Available lower-layer inputs:

- `meld-lang` value types and pure operations
- Graph subject, anchor, provenance, and traversal query surfaces
- Belief current views, subject views, revision history, evidence hydration, provenance, observation opportunities, and dirty-key query surfaces
- First externally configured belief slice with confidence, uncertainty, freshness, observation state, and provenance
- Existing `meld-world-model` test harness using temp sled stores, read facades, serde round trips, proptest, fuzz targets, source scans, and replay assertions

Verification evidence:

- `cargo check -p meld-world-model`
- `cargo test -p meld-world-model`
- `cargo test --workspace`
- `cargo clippy -p meld-world-model -- -D warnings`
- `cargo clippy --workspace -- -D warnings`
- `cargo +nightly fuzz run fuzz_planner_projection_contract -- -runs=1`
- `cargo check --manifest-path crates/meld-world-model/fuzz/Cargo.toml --bin fuzz_planner_projection_contract`

Mutation testing note:

- `cargo mutants --package meld-world-model --file crates/meld-world-model/src/planner.rs --file crates/meld-world-model/src/planner --test-tool cargo --timeout 120` reported zero mutants under the active filters.
- A crate-root retry with `--file src/planner.rs --file src/planner` also reported zero mutants.

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
| 0 | Scope lock and boundary audit | Belief, graph, and meld-lang assessments | Complete |
| 1 | Module scaffold and public boundary | Phase 0 | Complete |
| 2 | First-slice projection contracts | Phase 1 | Complete |
| 3 | Belief view to proposition mapping | Phase 2 | Complete |
| 4 | Graph scope projection | Phase 2 | Complete |
| 5 | WorldState assembly and grounding checks | Phases 3 and 4 | Complete |
| 6 | Query facade and public route shape | Phase 5 | Complete |
| 7 | Typed-loop handoff tests | Phase 6 | Complete |

## Completion Evidence

Completed implementation files:

- `crates/meld-world-model/src/planner.rs`
- `crates/meld-world-model/src/planner/contracts.rs`
- `crates/meld-world-model/src/planner/projection.rs`
- `crates/meld-world-model/src/planner/query.rs`
- `crates/meld-world-model/tests/planner.rs`
- `crates/meld-world-model/fuzz/fuzz_targets/fuzz_planner_projection_contract.rs`

Completed verification families:

- planner module boundary and no `mod.rs`
- execution boundary source scan
- family-specific Rust identifier source scan
- contract serde round trips
- confidence, stale, observation-needed, and accessible projection shape
- missing belief indeterminate handoff
- graph-only no fabricated belief support
- grounding through `WorldState::new`
- deterministic ordering for propositions, source refs, hydration refs, and warnings
- query facade through temp sled graph and belief stores
- reopen parity for graph and belief stores
- typed-loop handoff through `meld-lang::evaluate`
- property coverage for confidence, dimension ids, stale booleans, observation booleans, duplicate refs, and invalid field suffixes
- fuzz target registration and bounded nightly smoke run

Completed verification commands:

- `cargo check -p meld-world-model`
- `cargo test -p meld-world-model planner_module_boundary`
- `cargo test -p meld-world-model planner_contracts`
- `cargo test -p meld-world-model planner_belief_projection`
- `cargo test -p meld-world-model planner_graph_projection`
- `cargo test -p meld-world-model planner_world_state`
- `cargo test -p meld-world-model planner_query`
- `cargo test -p meld-world-model planner_typed_loop_handoff`
- `cargo test -p meld-world-model`
- `cargo test --workspace`
- `cargo clippy -p meld-world-model -- -D warnings`
- `cargo clippy --workspace -- -D warnings`
- `cargo +nightly fuzz run fuzz_planner_projection_contract -- -runs=1`
- `cargo check --manifest-path crates/meld-world-model/fuzz/Cargo.toml --bin fuzz_planner_projection_contract`

Mutation testing evidence:

- `cargo mutants --package meld-world-model --file crates/meld-world-model/src/planner.rs --file crates/meld-world-model/src/planner --test-tool cargo --timeout 120` reported zero mutants under the active filters.
- `cargo mutants --file src/planner.rs --file src/planner --test-tool cargo --timeout 120` from the crate root also reported zero mutants under the active filters.

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
