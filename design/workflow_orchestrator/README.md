# Workflow Orchestrator Roadmap

Date: 2026-03-09
Status: active
Scope: HTN ready workflow foundation and capability orchestration

## Entry Point

This is the primary entry point for the workflow orchestrator effort.
Use this file to understand the HTN aligned architectural framing, the major requirement areas, and the reading order for active design work.
Each requirement area lives in its own folder with a workload `README.md` as the anchor document.

This roadmap carries forward the capability orchestration direction after the workflow bootstrap milestone.
Bootstrap design history now lives in [Completed Workflow Bootstrap](../completed/workflow_bootstrap/README.md).

## Shared Vocabulary

- [HTN Glossary](htn_glossary.md)

## Working Direction

- workflow orchestrates capabilities and prepares the foundation for hierarchical task execution
- capabilities are the primitive task layer rather than the owner of global orchestration
- context generation is one primitive task family rather than the owner of global orchestration
- target ordering is a reusable primitive capability rather than a hidden rule inside `context generate`
- file materialization is a separate primitive capability with explicit write policy
- workflow runtime owns decomposition records, artifact handoff, resume state, repair state, and cross capability telemetry
- workflow configuration must compile into a reliable task network before runtime starts

## HTN Direction

- workflow owns top level intent, method selection, decomposition, and compiled task networks
- capabilities provide primitive task execution only
- durable workflow state must support checkpoints, repair records, and artifact lineage
- the current turn manager is a compatibility shape that should map into the future hierarchical model rather than define its long term limits
- this phase builds HTN readiness first and leaves broad planner search for later work if it becomes necessary

## Requirement Areas

1. [Workflow Definition](workflow_definition/README.md)
2. [Capability Contract](capability_contract/README.md)
3. [Ordering Capability](ordering_capability/README.md)
4. [Context Generate Integration](context_generate_integration/README.md)
5. [File Write Capability](file_write_capability/README.md)
6. [Write Policy](write_policy/README.md)
7. [Agent Chaining](agent_chaining/README.md)
8. [Telemetry Model](telemetry_model/README.md)
9. [Migration Plan](migration_plan/README.md)

## Outcome Target

Deliver a workflow architecture where orchestration is a first class domain layer above individual capabilities and where the resulting foundation can host HTN style decomposition without another architectural reset.
The workflow layer should compile declarative task networks, validate capability wiring, persist durable run state, and coordinate artifact flow across ordering, generation, validation, and file materialization.

## Why This Work Matters

- workflow design becomes broader than the first docs writer thread profile
- capability boundaries become explicit enough to serve as primitive task contracts
- runtime reliability improves when invalid task networks fail at load time rather than during partial execution
- reuse improves when ordering, generation, validation, and publishing can be recombined without copying logic
- migration risk drops when the current turn runtime can become a compatibility shape behind a stable workflow interface
- HTN adoption risk drops when decomposition can later build on stable capability and state contracts

## Reading Order

1. [HTN Glossary](htn_glossary.md)
2. [Workflow Definition](workflow_definition/README.md)
3. [Capability Contract](capability_contract/README.md)
4. [Ordering Capability](ordering_capability/README.md)
5. [Context Generate Integration](context_generate_integration/README.md)
6. [File Write Capability](file_write_capability/README.md)
7. [Write Policy](write_policy/README.md)
8. [Agent Chaining](agent_chaining/README.md)
9. [Telemetry Model](telemetry_model/README.md)
10. [Migration Plan](migration_plan/README.md)

## Initial Architectural Position

- `src/workflow` should own workflow planning, capability resolution, task network compilation, workflow runtime, and durable workflow state
- `src/context` should own context artifact production and retrieval through an explicit primitive task contract
- ordering policy should move out of `src/context` and become reusable by commands and workflows as a primitive capability
- file write and publish behavior should become capability owned rather than an incidental side effect of context workflows
- the current turn manager should be treated as a compatibility workflow shape rather than the permanent shape of workflow itself

## Immediate Deliverables

- define the workflow level task model
- define the primitive capability contract
- define ordering and file write capabilities as reusable building blocks
- define config compilation rules for workflow task networks
- define a compatibility path from the current turn workflow runtime

## What This Phase Does Not Build Yet

- automated method learning
- broad search across arbitrary action libraries
- full uncertainty policy synthesis
- a large workflow authoring language before capability contracts and runtime records are stable

## Related Design History

- [Completed Workflow Bootstrap](../completed/workflow_bootstrap/README.md)
- [Publish Arbiter Idea](../workflow_ideas/publish_arbiter_spec.md)
