# Metadata Contract Ready Code Review

Date: 2026-03-02
Scope: cleanup baseline review for metadata contract readiness

## Intent

Capture concrete remaining risks after current cleanup tracks.
This review isolates what must land before metadata contracts execution.

## Source Docs

- [Metadata Contract Ready Cleanup](README.md)
- [Workflow Metadata Contracts Spec](../../metadata_contracts/README.md)
- [Metadata Contracts Phase Technical Specification](../../metadata_contracts/technical_spec.md)

## Seam Map

Write boundary seams:
- `src/metadata/frame_write_contract.rs`
- `src/api.rs`
- `src/context/queue.rs`

Read boundary seams:
- `src/metadata/frame_types.rs`
- `src/cli/presentation/context.rs`
- `src/context/query/view_policy.rs`

Error and verification seams:
- `src/error.rs`
- `tests/integration/context_api.rs`
- `tests/integration/frame_queue.rs`

## Baseline Findings

### R1 Raw prompt payload still writes into frame metadata

Observed state:
- queue metadata builder writes `prompt` in `src/metadata/frame_write_contract.rs:35`
- queue generation still passes rendered user prompt into metadata build in `src/context/queue.rs:1225`

Gap:
- this conflicts with required metadata contract key set and forbidden payload rules

### R2 Forward digest keys are blocked by current allow list

Observed state:
- allow list includes `agent_id` `provider` `model` `provider_type` `prompt` and `deleted` in `src/metadata/frame_write_contract.rs:13`
- allow list does not include `prompt_digest` `context_digest` or `prompt_link_id`

Gap:
- R1 migration writes will fail as soon as digest key emission is enabled

### R3 Visibility projection is key local and not registry driven

Observed state:
- projection excludes only `agent_id` and `deleted` in `src/metadata/frame_types.rs:12`
- cli json and text output delegate directly to this projection in `src/cli/presentation/context.rs:57` and `src/cli/presentation/context.rs:103`

Gap:
- forbidden and non visible keys can leak by default when new keys are added

### R4 Policy failures are mostly string based and low signal

Observed state:
- validator returns `ApiError::FrameMetadataPolicyViolation` with free form text in `src/metadata/frame_write_contract.rs:43`
- error model lacks dedicated classes for unknown key forbidden key and budget overflow in `src/error.rs`

Gap:
- callers cannot branch on stable metadata policy failure classes

### R5 Runtime bypass risk remains unguarded in tests

Observed state:
- runtime paths use shared validation through `ContextApi::put_frame` at `src/api.rs:239`
- no guard test fails when a new runtime write path skips shared validation

Gap:
- future path additions can bypass policy checks without an immediate test failure

## Required Cleanup Outcomes

1. raw prompt and raw context metadata keys are forbidden at shared write boundary
2. forward digest key set is accepted at shared write boundary
3. read output defaults enforce registry driven visibility policy
4. metadata policy failures are typed and deterministic
5. no runtime write path can bypass shared validation without test failure
