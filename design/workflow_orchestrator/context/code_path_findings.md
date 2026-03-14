# Context Code Path Findings

Date: 2026-03-14
Scope: current code baseline findings for HTN ready `src/context` refactor

## Intent

Capture concrete code findings for the `src/context` orchestration split before refactor execution.
This document records the current seams, the strongest reuse boundaries, and the main gaps that still block cleaner HTN integration.

## Source Specs

- [Context Refactor Requirements](README.md)
- [Context Generate Task](../context_generate_task/README.md)
- [Workflow Definition](../workflow_definition/README.md)

## Baseline Findings

### C1 `src/context/generation/run.rs` still owns recursive target planning

Current state:
- `find_missing_descendant_heads` performs subtree readiness checks in `src/context/generation/run.rs`
- `collect_subtree_levels` derives bottom up execution levels in `src/context/generation/run.rs`
- `build_plan` decides recursive versus non recursive behavior, skip reuse behavior, plan ids, and level contents in `src/context/generation/run.rs`

Impact:
- context still mixes planning policy with generation execution setup
- workflow cannot yet fully own target expansion and ordering without duplicating context logic

### C2 `GenerationPlan` is a useful compatibility envelope but not yet a workflow compiled artifact

Current state:
- `GenerationPlan` stores `plan_id`, `source`, `session_id`, `levels`, `priority`, and `failure_policy` in `src/context/generation/plan.rs`
- `GenerationItem` stores execution details such as target path, agent, provider, frame type, force flag, and execution program in `src/context/generation/plan.rs`
- current plan data does not declare explicit dependency edges, scope digest, artifact type ids, or schema bindings

Impact:
- the current plan shape is strong enough for compatibility execution
- the current plan shape is not yet rich enough for workflow compilation to validate target plan artifacts and downstream bindings before runtime starts

### C3 `src/context/queue.rs` still owns execution mode branching

Current state:
- `process_request` branches on `TargetExecutionProgramKind` in `src/context/queue.rs`
- workflow backed requests are translated into `TargetExecutionRequest` and sent through `src/workflow/facade.rs`
- single shot requests are sent to `execute_generation_request` in `src/context/generation/orchestration.rs`

Impact:
- queue execution remains aware of workflow versus non workflow policy
- the atomic generation task family does not yet have one uniform dispatch contract below the queue layer

### C4 A clear workflow public seam already exists and should be preserved

Current state:
- `resolve_target_execution_program` maps agent workflow binding to execution mode in `src/context/generation/selection.rs`
- `TargetExecutionProgram`, `TargetExecutionRequest`, and `TargetExecutionResult` define the main public bridge in `src/context/generation/program.rs`
- `build_target_execution_request` and workflow target execution live in `src/workflow/facade.rs`

Impact:
- the codebase already has a useful public contract boundary between workflow and context
- future HTN integration should extend this seam rather than let `src/context` reach into workflow internals

### C5 Plan lineage and workflow lineage are still partial in execution telemetry

Current state:
- `TargetExecutionRequest` includes `plan_id`, `session_id`, and `level_index` in `src/context/generation/program.rs`
- workflow facade forwards `plan_id` and `level_index` into workflow execution in `src/workflow/facade.rs`
- `process_request` currently passes `None` for `level_index` in `src/context/queue.rs`
- atomic generation telemetry in `src/context/generation/orchestration.rs` emits `workflow_id`, `thread_id`, `turn_id`, `turn_seq`, `plan_id`, and `level_index` as `None`

Impact:
- execution records do not yet carry full target level lineage through all generation paths
- downstream repair, artifact handoff, and audit surfaces still lack part of the planned workflow context

### C6 Retry policy remains queue local and partly string matched

Current state:
- `is_retryable_error` branches by execution program kind in `src/context/queue.rs`
- workflow retryability is currently inferred by checking error message content such as `failed gate` in `src/context/queue.rs`
- single shot retryability is also still decided in the queue layer rather than in a typed task or workflow policy contract

Impact:
- retry and repair policy are not yet first class workflow inputs
- message based classification increases drift risk as runtime behavior evolves

### C7 `execute_generation_request` is the strongest domain owned atomic seam

Current state:
- `execute_generation_request` in `src/context/generation/orchestration.rs` owns prompt collection, provider preparation, metadata construction, metadata validation, model execution, frame creation, and frame persistence
- this path does real context work without needing to own recursive target derivation or workflow method choice

Impact:
- this is the main domain boundary to preserve during HTN integration
- refactor work should move planning and workflow policy around this seam rather than rewrite the seam itself

## Test Coverage Findings

### T1 Compatibility coverage exists for program selection and plan execution shape

Current state:
- `src/context/generation/program.rs` tests cover workflow and single shot program construction
- `src/context/generation/selection.rs` tests cover workflow binding selection
- `src/context/generation/executor.rs` tests cover execution behavior differences for workflow backed items such as wait timeout behavior

Impact:
- the basic compatibility bridge already has unit coverage and is a good baseline for refactor safety

### T2 Integration coverage proves context level planning still drives workflow backed generation

Current state:
- `tests/integration/progress_observability.rs` includes `context_generate_with_workflow_agent_uses_context_plan_levels`
- that test verifies `plan_constructed`, workflow events, and metadata validation events in one command session

Impact:
- there is already evidence that context owned plan levels still shape workflow backed execution today
- this test should remain a parity gate during the planning split

### T3 Coverage gaps remain for target plan artifacts and workflow lineage propagation

Needed coverage:
- validate typed target plan inputs once workflow owns ordering output
- verify `level_index` and related lineage fields survive queue to workflow dispatch
- replace retry message matching with typed failure classification coverage

## Governance Coverage Findings

### G1 The complex workflow policy does not currently represent this README structure

Current state:
- `governance/complex_change_workflow.md` requires one `PLAN` document when complex workflow mode is active
- the policy defines required `PLAN` sections such as overview, phases, tasks, seams, gates, order summary, related links, and exception list
- the policy does not define a README index, workload README layout, or a `code_path_findings.md` companion artifact

Impact:
- the new `design/workflow_orchestrator/context/README.md` structure is useful but not formally represented in the current governance policy
- reviewers cannot rely on governance alone to expect this README based decomposition

### G2 There is partial conceptual overlap with policy goals

Current state:
- the `context` README family already captures intent, scope, work items, code pressure, exit shape, and related links
- these sections overlap with policy concepts such as objective, key seams, outcome, and documentation links

Impact:
- the structure aligns with the spirit of complex workflow documentation
- the structure is still outside the current formal policy vocabulary because policy covers `PLAN` content only

## Refactor Order Suggested By Current Code

1. preserve `execute_generation_request` as the atomic context seam
2. extract target expansion and ordering out of `src/context/generation/run.rs`
3. turn `GenerationPlan` into a clearer workflow consumable target plan contract
4. move retry and repair policy out of queue local string checks
5. strengthen lineage propagation for `plan_id`, `level_index`, and workflow execution metadata
6. add typed artifact and binding coverage for downstream workflow tasks

## Exit Signals

- `src/context/generation/run.rs` no longer derives target graphs internally
- workflow submits typed target plans into context execution
- queue dispatch no longer hides workflow policy decisions that belong in compile time or runtime workflow state
- execution telemetry carries enough plan and workflow lineage for repair and audit
- governance either stays intentionally `PLAN` only or is updated to recognize README workload indexes and findings docs
