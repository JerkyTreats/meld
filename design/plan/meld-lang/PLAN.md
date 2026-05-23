# Meld Lang Phased Implementation Plan

Date: 2026-05-22
Status: active

## Overview

This plan converts the `meld-lang` spec into one durable implementation order.

The order is dependency driven, not module driven.

- First scaffold the crate and establish workspace integration
- Then build term and literal atoms that all other types reference
- Then build proposition and condition grammar over those atoms
- Then build effects, world state, and three-valued evaluation to prove the first testable loop
- Then build operator, resolution, and cost algebra for the capability contract boundary
- Then build composition graph and structural validation for the plan representation
- Then build goal specification for the execution input contract
- Then build method, unification, and substitution for the planning optimization path
- Then prove the full evaluation loop end-to-end in integration tests

Related specs:
- [Lang Domain](../../cognitive_architecture/meld-lang/README.md)
- [Lang Crate](../../cognitive_architecture/meld-lang/CRATE.md)
- [Lang Requirements](../../cognitive_architecture/meld-lang/requirements.md)
- [Lang Primitives](../../cognitive_architecture/meld-lang/primitives.md)
- [Operators and Resolution](../../cognitive_architecture/meld-lang/operators.md)
- [Compositions](../../cognitive_architecture/meld-lang/compositions.md)
- [Goals and Methods](../../cognitive_architecture/meld-lang/goals_and_methods.md)
- [World State and Evaluation](../../cognitive_architecture/meld-lang/world_state.md)

Related plan docs:
- [Meld Lang Assessment](assessment.md)
- [Typed Loop Integration](../integration/typed_loop.md)
- [Execution Gaps](../../cognitive_architecture/execution/GAPS.md)

---

## Guiding rules

| Rule | Statement |
|------|-----------|
| Purity | No IO, no async, no `std::fs`, no `std::net`, no `tokio`, no `async-trait`. Every public function takes values and returns values. |
| Derivation | All public types derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`. |
| Immutability | `WorldState` and `Bindings` are immutable. Operations return new values. |
| Single type | `Proposition` is the same type in all roles: world state assertion, goal target, method trigger, precondition, effect argument, sub-goal. |
| Runtime vocabulary | No compile-time enum variant represents a specific goal, method, operator, or action family. Grammar is finite. Vocabulary is open. |
| Error shape | Error types are structural only: validation failures, grounding errors, unbound variables. No IO errors. |
| Test shape | Tests are pure value-in value-out assertions. No mocks, no fixtures, no setup infrastructure beyond `#[test]`. |
| Comment gate | Only add Rustdoc where the WHY is non-obvious. No narrating what the code does. |

---

## Development phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 0 | Crate scaffold and baseline | None | Proposed |
| 1 | Term and Literal primitives | Phase 0 | Proposed |
| 2 | Proposition and Condition grammar | Phase 1 | Proposed |
| 3 | Effect, WorldState, and three-valued evaluation | Phase 2 | Proposed |
| 4 | Operator, Resolution, and CostEstimate | Phase 2, Phase 3 | Proposed |
| 5 | Composition graph and structural validation | Phase 4 | Proposed |
| 6 | Goal specification | Phase 2, Phase 4 | Proposed |
| 7 | Method, unification, and substitution | Phase 5, Phase 6 | Proposed |
| 8 | Full evaluation loop integration | Phase 0 through Phase 7 | Proposed |

---

### Phase 0 -- Crate scaffold and baseline

| Field | Value |
|-------|-------|
| Goal | Create `meld-lang` crate with correct workspace integration, `meld-events` dependency, and empty module structure. |
| Dependencies | None |
| Docs | [Lang Crate](../../cognitive_architecture/meld-lang/CRATE.md), [Lang Requirements](../../cognitive_architecture/meld-lang/requirements.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Create `crates/meld-lang/Cargo.toml` with `serde`, `serde_json`, and `meld-events` as dependencies. Verify no IO-capable dependency. | Proposed |
| 2 | Create `crates/meld-lang/src/lib.rs` with module declarations for `term`, `proposition`, `condition`, `effect`, `world_state`, `evaluate`, `operator`, `cost`, `composition`, `validate`, `goal`, `method`, `unify`, `substitute`. | Proposed |
| 3 | Create empty module files for each declared module. | Proposed |
| 4 | Add `meld-lang` to root workspace `Cargo.toml` members list. | Proposed |
| 5 | Verify `DomainObjectRef` is importable from `meld-events` within `meld-lang`. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| `cargo check -p meld-lang` succeeds. | Proposed |
| `cargo test -p meld-lang` succeeds with zero tests. | Proposed |
| `cargo clippy -p meld-lang -- -D warnings` succeeds. | Proposed |
| `cargo deny check advisories licenses sources` succeeds. | Proposed |
| `cargo audit` reports no vulnerabilities. | Proposed |
| `Cargo.toml` contains no `tokio`, `async-trait`, `std::fs`, `std::net`, or IO-capable dependency. | Proposed |
| `DomainObjectRef` resolves from `meld-events`. | Proposed |
| CI pipeline updated with clippy, cargo-deny, and cargo-audit steps. | Proposed |
| `deny.toml` exists at workspace root with configured allowlists. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Unblocks all subsequent phases by establishing the crate in the workspace. | Proposed |

---

### Phase 1 -- Term and Literal primitives

| Field | Value |
|-------|-------|
| Goal | Implement `Term` and `Literal` -- the atomic reference types all other types compose from. |
| Dependencies | Phase 0 |
| Docs | [Lang Primitives](../../cognitive_architecture/meld-lang/primitives.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `Literal` enum with variants: `Bool(bool)`, `Text(String)`, `Number(f64)`, `Duration(std::time::Duration)`. Derive required traits. | Proposed |
| 2 | Implement `Term` enum with variants: `Object(DomainObjectRef)`, `Dimension(String)`, `ArtifactType(String)`, `Literal(Literal)`, `Variable(String)`, `Derived { source_step: String, field_path: String }`. Derive required traits. | Proposed |
| 3 | Add unit tests for `Literal` serialization round-trip through `serde_json` for all variants. | Proposed |
| 4 | Add unit tests for `Term` serialization round-trip for all variants including `Object` with `DomainObjectRef`. | Proposed |
| 5 | Add unit tests for `Term` and `Literal` `PartialEq` across all variant combinations. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| All `Term` and `Literal` variants compile with required trait derivations. | Proposed |
| Serialization round-trip through `serde_json` preserves all variants exactly. | Proposed |
| `PartialEq` correctly distinguishes all variant combinations. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Provides the atomic reference type consumed by Proposition, Condition, Effect, Operator, Goal, and Method. | Proposed |

Verification:
- `cargo test -p meld-lang term`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject` (snapshot tests for all Term and Literal variants)
- `cargo llvm-cov -p meld-lang` (baseline coverage established)

Key files:
- new `crates/meld-lang/src/term.rs`
- new `crates/meld-lang/tests/snapshots/` (insta snapshot directory)

---

### Phase 2 -- Proposition and Condition grammar

| Field | Value |
|-------|-------|
| Goal | Implement `Proposition` and `Condition` -- the typed statement shapes the evaluator understands. |
| Dependencies | Phase 1 |
| Docs | [Lang Primitives](../../cognitive_architecture/meld-lang/primitives.md), [Lang Requirements](../../cognitive_architecture/meld-lang/requirements.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `Condition` enum with variants: `Above(Term)`, `Below(Term)`, `Equals(Term)`, `In(Vec<Term>)`, `Within(Term)`, `Exceeds(Term)`, `Present`, `Absent`. Derive required traits. | Proposed |
| 2 | Implement `Proposition` enum with variants: `Holds { subject: Term, dimension: Term, condition: Condition }`, `Exists { scope: Term, artifact_type: Term }`, `Accessible { scope: Term }`, `Related { src: Term, relation: Term, dst: Term }`, `All(Vec<Proposition>)`, `Any(Vec<Proposition>)`, `Not(Box<Proposition>)`. Derive required traits. | Proposed |
| 3 | Add helper method `Proposition::is_ground()` that returns true when all terms are concrete -- no `Variable`, no `Derived`. Recurse through `All`/`Any`/`Not` children. | Proposed |
| 4 | Add unit tests for proposition construction across all variants. | Proposed |
| 5 | Add unit tests for nested `All`/`Any`/`Not` serialization round-trip. | Proposed |
| 6 | Add unit tests for `is_ground()`: ground proposition returns true, proposition with `Variable` returns false, nested compound with one `Variable` child returns false. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| All proposition and condition shapes constructable from terms. | Proposed |
| `is_ground()` correctly identifies ground vs pattern propositions including nested compounds. | Proposed |
| Nested `All`/`Any`/`Not` serialize and deserialize correctly. | Proposed |
| Same `Proposition` type usable without wrapper types in all positions. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Provides the statement grammar consumed by WorldState, evaluate, Effect, Operator, Goal, and Method. | Proposed |

Verification:
- `cargo test -p meld-lang proposition`
- `cargo test -p meld-lang condition`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject`
- `cargo llvm-cov -p meld-lang` (coverage does not decrease from Phase 1)

Key files:
- new `crates/meld-lang/src/proposition.rs`
- new `crates/meld-lang/src/condition.rs`

---

### Phase 3 -- Effect, WorldState, and three-valued evaluation

| Field | Value |
|-------|-------|
| Goal | Implement state change primitives, the ground proposition set, and three-valued evaluation. First testable loop: construct propositions, build world state, evaluate, apply effects, re-evaluate. |
| Dependencies | Phase 2 |
| Docs | [Lang Primitives](../../cognitive_architecture/meld-lang/primitives.md), [World State and Evaluation](../../cognitive_architecture/meld-lang/world_state.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `Effect` enum: `Assert(Proposition)`, `Retract(Proposition)`, `Update { subject: Term, dimension: Term, value: Term }`. Derive required traits. | Proposed |
| 2 | Implement `GroundingError` struct with `index: usize` and `variable: String`. | Proposed |
| 3 | Implement `WorldState` struct with `propositions: Vec<Proposition>`. Implement `WorldState::new()` that validates all propositions are ground, rejecting `Variable` and `Derived` terms. Implement `WorldState::empty()`. | Proposed |
| 4 | Implement `WorldState::propositions()` returning `&[Proposition]`. | Proposed |
| 5 | Implement `EvalResult` enum: `Satisfied`, `Unsatisfied { gap: Vec<Proposition> }`, `Indeterminate { missing: Vec<Term> }`. | Proposed |
| 6 | Implement `evaluate()` as pure function from `&WorldState` and `&Proposition` to `EvalResult`. Implement evaluation rules per spec for each proposition variant: `Holds` with condition matching, `Exists`, `Accessible`, `Related`, `All`, `Any`, `Not` with three-valued propagation. | Proposed |
| 7 | Implement `WorldState::satisfies()` as convenience over `evaluate()`. | Proposed |
| 8 | Implement `WorldState::gap()` returning unsatisfied and indeterminate sub-propositions. | Proposed |
| 9 | Implement `WorldState::query()` taking a pattern proposition and returning `Vec<Bindings>` for all matches. Defer full implementation if `Bindings` is not yet available -- stub with empty return and mark for completion in Phase 7. | Proposed |
| 10 | Implement `WorldState::apply()` applying a slice of effects to produce a new `WorldState`. Assert adds or is idempotent. Retract removes or is no-op. Update retracts existing `Holds` for subject+dimension then asserts new. Reject effects containing `Variable` terms. | Proposed |
| 11 | Add evaluation rule tests: one test per proposition variant with expected result. | Proposed |
| 12 | Add three-valued logic tests: `Satisfied`, `Unsatisfied` with correct gap, `Indeterminate` for missing dimension. Verify `Indeterminate` is distinct from `Unsatisfied`. | Proposed |
| 13 | Add effect application tests: Assert add and idempotent, Retract remove and no-op, Update retract-then-assert. | Proposed |
| 14 | Add loop test: evaluate returns `Unsatisfied`, apply effects, evaluate returns `Satisfied`. | Proposed |
| 15 | Add grounding rejection test: `WorldState::new()` with `Variable` term returns `GroundingError`. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| `evaluate()` produces correct three-valued result for all proposition variants. | Proposed |
| `Indeterminate` is distinct from `Unsatisfied` -- missing dimension vs wrong value. | Proposed |
| Effect application is deterministic and sequential. | Proposed |
| `WorldState::new()` rejects propositions containing `Variable` or `Derived` terms. | Proposed |
| `WorldState::apply()` returns new state without mutating input. | Proposed |
| First testable loop passes: evaluate, apply effects, re-evaluate to `Satisfied`. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Provides the evaluation substrate for goal satisfaction checking, gap detection, and effect projection used by planning and goal curation. | Proposed |

Verification:
- `cargo test -p meld-lang effect`
- `cargo test -p meld-lang world_state`
- `cargo test -p meld-lang evaluate`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject`
- `cargo mutants -p meld-lang --timeout 60 -- --all-targets` (zero survivors in evaluate.rs, effect.rs, world_state.rs)
- `cargo llvm-cov -p meld-lang` (coverage does not decrease from Phase 2)
- `cargo fuzz run fuzz_evaluate -- -max_total_time=60` (local, no panics)

Key files:
- new `crates/meld-lang/src/effect.rs`
- new `crates/meld-lang/src/world_state.rs`
- new `crates/meld-lang/src/evaluate.rs`
- new `crates/meld-lang/fuzz/fuzz_targets/fuzz_evaluate.rs`

---

### Phase 4 -- Operator, Resolution, and CostEstimate

| Field | Value |
|-------|-------|
| Goal | Implement the runtime-constructed operator contract, capability resolution query shape, and multi-dimensional cost algebra. |
| Dependencies | Phase 2, Phase 3 |
| Docs | [Operators and Resolution](../../cognitive_architecture/meld-lang/operators.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `CostEstimate` struct with `time_ms: u64`, `money_microdollars: u64`, `provider_calls: u32`. Derive required traits. Implement `zero()`, `add()`, `exceeds()`. | Proposed |
| 2 | Implement `SlotConstraint` struct with `artifact_type_id: String` and `required: bool`. Derive required traits. | Proposed |
| 3 | Implement `CapabilityRef` struct with `capability_type_id: String` and `capability_version: u32`. Derive required traits. | Proposed |
| 4 | Implement `Resolution` struct with `requires_inputs: Vec<SlotConstraint>`, `requires_outputs: Vec<SlotConstraint>`, `scope_kind: Option<String>`, `tags: Vec<String>`, `specific: Option<CapabilityRef>`. Derive required traits. | Proposed |
| 5 | Implement `Operator` struct with `operator_id: String`, `preconditions: Vec<Proposition>`, `effects: Vec<Effect>`, `cost: CostEstimate`, `resolution: Resolution`. Derive required traits. | Proposed |
| 6 | Add cost algebra tests: addition sums all three dimensions, `exceeds()` returns true if ANY dimension exceeds ceiling, `zero()` is identity for addition. | Proposed |
| 7 | Add operator construction test: build operator from terms and propositions at runtime, verify serialization round-trip. | Proposed |
| 8 | Add resolution construction test: verify all fields serialize and deserialize. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| Operators constructable at runtime from terms and propositions without compile-time enum variants for specific operator kinds. | Proposed |
| Cost algebra correct for all three dimensions independently. | Proposed |
| `exceeds()` returns true if ANY single dimension exceeds the ceiling. | Proposed |
| All types serialize and deserialize cleanly. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Provides the operator and cost contract consumed by Composition steps and Method cost comparison. | Proposed |

Verification:
- `cargo test -p meld-lang operator`
- `cargo test -p meld-lang cost`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject`
- `cargo mutants -p meld-lang --timeout 60 -- --all-targets` (zero survivors in cost.rs, operator.rs)
- `cargo llvm-cov -p meld-lang` (coverage does not decrease from Phase 3)

Key files:
- new `crates/meld-lang/src/operator.rs`
- new `crates/meld-lang/src/cost.rs`

---

### Phase 5 -- Composition graph and structural validation

| Field | Value |
|-------|-------|
| Goal | Implement the composition graph and the structural validation function. Compositions are the plan representation in the language. |
| Dependencies | Phase 4 |
| Docs | [Compositions](../../cognitive_architecture/meld-lang/compositions.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `StepKind` enum: `Op(Operator)`, `Goal(Proposition)`. Derive required traits. | Proposed |
| 2 | Implement `Step` struct with `step_id: String` and `kind: StepKind`. Derive required traits. | Proposed |
| 3 | Implement `EdgeKind` enum: `Ordering`, `DataFlow { artifact_type: String }`, `Conditional { field_path: String, guard: Condition }`. Derive required traits. | Proposed |
| 4 | Implement `Edge` struct with `from: String`, `to: String`, `kind: EdgeKind`. Derive required traits. | Proposed |
| 5 | Implement `Composition` struct with `steps: Vec<Step>` and `edges: Vec<Edge>`. Derive required traits. | Proposed |
| 6 | Implement `ValidationError` enum: `DanglingEdge`, `DuplicateStepId`, `CycleDetected`, `ArtifactSourceMismatch`, `InvalidGuardOnGoalStep`, `UnboundVariable`. | Proposed |
| 7 | Implement `ValidationWarning` enum: `DisconnectedStep`, `UnusedEffects`, `HighCost`. | Proposed |
| 8 | Implement `ValidationResult` struct with `valid: bool`, `errors: Vec<ValidationError>`, `warnings: Vec<ValidationWarning>`. | Proposed |
| 9 | Implement `validate()` as pure function. Check: dangling edges, duplicate step IDs, cycles via topological sort, artifact source mismatch against operator resolution outputs, invalid guard on Goal step, unbound variables in post-substitution positions. | Proposed |
| 10 | Implement `CostEstimate::aggregate()` summing operator costs across a composition. | Proposed |
| 11 | Add validation tests: valid composition passes with no errors. | Proposed |
| 12 | Add error tests: one test per `ValidationError` variant triggering on correct input. | Proposed |
| 13 | Add cycle detection tests: simple cycle, transitive cycle, valid DAG passes. | Proposed |
| 14 | Add warning tests: disconnected step, unused effects. | Proposed |
| 15 | Add construction tests: single-step, linear chain, parallel branches, conditional edges. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| Compositions constructable as data from operators and propositions. | Proposed |
| `validate()` catches all structural errors without access to capability catalog. | Proposed |
| `validate()` does NOT check resolution or goal achievement. | Proposed |
| Cycle detection correct for complex graphs. | Proposed |
| `aggregate()` sums costs across all operator steps. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Provides the plan representation consumed by Method templates, substitution, and the planning loop. | Proposed |

Verification:
- `cargo test -p meld-lang composition`
- `cargo test -p meld-lang validate`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject`
- `cargo mutants -p meld-lang --timeout 60 -- --all-targets` (zero survivors in validate.rs)
- `cargo llvm-cov -p meld-lang` (coverage does not decrease from Phase 4)
- `cargo fuzz run fuzz_validate -- -max_total_time=60` (local, no panics)

Key files:
- new `crates/meld-lang/src/composition.rs`
- new `crates/meld-lang/src/validate.rs`
- new `crates/meld-lang/fuzz/fuzz_targets/fuzz_validate.rs`

---

### Phase 6 -- Goal specification

| Field | Value |
|-------|-------|
| Goal | Implement the Goal type with lifecycle, priority, source, and agent identity. Goals are propositions with operational metadata. |
| Dependencies | Phase 2, Phase 4 |
| Docs | [Goals and Methods](../../cognitive_architecture/meld-lang/goals_and_methods.md), [Goals](../../cognitive_architecture/execution/goals/README.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `GoalPriority` struct with `urgency: u32` and `cost_ceiling: Option<CostEstimate>`. Derive required traits. | Proposed |
| 2 | Implement `GoalSource` enum: `BeliefDivergence { dimension: String, observed: String, desired: String }`, `UserDirected { directive: String }`, `Maintenance { invariant_description: String }`, `Decomposed { parent_goal_id: String }`. Derive required traits. | Proposed |
| 3 | Implement `GoalLifecycle` enum: `Proposed`, `Active`, `Suspended { reason: String }`, `Satisfied { at_seq: u64 }`, `Abandoned { reason: String }`. Derive required traits including `Eq` on `GoalLifecycle`. | Proposed |
| 4 | Implement `Goal` struct with `goal_id: String`, `agent_id: String`, `target: Proposition`, `priority: GoalPriority`, `source: GoalSource`, `lifecycle: GoalLifecycle`. Derive required traits. | Proposed |
| 5 | Add goal construction test: build goal from proposition target at runtime, verify round-trip. | Proposed |
| 6 | Add test: goal target is a ground `Proposition`, verify it works with `evaluate()` from Phase 3. | Proposed |
| 7 | Add test: lifecycle transitions are expressible as value changes, no state machine enforcement in `meld-lang`. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| Goals constructable at runtime by composing terms into a proposition target. | Proposed |
| Goal target uses the same `Proposition` type as world state and operator preconditions. | Proposed |
| All goal types serialize and deserialize correctly. | Proposed |
| No compile-time enum for specific goal kinds. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Provides the execution input contract consumed by the planning loop, goal set, and method matching. | Proposed |

Verification:
- `cargo test -p meld-lang goal`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject`
- `cargo mutants -p meld-lang --timeout 60 -- --all-targets` (zero survivors in goal.rs)
- `cargo llvm-cov -p meld-lang` (coverage does not decrease from Phase 5)

Key files:
- new `crates/meld-lang/src/goal.rs`

---

### Phase 7 -- Method, unification, and substitution

| Field | Value |
|-------|-------|
| Goal | Implement methods, pattern unification, binding management, and composition substitution. This completes the planning loop's ability to match goals to decompositions. |
| Dependencies | Phase 5, Phase 6 |
| Docs | [Goals and Methods](../../cognitive_architecture/meld-lang/goals_and_methods.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `Bindings` struct wrapping `HashMap<String, Term>`. Implement `empty()`, `bind()` returning `Option<Bindings>`, `get()`, `merge()` returning `Option<Bindings>`, `iter()`. All operations return new values. | Proposed |
| 2 | Implement `unify()` as pure function from pattern `&Proposition` and concrete `&Proposition` to `Option<Bindings>`. Implement rules: `Variable` binds to any concrete term, concrete terms match by equality, same variable appearing twice must bind to same term, `All`/`Any`/`Not` unify structurally with pairwise children. | Proposed |
| 3 | Implement `SubstitutionError` struct with `unbound_variables: Vec<UnboundVariable>`. Implement `UnboundVariable` struct with `step_id: String` and `variable: String`. | Proposed |
| 4 | Implement `substitute()` as pure function from `&Composition` and `&Bindings` to `Result<Composition, SubstitutionError>`. Walk all `Term` positions in operators preconditions, effects, resolution hints, and sub-goal propositions. Replace `Variable` with bound value. Error if any variable is unbound. | Proposed |
| 5 | Implement `Method` struct with `method_id: String`, `trigger: Proposition`, `preconditions: Vec<Proposition>`, `composition: Composition`, `net_effects: Vec<Effect>`, `cost: CostEstimate`, `preference: u32`. Derive required traits. | Proposed |
| 6 | Complete `WorldState::query()` stub from Phase 3 using `Bindings`. | Proposed |
| 7 | Add unification tests: successful bind, shape mismatch fails, double-bind conflict fails. | Proposed |
| 8 | Add unification tests: structural unification of `All`/`Any`/`Not` with variable children. | Proposed |
| 9 | Add bindings tests: immutable semantics, merge success, merge conflict returns `None`. | Proposed |
| 10 | Add substitution tests: all variables replaced, unbound variable produces error with correct `step_id` and `variable`, nested operator and sub-goal substitution. | Proposed |
| 11 | Add method matching flow test: `unify` trigger against goal target, check preconditions against world state, verify net effects achieve goal via `apply` and `evaluate`, substitute composition, validate result. | Proposed |
| 12 | Add method JSON deserialization test: serialize a `Method` to JSON string, deserialize, verify structural equality with programmatic construction. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| `unify()` correctly binds variables from pattern to concrete proposition. | Proposed |
| `unify()` fails cleanly on shape mismatch or double-bind conflict. | Proposed |
| `substitute()` replaces all variables in a composition's operators and sub-goals. | Proposed |
| `substitute()` fails with explicit error listing unbound variables. | Proposed |
| Methods serialize and deserialize from JSON identically to programmatic construction. | Proposed |
| `Bindings` is immutable -- `bind()` and `merge()` return new values. | Proposed |
| `WorldState::query()` returns correct bindings for pattern matches. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Provides the planning optimization path for method matching, binding substitution, and cached decomposition reuse. Completes the full type surface for the typed loop integration. | Proposed |

Verification:
- `cargo test -p meld-lang method`
- `cargo test -p meld-lang unify`
- `cargo test -p meld-lang substitute`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject`
- `cargo mutants -p meld-lang --timeout 60 -- --all-targets` (zero survivors in unify.rs, substitute.rs)
- `cargo llvm-cov -p meld-lang` (coverage does not decrease from Phase 6)
- `cargo fuzz run fuzz_unify -- -max_total_time=60` (local, no panics)
- `cargo fuzz run fuzz_substitute -- -max_total_time=60` (local, no panics)

Key files:
- new `crates/meld-lang/src/method.rs`
- new `crates/meld-lang/src/unify.rs`
- new `crates/meld-lang/src/substitute.rs`
- new `crates/meld-lang/fuzz/fuzz_targets/fuzz_unify.rs`
- new `crates/meld-lang/fuzz/fuzz_targets/fuzz_substitute.rs`

---

### Phase 8 -- Full evaluation loop integration

| Field | Value |
|-------|-------|
| Goal | Prove the complete evaluation loop end-to-end. Wire `meld-lang` as a dependency into `meld-execution` and `meld-world-model`. This is the typed loop from `design/plan/integration/typed_loop.md`. |
| Dependencies | Phase 0 through Phase 7 |
| Docs | [Lang Requirements](../../cognitive_architecture/meld-lang/requirements.md), [World State and Evaluation](../../cognitive_architecture/meld-lang/world_state.md), [Typed Loop Integration](../integration/typed_loop.md) |
| Status | Proposed |

| Order | Task | Status |
|-------|------|--------|
| 1 | Write integration test: full docs_freshness loop. Construct initial `WorldState` with docs_freshness at 0.3. Construct `Goal` targeting docs_freshness above 0.7. Evaluate: `Unsatisfied`. Match method trigger via `unify()`. Substitute bindings into composition. Validate composition. Apply method effects. Re-evaluate: `Satisfied`. | Proposed |
| 2 | Write integration test: indeterminate path. Construct goal about an unknown dimension not in world state. Evaluate: `Indeterminate`. Construct observation composition. Apply assert effect adding the dimension. Re-evaluate: now `Unsatisfied` or `Satisfied` depending on value. | Proposed |
| 3 | Write integration test: method loaded from JSON string. Deserialize method. Use it in the matching flow. Verify identical behavior to programmatic construction. | Proposed |
| 4 | Write integration test: multi-step composition with data flow and conditional edge. Verify structural validation passes. | Proposed |
| 5 | Write integration test: recursive decomposition. Composition with `Goal` steps. Verify validation passes and sub-goals are inspectable. | Proposed |
| 6 | Write serialization compatibility test: pin a known JSON document for each major type. Verify it deserializes correctly. This guards against accidental serialization format changes. | Proposed |
| 7 | Add `meld-lang` as a dependency to `crates/meld-execution/Cargo.toml`. No code changes in `meld-execution`. Verify `cargo check -p meld-execution` succeeds. | Proposed |
| 8 | Add `meld-lang` as a dependency to `crates/meld-world-model/Cargo.toml`. No code changes in `meld-world-model`. Verify `cargo check -p meld-world-model` succeeds. | Proposed |
| 9 | Verify all public types are re-exported from `lib.rs` for clean consumer imports. | Proposed |

| Exit criterion | Status |
|----------------|--------|
| Complete evaluation loop proven end-to-end in a single test function with no external dependencies. | Proposed |
| Three-valued evaluation distinction produces correct planning-relevant results. | Proposed |
| Method loading from serialized form works identically to programmatic construction. | Proposed |
| Serialization compatibility test pins at least one JSON document per major type. | Proposed |
| `meld-execution` and `meld-world-model` compile with `meld-lang` declared as dependency. | Proposed |
| All public types re-exported from `lib.rs`. | Proposed |

| Dependency closure solved | Status |
|---------------------------|--------|
| Proves the typed loop integration target from `design/plan/integration/typed_loop.md`. Unblocks world model projection, goal curation, and runtime planning substrate targets. | Proposed |

Verification:
- `cargo test -p meld-lang`
- `cargo test -p meld-lang --test evaluation_loop`
- `cargo check -p meld-execution`
- `cargo check -p meld-world-model`
- `cargo clippy -p meld-lang -- -D warnings`
- `cargo fmt -p meld-lang -- --check`
- `cargo insta test --unreferenced reject`
- `cargo mutants -p meld-lang --timeout 60 -- --all-targets` (zero survivors workspace-wide for meld-lang)
- `cargo llvm-cov -p meld-lang --fail-under-lines 90`
- `cargo deny check advisories licenses sources`
- `cargo audit`
- All four fuzz targets pass 60-second runs locally with no panics

Key files:
- new `crates/meld-lang/tests/evaluation_loop.rs`
- modified `crates/meld-execution/Cargo.toml`
- modified `crates/meld-world-model/Cargo.toml`
- modified `.github/workflows/ci.yml` (coverage threshold enforced, mutation job active)

---

## Cross-phase gates

### Purity gates

- `meld-lang` `Cargo.toml` has no `tokio`, `async-trait`, `std::fs`, `std::net`, or IO-capable dependency at any phase
- no `pub fn` in the crate uses `&mut self`, `async`, or returns IO errors at any phase
- all tests run with `#[test]` only at every phase

### Type gates

- all public types derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` at every phase
- `Proposition` is the same type in all roles at Phase 2 and beyond
- `Term` is the same type in all roles at Phase 1 and beyond
- no compile-time enum variant represents a specific goal, method, operator, or action family at any phase

### Determinism gates

- same world state and same proposition produce same `EvalResult` at Phase 3 and beyond
- same pattern and same concrete produce same `Option<Bindings>` at Phase 7 and beyond
- same composition and same bindings produce same `Result<Composition, SubstitutionError>` at Phase 7 and beyond
- same effects and same world state produce same new `WorldState` at Phase 3 and beyond

### Immutability gates

- `WorldState::apply()` returns new `WorldState`, input unchanged at Phase 3 and beyond
- `Bindings::bind()` returns new `Bindings`, input unchanged at Phase 7 and beyond
- `Bindings::merge()` returns new `Bindings`, inputs unchanged at Phase 7 and beyond

### Serialization gates

- all types round-trip through `serde_json` without loss at every phase
- method loaded from JSON produces same `Method` as programmatic construction at Phase 7
- serialization compatibility test pins known JSON documents at Phase 8

### Required test gates

Each phase must pass the following test categories before exit. Categories accumulate -- once introduced they apply to all subsequent phases.

| Test category | Introduced | Tool | Gate rule |
|---------------|-----------|------|-----------|
| Unit tests | Phase 1 | `cargo test` | Every public function has at least one direct test. Every enum variant is constructed and matched in at least one test. |
| Clippy | Phase 0 | `cargo clippy -p meld-lang -- -D warnings` | Zero warnings. No `#[allow]` without a written justification comment. |
| Property tests | Phase 1 | `proptest` | Every type with `PartialEq` has a serialization round-trip property. Every pure function with domain invariants has at least one property test. |
| Snapshot tests | Phase 1 | `insta` | Every public type has a pinned `serde_json` snapshot. Snapshot changes require review. |
| Coverage | Phase 1 | `cargo-llvm-cov` | Line coverage for `meld-lang` does not decrease between phases. Phase 8 exit requires >= 90% line coverage. |
| Mutation tests | Phase 3 | `cargo-mutants` | Zero surviving mutants in modules completed by the current phase. Timeout mutants are investigated, not ignored. |
| Fuzz targets | Phase 3 | `cargo-fuzz` | `evaluate()`, `unify()`, `validate()`, and `substitute()` each have a fuzz target. Fuzz targets run for minimum 60 seconds with no panics as part of phase exit (not CI -- local verification). |

Phase-specific required tests:

| Phase | Required tests |
|-------|---------------|
| Phase 0 | `cargo check -p meld-lang`, `cargo clippy -p meld-lang -- -D warnings`, `cargo test -p meld-lang` (zero tests, passes clean) |
| Phase 1 | Unit: all `Term` and `Literal` variant construction and equality. Property: `Literal` and `Term` serde round-trip for arbitrary values. Snapshot: one pinned JSON per `Term` variant, one per `Literal` variant. |
| Phase 2 | Unit: all `Proposition` and `Condition` variant construction. Property: `is_ground()` invariant -- ground propositions contain no `Variable` or `Derived`. Snapshot: nested `All`/`Any`/`Not` pinned JSON. |
| Phase 3 | Unit: one test per evaluation rule, effect application, grounding rejection. Property: `evaluate()` determinism -- same inputs same output. Property: `apply(Assert(p))` then `apply(Retract(p))` restores original state. Mutation: zero survivors in `evaluate.rs`, `effect.rs`, `world_state.rs`. Fuzz: `evaluate()` target. |
| Phase 4 | Unit: cost algebra, operator construction, resolution round-trip. Property: `CostEstimate::add()` is commutative and associative. `exceeds()` is consistent with component comparison. Mutation: zero survivors in `cost.rs`, `operator.rs`. |
| Phase 5 | Unit: one test per `ValidationError` variant, cycle detection (simple, transitive, valid DAG). Property: valid composition passes validation, adding a dangling edge fails. Mutation: zero survivors in `validate.rs`. Fuzz: `validate()` target. |
| Phase 6 | Unit: goal construction, lifecycle values, round-trip. Property: goal target works with `evaluate()`. Snapshot: each `GoalSource` and `GoalLifecycle` variant pinned. |
| Phase 7 | Unit: unify success/failure, bindings immutability, substitution success/error. Property: `unify(pattern, concrete)` produces bindings that when substituted into pattern yield concrete. Property: `Bindings::merge(a, b)` equals `Bindings::merge(b, a)` when both succeed. Mutation: zero survivors in `unify.rs`, `substitute.rs`. Fuzz: `unify()` and `substitute()` targets. |
| Phase 8 | Integration: full evaluation loop end-to-end. Serialization compatibility: pinned JSON documents for all major types. Coverage: >= 90% line coverage across `meld-lang`. Mutation: zero survivors workspace-wide for `meld-lang`. |

### Dependency gates

- `meld-lang` depends only on `meld-events`, `serde`, `serde_json`, and std at every phase
- `meld-lang` does NOT depend on `meld-world-model`, `meld-execution`, or root `meld` at any phase
- `meld-execution` and `meld-world-model` declare dependency on `meld-lang` at Phase 8

---

## Implementation order summary

| Order | Phase | Summary |
|-------|-------|---------|
| 1 | Phase 0 | Scaffold crate in workspace with correct dependencies. |
| 2 | Phase 1 | Build Term and Literal atoms. |
| 3 | Phase 2 | Build Proposition and Condition grammar. |
| 4 | Phase 3 | Build Effect, WorldState, and three-valued evaluation. First testable loop. |
| 5 | Phase 4 | Build Operator, Resolution, and CostEstimate. |
| 6 | Phase 5 | Build Composition graph and structural validation. |
| 7 | Phase 6 | Build Goal with lifecycle, priority, and source. |
| 8 | Phase 7 | Build Method, unification, and substitution. |
| 9 | Phase 8 | Prove full evaluation loop. Wire dependencies into consumer crates. |

---

## Dependency resolution map

| Dependency need | Solved in phase |
|-----------------|-----------------|
| All types need Term and Literal atoms | Phase 1 |
| WorldState, Effect, Operator, Goal, Method need Proposition and Condition | Phase 2 |
| Goal satisfaction checking and gap detection need evaluate and WorldState | Phase 3 |
| Composition steps need Operator with cost and resolution | Phase 4 |
| Method templates and substitution need Composition and Goal | Phase 5 and Phase 6 |
| Planning loop method matching needs unify, substitute, and Method | Phase 7 |
| Consumer crates need meld-lang wired as dependency | Phase 8 |

---

## Risk notes

### DomainObjectRef dependency weight

`meld-lang` depends on `meld-events` for `DomainObjectRef`. If `meld-events` carries heavy transitive dependencies, the pure crate gains unwanted weight. Mitigation: verify `meld-events` exports `DomainObjectRef` without requiring the full event runtime. If needed, extract `DomainObjectRef` into a shared identity crate.

### f64 in PartialEq

`Literal::Number(f64)` does not implement `Eq` and has NaN comparison issues. `PartialEq` derivation works but `NaN != NaN` may cause surprising behavior in world state matching. Mitigation: document that `Number` should not contain NaN. Consider an ordered float wrapper if this causes test failures.

### Serialization stability

Method files serialized in one version must deserialize in the next. Adding new enum variants to `Proposition`, `Condition`, `Effect`, or `EdgeKind` is a grammar change that may break existing method files. Mitigation: serialization compatibility test in Phase 8 pins known JSON documents.

---

## CI pipeline updates

The current CI pipeline (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo build`, and `cargo test`. The following changes bring it to full coverage for `meld-lang` phase gates.

### Required CI additions

| Addition | Tool | When to add | CI step |
|----------|------|-------------|---------|
| Clippy | `cargo clippy` | Phase 0 | `cargo clippy --workspace --all-targets -- -D warnings` |
| Coverage reporting | `cargo-llvm-cov` | Phase 1 | `cargo llvm-cov --workspace --all-targets --lcov --output-path lcov.info` |
| Coverage threshold | `cargo-llvm-cov` | Phase 8 | `cargo llvm-cov --workspace --all-targets --fail-under-lines 90` |
| Snapshot review | `insta` | Phase 1 | `cargo insta test --unreferenced reject` -- fails if snapshots are pending review or stale |
| Mutation testing | `cargo-mutants` | Phase 3 | `cargo mutants -p meld-lang --timeout 60 -- --all-targets` |
| Dependency audit | `cargo-deny` | Phase 0 | `cargo deny check advisories licenses sources` |
| Vulnerability scan | `cargo-audit` | Phase 0 | `cargo audit` |

### CI implementation phases

**Phase 0 -- immediate (scaffold commit):**

Add to the `verify` job after the Format step:

```yaml
- name: Clippy
  shell: bash
  run: cargo clippy --workspace --all-targets -- -D warnings

- name: Dependency audit
  shell: bash
  run: |
    cargo install cargo-deny --locked || true
    cargo deny check advisories licenses sources

- name: Vulnerability scan
  shell: bash
  run: |
    cargo install cargo-audit --locked || true
    cargo audit
```

Add `deny.toml` to workspace root with license allowlist (`MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`, `Zlib`) and advisory database check enabled.

**Phase 1 -- first types:**

Add to the `verify` job:

```yaml
- name: Snapshot tests
  shell: bash
  run: |
    cargo install cargo-insta --locked || true
    cargo insta test --unreferenced reject

- name: Coverage
  shell: bash
  run: |
    cargo install cargo-llvm-cov --locked || true
    cargo llvm-cov --workspace --all-targets --lcov --output-path lcov.info
```

Add `insta` and `proptest` to `meld-lang` dev-dependencies.

**Phase 3 -- first evaluator:**

Add a separate CI job (mutation tests are slow, run in parallel):

```yaml
mutants:
  name: Mutation testing
  if: ${{ github.event_name != 'push' || !contains(github.event.head_commit.message, '[skip ci]') }}
  runs-on: ubuntu-latest
  permissions:
    contents: read
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - name: Run cargo-mutants
      shell: bash
      run: |
        cargo install cargo-mutants --locked || true
        cargo mutants -p meld-lang --timeout 60 -- --all-targets
```

**Phase 8 -- final gate:**

Update coverage step to enforce threshold:

```yaml
- name: Coverage threshold
  shell: bash
  run: |
    cargo install cargo-llvm-cov --locked || true
    cargo llvm-cov --workspace --all-targets --fail-under-lines 90
```

### Dev-dependency additions to `crates/meld-lang/Cargo.toml`

```toml
[dev-dependencies]
proptest = "1.4"
insta = { version = "1.39", features = ["json"] }
```

### Workspace-level tool configuration

`deny.toml` at workspace root:
- `[advisories]` -- vulnerability = "deny", unmaintained = "warn"
- `[licenses]` -- unlicensed = "deny", allow list as above
- `[sources]` -- unknown-registry = "deny", unknown-git = "deny"

### Release pipeline updates

The release job in `ci.yml` must also update its manifest list when `meld-lang` is added to the workspace. In the "Apply computed version to Cargo.toml" step, add `crates/meld-lang/Cargo.toml` to the `manifests` array. In the "Commit and push version bump" step, add it to the `git add` list. In the "Publish to crates.io" step, add `meld-lang` to the `packages` array before `meld-execution` (dependency order: `meld-events`, `meld-lang`, `meld-execution`, `meld-world-model`, `meld`).

### Fuzz targets (local verification, not CI)

Fuzz targets live in `crates/meld-lang/fuzz/` using `cargo-fuzz`. They are not run in CI due to execution time. Phase exit verification runs each target for 60 seconds locally:

```bash
cargo fuzz run fuzz_evaluate -- -max_total_time=60
cargo fuzz run fuzz_unify -- -max_total_time=60
cargo fuzz run fuzz_validate -- -max_total_time=60
cargo fuzz run fuzz_substitute -- -max_total_time=60
```

Fuzz target creation schedule:
- Phase 3: `fuzz_evaluate`
- Phase 5: `fuzz_validate`
- Phase 7: `fuzz_unify`, `fuzz_substitute`

---

## What this plan does not cover

- Runtime method library loading and indexing. That is an `meld-execution` concern.
- Capability catalog resolution against `Resolution` queries. That is an `meld-execution` concern.
- WorldState construction from graph and belief state. That is an `meld-world-model` concern.
- Goal curation policy and cost-benefit comparators. That is a `world_model/agent` concern.
- Task compilation from compositions. That is an `meld-execution` concern.
- Outcome publication bridging task results to effects. That is a cross-domain concern.
- Event publication. That is an `meld-events` concern.
