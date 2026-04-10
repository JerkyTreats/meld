# Git Diff Summary Capability

Date: 2026-04-08
Status: proposed
Scope: static analysis capability producing a structured summary of git-tracked changes scoped to a workspace node

## Intent

Produce a quantified summary of what has changed in a node's file scope since a reference point.
This artifact is one evidence input to the `bayesian_evaluation` capability.
It is not an interpretation of those changes — it is a measurement.

## Boundary

This capability:

- accepts a node scope and a reference point
- runs git operations scoped to the node's path
- produces a structured `ChangeSummary` artifact

This capability does not:

- interpret whether changes are significant
- make documentation decisions
- perform AST-level analysis (that is `ast_change_impact`)

## Runtime Initialization

Runtime initialization should hold:

- capability instance identity
- workspace root binding
- git executable path or default resolution binding

## Input Slots

- `node_ref` — `NodeRef` artifact identifying the node to analyze
- `reference_point` — optional `FrameRef` artifact pointing to the last known documentation frame for this node; if absent, defaults to the current head frame for the node, if any

When `reference_point` is absent and no head frame exists, the capability treats the node as never documented and sets `days_since_last_doc_update` to a sentinel indicating no prior record.

## Artifacts Out

```json
{
  "artifact_type_id": "change_summary",
  "schema_version": "v1",
  "content": {
    "node_id": "9f6d8d7f...",
    "files_changed": ["src/lib.rs", "src/auth/mod.rs"],
    "lines_added": 47,
    "lines_removed": 12,
    "commit_count_since_reference": 8,
    "days_since_last_doc_update": 14,
    "reference_commit": "abc123def",
    "no_prior_record": false
  }
}
```

`no_prior_record: true` when no reference point exists. The `bayesian_evaluation` capability
treats this as a maximum-age signal.

## Failure Shape

```json
{
  "failure_kind": "GitOperationFailed",
  "message": "git diff exited with non-zero status",
  "details": { "exit_code": 128, "stderr": "..." }
}
```

Git failures are hard failures. A missing or non-git workspace produces a compile-time binding
error, not a runtime failure.

## Implementation Backend

First slice: subprocess invocation of `git diff --stat` and `git log --oneline --since` scoped
to the node's resolved filesystem path. The sig adapter constructs the command arguments from
the bound workspace root and the resolved node path.

The implementation backend is an internal detail. The capability contract is stable regardless
of whether the backend is a subprocess, a `git2` library call, or a future alternative.

## Publication Rule

Published as a task-facing capability because:

- its output is a typed structured artifact consumed by `bayesian_evaluation`
- it can be scheduled independently from generation work
- it is domain-neutral over git history and does not depend on generation internals

## Read With

- [AST Change Impact](../ast_change_impact/README.md)
- [Bayesian Evaluation](../bayesian_evaluation/README.md)
- [Impact Assessment](../../../control/impact_assessment.md)
- [Capability Model](../README.md)
