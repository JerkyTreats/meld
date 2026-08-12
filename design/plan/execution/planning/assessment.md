# Execution Planning Readiness Assessment

Status: first slice implemented
Typed substrate status: ready
Depends on: `design/plan/execution/goals/assessment.md`, `design/plan/world_model/planner/assessment.md`, `design/plan/meld-lang/assessment.md`
Design source: `design/cognitive_architecture/execution/planning/README.md`, `design/cognitive_architecture/execution/planning/planning_pipeline.md`, `design/cognitive_architecture/execution/planning/guard_expression_semantics.md`, `design/cognitive_architecture/execution/planning/observation_wait_semantics.md`, `design/cognitive_architecture/execution/task_network.md`, `design/plan/execution/gaps.md`, `design/cognitive_architecture/meld-lang/README.md`
Evidence date: 2026-05-31

Note 2026-08-12: planning has since gained the authorized-candidate realization path that bypasses Method search, commits b48c09d and 4894b73. This assessment predates that path and stands as evidence for the earlier slice.

## Verdict Summary

Execution planning has implemented the first runtime slice from one active ground `meld-lang::Goal` and one projected `meld-lang::WorldState` to an execution owned composition artifact.

Task network command acceptance, mutation reduction, dispatch, durable task runtime state, and flywheel outcome publication remain deferred.

## Conceptual Correctness

The graphs-lower-graphs model is sound. Capabilities compose into tasks, tasks compose into task networks, and both levels use ready-set execution.

The typed planning substrate gives execution a formal way to evaluate goals and prepare compositions without interpreting world model semantics.

## Typed Planning Substrate

The typed substrate is ready for:

- evaluating goals against `WorldState`
- method trigger matching
- substitution
- composition validation
- effect application
- method-level goal satisfaction proof

This readiness comes from `meld-lang` and does not require task dispatch.

## Runtime Planning

The implemented runtime planning slice covers:

- execution goal contracts and lifecycle storage
- durable goal command outcome replay and source identity dedupe
- method library loading from JSON and in memory methods
- method verification for ids, trigger coverage, variable coverage, composition templates, and operator resolution diagnostics
- request and result contracts for planning
- goal satisfaction and indeterminate evaluation
- method candidate selection through trigger unification, precondition evaluation, cost ceiling filtering, and projected effect checks
- concrete composition substitution, validation, and execution composition handoff

Runtime planning is still blocked for task network execution by:

- task network graph executor
- task network command acceptance and mutation reduction
- plan diffing
- switching cost model
- durable planning attempt log
- durable method library identity in planning attempt records
- durable capability catalog path for runtime registered or synthesized capabilities
- outcome publication bridge
- workflow integration strategy

## Boundary Clarity

Execution planning owns goal evaluation, HTN method selection, composition preparation, task network mutation proposal decisions, conditional edge evaluation, observation waits, cost-aware plan transitions, and synthesis task insertion.

Execution planning does not own belief settlement, agent normative judgment, world model projection authority, event ledger authority, or provider implementation details.

## Dependency Readiness

Goals have a first slice runtime store for one ground active goal and durable lifecycle state.

World model planner is conditionally ready for one `WorldState`.

`meld-lang` is ready for typed-loop operations.

## First-Slice Feasibility

Execution planning supports the typed loop by evaluating one `Goal`, selecting one `Method`, validating one `Composition`, applying one `Effect`, and proving satisfaction.

No task network graph executor is required for this slice.

## Current Implementation Evidence

- `crates/meld-execution/src/goals.rs`
- `crates/meld-execution/src/goals/contracts.rs`
- `crates/meld-execution/src/goals/persistent_store.rs`
- `crates/meld-execution/src/goals/query.rs`
- `crates/meld-execution/src/goals/store.rs`
- `crates/meld-execution/src/planning.rs`
- `crates/meld-execution/src/planning/contracts.rs`
- `crates/meld-execution/src/planning/method_library.rs`
- `crates/meld-execution/src/planning/runtime.rs`
- `crates/meld-execution/src/planning/world_state.rs`
- `crates/meld-execution/src/task.rs`
- `crates/meld-execution/src/task/readiness.rs`
- `crates/meld-execution/src/task/runtime.rs`
- `crates/meld-execution/src/workflow.rs`
- `crates/meld-execution/src/workflow/executor.rs`
- `crates/meld-execution/tests/goals.rs`
- `crates/meld-execution/tests/method_library.rs`
- `crates/meld-execution/tests/planning_contracts.rs`
- `crates/meld-execution/tests/planning_runtime.rs`
- `tests/integration/task_executor.rs`
- `tests/integration/workflow_task_compatibility.rs`
- `tests/integration/workflow_cli.rs`
- `tests/integration/workflow_contracts_conformance.rs`
- `design/cognitive_architecture/execution/planning/planning_pipeline.md`
- `design/cognitive_architecture/execution/task_network.md`
- `design/cognitive_architecture/meld-lang/README.md`

## Gaps

- Task network graph executor is not specified enough for runtime implementation.
- Task network command acceptance and mutation reduction are not specified enough for runtime implementation.
- Execution runtime assembly has not selected the persistent goal store path.
- Agent goal command ingestion into execution goal storage is not wired.
- Planning attempts do not yet have an append only audit log.
- Method library identity is not recorded for replay.
- Runtime registered or synthesized capability contracts do not yet have durable catalog storage.
- Switching cost model is not specified.
- Plan diffing and affected-subtree selection are not specified.
- Outcome publication bridge remains open.

## Open Questions

- Which runtime graph executor structure lands first.
- Which first method bridges from typed composition to task compilation.
- Which runtime assembly path opens and owns execution storage.
- Which outcome fact shape closes belief revision after task execution.

## Recommendation

Proceed to execution composition lowering and task network mutation contracts. Keep goal storage authoritative and add planning attempt records as replayable audit data rather than as an authoritative plan store.

## Evidence note 2026-08-12

Implementation status moved from `design/cognitive_architecture/execution/planning/planning_pipeline.md`, which now carries only canonical pipeline semantics.

- Implemented: the task compiler, task executor, capability catalog, task lifecycle event builders, readiness computation, execution goal store, method library loading and verification, and the planning runtime that turns one active goal and one projected world state into an `ExecutionComposition`. The Phase 8 expanded execution slice lowers multi node compositions, materializes task init payloads, dispatches real task runs, and replays accepted graph state; see `design/plan/execution/task_network/PHASE8.md`.
- Designed but not fully implemented: exact subgoal realization where later planning expands the concrete child Composition or Method invocation resolved inside the authorized candidate; guard expressions as conditional dependency edges; observation wait semantics as data-flow dependencies from observation tasks; and the Phase 7 task network first slice in `design/plan/execution/task_network/PLAN.md` covering one inject mutation, one ready task, one dispatch, and one publication handoff.
- Deferred beyond the expanded slice: continuous planning loop operation with world-model reads and cost-aware mutation proposal decisions; task network deepening covering recursive sub-goal lowering plus cancel, relink, preserve, and prune; diff native sensory runtime until task network deepening has the required runtime hooks; the switching cost model covering cleanup estimation, sunk cost calculation, and benefit comparison; and plan diffing that identifies affected subtrees from belief changes and computes minimal mutations.
- Method library: the first slice supports serialized `meld-lang::Method` values with loading, verification, and the `docs_freshness` fixture as configured-path compatibility. Remaining: method authoring beyond docs freshness such as test status and course generation; method library operations covering indexing by trigger shape and cache invalidation on file change; and Method promotion, whether generalized Strategy candidates can become verified reusable Methods.
- Task network deepening: the Phase 7 and Phase 8 slices cover inject mutation, reduced state, ready set computation, dispatch claim fencing, multi node graph execution, task init materialization, the real task runtime bridge, durable publication handoff, and replay. Later work requires conditional edge evaluation with guard expressions on dependency edges, recursive sub-goal lowering, cancel, relink, preserve, and prune mutation behavior, and task equivalence for shared-task reuse across goals.
- Switching cost model: cost-aware plan transitions require cost estimates on tasks and a model for computing switching cost, including cleanup cost estimation, sunk cost of cancelled work, and benefit estimation of the new plan.
- Workflow compatibility: workflows short-circuit the planning loop; the user-authored workflow profile is the plan. A workflow profile lowers into an initially linear task network graph through task package lowering. As the planning loop matures it can produce graphs with parallelism, branching, and observation points, and the task network executor handles both identically. The existing workflow executor occupies the task network graph executor role as a specialized instance of the general pattern: workflow advances through turns, evaluates gates, and persists state, while task network execution advances through the ready set, evaluates conditional edges, and persists task network state.
