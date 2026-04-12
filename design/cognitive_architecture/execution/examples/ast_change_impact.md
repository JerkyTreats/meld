# AST Change Impact Capability

Date: 2026-04-08
Status: proposed
Scope: static analysis capability producing a structured public API impact summary for changed source files

## Intent

Determine whether changes to a node's source files affect the public API surface.
Raw line counts alone are a weak documentation signal.
A function signature change or a new public export carries far more documentation weight than
an internal comment edit.
This capability surfaces that distinction as a typed artifact.

## Boundary

This capability:

- accepts the list of changed files from a `ChangeSummary` artifact
- parses changed files for public API surface differences
- produces a structured `AstImpact` artifact

This capability does not:

- perform git operations (that is `git_diff_summary`)
- check semantic correctness or test coverage
- make documentation decisions

## Input Slots

- `change_summary` — `ChangeSummary` artifact from `git_diff_summary`, providing the list of
  changed files scoped to this node

If `change_summary.files_changed` is empty, this capability emits an `AstImpact` with all
counts at zero and returns immediately without parsing.

## Artifacts Out

```json
{
  "artifact_type_id": "ast_impact",
  "schema_version": "v1",
  "content": {
    "node_id": "9f6d8d7f...",
    "public_api_changed": true,
    "new_exports": 2,
    "removed_exports": 0,
    "signature_changes": 1,
    "files_parsed": 2,
    "parse_failures": []
  }
}
```

`parse_failures` lists files that could not be parsed. Partial results with parse failures are
valid and expected for mixed-language nodes or files with syntax errors. The `bayesian_evaluation`
capability handles missing or partial evidence by falling back toward the prior.

`public_api_changed` is true when any of `new_exports`, `removed_exports`, or
`signature_changes` is nonzero.

## Failure Shape

```json
{
  "failure_kind": "AstParseUnavailable",
  "message": "no parser available for detected file types",
  "details": { "file_types_detected": ["*.toml", "*.md"] }
}
```

This is a soft failure. The `bayesian_evaluation` capability can proceed without `AstImpact`
if the evidence is marked absent, falling back toward the prior for the affected factors.

## Implementation Backend

First slice: `tree-sitter` via subprocess or embedded library for language-agnostic parse.
A Rust-specific path using `syn` may be introduced as a binding variant for higher fidelity
on Rust nodes.

The binding selects the parser backend. The capability contract is stable across backends.

## Publication Rule

Published as a task-facing capability because:

- its output is a typed structured artifact consumed by `bayesian_evaluation`
- it operates over structured file input, not over generation artifacts
- it is schedulable independently from generation work and from `git_diff_summary`

## Read With

- [Git Diff Summary](git_diff_summary.md)
- [Bayesian Evaluation](bayesian_evaluation.md)
- [Await Observation Semantics](../control/program/await_observation_semantics.md)
