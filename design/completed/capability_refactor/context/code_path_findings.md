# Context Code Path Findings

Date: 2026-04-03
Status: completed
Scope: current code baseline for the `src/context` domain as it exists today

## Intent

Capture how `context` actually works in the current codebase.
This document is descriptive, not aspirational.
Its purpose is to show where control lives today, where workflow behavior is inverted relative to the target design, and which seams are worth preserving for a future capability and task layer.

## Source Set

- [Context Capability Readiness](README.md)
- [Capability And Task Design](../README.md)
- [Domain Architecture](../domain_architecture.md)
- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)
- [plan.rs](/home/jerkytreats/meld/src/context/generation/plan.rs)
- [executor.rs](/home/jerkytreats/meld/src/context/generation/executor.rs)
- [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs)
- [queue.rs](/home/jerkytreats/meld/src/context/queue.rs)
- [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs)
- [program.rs](/home/jerkytreats/meld/src/context/generation/program.rs)
- [workflow facade](/home/jerkytreats/meld/src/workflow/facade.rs)
- [progress observability test](/home/jerkytreats/meld/tests/integration/progress_observability.rs)

## Current Domain Shape

Today `context` is not only a domain behavior provider.
It is also the main host for target resolution, compatibility planning, queue startup, execution mode choice, and part of workflow routing.

The public surface in [context.rs](/home/jerkytreats/meld/src/context.rs) and [facade.rs](/home/jerkytreats/meld/src/context/facade.rs) exports:

- `GenerationPlan`
- `GenerationExecutor`
- `TargetExecutionProgram`
- `TargetExecutionRequest`
- queue types and queue options

That means the domain boundary already includes both atomic generation behavior and a compatibility execution envelope.

## Baseline Findings

### C1 `run_generate` is the real top-level orchestrator for context generation

Current state:

- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs) resolves node identity from path or node id
- it resolves the agent and validates provider presence
- it chooses execution mode through direct `workflow_id` input or agent workflow binding
- it computes recursive behavior for directory targets
- it calls `build_plan`
- it emits `plan_constructed`
- it creates a Tokio runtime, creates the queue, starts the queue, and runs `GenerationExecutor`

Impact:

- `context` currently owns the main command-side orchestration path for generation
- the domain is not yet reduced to atomic behavior behind a capability contract
- compatibility planning and execution startup still live inside the domain entry path

### C2 `build_plan` is context-owned target derivation and ordering

Current state:

- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs) contains `find_missing_descendant_heads`
- it also contains `collect_subtree_levels`
- `build_plan` derives bottom-up subtree levels for recursive generation
- `build_plan` enforces descendant head checks for non-recursive directory generation
- `build_plan` decides head reuse skip behavior
- `build_plan` assigns `plan_id`, `source`, `priority`, `failure_policy`, `total_nodes`, and `total_levels`

Impact:

- target graph shape is currently derived inside `context`
- ordering is level-based and produced before queue or workflow execution begins
- a future task compiler cannot own target derivation cleanly until this logic moves or is wrapped as explicit compatibility input

### C3 `GenerationPlan` is the actual compatibility execution envelope

Current state:

- [plan.rs](/home/jerkytreats/meld/src/context/generation/plan.rs) defines `GenerationPlan`
- that shape is level-based rather than dependency-edge based
- each `GenerationItem` carries node id, path, agent, provider, frame type, force, and `TargetExecutionProgram`
- `GenerationPlan` carries `plan_id`, `session_id`, `levels`, `priority`, `failure_policy`, `target_path`, `total_nodes`, and `total_levels`
- [executor.rs](/home/jerkytreats/meld/src/context/generation/executor.rs) consumes this type directly

Impact:

- the current durable compatibility contract is a context-owned execution envelope
- this shape is strong enough for current queue and workflow execution
- this shape is not yet a compiled task artifact with explicit dependency edges, artifact contracts, or capability instance records

### C4 Workflow control is inverted relative to the target design

Current state:

- [selection.rs](/home/jerkytreats/meld/src/context/generation/selection.rs) chooses workflow mode from agent workflow binding
- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs) also allows explicit `workflow_id` override on the request
- [executor.rs](/home/jerkytreats/meld/src/context/generation/executor.rs) still iterates context-owned plan levels even when the program kind is `workflow`
- [queue.rs](/home/jerkytreats/meld/src/context/queue.rs) branches on `TargetExecutionProgramKind::Workflow`
- for workflow items, the queue builds a target execution request through [workflow facade](/home/jerkytreats/meld/src/workflow/facade.rs)
- that request includes `plan_id` and `level_index`
- the workflow layer then executes as a downstream branch of the context-owned plan

Impact:

- workflow is not the outer controller of `context`
- `context` decides the execution envelope first, then routes one branch into workflow
- this is the opposite of the target direction where a higher-order task or control layer should decide multi-step structure and then call atomic domain capabilities

### C5 Queue owns both transport and execution mode policy

Current state:

- [queue.rs](/home/jerkytreats/meld/src/context/queue.rs) deduplicates requests by node, agent, provider fingerprint, frame type, and program
- `GenerationRequestOptions` includes `plan_id`
- queue ordering gives requests with `plan_id` a higher rank than otherwise equal requests
- `process_request` branches between direct orchestration and workflow execution
- `is_retryable_error` has workflow-specific handling
- workflow retryability is inferred from message text such as `failed gate`

Impact:

- queue behavior is not only transport
- queue still owns durable execution policy through program-kind branching and retry classification
- workflow compatibility currently reaches deeply into queue semantics

### C6 `execute_generation_request` is the strongest atomic context seam

Current state:

- [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs) owns prompt collection
- it prepares provider execution
- it prepares prompt-context lineage
- it builds and validates metadata
- it executes the completion
- it writes the resulting frame

Impact:

- this is the cleanest domain seam to preserve during refactor
- it performs real context work without needing subtree traversal, level planning, or workflow turn selection
- future `ContextGeneratePrepare` and `ContextGenerateFinalize` contracts should anchor on this seam rather than re-center the domain around `run_generate`

### C7 Telemetry and lineage are compatibility-first and still partial

Current state:

- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs) emits `plan_constructed`
- [executor.rs](/home/jerkytreats/meld/src/context/generation/executor.rs) emits `generation_started`, `level_started`, `node_generation_started`, `node_generation_completed`, and related level events
- [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs) emits metadata validation events
- those atomic metadata events currently set `workflow_id`, `plan_id`, and `level_index` to `None`
- workflow execution carries `plan_id` and `level_index` through [workflow facade](/home/jerkytreats/meld/src/workflow/facade.rs)

Impact:

- the compatibility layer already has plan and level lineage
- the atomic context seam does not yet preserve that lineage all the way through
- a future task layer will need explicit lineage propagation rather than best-effort compatibility fields

### C8 Tests confirm that context owns the current multi-step shape

Current state:

- [progress observability test](/home/jerkytreats/meld/tests/integration/progress_observability.rs) includes `context_generate_plan_constructed_includes_path_field`
- the same file includes `context_generate_with_workflow_agent_uses_context_plan_levels`
- that workflow-backed test expects `plan_constructed`
- it expects context-owned `total_nodes` and `total_levels`
- it expects `program_kind` to be `workflow`
- it expects workflow events to happen after the context-owned planning events

Impact:

- the tests verify the current inversion of control
- parity work must treat these tests as evidence of how the system behaves today
- a refactor that assumes workflow already owns multi-step planning would misread the actual codebase

## Current Interpretation

The current `context` domain has three distinct layers mixed together:

- atomic generation behavior
- compatibility planning and execution envelope creation
- workflow-aware routing and queue policy

The atomic seam is viable.
The surrounding host logic is the part that conflicts with the capability and task design.

## Refactor Implications

The code suggests these refactor rules:

1. Preserve `execute_generation_request` as the atomic context seam.
2. Treat `GenerationPlan` as a compatibility envelope, not as the future durable task artifact.
3. Move target derivation and ordering out of `run_generate` before claiming task compilation owns graph structure.
4. Remove workflow branching from queue only after a replacement task input path exists.
5. Preserve `TargetExecutionRequest` style bridge shapes only as temporary compatibility inputs if they still help migration.
6. Do not assume workflow currently controls context.
Current code shows the reverse.

## Exit Signals For A Future Refactor

- `context` no longer derives subtree ordering internally
- queue no longer branches on workflow program kind
- workflow no longer receives `plan_id` and `level_index` from a context-owned planner
- atomic generation telemetry carries explicit upstream task lineage when present
- public `context` exports center on capability-facing contracts rather than compatibility plan types
