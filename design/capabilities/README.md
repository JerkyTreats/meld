# Capability And Plan Design

Date: 2026-03-27
Status: active
Scope: capability contracts, plan compilation, and plan-ready refactors for current behavior

![[screenshot-2026-03-27_07-30-27.png]]
## Thesis

This design set defines the durable orchestration model for Meld.
The model has two primary concerns: `capability` and `plan`.

Capabilities are domain-owned contracts.
Plans are validated graphs of bound capability instances.

Compiler is the first plan concern.
It is not a planner.
It does not choose goals, choose capabilities, or search for decompositions.
It receives a candidate capability graph, validates it, locks it, and emits a plan record.

Execution is a later concern.
Current work is about making existing behavior capability-ready and plan-ready.

## Durable Structure

The durable structure is `capability/`, `plan/`, `context/`, and `migration_plan/`.

## Core Decisions

- capabilities are separate from functionality
- capability contracts are owned by the domain that provides the behavior
- functionality remains behind the capability contract
- plans are DAGs of bound capability instances
- `petgraph` is the graph substrate for plan graph infrastructure
- compiler validates and locks candidate capability graphs
- compiler is goal-agnostic
- compiler is parallel-ready
- execution may begin conservatively, but plans do not encode serial assumptions
- current `context generate` behavior must be refactored to remove mixed concerns before full plan cutover

## Read Order

1. [Implementation Plan](PLAN.md)
2. [Capability Model](capability/README.md)
3. [Merkle Traversal Capability](capability/merkle_traversal/README.md)
4. [Merkle Traversal Technical Spec](capability/merkle_traversal/technical_spec.md)
5. [Merkle Traversal Code Path Findings](capability/merkle_traversal/code_path_findings.md)
6. [Context Capability Readiness](context/README.md)
7. [Context Generate Technical Spec](context/technical_spec.md)
8. [Workflow Refactor](workflow_refactor/README.md)
9. [Workflow Cleanup Technical Spec](workflow_refactor/technical_spec.md)
10. [Workflow Refactor Code Path Findings](workflow_refactor/code_path_findings.md)
11. [Plan Model](plan/README.md)
12. [Plan Compiler](plan/compiler/README.md)
13. [Plan Graph Model](plan/compiler/graph_model.md)
14. [Petgraph Choice](plan/compiler/petgraph.md)
15. [Plan Record](plan/record/README.md)
16. [Plan Execution](plan/execution/README.md)
17. [Migration Plan](migration_plan/README.md)

## Non Goals

- preserving `workflow` as the durable abstraction
- preserving task-first vocabulary where capability-first language is clearer
- mixing planning strategy into compiler
- defining full HTN behavior before capability and plan contracts are stable
