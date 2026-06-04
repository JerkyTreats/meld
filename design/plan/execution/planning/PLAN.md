# Execution Planning Runtime Phased Implementation Plan

Status: first slice implemented
Scope: first execution planning runtime slice from active `meld-lang::Goal` to execution owned composition artifact

## Overview

This plan converts the execution planning assessment into one dependency ordered implementation path.

The slice proves the first execution owned planning handoff after world model agent curation. It accepts one active ground `meld-lang::Goal`, reads a goal scoped `meld-lang::WorldState` projection, loads verified `meld-lang::Method` definitions, selects one applicable method, substitutes bindings into the method composition, validates the resulting raw composition, and returns an execution owned composition artifact.

The slice stops at execution composition. It does not lower into task definitions, mutate the task network, dispatch tasks, invoke capabilities, publish outcomes, or perform continuous replanning.

## Implementation Progress

Evidence date: 2026-06-02

Implemented in `meld-execution`:

- execution goal command contracts and lifecycle records
- deterministic in memory `GoalSetStore`
- durable `PersistentGoalSetStore` over caller provided `sled::Db`
- command outcome replay by command id
- source identity dedupe across process restart
- active goal read path for planning and curation handoff
- planning request, result, diagnostic, and composition contracts
- method library loading from JSON files and in memory method values
- method verification for id, trigger, variable coverage, template validity, and operator resolution diagnostics
- one goal `PlanningRuntime` from active goal and projected `WorldState` to `PlanningResult`
- focused tests for goal storage, method loading, planning contracts, and planning runtime

The remaining mutation, coverage, fuzz duration, and comment review gates are scheduled quality hardening. They do not block the Phase 7 handoff because the runtime facade already produces `ExecutionComposition` and focused tests cover the first slice result variants.

Still deferred in this plan:

- execution runtime assembly that chooses the persistent goal store path
- direct ingestion of world model `AgentGoalCommand` into execution goal storage
- durable planning attempt log
- durable method library identity in planning attempt records
- durable capability catalog path for runtime registered or synthesized capabilities
- execution composition lowering into task network commands
- durable task executor state for compiled task shape after expansion, init payload, artifacts, invocation attempts, applied expansions, completion state, and task events
- durable invocation journal or equivalent idempotency barrier before capability dispatch
- task event outbox or publication replay cursor
- durable outcome publication handoff to the world model
- task dispatch and outcome publication
- task network persistence

## Wider Persistent Storage Capture

The goal store is the first authoritative persistent store, but it does not cover all existing execution state.

The next persistent storage work should separate authoritative runtime state from replayable diagnostics:

- Authoritative now: goal lifecycle records, command outcomes, and source identity dedupe.
- Authoritative next: task network command journal, mutation log, reduced task network state, task run state, invocation attempts, task artifacts, expansion records, and outcome publication handoff.
- Replayable diagnostics: planning attempt records, method library identity, selected candidate reports, task event publication cursors, and workflow compatibility telemetry.

The existing `WorkflowStateStore` already persists workflow thread, turn, gate, and prompt link records as JSON files. It can remain as a compatibility store while task network storage is designed, but runtime assembly must avoid splitting new execution state across unrelated roots.

`TaskExecutor` and `TaskArtifactRepo` currently expose durable record shapes but keep live state in memory. That pattern is acceptable for focused tests and single process execution, but it is not sufficient for restart, retry dedupe, expansion idempotency, or safe external capability dispatch.

No separate generic persistent plan store should be added before Phase 7. The durable plan shape should be the task network command journal, mutation log, and reduced network state. Planning attempt records can be added earlier as an append only audit log because they do not own execution commitment.

Verification recorded:

```sh
cargo fmt --check
cargo test -p meld-execution --test goals
cargo test -p meld-execution --test planning_runtime
cargo test -p meld-execution --all-targets
cargo clippy -p meld-execution --all-targets -- -D warnings
cargo test --workspace --all-targets
```

The order is dependency driven:

- Define execution goal set contracts for one active ground goal
- Add an execution planning domain entry under `meld-execution`
- Define planning input and result contracts
- Add method library loading and verification
- Add method candidate matching through `meld-lang::unify`
- Add goal scoped world state projection request contracts
- Evaluate goal and method preconditions through `meld-lang::evaluate`
- Substitute bindings into compositions through `meld-lang::substitute`
- Validate concrete compositions through `meld-lang::validate`
- Return an execution composition result for downstream task network lowering

Related specs:

- [Execution Planning](../../../cognitive_architecture/execution/planning/README.md)
- [Planning Pipeline](../../../cognitive_architecture/execution/planning/planning_pipeline.md)
- [Task Network](../../../cognitive_architecture/execution/task_network.md)
- [Execution Goals](../../../cognitive_architecture/execution/goals/README.md)
- [Execution Gaps](../../../cognitive_architecture/execution/GAPS.md)
- [Meld Lang](../../../cognitive_architecture/meld-lang/README.md)
- [Meld Lang Goals And Methods](../../../cognitive_architecture/meld-lang/goals_and_methods.md)
- [Meld Lang Compositions](../../../cognitive_architecture/meld-lang/compositions.md)
- [Meld Lang Operators](../../../cognitive_architecture/meld-lang/operators.md)
- [World Model Planner](../../../cognitive_architecture/world_model/planner/README.md)
- [World Model Agent](../../../cognitive_architecture/world_model/agent/README.md)

Related plan docs:

- [Cognitive Architecture Implementation Plan](../../README.md)
- [Execution Planning Assessment](assessment.md)
- [Execution Goals Assessment](../goals/assessment.md)
- [World Model Planner Assessment](../../world_model/planner/assessment.md)
- [World Model Agent Assessment](../../world_model/agent/assessment.md)
- [Typed Loop Integration](../../integration/typed_loop.md)
- [Meld Lang Assessment](../../meld-lang/assessment.md)

## Goal To Method To Task Translation

This slice must keep the translation levels separate:

| Level | Artifact | Owner | Role |
|-------|----------|-------|------|
| Desired state | `meld-lang::Goal` | execution goal set | Declares one ground target proposition that should hold. |
| Reusable plan template | `meld-lang::Method` | execution method library | Matches a goal through trigger unification and supplies preconditions, net effects, cost, and a composition template. |
| Concrete planning output | `ExecutionComposition` | execution planning | Carries the selected method, bindings, substituted `meld-lang::Composition`, projected effects, validation, diagnostics, and world state frame provenance. |
| Executable graph | task network command carrying mutation set | downstream task network lowering | Converts the execution composition into task network task nodes, dependency edges, lineage, and init artifacts. |
| Dispatch state | task network and task executor | task runtime | Tracks ready sets, in flight work, artifacts, completion, failure, retry, and cancellation. |

The first slice performs only the first three rows. It proves that an active goal can become a validated execution composition. It does not create task network commands or dispatch state.

The translation is deterministic:

```text
Goal.target
  -> unify with Method.trigger
  -> Bindings
  -> substitute Method.preconditions
  -> evaluate preconditions against WorldState
  -> apply Method.net_effects to WorldState
  -> evaluate Goal.target against projected WorldState
  -> substitute Method.composition
  -> validate concrete Composition
  -> ExecutionComposition
  -> deferred task network lowering into mutations
```

Method composition is therefore not a task. A method owns a composition template in shared language terms. Planning owns the selected bindings and validation context. Lowering later maps the concrete composition graph into task network commands carrying mutation sets.

Composition steps map to later task work as follows:

| Composition part | Planning slice handling | Downstream lowering handling |
|------------------|-------------------------|---------------------------|
| `StepKind::Op` | Verify operator resolution diagnostics against the capability catalog. | Compile or reference a task that can execute the resolved capability graph. |
| `StepKind::Goal` | Preserve as a recursive planning boundary and report it in diagnostics. | Re-enter planning for the sub-goal, then inline or link the lowered subgraph. |
| `EdgeKind::DataFlow` | Validate structural artifact edge shape. | Create task network data dependency edges and init artifact wiring. |
| `EdgeKind::Ordering` | Preserve order edge identity. | Create task network ordering dependency edges. |
| `EdgeKind::Conditional` | Preserve guard expression and validate source shape. | Create task network conditional edges evaluated from task output artifacts. |

The `ExecutionComposition` handoff must contain enough information for lowering to run without reselecting a method: stable composition id, goal id, method id, bindings, concrete composition, projected effects, operator resolution reports, validation report, world state frame ref, and deterministic diagnostics. Lowering may reject or adapt the artifact, but it must not need to ask why the method was selected.

## Target Slice

The first runtime planning slice turns this input set:

- one execution owned active ground `Goal`
- one goal scoped `WorldState` projection
- one verified method library
- one capability catalog for operator resolution checks

into this output:

- `PlanningResult::Satisfied` when the goal target evaluates as satisfied
- `PlanningResult::Composed` when a method produces an execution composition artifact
- `PlanningResult::NoApplicableMethod` when no method can achieve the goal under the supplied world state
- `PlanningResult::Indeterminate` when the world state lacks enough projected facts for planning
- `PlanningResult::InvalidMethod` when method verification or composition validation fails

## Boundary Statement

`execution/planning` owns mechanical planning over `meld-lang` values.

It owns:

- active goal evaluation
- goal scoped world state request contracts
- method library loading
- method verification
- method candidate ordering
- trigger unification
- method precondition evaluation
- cost ceiling filtering
- net effect achievement checks
- composition substitution
- composition validation
- execution composition result contracts

It does not own:

- belief settlement
- planner projection rules inside world model
- agent normative judgment
- task network state
- task network command acceptance and mutation reduction
- composition lowering into task definitions
- task dispatch
- capability invocation
- provider execution
- event spine append
- outcome publication
- continuous replanning
- switching cost decisions

## Guiding Rules

| Rule | Statement |
|------|-----------|
| Execution owns goals | The goal set is execution data even though goal values come from `meld-lang`. |
| Agent owns judgment | World model agent decides what should become a goal. |
| Projection by request | Planning requests a goal scoped world state frame rather than reading a global world state. |
| Mechanical planning | Planning evaluates formal propositions and does not interpret belief semantics. |
| Method library is execution owned | `meld-lang` defines `Method`; `meld-execution` loads, verifies, and queries methods. |
| Composition before tasks | This slice returns an execution owned composition artifact, not a `TaskDefinition`. |
| Catalog checks only | Capability catalog use is limited to operator resolution verification. |
| No dispatch | Planning does not run tasks, call capabilities, or publish outcome events. |
| Deterministic order | Same goal, world state, method library, and catalog produce the same selected result. |
| Modern modules | Use `planning.rs` and `planning/*.rs`. Do not add `mod.rs`. |

## First Slice Contracts

| Contract | First value |
|----------|-------------|
| Goal input | one active ground `meld-lang::Goal` |
| Goal owner | execution goal set |
| Goal producer | world model agent command |
| Projection input | goal target, agent id, perspective id, branch id, and requested dimensions |
| Projection output | `meld-lang::WorldState` plus projection metadata and diagnostics |
| Method library | directory or in memory collection of serialized `meld-lang::Method` values |
| Method candidate key | successful unification of `method.trigger` with `goal.target` |
| Preconditions | method preconditions after binding substitution evaluated against `WorldState` |
| Achievement check | method net effects applied to `WorldState` satisfy the goal target |
| Cost check | method cost does not exceed the goal cost ceiling when a ceiling exists |
| Raw composition output | concrete `meld-lang::Composition` after substitution |
| Required checks | structural validation, operator resolution diagnostics, and projected effect checks |
| Handoff result | execution composition prepared for downstream task network lowering |

## Proposed Module Shape

The execution crate should add domain first modules:

```text
crates/meld-execution/src/goals.rs
crates/meld-execution/src/goals/contracts.rs
crates/meld-execution/src/goals/store.rs
crates/meld-execution/src/goals/persistent_store.rs
crates/meld-execution/src/goals/query.rs
crates/meld-execution/src/planning.rs
crates/meld-execution/src/planning/contracts.rs
crates/meld-execution/src/planning/method_library.rs
crates/meld-execution/src/planning/runtime.rs
crates/meld-execution/src/planning/world_state.rs
```

`goals` owns the execution goal set and curation API surface.

`planning` owns method library and planning runtime behavior.

`task` remains the lower execution layer for compiled task graphs.

`capability` remains the executable contract catalog and runtime invocation layer.

## Result Contract Shape

The first result contract should make the downstream boundary explicit:

```rust
pub enum PlanningResult {
    Satisfied(PlanningSatisfied),
    Composed(ExecutionComposition),
    NoApplicableMethod(NoApplicableMethod),
    Indeterminate(PlanningIndeterminate),
    InvalidMethod(InvalidMethodReport),
}

pub struct ExecutionComposition {
    pub composition_id: String,
    pub goal: meld_lang::Goal,
    pub world_state_frame: PlanningWorldStateFrameRef,
    pub method_id: String,
    pub bindings: meld_lang::Bindings,
    pub composition: meld_lang::Composition,
    pub projected_effects: Vec<meld_lang::Effect>,
    pub operator_resolutions: Vec<OperatorResolutionReport>,
    pub validation: CompositionValidationReport,
    pub diagnostics: Vec<PlanningDiagnostic>,
}
```

The result carries raw composition data plus execution planning context for explanation and downstream task network lowering. It must not carry task execution state.

## Method Library Shape

The method library is execution owned storage around `meld-lang::Method`.

```rust
pub struct MethodLibrary {
    pub entries: Vec<VerifiedMethodEntry>,
}

pub struct VerifiedMethodEntry {
    pub method: meld_lang::Method,
    pub source_ref: MethodSourceRef,
    pub verification: MethodVerification,
}
```

The loader accepts serialized method files and returns verified entries in deterministic order.

Verification checks:

- method id is present and unique
- trigger is a usable proposition pattern
- composition template validates after variable coverage checks
- preconditions reference bound variables only
- net effects reference bound variables only
- cost estimate is valid
- preference is deterministic
- each operator resolution can match the capability catalog or names a missing capability diagnostic

## Goal Scoped World State Request

Planning should request the smallest world state frame needed for the active goal and candidate methods.

```rust
pub struct PlanningWorldStateRequest {
    pub goal_id: String,
    pub agent_id: String,
    pub target: meld_lang::Proposition,
    pub perspective_id: String,
    pub branch_id: String,
    pub required_preconditions: Vec<meld_lang::Proposition>,
}
```

The world model planner owns projection rules. Execution owns only the request shape and the consumption of returned `WorldState`.

The first implementation may receive the projected `WorldState` as direct input in tests. The public contract should still name the request shape so the runtime boundary remains explicit.

## Development Phases

| Phase | Goal | Dependencies | State |
|-------|------|--------------|-------|
| 0 | Scope lock and contract inventory | execution goals, world model planner, meld-lang assessments | Complete |
| 1 | Module scaffold and public boundary | Phase 0 | Complete |
| 2 | Goal set contracts | Phase 1 | Complete |
| 3 | Planning result contracts | Phase 2 | Complete |
| 4 | Method library loading and verification | Phase 3 | Complete for first slice |
| 5 | Goal scoped world state request contract | Phase 3 | Complete |
| 6 | Method candidate selection | Phase 4 and Phase 5 | Complete |
| 7 | Composition preparation and validation | Phase 6 | Complete |
| 8 | Runtime facade and typed loop tests | Phase 7 | Complete for Phase 7 handoff |

Each phase lists focused verification for the work introduced in that phase. The Quality Bar section is cumulative and applies to every phase exit once the related harness exists.

High cost mutation, fuzz duration, and coverage gates remain scheduled unless they are listed in the verification evidence above.

---

## Phase 0 -- Scope Lock And Contract Inventory

| Field | Value |
|-------|-------|
| Goal | Freeze the exact planning runtime surface for one active goal to execution composition. |
| Dependencies | execution goals, world model planner, world model agent, meld-lang |
| Docs | [Execution Planning Assessment](assessment.md), [Planning Pipeline](../../../cognitive_architecture/execution/planning/planning_pipeline.md), [Meld Lang Goals And Methods](../../../cognitive_architecture/meld-lang/goals_and_methods.md) |
| State | Complete |

| Order | Task | State |
|-------|------|-------|
| 1 | Confirm the first goal input is one active ground `meld-lang::Goal`. | Complete |
| 2 | Confirm the first projected world state request fields. | Complete |
| 3 | Confirm the first method file format is serialized `meld-lang::Method`. | Complete |
| 4 | Confirm capability catalog checks are verification only. | Complete |
| 5 | Record all deferred task network and dispatch concerns in this plan. | Complete |
| 6 | Confirm the translation boundary from goal to method to execution composition to deferred task network lowering. | Complete |

| Exit Criterion | State |
|----------------|-------|
| The planning runtime input and output contracts are named. | Complete |
| The slice requires no task network command or mutation support. | Complete |
| The deferred runtime concerns are explicit. | Complete |
| Method selection can be explained without task lowering details. | Complete |

Verification:

```sh
cargo check -p meld-execution
```

---

## Phase 1 -- Module Scaffold And Public Boundary

| Field | Value |
|-------|-------|
| Goal | Add execution goal and planning module entries without changing task dispatch behavior. |
| Dependencies | Phase 0 |
| State | Complete |

| Order | Task | State |
|-------|------|-------|
| 1 | Add `crates/meld-execution/src/goals.rs`. | Complete |
| 2 | Add `crates/meld-execution/src/goals/contracts.rs`. | Complete |
| 3 | Add `crates/meld-execution/src/goals/store.rs`. | Complete |
| 4 | Add `crates/meld-execution/src/goals/query.rs`. | Complete |
| 5 | Add `crates/meld-execution/src/planning.rs`. | Complete |
| 6 | Add `crates/meld-execution/src/planning/contracts.rs`. | Complete |
| 7 | Add `crates/meld-execution/src/planning/method_library.rs`. | Complete |
| 8 | Add `crates/meld-execution/src/planning/runtime.rs`. | Complete |
| 9 | Add `crates/meld-execution/src/planning/world_state.rs`. | Complete |
| 10 | Re-export only public contracts and runtime facades. | Complete |

| Exit Criterion | State |
|----------------|-------|
| The crate exposes goal and planning public boundaries. | Complete |
| No task runtime behavior changes. | Complete |
| No `mod.rs` file is added. | Complete |

Verification:

```sh
cargo check -p meld-execution
cargo test -p meld-execution
```

---

## Phase 2 -- Goal Set Contracts

| Field | Value |
|-------|-------|
| Goal | Define execution owned goal set records and one active ground goal path. |
| Dependencies | Phase 1 |
| State | Complete |

| Order | Task | State |
|-------|------|-------|
| 1 | Define `ExecutionGoalRecord` wrapping `meld-lang::Goal` with execution metadata. | Complete |
| 2 | Define goal command records for add, modify, remove, satisfy, suspend, resume, and read. | Complete |
| 3 | Add validation rejecting active goals with non ground targets. | Complete |
| 4 | Add in memory `GoalSetStore` for deterministic first slice tests. | Complete |
| 5 | Add `GoalSetQuery` for active goal reads. | Complete |
| 6 | Add duplicate command behavior based on goal id and source identity. | Complete |
| 7 | Add durable `PersistentGoalSetStore` for goal records, command outcomes, and source identity dedupe. | Complete |

| Exit Criterion | State |
|----------------|-------|
| One active ground goal can be inserted and read deterministically. | Complete |
| Non ground active goals are rejected with typed diagnostics. | Complete |
| The goal set stores lifecycle data without interpreting goal semantics. | Complete |
| Durable goal records, command outcomes, and source identity dedupe survive reopen. | Complete |

Verification:

```sh
cargo test -p meld-execution --test goals
cargo clippy -p meld-execution -- -D warnings
```

---

## Phase 3 -- Planning Result Contracts

| Field | Value |
|-------|-------|
| Goal | Define typed planning request, diagnostics, and result records. |
| Dependencies | Phase 2 |
| State | Complete |

| Order | Task | State |
|-------|------|-------|
| 1 | Define `PlanningRequest` carrying goal id, projection frame, and method library ref. | Complete |
| 2 | Define `PlanningResult` variants for satisfied, composed, no applicable method, indeterminate, and invalid method. | Complete |
| 3 | Define `ExecutionComposition` carrying stable id, goal, method id, bindings, raw composition, effects, resolution reports, validation report, and diagnostics. | Complete |
| 4 | Define structured diagnostics for missing projection data, method mismatch, failed precondition, cost ceiling, invalid composition, and unresolved operator. | Complete |
| 5 | Add serde round trip tests for public contracts. | Complete |
| 6 | Add public Rustdoc for ownership boundaries and invariants where type names do not make them obvious. | Complete |
| 7 | Add a lowering readiness assertion that composed results contain all data downstream lowering needs without task runtime state. | Complete |

| Exit Criterion | State |
|----------------|-------|
| Planning results are serializable and deterministic. | Complete |
| The composed result carries no task execution state. | Complete |
| Diagnostics explain why candidate methods were rejected. | Complete |
| The composed result names the selected method and concrete composition without implying dispatch. | Complete |

Verification:

```sh
cargo test -p meld-execution planning::contracts
```

---

## Phase 4 -- Method Library Loading And Verification

| Field | Value |
|-------|-------|
| Goal | Load serialized `meld-lang::Method` values and verify them against planning rules and the capability catalog. |
| Dependencies | Phase 3 |
| State | Complete for first slice |

| Order | Task | State |
|-------|------|-------|
| 1 | Add `MethodLibrary` and `VerifiedMethodEntry`. | Complete |
| 2 | Add JSON file loading from a directory with deterministic path ordering. | Complete |
| 3 | Reject duplicate method ids. | Complete |
| 4 | Verify trigger, preconditions, composition, net effects, cost, and preference. | Complete |
| 5 | Verify variable coverage across preconditions, composition, and net effects. | Complete |
| 6 | Verify operator resolution against `CapabilityCatalog`. | Complete |
| 7 | Preserve unresolved operator diagnostics without invoking capabilities. | Complete |
| 8 | Add pinned JSON fixture for `refresh_docs_v1`. | Complete |
| 9 | Add property tests proving load order normalization is deterministic across path order variation. | Complete |
| 10 | Add mutation test coverage for duplicate id, variable coverage, and operator resolution branches. | Scheduled high cost gate |

| Exit Criterion | State |
|----------------|-------|
| Method files load into verified entries. | Complete |
| Invalid method files produce typed diagnostics. | Complete |
| Capability catalog checks happen without dispatch. | Complete |
| Method ordering is deterministic by preference, cost, and method id. | Complete |
| Method verification distinguishes reusable template validity from runtime task lowering validity. | Complete |

Verification:

```sh
cargo test -p meld-execution method_library
cargo clippy -p meld-execution -- -D warnings
```

---

## Phase 5 -- Goal Scoped World State Request Contract

| Field | Value |
|-------|-------|
| Goal | Define the execution side request for a goal scoped world state frame. |
| Dependencies | Phase 3 |
| State | Complete |

| Order | Task | State |
|-------|------|-------|
| 1 | Define `PlanningWorldStateRequest`. | Complete |
| 2 | Define `PlanningWorldStateFrameRef` for provenance and diagnostics. | Complete |
| 3 | Include goal target and candidate preconditions in the request. | Complete |
| 4 | Keep world model projection logic out of execution. | Complete |
| 5 | Allow tests to provide `WorldState` directly while preserving request records. | Complete |

| Exit Criterion | State |
|----------------|-------|
| Planning requests a scoped frame rather than a global world state. | Complete |
| Execution consumes returned `WorldState` mechanically. | Complete |
| Projection ownership remains in world model. | Complete |

Verification:

```sh
cargo test -p meld-execution planning_world_state
```

---

## Phase 6 -- Method Candidate Selection

| Field | Value |
|-------|-------|
| Goal | Select one applicable method for one active goal and projected world state. |
| Dependencies | Phase 4 and Phase 5 |
| State | Complete |

| Order | Task | State |
|-------|------|-------|
| 1 | Evaluate the goal target against the supplied `WorldState`. | Complete |
| 2 | Return satisfied result when evaluation is satisfied. | Complete |
| 3 | Return indeterminate result when evaluation lacks required projection data. | Complete |
| 4 | Run `meld-lang::unify` between method trigger and goal target. | Complete |
| 5 | Substitute bindings into method preconditions. | Complete |
| 6 | Evaluate method preconditions against `WorldState`. | Complete |
| 7 | Filter methods that exceed goal cost ceiling. | Complete |
| 8 | Apply method net effects to projected `WorldState` and verify goal satisfaction. | Complete |
| 9 | Select the deterministic best candidate by preference, cost, and method id. | Complete |
| 10 | Preserve per candidate diagnostics for trigger miss, precondition miss, indeterminate projection, cost rejection, and effect miss. | Complete |
| 11 | Add property tests proving repeated selection with the same inputs returns the same selected method and diagnostics. | Complete |

| Exit Criterion | State |
|----------------|-------|
| One active goal can produce one selected method candidate. | Complete |
| Candidate rejection diagnostics are deterministic. | Complete |
| Planning remains semantic free. | Complete |
| The selected candidate can be explained as trigger bindings plus mechanical checks. | Complete |

Verification:

```sh
cargo test -p meld-execution planning_candidate
```

---

## Phase 7 -- Composition Preparation And Validation

| Field | Value |
|-------|-------|
| Goal | Prepare an execution composition artifact from the selected method. |
| Dependencies | Phase 6 |
| State | Complete |

| Order | Task | State |
|-------|------|-------|
| 1 | Substitute candidate bindings into `method.composition`. | Complete |
| 2 | Return typed substitution diagnostics for unbound variables. | Complete |
| 3 | Run `meld-lang::validate` on the concrete composition. | Complete |
| 4 | Ensure each `StepKind::Op` has resolution diagnostics from the verified method entry. | Complete |
| 5 | Preserve `StepKind::Goal` entries as recursive planning boundaries rather than lowering them. | Complete |
| 6 | Preserve composition edges for task network lowering. | Complete |
| 7 | Return `PlanningResult::Composed` with the execution composition artifact. | Complete |
| 8 | Add tests proving `ExecutionComposition` contains no `TaskDefinition`, task instance id, ready set, in flight state, or artifact repo state. | Complete |

| Exit Criterion | State |
|----------------|-------|
| A selected method yields an execution composition artifact. | Complete |
| Invalid composition results are typed and serializable. | Complete |
| No `TaskDefinition` is created in this phase. | Complete |
| The artifact is complete enough for downstream task network lowering without method reselection. | Complete |

Verification:

```sh
cargo test -p meld-execution planning_composition
cargo clippy -p meld-execution -- -D warnings
```

---

## Phase 8 -- Runtime Facade And Typed Loop Tests

| Field | Value |
|-------|-------|
| Goal | Expose one planning runtime facade and prove the first docs freshness path. |
| Dependencies | Phase 7 |
| State | Complete for Phase 7 handoff |

| Order | Task | State |
|-------|------|-------|
| 1 | Add `PlanningRuntime` facade over goal query, method library, world state frame, and catalog. | Complete |
| 2 | Add docs freshness method fixture. | Complete |
| 3 | Add test where low confidence docs goal produces composed result. | Complete |
| 4 | Add test where satisfied goal produces satisfied result. | Complete |
| 5 | Add test where absent projection data produces indeterminate result. | Complete |
| 6 | Add test where no method matches produces no applicable method result. | Complete |
| 7 | Add test where invalid method file produces invalid method diagnostics. | Complete |
| 8 | Add source scans for module boundary and `mod.rs` rule. | Complete |
| 9 | Add local fuzz targets for planning contract decode, method library verification, and runtime candidate selection. | Complete |
| 10 | Add mutation gate commands for the new `goals` and `planning` modules. | Scheduled high cost gate |
| 11 | Add comment consistency review for new Rustdoc and inline comments. | Scheduled policy review |

| Exit Criterion | State |
|----------------|-------|
| The runtime facade plans one goal into one execution composition. | Complete |
| All result variants in the first slice have focused tests. | Complete |
| The crate still passes focused and workspace verification. | Complete |
| High cost verification gates are documented as scheduled evidence outside normal CI. | Complete |

Verification:

```sh
cargo check -p meld-execution
cargo test -p meld-execution
cargo clippy -p meld-execution -- -D warnings
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Deferred

The following concerns are outside this plan:

- composition lowering into `TaskDefinition`
- task network graph executor
- task network command acceptance and mutation reduction
- ready set computation over task nodes
- conditional edge guard evaluation at task network level
- observation task dispatch
- task cancellation
- task preservation
- task pruning
- task dispatch
- capability invocation
- provider execution
- outcome publication
- execution outcome fact shape
- continuous replanning
- plan diffing
- affected subtree selection
- switching cost model
- synthesis task insertion
- workflow integration beyond existing compatibility surfaces

## Quality Bar

Required verification families:

- contract tests for goal records, planning requests, result variants, method library entries, and diagnostics
- serde round trip tests for all public planning contracts
- property tests for deterministic goal reads, method loading, candidate diagnostics, selected candidate, and result serialization
- method file fixture tests with pinned JSON
- negative tests for duplicate method ids, unbound variables, invalid compositions, failed preconditions, cost ceiling rejection, and unresolved operators
- mutation tests for every new `goals` and `planning` module completed by the current phase
- fuzz targets for planning contract decode, method verification, and runtime candidate selection
- coverage checks that do not decrease line coverage for `meld-execution` planning and goals code between phases
- boundary tests proving planning imports `meld-lang`, `capability`, and `goals`, but does not import task runtime dispatch internals
- source scan proving no `crates/meld-execution/src/planning/mod.rs` or `crates/meld-execution/src/goals/mod.rs` exists
- integration style typed loop test proving a docs freshness goal becomes an execution composition
- comment consistency review proving public Rustdoc explains ownership and invariants, inline comments explain non-obvious sequencing only, and stale or narrating comments are removed

Gate policy:

| Gate | Tool | Rule |
|------|------|------|
| Format | `cargo fmt` | Must pass before phase exit. |
| Compile | `cargo check` | Must pass for `meld-execution` and workspace all targets. |
| Unit and integration tests | `cargo test` | Focused phase tests pass before broad workspace tests. |
| Lint | `cargo clippy` | Zero warnings with all targets. The first slice should add no `#[allow]` in new goals or planning modules. |
| Property tests | `proptest` | Public contracts round trip through serde and deterministic functions return the same result for the same inputs. |
| Fixture compatibility | pinned JSON fixtures | Public method, goal, request, and result examples remain stable unless an intentional compatibility change is documented. |
| Mutation tests | `cargo-mutants` | Zero surviving mutants in new phase modules. Timeout mutants are investigated and recorded, not ignored. |
| Fuzz targets | `cargo-fuzz` | Each target runs for a bounded local pass with no panics. |
| Coverage | `cargo-llvm-cov` | Planning and goals line coverage does not decrease across phases. Final slice target is at least 90 percent for new modules. |
| Boundary scans | `rg` plus focused tests | No `mod.rs`, no task dispatch imports from planning, no capability invocation from planning. |
| Comment consistency | policy review plus source scan | Comments follow [Commenting Policy](../../../governance/commenting_policy.md) and do not narrate obvious code. |

Phase specific gate additions:

| Phase | Added gates |
|-------|-------------|
| Phase 1 | Source scan for module shape and public boundary exports. |
| Phase 2 | Property tests for goal record serde, active ground goal rejection, duplicate command determinism, and lifecycle command stability. |
| Phase 3 | Property tests for planning result serde and deterministic diagnostics. Pinned JSON fixtures for every result variant. |
| Phase 4 | Property tests for deterministic method load order. Mutation tests for duplicate ids, variable coverage, cost checks, preference ordering, and operator diagnostics. |
| Phase 5 | Property tests for world state request serde and deterministic request construction from goal plus candidate preconditions. |
| Phase 6 | Property tests for candidate selection determinism. Mutation tests for trigger miss, precondition miss, indeterminate projection, cost rejection, and effect miss branches. |
| Phase 7 | Fuzz target for composition substitution and validation through execution planning contracts. Mutation tests for invalid composition, unbound variable, and task state exclusion. |
| Phase 8 | End to end fuzz target for runtime candidate selection with bounded generated inputs. Workspace and coverage gates. |

Fuzz target schedule:

| Target | Introduced | Focus |
|--------|------------|-------|
| `fuzz_planning_contracts` | Phase 3 | Decode and validate planning request, result, diagnostic, and execution composition records without panics. |
| `fuzz_method_library` | Phase 4 | Decode serialized methods, verify variable coverage, operator diagnostics, and deterministic rejection without panics. |
| `fuzz_planning_composition` | Phase 7 | Substitute bindings into method compositions and validate concrete compositions without panics. |
| `fuzz_planning_runtime` | Phase 8 | Exercise goal evaluation, candidate matching, rejection diagnostics, and composed result construction with bounded generated inputs. |

Required verification commands:

```sh
cargo fmt --check
cargo check -p meld-execution --all-targets
cargo test -p meld-execution --all-targets
cargo clippy -p meld-execution --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

High cost local or scheduled verification:

```sh
cargo llvm-cov -p meld-execution --all-targets --fail-under-lines 90
cargo mutants -p meld-execution --timeout 60 \
  --file crates/meld-execution/src/goals.rs \
  --file crates/meld-execution/src/goals/contracts.rs \
  --file crates/meld-execution/src/goals/store.rs \
  --file crates/meld-execution/src/goals/query.rs \
  --file crates/meld-execution/src/planning.rs \
  --file crates/meld-execution/src/planning/contracts.rs \
  --file crates/meld-execution/src/planning/method_library.rs \
  --file crates/meld-execution/src/planning/runtime.rs \
  --file crates/meld-execution/src/planning/world_state.rs \
  -- --all-targets
cd crates/meld-execution
cargo +nightly fuzz run fuzz_planning_contracts -- -max_total_time=60
cargo +nightly fuzz run fuzz_method_library -- -max_total_time=60
cargo +nightly fuzz run fuzz_planning_composition -- -max_total_time=60
cargo +nightly fuzz run fuzz_planning_runtime -- -max_total_time=60
```

Boundary and comment scans:

```sh
test ! -e crates/meld-execution/src/goals/mod.rs
test ! -e crates/meld-execution/src/planning/mod.rs
! rg -n "crate::task::runtime|execute_task_to_completion|CapabilityInvocationPayload|TaskExecutor" crates/meld-execution/src/planning.rs crates/meld-execution/src/planning
! rg -n "#\\[allow" crates/meld-execution/src/goals.rs crates/meld-execution/src/goals crates/meld-execution/src/planning.rs crates/meld-execution/src/planning
```

## Acceptance Criteria

The execution planning runtime slice is complete when all statements are true:

- execution stores and reads one active ground goal
- method library loads serialized `meld-lang::Method` files
- invalid methods return typed diagnostics
- planning requests or accepts a goal scoped `WorldState`
- satisfied goals return a satisfied result without method selection
- unsatisfied goals can select a matching verified method
- method preconditions are evaluated mechanically against `WorldState`
- selected method net effects satisfy the goal target when applied
- method composition is substituted and validated
- composed result carries an execution composition artifact
- composed result carries no task runtime state
- no task network command is issued
- no task is dispatched
- no capability is invoked
- all required verification commands pass
