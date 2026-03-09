# Workflow Orchestrator Roadmap

Date: 2026-03-09
Status: active
Scope: capability orchestration and durable workflow architecture

## Entry Point

This is the primary entry point for the workflow orchestrator effort.
Use this file to understand the new architectural framing, the major requirement areas, and the reading order for active design work.
Each requirement area lives in its own folder with a workload `README.md` as the anchor document.

This roadmap carries forward the capability orchestration direction after the workflow bootstrap milestone.
Bootstrap design history now lives in [Completed Workflow Bootstrap](../completed/workflow_bootstrap/README.md).

## Working Direction

- workflow orchestrates capabilities
- context generation is one capability rather than the owner of global orchestration
- target ordering is a reusable capability rather than a hidden rule inside `context generate`
- file materialization is a separate capability with explicit write policy
- workflow runtime owns artifact handoff, resume state, and cross capability telemetry
- workflow configuration must compile into a reliable execution plan before runtime starts

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

Deliver a workflow architecture where orchestration is a first class domain layer above individual capabilities.
The workflow layer should compile declarative plans, validate capability wiring, persist durable run state, and coordinate artifact flow across ordering, generation, validation, and file materialization.

## Why This Work Matters

- workflow design becomes broader than the first docs writer thread profile
- configuration becomes easier to reason about because capability boundaries are explicit
- runtime reliability improves when invalid plans fail at load time rather than during partial execution
- reuse improves when ordering, generation, validation, and publishing can be recombined without copying logic
- migration risk drops when the current turn runtime can become one capability behind a stable orchestrator interface

## Reading Order

1. [Workflow Definition](workflow_definition/README.md)
2. [Capability Contract](capability_contract/README.md)
3. [Ordering Capability](ordering_capability/README.md)
4. [Context Generate Integration](context_generate_integration/README.md)
5. [File Write Capability](file_write_capability/README.md)
6. [Write Policy](write_policy/README.md)
7. [Agent Chaining](agent_chaining/README.md)
8. [Telemetry Model](telemetry_model/README.md)
9. [Migration Plan](migration_plan/README.md)

## Initial Architectural Position

- `src/workflow` should own workflow planning, capability resolution, workflow runtime, and durable workflow state
- `src/context` should own context artifact production and retrieval through an explicit capability contract
- ordering policy should move out of `src/context` and become reusable by commands and workflows
- file write and publish behavior should become capability owned rather than an incidental side effect of context workflows
- the current turn manager should be treated as a compatibility capability rather than the permanent shape of workflow itself

## Immediate Deliverables

- define the orchestrator level workflow model
- define the capability execution contract
- define ordering and file write capabilities as reusable building blocks
- define config compilation rules for capability workflows
- define a compatibility path from the current turn workflow runtime

## Related Design History

- [Completed Workflow Bootstrap](../completed/workflow_bootstrap/README.md)
- [Publish Arbiter Idea](../workflow_ideas/publish_arbiter_spec.md)
