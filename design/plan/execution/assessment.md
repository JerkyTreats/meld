# Execution Readiness Assessment

Status: conditionally ready
Depends on: `design/plan/meld-lang/assessment.md`, `design/plan/execution/goals/assessment.md`, `design/plan/execution/planning/assessment.md`
Design source: `design/cognitive_architecture/execution/README.md`, `design/cognitive_architecture/execution/GAPS.md`, `design/cognitive_architecture/execution/task_network.md`, `design/cognitive_architecture/execution/planning/README.md`, `design/cognitive_architecture/execution/goals/README.md`
Evidence date: 2026-05-31

## Verdict Summary

Execution is conditionally ready for goal data alignment and typed planning substrate.

Execution is blocked for runtime flywheel behavior until goal set storage, task network execution, runtime mutation, switching cost, outcome publication, and workflow integration contracts are specified.

## Conceptual Correctness

Execution owns operational commitment. It stores goals, evaluates desired state against `WorldState`, selects or constructs methods, compiles work into task structures, dispatches tasks, and publishes outcomes.

The boundary is sound because execution does not own belief semantics or normative judgment. Those remain world model and agent responsibilities.

## Completeness

The ready slice covers:

- goal data shape through `meld-lang::Goal`
- typed evaluation through `meld-lang::evaluate`
- method matching through `meld-lang::unify`
- composition preparation through `meld-lang::substitute` and `meld-lang::validate`
- projected effect application through `meld-lang::apply_effects`

The runtime slice is incomplete. Task network graph execution, mutation acceptance, plan diffing, switching cost, capability catalog bridge, outcome publication, and workflow integration need concrete contracts.

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
- Task network state is still unimplemented. When Phase 7 lands, the authoritative persistent shape should be the task network mutation log and reduced network state, not a separate ad hoc plan store.
- Outcome publication needs durable handoff state so completed task outputs are not lost between task completion and world model ingestion.

The storage policy applies to all of these concerns: runtime state must stay outside the target workspace path, including fallback paths and lock files.

## Boundary Clarity

Execution owns goal storage, lifecycle mutation, typed planning evaluation, runtime planning loop, task network graph, task compilation, dispatch, and outcome publication.

Execution does not own graph truth, belief confidence, causal inference, regime identity, agent curation policy, event ledger authority, or provider implementation details.

## Dependency Readiness

`meld-lang` is ready for typed-loop implementation.

Execution goals are design ready for one goal set and one ground active goal.

Execution planning is ready only as typed substrate. Runtime planning remains blocked.

## First-Slice Feasibility

Execution can participate in the typed-loop design path by storing one `Goal`, evaluating it against one `WorldState`, selecting one `Method`, validating one `Composition`, applying one `Effect`, and proving satisfaction.

No task dispatch is required for this slice.

Runtime goal storage and planning orchestration are not implemented.

## Current Implementation Evidence

- `crates/meld-execution/src/task.rs`
- `crates/meld-execution/src/task/readiness.rs`
- `crates/meld-execution/src/task/runtime.rs`
- `crates/meld-execution/src/workflow.rs`
- `crates/meld-execution/src/workflow/`
- `tests/integration/task_executor.rs`
- `tests/integration/workflow_task_compatibility.rs`
- `design/cognitive_architecture/execution/README.md`
- `design/cognitive_architecture/execution/GAPS.md`
- `design/cognitive_architecture/execution/task_network.md`

## Gaps

- Task network graph executor is not specified enough for implementation.
- Goal set durable storage exists, but runtime assembly has not selected the persistent store path.
- Agent goal command ingestion into durable goal storage is not wired.
- Task executor, task artifact repository, capability invocation attempts, task expansion records, and task events are still process local.
- Graph mutation acceptance is not specified enough for runtime planning.
- Plan diffing and affected-subtree selection are not specified.
- Switching cost model is not specified.
- Outcome publication bridge from task completion to world-model-legible facts is not specified.
- Workflow integration strategy remains deferred.

## Open Questions

- Which execution module owns the method library loader.
- Which runtime assembly path owns the shared execution storage root.
- Whether workflow JSON state remains a compatibility store or migrates behind a shared execution store adapter.
- Which task result shape becomes the first outcome publication bridge.
- Which workflow subsystem maps first into task network runtime.

## Recommendation

Proceed with typed-loop execution participation. Defer runtime flywheel implementation until runtime planning and outcome publication contracts are specified.
