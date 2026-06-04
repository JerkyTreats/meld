# Execution Readiness Assessment

Status: conditionally ready for Phase 8 expanded execution after hardening
Depends on: `design/plan/meld-lang/assessment.md`, `design/plan/execution/goals/assessment.md`, `design/plan/execution/planning/assessment.md`
Design source: `design/cognitive_architecture/execution/README.md`, `design/cognitive_architecture/execution/GAPS.md`, `design/cognitive_architecture/execution/task_network.md`, `design/cognitive_architecture/execution/task_initialization.md`, `design/cognitive_architecture/execution/planning/README.md`, `design/cognitive_architecture/execution/goals/README.md`
Evidence date: 2026-06-04

## Verdict Summary

Execution is conditionally ready for the Phase 8 expanded execution slice after the dedicated hardening task closes the current test and clippy gates.

The Phase 7 task network base now defines command acceptance, mutation reduction, reduced graph state, ready set computation, fenced dispatch claims, and durable outcome publication handoff for the first slice.

The next viable change is expanded execution rather than sensory. Phase 8 adds multi node composition lowering, atomic graph commit, task init source planning, data flow materialization, real task runtime dispatch, and replay over the expanded graph.

Execution remains incomplete for runtime assembly, direct agent command ingestion, broad workflow migration, recursive sub-goal lowering, plan diffing, switching cost, and later cancellation or preservation behavior.

## Conceptual Correctness

Execution owns operational commitment. It stores goals, evaluates desired state against `WorldState`, selects or constructs methods, compiles work into task structures, dispatches tasks, and publishes outcomes.

The boundary is sound because execution does not own belief semantics or normative judgment. Those remain world model and agent responsibilities.

## Completeness

The execution planning base covers:

- goal data shape through `meld-lang::Goal`
- typed evaluation through `meld-lang::evaluate`
- method matching through `meld-lang::unify`
- composition preparation through `meld-lang::substitute` and `meld-lang::validate`
- projected effect application through `meld-lang::apply_effects`

The Phase 7 first runtime slice has concrete contracts in `design/plan/execution/task_network/PLAN.md`.

The Phase 8 expanded execution slice has concrete contracts in `design/plan/execution/task_network/PHASE8.md`.

The runtime slice is still incomplete beyond that expanded slice. Recursive sub-goal lowering, plan diffing, switching cost, runtime capability catalog persistence, sensory runtime, and workflow integration remain deferred.

## Persistent Storage Concerns

Existing execution functionality has more durable state needs than goal lifecycle storage.

Goal storage is the right first persistent store because it carries authoritative operational commitment and command idempotency. The wider execution runtime still needs persistent stores or explicit replay contracts for state that is currently process local.

The missing persistent storage concerns are:

- Runtime assembly must choose and own an execution storage root outside the target workspace, then pass a shared database handle to execution stores.
- Goal command ingestion must use durable command outcome replay and source identity dedupe for every `AgentGoalCommand` path, not only direct store calls.
- Planning attempts should have an append only decision log keyed by goal id, world state frame, method library identity, selected method, diagnostics, and result. This is not authoritative state, but it is needed for audit, replay, and plan debugging.
- Method library loading can remain file backed for authored methods, but verified method snapshots need an identity hash in planning attempt records so later replays can tell which library version selected a composition.
- Capability catalog registration is in memory. Compiled built in capability contracts can reload from code, but synthesized or runtime registered capability contracts need the runtime catalog described by the synthesis design before they can survive restart.
- Task executor state is live memory today. `TaskExecutor` owns compiled task shape after expansion, init payload, artifact repo contents, invocation records, applied expansion ids, completed instances, in flight instances, and task events. Durable record types exist, but there is no executor snapshot or event sourced store that can resume a task run.
- Task artifact repository state is in memory today. Artifacts and artifact links have durable record shapes, but emitted task artifacts are lost unless copied into a wider store before process exit.
- Capability invocation attempts are in memory today. Attempt ids, supplied inputs, emitted artifacts, and failure summaries must be persisted before dispatch or committed through an invocation journal to avoid duplicate external work after restart.
- In flight capability dispatch needs a recovery policy. A restart must decide whether queued or session class invocations are resumed, queried from an external provider, cancelled, or marked unknown.
- Task expansion application needs durable idempotency. Applied expansion ids and expansion records must persist with the task run so replay does not duplicate dynamic capability instances or dependency edges.
- Task events are emitted from process memory and published opportunistically. Event publication needs an outbox or replay cursor if execution events are expected to survive publication failure.
- Workflow state already has file backed thread, turn, gate, and prompt link records. It still needs a migration path into the shared execution storage root if the long term runtime model expects one persistence substrate rather than mixed JSON files and sled trees.
- Task network state has a first slice shape. Phase 8 must expand it to multi node graphs, init source plans, data flow materialization, and replay over real task outcomes.
- Outcome publication needs durable handoff state so completed task outputs are not lost between task completion and world model ingestion.

The storage policy applies to all of these concerns: runtime state must stay outside the target workspace path, including fallback paths and lock files.

## Boundary Clarity

Execution owns goal storage, lifecycle mutation, typed planning evaluation, runtime planning loop, task network graph, task compilation, dispatch, and outcome publication.

Execution does not own graph truth, belief confidence, causal inference, regime identity, agent curation policy, event ledger authority, or provider implementation details.

## Dependency Readiness

`meld-lang` is ready for typed-loop implementation.

Execution goals are design ready for one goal set and one ground active goal.

Execution planning has implemented the first runtime handoff to `ExecutionComposition`.

Task network execution is ready to expand after hardening because the Phase 7 base now specifies and implements the command, mutation, state, storage, readiness, dispatch, and publication contracts needed by Phase 8.

## Next Slice Feasibility

Execution can participate in the typed-loop design path by storing one `Goal`, evaluating it against one `WorldState`, selecting one `Method`, validating one `Composition`, applying one `Effect`, and proving satisfaction.

Execution is ready to start the next implementation slice that lowers one `ExecutionComposition` into an accepted multi node task network graph, materializes task init payloads, dispatches real task runs, and replays the graph state across reopen.

Runtime assembly, direct world model agent command ingestion, and sensory runtime remain outside that expanded execution slice.

## Current Implementation Evidence

- `crates/meld-execution/src/task.rs`
- `crates/meld-execution/src/task/readiness.rs`
- `crates/meld-execution/src/task/runtime.rs`
- `crates/meld-execution/src/workflow.rs`
- `crates/meld-execution/src/workflow/`
- `tests/integration/task_executor.rs`
- `tests/integration/workflow_task_compatibility.rs`
- `design/plan/execution/task_network/PLAN.md`
- `design/plan/execution/task_network/PHASE8.md`
- `design/cognitive_architecture/execution/task_initialization.md`
- `design/cognitive_architecture/execution/README.md`
- `design/cognitive_architecture/execution/GAPS.md`
- `design/cognitive_architecture/execution/task_network.md`

## Gaps

- Current task network hardening gates still need closure before Phase 8 starts.
- Goal set durable storage exists, but runtime assembly has not selected the persistent store path.
- Agent goal command ingestion into durable goal storage is not wired.
- Task executor, task artifact repository, capability invocation attempts, task expansion records, and task events are still process local.
- Multi node composition lowering is not implemented.
- Task init source planning and data flow materialization are not implemented.
- Real task runtime dispatch from task network claims is not fully proven in the expanded graph slice.
- Plan diffing and affected-subtree selection are not specified.
- Switching cost model is not specified.
- Workflow integration strategy remains deferred.

## Open Questions

- Which runtime assembly path owns the shared execution storage root.
- Whether workflow JSON state remains a compatibility store or migrates behind a shared execution store adapter.
- Which workflow subsystem maps first into task network runtime.
- Which artifact contract validators should be registered first for Phase 8 data flow materialization.
- Whether planning attempt audit records should land before or after expanded execution.

## Recommendation

Proceed with the dedicated hardening task first. Then implement Phase 8 expanded execution from `design/plan/execution/task_network/PHASE8.md`. Keep sensory, plan diffing, switching cost, cancellation, preservation, and workflow migration deferred beyond the expanded execution slice.
