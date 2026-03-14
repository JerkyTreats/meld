# Task Model

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Define the workflow facing task model before advanced HTN composition work.
This document treats the task as the foundational unit for orchestration design.

## HTN Position

- a task is the workflow facing unit of execution and composition
- a primitive task is an atomic task with no further workflow decomposition
- a compound task exists only at the workflow planning layer and refines into other tasks
- a task network of one primitive task is the smallest executable workflow
- task types should be rich enough to feed a compiler style workflow planner rather than a loose runtime dispatcher

## Provisional Answers

### Atomic Task Criteria

- one owning domain operation
- no further workflow decomposition
- explicit input slots
- explicit output slots
- bounded side effect surface
- explicit idempotency class
- explicit capability requirements
- explicit artifact schema contracts
- explicit effect summary shape
- explicit compensation posture
- explicit observation shape
- explicit retry boundary
- explicit durable execution record

### Scope Kinds

- target scoped tasks operate on one resolved target or one deterministic target identity
- batch scoped tasks operate on a deterministic set of targets
- workflow scoped tasks operate once per workflow run regardless of target count

### Task Record Shape

- `task_type_id`
- `task_type_version`
- `scope_kind`
- `owner_domain`
- `input_slots`
- `output_slots`
- `artifact_input_contracts`
- `artifact_output_contracts`
- `side_effect_class`
- `idempotency_class`
- `determinism_class`
- `capability_requirements`
- `effect_summary_schema`
- `compensation_policy`
- `observation_schema`
- `retry_guidance`
- `timeout_guidance`

### Initial Atomic Task Families

- `ordering_task`
- `context_generate_task`
- `artifact_validate_task`
- `file_write_task`
- `workspace_refresh_task`
- `publish_record_task`

## Design Goal

Make workflow composition rest on a stable inventory of atomic tasks before method libraries and richer decomposition rules are introduced.
That inventory should already reflect the richer contract depth seen in modern HTN compiler and service stacks so later method work does not force a second task model redesign.

## Residual Questions

- which validation and publish tasks belong in the first phase and which should stay conceptual for now
- how narrow should first phase task boundaries be before task count becomes noisy rather than useful
- where should the first task catalog live relative to workflow config and workflow runtime code

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Primitive Task Contract](../primitive_task_contract/README.md)
- [Workflow Definition](../workflow_definition/README.md)
- [Migration Plan](../migration_plan/README.md)
- [HTN Codebase Structure Report](../../research/htn/README.md)
