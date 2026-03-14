# Workflow Contract Boundary

Date: 2026-03-14
Status: active

## Parent Requirements

- [Context Refactor Requirements](../README.md)
- [Workflow Definition](../../workflow_definition/README.md)

## Intent

Keep `src/context` dependent only on workflow public contracts.
The context domain should never reach into workflow internals to recover orchestration state.

## Current Pressure

- `TargetExecutionProgram` already gives `context` a small public contract for execution mode choice
- future HTN integration will increase pressure to pass richer workflow data into context execution
- without a firm boundary, context could start depending on workflow internal records, task graph details, or runtime storage models

## Requirements

- all workflow to context integration must flow through explicit public contract types or facades
- context must not import workflow internal execution records, planner state, or storage details
- new workflow data passed into context must stay limited to execution relevant bindings and typed artifacts
- boundary changes must favor additive contract evolution over domain leakage

## Planned Deliverables

- a documented contract surface for workflow backed context execution
- boundary rules for request data, result data, and telemetry handoff
- migration cleanup steps that remove transitional shortcuts after workflow facades are complete

## Verification Focus

- `src/context` compiles against workflow public seams only
- workflow internal refactors do not force context domain rewrites
- telemetry and execution summaries cross the boundary without leaking internal runtime models

## Related Code

- `src/context/generation/program.rs`
- `src/context/generation/selection.rs`
- `src/context/generation/contracts.rs`
- `src/workflow`
