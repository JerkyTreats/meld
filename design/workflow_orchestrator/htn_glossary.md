# HTN Glossary

Date: 2026-03-11
Status: active

## Intent

Define the shared HTN aligned vocabulary for workflow orchestration work.
This glossary keeps the active design set consistent while Meld builds the foundation that will later support hierarchical workflow execution.

## Core Terms

- `top_level_task` — the single declared unit of intent a workflow run is asked to complete
- `compound_task` — a task that cannot execute directly and must be refined through one method
- `primitive_task` — a directly executable workflow task backed by one domain operation
- `method` — a named refinement rule that decomposes one compound task into an ordered or partially ordered task network
- `task_network` — the compiled graph of task instances, dependencies, artifact handoffs, and target scope derived from one method choice
- `task_instance` — one concrete occurrence of either a compound task or a primitive task within one workflow run
- `task_type` — the stable task definition used by workflow planning and validation before one task instance exists
- `checkpoint` — a durable record that marks a safe resume boundary after validated planning or successful execution work
- `repair_record` — a durable record describing why execution diverged, what state was preserved, and what recovery decision was taken
- `observation` — structured runtime facts returned by a primitive task that may influence downstream execution or later repair
- `artifact_handoff` — the explicit binding from one task output slot to another task input slot
- `scope_digest` — a deterministic summary of the target set or repository slice used when compiling or validating a plan

## Meld Mapping

- workflow owns `top_level_task`, `compound_task`, `method`, `task_network`, `checkpoint`, and `repair_record`
- primitive tasks are the workflow facing execution layer
- domain modules own the underlying operations used by primitive tasks
- workflow runtime executes a compiled task network and records durable state for resume, repair, and audit

## Hard Rules

- only `src/workflow` owns decomposition and task network planning
- only primitive task contracts define workflow facing atomic execution boundaries
- `src/context` does not own global orchestration policy
- artifact handoff must be explicit at workflow compile time
- side effecting primitive tasks must declare idempotency and write scope expectations before runtime starts

## Related Areas

- [Workflow Orchestrator Roadmap](README.md)
- [Task Model](task_model/README.md)
- [Primitive Task Contract](primitive_task_contract/README.md)
- [Workflow Definition](workflow_definition/README.md)
- [Telemetry Model](telemetry_model/README.md)
