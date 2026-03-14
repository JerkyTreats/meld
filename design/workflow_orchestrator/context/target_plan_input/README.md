# Target Plan Input

Date: 2026-03-14
Status: active

## Parent Requirements

- [Context Refactor Requirements](../README.md)
- [Ordering Task](../../ordering_task/README.md)

## Intent

Allow workflow to submit ordered target plans into `src/context` rather than forcing the context domain to derive those plans internally.
This is the main handoff seam between planning and execution.

## Current Pressure

- `collect_subtree_levels` in `src/context/generation/run.rs` still produces an implicit ordering artifact inside the execution path
- subtree readiness checks and target expansion stay entangled with generation dispatch
- downstream execution has no stable input artifact that records the chosen target shape

## Requirements

- context execution must accept an explicit ordered target plan input
- the target plan input must preserve deterministic target identity, level ordering, and scope digest information
- target expansion and ordering policy must move to workflow or another planning task surface
- context execution must not recompute traversal policy once a target plan has been bound

## Planned Deliverables

- a typed target plan contract consumed by context generation entry points
- removal of implicit subtree ordering logic from the main execution path
- durable plan lineage from ordering output into context generation results

## Verification Focus

- equivalent target plans produce equivalent generation work and frame outputs
- stale or mismatched target plans fail with deterministic validation errors
- ordering policy changes do not require prompt assembly or provider execution rewrites

## Related Code

- `src/context/generation/run.rs`
- `src/context/generation/plan.rs`
- `src/context/generation/contracts.rs`
- `src/context/queue.rs`
