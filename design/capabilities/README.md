# Capability And Task Design

Date: 2026-03-27
Status: active
Scope: capability contracts, task definition and compilation, provider and context refactor, temporary control extraction, and workflow cleanup for current behavior

![[screenshot-2026-03-27_07-30-27.png]]
## Thesis

This design set defines the durable orchestration model for Meld.
Within this implementation layer, the primary concerns are `capability` and `task`.

Capabilities are domain-owned contracts.
Tasks are compiled capability graphs.

Task compilation is the first graph concern in this layer.
It is not planning.
It does not choose goals, choose tasks, or search for decompositions.
It receives a candidate capability graph, validates it, locks it, and emits a compiled task record.

Current work is about making existing behavior capability-ready and task-ready.

`plan` still exists as a control concern above this layer.
It should describe ordering, graph execution, and modification of tasks, not the primitive compiled graph unit itself.

## Durable Structure

The durable structure in this layer is `capability/`, `task/`, `provider/`, `context/`, `workflow_refactor/`, and `migration_plan/`.

## Core Decisions

- capabilities are separate from functionality
- capability contracts are owned by the domain that provides the behavior
- functionality remains behind the capability contract
- tasks are directed compiled capability graphs
- tasks own task-scoped artifact persistence and invocation records
- capabilities remain stateless execution boundaries over structured data
- workflow files should converge into task definitions, not remain the durable runtime abstraction
- `petgraph` is the graph substrate for task compilation infrastructure
- compiler validates and locks candidate capability graphs into compiled tasks
- compiler is goal-agnostic
- compiler is parallel-ready
- execution may begin conservatively, but compiled tasks do not encode serial assumptions
- provider owns service-execution optimization for ready generation work
- control temporarily owns extracted orchestration during the refactor window
- current `context generate` behavior must be refactored to remove mixed concerns before full task cutover
- plan and graph execution remain coherent control concerns above this layer

## Read Order

1. [Implementation Plan](PLAN.md)
2. [Capability Model](capability/README.md)
3. [Task Design](task/README.md)
4. [Docs Writer Package](task/docs_writer_package.md)
5. [Task Control Boundary](task_control_boundary.md)
6. [Capabilities By Domain](capability/by_domain.md)
7. [Domain Architecture](domain_architecture.md)
8. [Provider Capability Design](provider/README.md)
9. [Context Capability Readiness](context/README.md)
10. [Context Code Path Findings](context/code_path_findings.md)
11. [Context Technical Spec](context/technical_spec.md)
12. [Workflow Refactor](workflow_refactor/README.md)
13. [Workflow Cleanup Technical Spec](workflow_refactor/technical_spec.md)
14. [Workflow Refactor Code Path Findings](workflow_refactor/code_path_findings.md)
15. [Merkle Traversal Capability](capability/merkle_traversal/README.md)
16. [Merkle Traversal Technical Spec](capability/merkle_traversal/technical_spec.md)
17. [Merkle Traversal Code Path Findings](capability/merkle_traversal/code_path_findings.md)
18. [Migration Plan](migration_plan/README.md)

## Non Goals

- preserving `workflow` as the durable abstraction
- making `plan` a primitive implementation concern inside this layer
- mixing planning strategy into the capability or task compiler
- defining full HTN behavior before capability and task contracts are stable
