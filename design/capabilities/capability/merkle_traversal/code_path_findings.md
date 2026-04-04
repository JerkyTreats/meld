# Merkle Traversal Code Path Findings

Date: 2026-03-28
Status: active
Scope: current code baseline findings for `merkle_traversal` extraction

## Intent

Capture the current code paths that own traversal derivation today.
This document records what already exists, what is reusable, and what still prevents `merkle_traversal` from standing as its own capability.

## Source Specs

- [Capability And Task Design](../../README.md)
- [Capability Model](../README.md)
- [Merkle Traversal Capability](README.md)
- [Context Capability Readiness](../../context/README.md)

## Baseline Findings

### M1 Traversal derivation is embedded in `src/context/generation/run.rs`

Today the traversal logic is not a separate domain seam.
`find_missing_descendant_heads`, `collect_subtree_levels`, and `build_plan` all live in [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs).
That means traversal derivation is still coupled to context generation entry behavior rather than exposed as a capability contract.

### M2 Current traversal shape is breadth-first by depth, then reversed to bottom-up levels

`collect_subtree_levels` walks the subtree with a queue, groups nodes by depth, then sorts depths in reverse order before returning them. See [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs).
That gives the current bottom-up behavior, but it is hardcoded as the only traversal strategy.
There is no explicit strategy input for top-down, recency-based, deepest-first, or other traversal variants.

### M3 Descendant readiness checking is mixed with traversal concerns

`find_missing_descendant_heads` walks descendant nodes and checks for missing heads before recursive generation proceeds. See [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs).
This is related to traversal, but it is not the same concern as deriving an ordered Merkle node set.
Right now readiness validation and traversal derivation are blended into one control path.

### M4 `build_plan` still owns traversal policy choice

`build_plan` decides whether recursive behavior is used, whether descendant validation runs, whether head reuse causes skip behavior, and how depth levels turn into generation items. See [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs).
That means traversal policy is still chosen inside the generation path instead of being expressed as an explicit capability input.

### M5 The durable artifact today is level-shaped, not traversal-shaped

`GenerationPlan` stores `levels`, `total_levels`, and `total_nodes` in [plan.rs](/home/jerkytreats/meld/src/context/generation/plan.rs).
That structure is good enough for current execution, but it is not yet a traversal artifact.
It does not carry traversal strategy identity, ordered Merkle node set identity, or a capability-level output contract.

### M6 Execution and telemetry still consume levels directly

`GenerationExecutor` iterates `plan.levels` and emits `level_started`, `level_completed`, and related level-index telemetry in [executor.rs](/home/jerkytreats/meld/src/context/generation/executor.rs).
`TargetExecutionRequest` and workflow-facing execution paths also carry `level_index`. See [program.rs](/home/jerkytreats/meld/src/context/generation/program.rs) and [executor.rs](/home/jerkytreats/meld/src/workflow/executor.rs).
That means traversal output is still coupled to one execution projection, namely level-by-level generation.

### M7 The tree APIs already provide the raw structure needed for a traversal capability

Node records expose `children`, node ids, node type, and path data through the node store and Merkle tree structures. See [builder.rs](/home/jerkytreats/meld/src/tree/builder.rs) and [store.rs](/home/jerkytreats/meld/src/store.rs).
So the codebase already has the underlying tree data needed for `merkle_traversal`.
What is missing is a dedicated contract, strategy input, and typed ordered node set output.

## Extraction Implications

The first extraction step should not be a new execution engine.
It should be a clean traversal seam that:

- accepts a Merkle scope and traversal strategy
- derives one ordered Merkle node set
- emits that result as a typed artifact
- leaves generation execution to `context_generate`

That would let the current bottom-up behavior survive as the first strategy while making other strategies possible without rewriting `context generate`.
