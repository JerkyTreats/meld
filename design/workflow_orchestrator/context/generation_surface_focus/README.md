# Generation Surface Focus

Date: 2026-03-14
Status: active

## Parent Requirements

- [Context Refactor Requirements](../README.md)
- [Context Generate Task](../../context_generate_task/README.md)

## Intent

Keep `src/context` focused on context artifact production, queue execution, frame persistence, and retrieval.
Global orchestration policy should not stay hidden inside the generation domain.

## Current Pressure

- `find_missing_descendant_heads` in `src/context/generation/run.rs` still encodes recursive subtree readiness checks
- `collect_subtree_levels` in `src/context/generation/run.rs` still encodes bottom up target shaping
- `src/context/queue.rs` still assumes the domain owns most execution sequencing details

## Requirements

- `src/context` must remain the owner of prompt assembly, provider calls, frame writes, and retrieval semantics
- `src/context` must stop owning workflow level method choice and target graph derivation
- local queue scheduling may remain internal only when it does not choose global task order or workflow decomposition shape
- context entry points should accept already shaped work and should report execution results without recreating global planning policy

## Planned Deliverables

- a narrower generation entry contract that accepts pre shaped execution input
- removal of workflow level planning decisions from `src/context/generation/run.rs`
- characterization coverage that proves context output remains stable after the surface split

## Verification Focus

- directory generation output remains unchanged for equivalent target scope
- queue execution still produces the same frame lineage and summary behavior
- no new workflow internal type leaks into `src/context` adapters

## Related Code

- `src/context/generation/run.rs`
- `src/context/queue.rs`
- `src/context/generation/orchestration.rs`
