# Metadata Contracts Phase Technical Specification

Date: 2026-03-01
Status: active

## Intent

Provide one technical execution specification for the full metadata contracts phase.
This document defines the path from phase start to phase completion after cleanup is complete.

## Source Synthesis

This specification synthesizes:

- [Workflow Metadata Contracts Spec](README.md)
- [Post Cleanup Findings](code_path_findings.md)

## Phase Boundary

Start condition:
- cleanup workload is complete and verified

End condition:
- R1 and R2 deliverables are implemented
- verification gates pass for write path and read path behavior

## Entry Criteria

All cleanup preconditions must be true.

1. [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)
2. [Domain Metadata Separation Spec](../foundation_cleanup/domain_metadata_separation_spec.md)
3. [Frame Integrity Boundary Spec](../foundation_cleanup/frame_integrity_boundary_spec.md)
4. [Generation Orchestration Split Spec](../foundation_cleanup/generation_orchestration_split_spec.md)

## Goals

1. move prompt and context payload out of frame metadata and into local CAS artifacts
2. enforce metadata key registry with mutability classes and size budgets
3. enforce metadata visibility and redaction behavior on read paths
4. keep ownership boundaries explicit across `src/metadata`, `src/context`, `src/prompt_context`, and `src/workflow`

## Non Goals

- encryption rollout
- remote blob services
- multi thread orchestration
- generalized non docs workflows

## Domain Contracts

Ownership:
- `src/metadata` owns key registry mutability classes validation and budgets
- `src/prompt_context` owns prompt and context artifact storage and digest verification
- `src/context` owns frame write and read orchestration using explicit metadata contracts
- `src/workflow` consumes metadata contracts for thread and turn records

Boundary rules:
- metadata contract logic must execute in one shared frame write boundary
- read surfaces must not bypass visibility policy
- cross domain calls must use explicit contracts only

## Code Seam Map

Write boundary seams:
- `src/api.rs`
- `src/agent/context_access/context_api.rs`
- `src/context/queue.rs`

Artifact seams:
- `src/context/generation/`
- `src/prompt_context/` planned domain

Read boundary seams:
- `src/context/query/service.rs`
- `src/cli/presentation/context.rs`
- `src/cli/route.rs`

Metadata contract seams:
- `src/metadata/` planned domain
- `src/context/frame.rs`

## Execution Plan

### Step 1 Metadata Contract Skeleton

Deliverables:
- metadata key registry structure
- key descriptor model with owner class hash impact max bytes retention redaction visibility
- typed validation errors for key allow list and budget failures

Acceptance:
- unknown key writes fail for identity and attested classes
- registry lookup is deterministic

### Step 2 R1 Prompt and Context Artifact Placement

Deliverables:
- local CAS writes for rendered prompt payload and context payload
- digest emission in frame metadata using `prompt_digest` and `context_digest`
- link emission using `prompt_link_id`
- removal of any raw prompt metadata writes

Acceptance:
- frame metadata contains typed references only
- digest references resolve to artifacts
- no raw prompt text in frame metadata output
- no raw context payload in frame metadata output

### Step 3 R2 Mutability and Key Policy Enforcement

Deliverables:
- mutability class enforcement for identity attested annotation ephemeral
- write time class rules at shared frame write boundary
- deterministic rejection for invalid class transitions

Acceptance:
- invalid key writes fail with typed errors
- mutability violations fail with typed errors

### Step 4 R2 Size Budgets

Deliverables:
- per key byte limit enforcement
- total frame metadata byte limit enforcement
- prompt artifact and context artifact max byte enforcement

Acceptance:
- oversize writes fail with typed errors
- budget checks are covered for direct writes and queue writes

### Step 5 R2 Visibility and Redaction

Deliverables:
- default metadata visibility policy at query service boundary
- redaction policy for sensitive keys
- privileged access path for prompt payload retrieval by artifact reference

Acceptance:
- default `context get` output excludes forbidden and non visible data
- metadata json output follows visibility policy
- privileged path is explicit and test covered

### Step 6 Phase Verification And Lock

Deliverables:
- characterization parity coverage for existing stable behavior
- new contract tests for registry classes budgets and visibility
- final checklist sign off

Acceptance:
- all metadata contract gates pass
- docs writer and turn manager specs can consume metadata contracts without additional contract changes

## Test Strategy

Characterization coverage:
- preserve stable behavior for valid frame writes
- preserve queue retry and completion behavior

Contract coverage:
- key allow list failures
- mutability rule failures
- size budget failures
- forbidden key exposure checks on read paths
- digest and artifact resolution checks

Integration coverage:
- queue generated frame write path
- direct adapter write path through shared boundary
- context read text and json outputs with metadata enabled

## Milestone Gates

Data safety gates:
- no raw prompt text in frame metadata
- no raw context payload in frame metadata
- metadata budgets enforced

Write correctness gates:
- shared write boundary enforces all metadata contracts
- queue and direct writes use identical validation behavior

Read correctness gates:
- default read paths honor visibility policy
- privileged paths require explicit opt in

## Completion Criteria

The metadata contracts phase is complete when all of the following are true:

1. R1 artifact placement and digest reference behavior is live and verified
2. R2 key registry mutability and size budgets are enforced on all frame writes
3. read path visibility and redaction behavior is enforced by default
4. tests cover direct writes queue writes and metadata output surfaces
5. downstream workflow work can proceed without additional metadata contract redesign

