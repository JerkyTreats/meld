# Quality And Metadata Parity

Date: 2026-03-14
Status: active

## Parent Requirements

- [Context Refactor Requirements](../README.md)
- [Context Generate Task](../../context_generate_task/README.md)

## Intent

Protect frame quality and metadata guarantees while orchestration boundaries change.
HTN readiness should not lower artifact quality or weaken metadata contracts.

## Current Pressure

- `src/context/generation/orchestration.rs` is the main seam where prompt assembly, provider execution, metadata construction, and frame persistence meet
- `src/context/generation/metadata_construction.rs` and `src/context/frame_metadata_keys.rs` hold contract sensitive behavior that downstream workflow changes must not bypass
- refactors that move planning out of context can still cause silent quality drift if prompt inputs or metadata lineage change shape

## Requirements

- generated frame content quality must remain stable for equivalent inputs
- metadata lineage, digest keys, and prompt link behavior must remain governed by the current context and metadata domains
- workflow level bindings must not bypass existing metadata validation and write boundaries
- parity checks must cover both content generation behavior and stored metadata contracts

## Planned Deliverables

- characterization baselines for prompt rendering, metadata lineage, and frame output
- refactor checkpoints that compare old and new generation paths on identical inputs
- explicit ownership notes for `src/context` and `src/metadata` validation seams

## Verification Focus

- rendered prompt and context payload lineage remain deterministic
- frame metadata validation still runs on every write path
- equivalent requests keep the same quality and provenance guarantees after orchestration changes

## Related Code

- `src/context/generation/orchestration.rs`
- `src/context/generation/metadata_construction.rs`
- `src/context/frame_metadata_keys.rs`
- `src/metadata`
