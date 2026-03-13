# Workflow Orchestrator Roadmap

Date: 2026-03-11
Status: active
Scope: HTN ready workflow foundation and task orchestration

## Entry Point

This is the primary entry point for the workflow orchestrator effort.
Use this file to understand the HTN aligned architectural framing, the major requirement areas, and the reading order for active design work.
Each requirement area lives in its own folder with a workload `README.md` as the anchor document.

This roadmap carries forward the workflow direction after the workflow bootstrap milestone.
Bootstrap design history now lives in [Completed Workflow Bootstrap](../completed/workflow_bootstrap/README.md).

## Shared Vocabulary

- [HTN Glossary](htn_glossary.md)

## Working Direction

- workflow orchestrates tasks
- domains own their own behavior and internal operations
- tasks define the workflow facing execution units that contract with domain operations
- task modeling comes before advanced HTN composition
- ordering, generation, validation, and file materialization should be expressed as atomic task families
- workflow runtime owns task network compilation, checkpoints, repair state, and cross task telemetry
- workflow configuration must compile into a reliable task network before runtime starts

## HTN Direction

- workflow owns top level intent, method selection, decomposition, and compiled task networks
- primitive tasks are the atomic execution layer beneath workflow composition
- domains remain named by domain concern rather than HTN concern
- current workflows and commands should migrate through compatibility task networks rather than a disruptive rewrite
- this phase builds HTN readiness first and leaves method learning and broad planner search for later work

## Requirement Areas

1. [Task Model](task_model/README.md)
2. [Primitive Task Contract](primitive_task_contract/README.md)
3. [Ordering Task](ordering_task/README.md)
4. [Context Generate Task](context_generate_task/README.md)
5. [File Write Task](file_write_task/README.md)
6. [Write Policy](write_policy/README.md)
7. [Agent Binding](agent_binding/README.md)
8. [Workflow Definition](workflow_definition/README.md)
9. [Telemetry Model](telemetry_model/README.md)
10. [Migration Plan](migration_plan/README.md)

## Outcome Target

Deliver a workflow architecture where orchestration is a first class domain layer above domain operations and where the resulting foundation can host HTN style decomposition without another architectural reset.
The workflow layer should compile declarative task networks, validate task wiring, persist durable run state, and coordinate artifact flow across ordering, generation, validation, and file materialization.

## Why This Work Matters

- task boundaries become explicit enough to support atomic execution, retry, reuse, and repair
- configuration becomes easier to reason about because workflow composes known task types rather than hidden domain internals
- runtime reliability improves when invalid task networks fail at load time rather than during partial execution
- reuse improves when ordering, generation, validation, and publishing can be recombined without copying logic
- migration risk drops when current command paths become compatibility task networks behind a stable workflow interface
- HTN adoption risk drops when decomposition can later build on stable task and state contracts

## Reading Order

1. [HTN Glossary](htn_glossary.md)
2. [Task Model](task_model/README.md)
3. [Primitive Task Contract](primitive_task_contract/README.md)
4. [Ordering Task](ordering_task/README.md)
5. [Context Generate Task](context_generate_task/README.md)
6. [File Write Task](file_write_task/README.md)
7. [Write Policy](write_policy/README.md)
8. [Agent Binding](agent_binding/README.md)
9. [Workflow Definition](workflow_definition/README.md)
10. [Telemetry Model](telemetry_model/README.md)
11. [Migration Plan](migration_plan/README.md)

## Initial Architectural Position

- `src/workflow` should own task modeling, task resolution, task network compilation, workflow runtime, and durable workflow state
- `src/context` should own context artifact production and retrieval through explicit task contracts
- ordering policy should move out of `src/context` and become reusable by commands and workflows as a task family
- file write and publish behavior should become task owned rather than an incidental side effect of context workflows
- the current turn manager should be treated as a compatibility workflow shape rather than the permanent shape of workflow itself

## Immediate Deliverables

- define the task model and atomic task criteria
- define the primitive task contract and task catalog shape
- define ordering and file write as reusable task families
- define workflow composition rules above tasks
- define a compatibility path from the current turn workflow runtime

## What This Phase Does Not Build Yet

- automated method learning
- broad search across arbitrary action libraries
- full uncertainty policy synthesis
- a large workflow authoring language before task contracts and runtime records are stable

## Related Design History

- [Completed Workflow Bootstrap](../completed/workflow_bootstrap/README.md)
- [Publish Arbiter Idea](../workflow_ideas/publish_arbiter_spec.md)
