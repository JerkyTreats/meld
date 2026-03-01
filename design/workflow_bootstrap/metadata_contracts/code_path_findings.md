# Metadata Contracts Post Cleanup Findings

Date: 2026-03-01
Scope: post cleanup baseline findings for metadata contracts execution

## Intent

Define the expected state after cleanup completion and identify only the remaining work for metadata contracts.
This document is normative for R1 and R2 planning.

## Cleanup Preconditions

Cleanup specs must be complete before metadata contracts work begins.

1. [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)
2. [Domain Metadata Separation Spec](../foundation_cleanup/domain_metadata_separation_spec.md)
3. [Frame Integrity Boundary Spec](../foundation_cleanup/frame_integrity_boundary_spec.md)
4. [Generation Orchestration Split Spec](../foundation_cleanup/generation_orchestration_split_spec.md)

## Post Cleanup Baseline Assertions

### B1 Shared frame metadata write boundary exists

Expected state:
- all frame writes pass through one validation boundary
- queue and non queue writers use the same validation service

Primary code seams:
- `src/api.rs`
- `src/agent/context_access/context_api.rs`

### B2 Storage integrity checks are metadata structure independent

Expected state:
- storage hash verification does not depend on free form metadata key lookup
- integrity path validates structural identity inputs only

Primary code seams:
- `src/context/frame/storage.rs`
- `src/context/frame/id.rs`

### B3 Generation orchestration is split into focused units

Expected state:
- queue lifecycle and retry behavior remain in queue domain
- prompt assembly provider execution metadata build and frame write are delegated through focused units

Primary code seams:
- `src/context/queue.rs`
- `src/context/generation/`

### B4 Metadata domains are separated

Expected state:
- frame metadata contracts are isolated from node metadata and agent metadata
- cross domain metadata conversion uses explicit adapters only

Primary code seams:
- `src/context/`
- `src/store/`
- `src/agent/`

## Remaining Findings For Metadata Contracts

### M1 R1 artifact placement and digest emission

Required state:
- prompt render and context payload are written to local CAS artifacts
- frame metadata stores only typed references such as `prompt_digest`, `context_digest`, and `prompt_link_id`

Primary code seams:
- `src/context/generation/`
- `src/context/queue.rs`
- `src/context/frame.rs`

### M2 R2 key registry and mutability enforcement

Required state:
- metadata key registry governs allowed keys and mutability classes
- invalid keys fail deterministically on write

Primary code seams:
- `src/metadata/` planned domain
- `src/api.rs` shared write boundary

### M3 R2 size budgets and typed write failures

Required state:
- frame metadata total bytes and per key byte budgets are enforced
- oversize writes fail with typed errors

Primary code seams:
- `src/metadata/` planned domain
- `src/api.rs`

### M4 R2 visibility and redaction policy at read boundaries

Required state:
- default read paths emit only allowed metadata visibility class data
- privileged paths are explicit and audited

Primary code seams:
- `src/context/query/service.rs`
- `src/cli/presentation/context.rs`
- `src/cli/route.rs`

## Metadata Contracts Order After Cleanup

1. implement R1 artifact placement and digest reference writes
2. implement R2 key registry and mutability enforcement
3. implement R2 size budgets and typed errors
4. implement R2 visibility and redaction policy
5. run parity and characterization tests for write and read boundaries

## Exclusions For This Document

This document does not track cleanup execution details.
Cleanup verification belongs to the foundation cleanup workload.
