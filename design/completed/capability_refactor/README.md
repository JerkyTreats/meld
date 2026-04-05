# Capability Refactor Completion

Date: 2026-04-04
Status: completed
Scope: completed refactor slices that made the current runtime capability-ready and task-ready enough for the first task-backed docs-writer path

## Intent

This archive captures the parts of the capability program that are now implemented and no longer need to live in the active design set.

These documents describe the ownership corrections and compatibility work that were completed during the refactor window:

- `context` narrowing
- `provider` execution extraction
- `workflow` compatibility cleanup
- `merkle_traversal` extraction

The active work that remains in `design/capabilities` is narrower:

- generic capability contract refinement
- task authoring and compilation as structured data
- docs-writer package convergence away from custom lowering code
- final workflow retirement after task authoring is generic

## Completed Slices

1. [Context Capability Readiness](context/README.md)
2. [Context Code Path Findings](context/code_path_findings.md)
3. [Context Technical Spec](context/technical_spec.md)
4. [Provider Capability Design](provider/README.md)
5. [Workflow Refactor](workflow_refactor/README.md)
6. [Workflow Refactor Code Path Findings](workflow_refactor/code_path_findings.md)
7. [Workflow Cleanup Technical Spec](workflow_refactor/technical_spec.md)
8. [Merkle Traversal Capability](merkle_traversal/README.md)
9. [Merkle Traversal Code Path Findings](merkle_traversal/code_path_findings.md)
10. [Merkle Traversal Technical Spec](merkle_traversal/technical_spec.md)

## Read With

1. [Capability And Task Design](../../capabilities/README.md)
2. [Capability And Task Implementation Plan](../../capabilities/PLAN.md)
3. [Task Design](../../capabilities/task/README.md)
4. [Task Control Boundary](../../capabilities/task_control_boundary.md)
5. [Control Design](../../control/README.md)
