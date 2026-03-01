# Frame Integrity Boundary Spec

Date: 2026-03-01

## Intent

Make frame integrity validation independent from free form metadata key lookup and establish one shared validation boundary for frame writes.

## Problem

Current storage integrity checks read `agent_id` from metadata map.
Current write validation checks only `agent_id` match and basis alignment.
This design couples integrity logic to mutable metadata structure.

## Target Integrity Model

Ownership:
- `src/metadata` validates and normalizes frame metadata before write
- `src/context` submits normalized frame records to storage
- `src/context/frame/storage` verifies hash and structural invariants only

Rules:
- storage integrity checks must not read arbitrary metadata keys
- metadata key policy enforcement must happen before storage write
- all frame writers must pass through one shared validation boundary

## Required Changes

1. add a shared frame write validation service used by `ContextApi::put_frame`

2. ensure normalized frame identity fields used in hash checks are not sourced from free form metadata lookup

3. reject unknown or forbidden keys before storage write

4. add typed errors for key policy failures and size budget failures

## Done Criteria

- storage hash verification does not depend on free form metadata map lookup
- non queue write path and queue write path both use same validation boundary
- invalid metadata key writes fail with deterministic typed errors

## Verification

- add tests for direct `put_frame` writes with invalid metadata
- add tests for queue generated writes with invalid metadata
- add tests for storage hash verification without metadata lookup dependency
