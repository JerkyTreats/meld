# Meld Lang Construction Contract

Date: 2026-05-20
Status: reference contract
Scope: declarative construction sequence for the shared proposition language between world model and execution

## Overview

Required construction:

- create `crates/meld-lang` as a new pure crate
- implement the three primitives: Term, Proposition, Effect
- implement the structural types: Operator, Composition, Goal, Method
- implement the core operations: evaluate(), unify(), substitute(), validate(), apply()
- implement WorldState as ground proposition set with three-valued evaluation
- prove the full evaluation loop in an integration test: construct goal, evaluate against world state, match method, substitute bindings, validate composition, apply effects, re-evaluate showing satisfaction

Required result:

- `meld-lang` exists as a pure crate with no IO, no async, no side effects
- both `meld-world-model` and `meld-execution` can depend on `meld-lang` for all planning types
- the shared language resolves Gaps 1 (goal model), 2 (planning pipeline substrate), and 3 (world model read interface) from `execution/GAPS.md`
- method libraries can be loaded from serialized JSON files at runtime

## Related Specs

- [Lang Domain](README.md)
- [Lang Crate](CRATE.md)
- [Lang Requirements](requirements.md)
- [Lang Primitives](primitives.md)
- [Operators and Resolution](operators.md)
- [Compositions](compositions.md)
- [Goals and Methods](goals_and_methods.md)
- [World State and Evaluation](world_state.md)
- [Execution Domain](../execution/README.md)
- [Execution Gaps](../execution/GAPS.md)
- [Planning Pipeline](../execution/planning/planning_pipeline.md)
- [World Model Planner](../world_model/planner/README.md)
- [World Model Agent](../world_model/agent/README.md)

## Guiding Rules

- all public types derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`
- all public functions are pure: deterministic, no side effects, no mutation of inputs
- no `tokio`, `async-trait`, `std::fs`, `std::net`, or any IO-capable dependency
- tests are pure value-in value-out assertions with no setup, no mocks, no fixtures
- `WorldState` and `Bindings` are immutable — operations return new values
- error types are structural (validation failures, grounding errors, unbound variables), not IO errors
- the same `Proposition` type is used across all roles: world state assertion, goal target, method trigger, operator precondition, effect argument, and sub-goal

## Development Phases

| Phase | Goal | Dependencies |
|-------|------|--------------|
| 0 | Crate scaffold and baseline | None |
| 1 | Term and Literal primitives | Phase 0 |
| 2 | Proposition and Condition grammar | Phase 1 |
| 3 | Effect, WorldState, and evaluation | Phase 2 |
| 4 | Operator, Resolution, and CostEstimate | Phase 2 and Phase 3 |
| 5 | Composition graph and structural validation | Phase 4 |
| 6 | Goal specification | Phase 2 and Phase 4 |
| 7 | Method, unification, and substitution | Phase 5 and Phase 6 |
| 8 | Full evaluation loop integration | Phase 0 through Phase 7 |

---

### Phase 0 - Crate scaffold and baseline

**Goal**: create the `meld-lang` crate with correct workspace integration, dependency on `meld-events`, and module structure.

**Source docs**:
- [Lang Crate](CRATE.md)
- [Lang Requirements](requirements.md)

| Task | Contract |
|------|----------|
| Create `crates/meld-lang/Cargo.toml` with `serde`, `serde_json`, and `meld-events` dependencies. | Required |
| Create `crates/meld-lang/src/lib.rs` with module declarations. | Required |
| Add `meld-lang` to workspace `Cargo.toml`. | Required |
| Create empty module files for `term`, `proposition`, `effect`, `operator`, `composition`, `goal`, `method`, `world_state`, `evaluate`, `unify`, `substitute`, `validate`, `cost`. | Required |
| Verify the crate compiles and `meld-events` dependency resolves. | Required |

**Exit criteria**:
- `meld-lang` exists in the workspace and compiles
- `DomainObjectRef` is importable from `meld-events`
- no IO, async, or forbidden dependencies in `Cargo.toml`

**Key files and seams**:
- new `crates/meld-lang/Cargo.toml`
- new `crates/meld-lang/src/lib.rs`
- `Cargo.toml` (workspace)

**Verification**:
- `cargo check -p meld-lang`
- `cargo test -p meld-lang`

---

### Phase 1 - Term and Literal primitives

**Goal**: implement the atomic reference type and literal value type. These are the foundation that all other types compose.

**Source docs**:
- [Lang Primitives](primitives.md)

| Task | Contract |
|------|----------|
| Implement `Literal` enum: `Bool(bool)`, `Text(String)`, `Number(f64)`, `Duration(std::time::Duration)`. | Required |
| Implement `Term` enum: `Object(DomainObjectRef)`, `Dimension(String)`, `ArtifactType(String)`, `Literal(Literal)`, `Variable(String)`, `Derived { source_step, field_path }`. | Required |
| Derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` on all types. | Required |
| Add unit tests for `Term` and `Literal` serialization round-trip. | Required |
| Add unit tests for `Term` equality across all variants. | Required |

**Exit criteria**:
- `Term` and `Literal` compile with all variants
- serialization round-trip through `serde_json` preserves all term variants exactly
- `PartialEq` correctly distinguishes all variants

**Key files and seams**:
- new `crates/meld-lang/src/term.rs`
- `crates/meld-lang/src/lib.rs`

**Verification**:
- `cargo test -p meld-lang term`
- `cargo fmt -p meld-lang -- --check`

**Comment gate**:
- all public types and variants have Rustdoc comments

---

### Phase 2 - Proposition and Condition grammar

**Goal**: implement the typed statement primitive and condition comparison types. These define the grammar of what the language can express about the world.

**Source docs**:
- [Lang Primitives](primitives.md)
- [Lang Requirements](requirements.md)

| Task | Contract |
|------|----------|
| Implement `Condition` enum: `Above(Term)`, `Below(Term)`, `Equals(Term)`, `In(Vec<Term>)`, `Within(Term)`, `Exceeds(Term)`, `Present`, `Absent`. | Required |
| Implement `Proposition` enum: `Holds { subject, dimension, condition }`, `Exists { scope, artifact_type }`, `Accessible { scope }`, `Related { src, relation, dst }`, `All(Vec<Proposition>)`, `Any(Vec<Proposition>)`, `Not(Box<Proposition>)`. | Required |
| Derive required traits on all types. | Required |
| Add helper method `Proposition::is_ground()` to check whether all terms are concrete with no variable or derived terms. | Required |
| Add unit tests for proposition construction, `All`, `Any`, `Not`, and ground checking. | Required |
| Add unit tests for serialization round-trip of all proposition and condition variants including nested structures. | Required |

**Exit criteria**:
- all proposition shapes constructable from terms
- `is_ground()` correctly identifies ground vs pattern propositions
- nested `All`/`Any`/`Not` serialize and deserialize correctly
- same `Proposition` type used in all positions without wrapper types

**Key files and seams**:
- new `crates/meld-lang/src/proposition.rs`
- new `crates/meld-lang/src/condition.rs`
- `crates/meld-lang/src/lib.rs`

**Verification**:
- `cargo test -p meld-lang proposition`
- `cargo test -p meld-lang condition`
- `cargo fmt -p meld-lang -- --check`

**Comment gate**:
- each proposition variant has Rustdoc explaining what it expresses
- each condition variant has Rustdoc explaining what comparison it performs

---

### Phase 3 - Effect, WorldState, and evaluation

**Goal**: implement state change primitives, the ground proposition set, and three-valued evaluation. This is the first testable loop: construct propositions, build a world state, evaluate a proposition against it.

**Source docs**:
- [Lang Primitives](primitives.md)
- [World State and Evaluation](world_state.md)

| Task | Contract |
|------|----------|
| Implement `Effect` enum: `Assert(Proposition)`, `Retract(Proposition)`, `Update { subject, dimension, value }`. | Required |
| Implement `WorldState` with `new()` for ground-only validation plus `empty()`, `satisfies()`, `propositions()`, and `query()`. | Required |
| Implement `EvalResult` enum: `Satisfied`, `Unsatisfied { gap }`, `Indeterminate { missing }`. | Required |
| Implement `evaluate()`: pure function from `&WorldState` and `&Proposition` to `EvalResult`. | Required |
| Implement `WorldState::gap()` convenience over `evaluate()`. | Required |
| Implement `WorldState::apply()`: apply effects to produce new world state. | Required |
| Implement `GroundingError` for world state construction with non-ground propositions. | Required |
| Add evaluation rule tests for each `Proposition` variant per the spec. | Required |
| Add three-valued logic tests for satisfied, unsatisfied with the correct gap, and indeterminate with a missing dimension. | Required |
| Add effect application tests for idempotent assert, remove or no-op retract, and update through retract plus assert. | Required |
| Add test: evaluate → Unsatisfied, apply effects, evaluate → Satisfied. | Required |

**Exit criteria**:
- `evaluate()` produces correct three-valued result for all proposition variants
- `Indeterminate` is distinct from `Unsatisfied` (missing dimension vs wrong value)
- effect application is deterministic and sequential
- `WorldState::new()` rejects propositions containing Variable or Derived terms
- `WorldState.apply()` returns new state without mutating input

**Key files and seams**:
- new `crates/meld-lang/src/effect.rs`
- new `crates/meld-lang/src/world_state.rs`
- new `crates/meld-lang/src/evaluate.rs`

**Verification**:
- `cargo test -p meld-lang effect`
- `cargo test -p meld-lang world_state`
- `cargo test -p meld-lang evaluate`
- `cargo fmt -p meld-lang -- --check`

**Comment gate**:
- `evaluate()` has Rustdoc explaining the three-valued semantics
- each evaluation rule has a brief inline comment linking to the design spec

---

### Phase 4 - Operator, Resolution, and CostEstimate

**Goal**: implement the runtime-constructed operator contract, capability resolution query, and multi-dimensional cost algebra.

**Source docs**:
- [Operators and Resolution](operators.md)

| Task | Contract |
|------|----------|
| Implement `CostEstimate` with `add()`, `exceeds()`, `zero()`. | Required |
| Implement `SlotConstraint` with `artifact_type_id` and `required` flag. | Required |
| Implement `CapabilityRef` with `capability_type_id` and `capability_version`. | Required |
| Implement `Resolution` with `requires_inputs`, `requires_outputs`, `scope_kind`, `tags`, `specific`. | Required |
| Implement `Operator` with `operator_id`, `preconditions`, `effects`, `cost`, `resolution`. | Required |
| Derive required traits on all types. | Required |
| Add cost algebra tests: addition, ceiling comparison, zero identity. | Required |
| Add operator construction tests: build operator from terms, verify serialization round-trip. | Required |

**Exit criteria**:
- operators constructable at runtime from terms and propositions without compile-time enum variants
- cost algebra is correct for all three dimensions independently
- `exceeds()` returns true if ANY dimension exceeds the ceiling
- all types serialize and deserialize cleanly

**Key files and seams**:
- new `crates/meld-lang/src/operator.rs`
- new `crates/meld-lang/src/cost.rs`

**Verification**:
- `cargo test -p meld-lang operator`
- `cargo test -p meld-lang cost`
- `cargo fmt -p meld-lang -- --check`

---

### Phase 5 - Composition graph and structural validation

**Goal**: implement the composition graph (steps + edges) and the structural validation function. This is the plan representation in the language.

**Source docs**:
- [Compositions](compositions.md)

| Task | Contract |
|------|----------|
| Implement `Step` with `step_id` and `StepKind` variants for operation and goal steps. | Required |
| Implement `Edge` with `from`, `to`, and `EdgeKind` variants for ordering, data flow, and conditional edges. | Required |
| Implement `Composition` with `steps` and `edges`. | Required |
| Implement `ValidationResult`, `ValidationError`, `ValidationWarning` per spec. | Required |
| Implement `validate()`: check dangling edges, duplicate step IDs, cycles, artifact source mismatch, invalid guard on goal step, unbound variables. | Required |
| Implement `CostEstimate::aggregate()`: sum operator costs across a composition. | Required |
| Add validation tests: valid composition passes, each error kind triggers on the correct input. | Required |
| Add warning tests: disconnected steps, unused effects. | Required |
| Add cycle detection tests: simple cycle, transitive cycle, DAG passes. | Required |
| Add composition construction tests: single-step, linear chain, parallel branches, conditional edges. | Required |

**Exit criteria**:
- compositions are constructable as data from operators and propositions
- `validate()` catches all structural errors without access to capability catalog
- `validate()` does NOT check resolution or goal achievement (those are consumer concerns)
- cycle detection is correct for complex graphs

**Key files and seams**:
- new `crates/meld-lang/src/composition.rs`
- new `crates/meld-lang/src/validate.rs`

**Verification**:
- `cargo test -p meld-lang composition`
- `cargo test -p meld-lang validate`
- `cargo fmt -p meld-lang -- --check`

**Comment gate**:
- `validate()` has Rustdoc explaining what it checks and what it explicitly does NOT check
- each validation error variant documents what input triggers it

---

### Phase 6 - Goal specification

**Goal**: implement the Goal type with its operational metadata. Goals are propositions with lifecycle, priority, source, and agent identity.

**Source docs**:
- [Goals and Methods](goals_and_methods.md)
- [Goals](../execution/goals/README.md)

| Task | Contract |
|------|----------|
| Implement `GoalPriority` with `urgency` and optional `cost_ceiling`. | Required |
| Implement `GoalSource` enum: `BeliefDivergence`, `UserDirected`, `Maintenance`, `Decomposed`. | Required |
| Implement `GoalLifecycle` enum: `Proposed`, `Active`, `Suspended`, `Satisfied`, `Abandoned`. | Required |
| Implement `Goal` struct with `goal_id`, `agent_id`, `target`, `priority`, `source`, `lifecycle`. | Required |
| Add goal construction tests: build goal from proposition target, verify round-trip. | Required |
| Add test: goal target is a ground Proposition, goal lifecycle transitions. | Required |

**Exit criteria**:
- goals constructable at runtime by composing terms into a proposition target
- goal target uses the same `Proposition` type as world state and operator preconditions
- all goal types serialize and deserialize correctly
- no compile-time enum for specific goal kinds

**Key files and seams**:
- new `crates/meld-lang/src/goal.rs`

**Verification**:
- `cargo test -p meld-lang goal`
- `cargo fmt -p meld-lang -- --check`

---

### Phase 7 - Method, unification, and substitution

**Goal**: implement methods (cached compositions with trigger patterns), pattern unification, binding management, and composition substitution. This completes the planning loop's ability to match goals to decompositions.

**Source docs**:
- [Goals and Methods](goals_and_methods.md)

| Task | Contract |
|------|----------|
| Implement `Bindings` with `empty()`, `bind()`, `get()`, `merge()`, `iter()`. Immutable. | Required |
| Implement `unify()`: pure function from pattern `Proposition` and concrete `Proposition` to `Option<Bindings>`. | Required |
| Implement `SubstitutionError` and `UnboundVariable`. | Required |
| Implement `substitute()`: pure function from `Composition` and `Bindings` to `Result<Composition, SubstitutionError>`. | Required |
| Implement `Method` struct with `method_id`, `trigger`, `preconditions`, `composition`, `net_effects`, `cost`, `preference`. | Required |
| Add unification tests: successful bind, shape mismatch fails, double-bind conflict fails, structural unification of All/Any/Not. | Required |
| Add bindings tests: immutable, merge success, merge conflict returns None. | Required |
| Add substitution tests: all variables replaced, unbound variable produces error, nested operator and sub-goal substitution. | Required |
| Add method matching flow test: unify trigger → check preconditions → verify net effects → substitute composition. | Required |
| Add a method deserialization test that proves runtime loadability from JSON. | Required |

**Exit criteria**:
- `unify()` correctly binds variables from pattern to concrete proposition
- `unify()` fails cleanly on shape mismatch or double-bind conflict
- `substitute()` replaces all variables in a composition's operators and sub-goals
- `substitute()` fails with explicit error listing unbound variables
- methods are serializable and deserializable from JSON
- `Bindings` is immutable — `bind()` and `merge()` return new values

**Key files and seams**:
- new `crates/meld-lang/src/method.rs`
- new `crates/meld-lang/src/unify.rs`
- new `crates/meld-lang/src/substitute.rs`

**Verification**:
- `cargo test -p meld-lang method`
- `cargo test -p meld-lang unify`
- `cargo test -p meld-lang substitute`
- `cargo fmt -p meld-lang -- --check`

**Comment gate**:
- `unify()` has Rustdoc explaining the unification rules and asymmetry (pattern vs concrete)
- `substitute()` has Rustdoc explaining what positions are walked

---

### Phase 8 - Full evaluation loop integration

**Goal**: prove the complete evaluation loop in a single integration test. This is the first-slice requirement from `requirements.md`: construct goal, evaluate against world state, match method, substitute bindings, validate composition, apply effects, re-evaluate showing satisfaction.

**Source docs**:
- [Lang Requirements](requirements.md) — first slice requirements
- [World State and Evaluation](world_state.md) — consumption example

| Task | Contract |
|------|----------|
| Write an integration test for the full docs freshness loop from goal construction through satisfied evaluation. | Required |
| Write an integration test for an indeterminate goal that becomes evaluable after an observation effect. | Required |
| Write integration test: method loaded from JSON string, deserialized, used in matching flow. | Required |
| Write integration test: multi-step composition with conditional edge, evaluate guard. | Required |
| Write integration test: recursive decomposition — composition with Goal steps, verify they validate. | Required |
| Add `meld-lang` as a dependency to the `meld-execution` Cargo manifest through dependency wiring only. | Required |
| Add `meld-lang` as a dependency to the `meld-world-model` Cargo manifest through dependency wiring only. | Required |
| Verify full crate compiles cleanly with workspace. | Required |

**Exit criteria**:
- complete evaluation loop proven end-to-end in a single test function with no external dependencies
- the three-valued evaluation distinction (Satisfied/Unsatisfied/Indeterminate) produces correct planning decisions
- method loading from serialized form works identically to programmatic construction
- `meld-execution` and `meld-world-model` can declare dependency on `meld-lang`
- all public types are re-exported from `lib.rs` for clean consumer imports

**Key files and seams**:
- new `crates/meld-lang/tests/evaluation_loop.rs`
- `crates/meld-execution/Cargo.toml`
- `crates/meld-world-model/Cargo.toml`

**Verification**:
- `cargo test -p meld-lang`
- `cargo test -p meld-lang --test evaluation_loop`
- `cargo check -p meld-execution`
- `cargo check -p meld-world-model`
- `cargo fmt -p meld-lang -- --check`
- `cargo clippy -p meld-lang -- -D warnings`

---

## Cross-Phase Gates

### Purity Gates

- `meld-lang` Cargo.toml has no `tokio`, `async-trait`, `std::fs`, `std::net`, or IO-capable dependency
- no `pub fn` in the crate uses `&mut self`, `async`, or returns IO errors
- all tests run with `#[test]` — no `#[tokio::test]`, no fixtures, no mocks

### Type Gates

- all public types derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`
- `Proposition` is the same type in all roles (world state, goal target, method trigger, precondition, effect argument, sub-goal)
- `Term` is the same type in all roles (world state reference, pattern variable, derived output reference)
- no compile-time enum variant represents a specific goal, method, operator, or action family

### Determinism Gates

- same world state + same proposition → same `EvalResult`
- same pattern + same concrete → same `Option<Bindings>`
- same composition + same bindings → same `Result<Composition, SubstitutionError>`
- same effects + same world state → same new `WorldState`

### Immutability Gates

- `WorldState::apply()` returns new `WorldState`, input unchanged
- `Bindings::bind()` returns new `Bindings`, input unchanged
- `Bindings::merge()` returns new `Bindings`, inputs unchanged

### Serialization Gates

- all types round-trip through `serde_json` without loss
- method loaded from JSON produces same `Method` as programmatic construction
- serialization format is stable across test runs (deterministic field ordering)

### Dependency Gates

- `meld-lang` depends only on `meld-events`, `serde`, `serde_json`, and std
- `meld-lang` does NOT depend on `meld-world-model`, `meld-execution`, or root `meld`
- `meld-execution` depends on `meld-lang` (declared, compiles)
- `meld-world-model` depends on `meld-lang` (declared, compiles)

## Dependency Order Summary

1. scaffold the crate in the workspace
2. build term primitives (the atoms everything references)
3. build proposition and condition grammar (the statement shapes)
4. build effects, world state, and evaluation (the first testable loop)
5. build operators, resolution, and cost algebra (the contract boundary)
6. build compositions and validation (the plan representation)
7. build goals (the execution input)
8. build methods, unification, and substitution (the planning optimization)
9. prove the full evaluation loop end-to-end

## Risk Notes

### DomainObjectRef dependency

`meld-lang` depends on `meld-events` for `DomainObjectRef`. If `meld-events` has heavy transitive dependencies, this could pull unwanted weight into the pure crate. Mitigation: verify `meld-events` exports `DomainObjectRef` without requiring the full event runtime. If needed, extract `DomainObjectRef` into a shared identity crate.

### f64 in PartialEq

`Literal::Number(f64)` uses `f64`, which does not implement `Eq` and has NaN comparison issues. `PartialEq` derivation works but `NaN != NaN` may cause surprising behavior in world state matching. Mitigation: document that `Number` should not contain NaN. Consider a wrapper type (ordered float) if this causes test failures.

### Serialization stability

Method files serialized in one version must deserialize in the next. Adding new enum variants to `Proposition`, `Condition`, `Effect`, or `EdgeKind` is a grammar change that may break existing method files. Mitigation: add a serialization compatibility test in Phase 8 that pins a known JSON document and verifies it deserializes correctly.

## Read With

- [Lang Domain](README.md)
- [Lang Crate](CRATE.md)
- [Lang Requirements](requirements.md)
- [Execution Domain](../execution/README.md)
- [Execution Gaps](../execution/GAPS.md)
- [Planning Pipeline](../execution/planning/planning_pipeline.md)
- [World Model Planner](../world_model/planner/README.md)
- [World Model Agent](../world_model/agent/README.md)
