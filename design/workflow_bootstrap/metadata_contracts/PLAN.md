# Workflow Metadata Contracts Implementation Plan

Date: 2026-03-04
Status: active
Scope: workflow bootstrap metadata contracts

## Overview

This document defines the phased implementation plan for `workflow_bootstrap/metadata_contracts`.
The plan maps metadata contract goals to dependency ordered phases with explicit gates, ownership seams, and verification evidence.

Primary objective:
- complete R1 context placement and R2 metadata contract enforcement on top of the verified foundation cleanup baseline

Metadata contracts outcome:
- metadata registry descriptors are complete and policy ready
- one shared write boundary enforces deterministic metadata contract behavior for direct and queue paths
- prompt and context payloads move to local CAS artifacts with digest verified lineage links
- default read projection remains safe and registry driven
- privileged prompt and context retrieval is explicit, scoped, authenticated, and audit logged
- canonical schema contracts are published for thread turn gate and prompt link records

## Related Specs

- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Requirements Decomposition](decomposition.md)
- [Metadata Contracts Code Path Findings](code_path_findings.md)
- [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)
- [Metadata Contract Ready Cleanup](../foundation_cleanup/metadata_contract_ready/README.md)
- [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md)

## CLI Path Default Exception List

Project direction is path first targeting.
Current command surfaces that still include non default path behavior:

- `merkle context generate` accepts `--node` as an alternate selector
- `merkle context regenerate` accepts `--node` as an alternate selector
- `merkle context get` accepts `--node` as an alternate selector
- `merkle workspace delete` accepts `--node` as an alternate selector
- `merkle workspace restore` accepts `--node` as an alternate selector

This metadata contracts plan does not expand non default path behavior.

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 1 | Registry descriptor expansion | None | complete |
| 2 | Write boundary contract upgrade | Phase 1 | pending |
| 3 | Prompt context artifact placement | Phase 2 | pending |
| 4 | Read visibility and privileged query contract | Phase 2 and Phase 3 | pending |
| 5 | Workflow consumer schema contracts | Phase 4 | pending |
| 6 | Verification lock and readiness signoff | Phase 1 through Phase 5 | pending |

---

### Phase 1 - Registry descriptor expansion

**Goal**: Extend metadata key descriptors so policy evaluation uses one canonical contract model.

**Source docs**:
- [Metadata Contracts Requirements Decomposition](decomposition.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Code Path Findings](code_path_findings.md)

| Task | Completion |
|------|------------|
| Extend descriptor model with mutability class hash impact retention redaction per key budget and owner metadata. | Complete |
| Define explicit bootstrap descriptors for `agent_id` `provider` `model` `provider_type` `prompt_digest` `context_digest` and `prompt_link_id`. | Complete |
| Keep unknown and forbidden key policy deterministic for direct and queue writes. | Complete |
| Add descriptor contract tests for read and write lookup stability. | Complete |

**Exit criteria**:
- descriptor contract supports policy evaluation with no side table dependencies
- bootstrap keys expose complete descriptor metadata
- unknown and forbidden behavior remains deterministic

**Key files and seams**:
- `src/metadata/frame_key_registry.rs`
- `src/metadata/frame_types.rs`
- `src/metadata/frame_write_contract.rs`
- `tests/integration/context_api.rs`
- `tests/integration/frame_queue.rs`

**Implementation evidence**:
- unit gate passed: `cargo test frame_key_registry`
- integration gate passed: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_non_frame_metadata_key`
- integration gate passed: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_forbidden_metadata_key`
- integration gate passed: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_per_key_metadata_budget_overflow`
- integration gate passed: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_total_metadata_budget_overflow`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_metadata_policy_violation`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_forbidden_metadata_key`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_per_key_budget_overflow`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_total_budget_overflow`

**Phase completion notes**:
- descriptor contracts now include mutability hash impact retention redaction owner schema and per key budget metadata
- context and provider domains now declare owned frame metadata descriptors and metadata registry now acts as canonical aggregation and lookup surface
- registry lookup and default visibility behavior remain stable
- direct and queue metadata policy failure behavior remains unchanged under existing gates

---

### Phase 2 - Write boundary contract upgrade

**Goal**: Upgrade shared write validation to enforce descriptor driven mutability and budget rules and require full digest key output.

**Source docs**:
- [Metadata Contracts Requirements Decomposition](decomposition.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)

| Task | Completion |
|------|------------|
| Upgrade shared write validator to consume expanded descriptor fields from Phase 1. | Pending |
| Enforce mutability transition rules and size budget checks with typed deterministic failures. | Pending |
| Ensure generated metadata includes `prompt_digest` `context_digest` and `prompt_link_id` on new writes. | Pending |
| Preserve direct and queue parity through shared `put_frame` boundary usage. | Pending |
| Add deterministic failure tests for unknown forbidden mutability and budget classes across direct and queue paths. | Pending |

**Exit criteria**:
- generated frame metadata includes the required digest key set
- direct and queue writes emit identical typed failures for invalid metadata inputs
- no write path bypasses the shared metadata write boundary

**Key files and seams**:
- `src/metadata/frame_write_contract.rs`
- `src/api.rs`
- `src/context/generation/metadata_construction.rs`
- `src/context/queue.rs`
- `tests/integration/context_api.rs`
- `tests/integration/frame_queue.rs`

---

### Phase 3 - Prompt context artifact placement

**Goal**: Move prompt and context payload storage to local content addressed artifacts and link lineage metadata through canonical contracts.

**Source docs**:
- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Requirements Decomposition](decomposition.md)

| Task | Completion |
|------|------------|
| Create `src/prompt_context` domain for prompt and context artifact write read and digest verification behavior. | Pending |
| Write rendered prompt payload and context payload into local CAS artifacts before frame write. | Pending |
| Replace raw payload metadata values with typed identifiers digests and prompt link references only. | Pending |
| Define deterministic lineage unit behavior for retry orphan handling and idempotent replay. | Pending |
| Add artifact integrity tests for digest and size verification on read. | Pending |

**Exit criteria**:
- frame metadata stores no raw prompt text and no raw context payload
- artifact ids and digests are emitted for prompt and context payload lineage
- artifact reads fail deterministically on digest or size mismatch

**Key files and seams**:
- `src/prompt_context`
- `src/context/generation/prompt_collection.rs`
- `src/context/generation/orchestration.rs`
- `src/context/generation/metadata_construction.rs`
- `tests/integration/generation_parity.rs`

---

### Phase 4 - Read visibility and privileged query contract

**Goal**: Preserve default safe metadata projection and add explicit privileged prompt and context retrieval with authorization and audit behavior.

**Source docs**:
- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Code Path Findings](code_path_findings.md)

| Task | Completion |
|------|------------|
| Keep default `context get` projection registry driven and unchanged for non privileged calls. | Pending |
| Add explicit privileged query path to resolve prompt and context artifacts by typed reference. | Pending |
| Enforce authenticated scoped grants expiry and reason code checks for privileged reads. | Pending |
| Emit immutable audit events for privileged allow and deny outcomes before payload return. | Pending |
| Add compatibility read behavior for legacy frames without full digest key set. | Pending |
| Add integration tests for allow deny compatibility and digest verification behavior. | Pending |

**Exit criteria**:
- default read output remains safe by default and excludes hidden and forbidden payload values
- privileged retrieval path is explicit scoped audited and digest verified
- legacy and current frame read behavior is deterministic

**Key files and seams**:
- `src/metadata/frame_types.rs`
- `src/context/query/service.rs`
- `src/context/query`
- `src/cli/presentation/context.rs`
- `tests/integration/context_cli.rs`
- `tests/integration/context_api.rs`

---

### Phase 5 - Workflow consumer schema contracts

**Goal**: Publish canonical metadata owned schema contracts and validators for thread turn gate and prompt link records.

**Source docs**:
- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Requirements Decomposition](decomposition.md)

| Task | Completion |
|------|------------|
| Define canonical schema contracts and version policy for thread turn gate and prompt link records in metadata domain. | Pending |
| Define reference integrity validation for frame and artifact identifiers in record payloads. | Pending |
| Publish metadata owned validator surfaces for workflow and context consumer domains. | Pending |
| Add consumer conformance tests that verify import and validator usage with no schema redefinition in consumer domains. | Pending |
| Document compatibility contract for legacy and current metadata states in consumer reads. | Pending |

**Exit criteria**:
- canonical record schema contracts are explicit versioned and test covered
- reference integrity rules are enforced by metadata validators
- consumer domains use metadata owned contracts directly

**Key files and seams**:
- `src/metadata`
- `src/workflow`
- `src/context`
- `tests/integration`

---

### Phase 6 - Verification lock and readiness signoff

**Goal**: Lock deterministic behavior and produce readiness evidence for downstream turn manager and docs writer tracks.

| Task | Completion |
|------|------------|
| Run characterization gates for deterministic write and queue retry behavior. | Pending |
| Run contract gates for unknown forbidden mutability budget and required digest key checks. | Pending |
| Run artifact gates for placement digest verification and lineage determinism behavior. | Pending |
| Run read gates for default projection privileged allow deny and audit persistence behavior. | Pending |
| Run workflow record gates for schema versioning and reference integrity validation behavior. | Pending |
| Publish phase completion notes unresolved risks and evidence links in this plan. | Pending |

**Exit criteria**:
- all gate classes pass with no unresolved high risk issues
- R1 and R2 phase outputs are ready for downstream workflow runtime consumption
- no additional foundation cleanup scope is required before turn manager implementation

**Key files and seams**:
- `tests/integration/context_api.rs`
- `tests/integration/frame_queue.rs`
- `tests/integration/context_cli.rs`
- `tests/integration/generation_parity.rs`
- `tests/fixtures/generation_parity`

## Verification Strategy

Characterization gates:
- deterministic valid frame write behavior remains stable
- queue retry semantics remain stable for retryable provider failures

Contract gates:
- unknown forbidden mutability and budget failures are typed and deterministic
- generated frame metadata includes required digest keys for new writes

Artifact gates:
- prompt and context artifact writes are content addressed and digest verified
- lineage behavior is deterministic for retry and failure handling

Read gates:
- default read projection remains registry driven and safe by default
- privileged path is explicit, authenticated, scoped, audited, and digest verified

Workflow record gates:
- canonical schema contracts are deterministic and versioned
- reference integrity validators enforce payload linkage rules
- consumer domains import metadata contracts without schema redefinition

## Implementation Order Summary

1. Execute Phase 1 registry descriptor expansion
2. Execute Phase 2 write boundary contract upgrade
3. Execute Phase 3 prompt context artifact placement
4. Execute Phase 4 read visibility and privileged query contract
5. Execute Phase 5 workflow consumer schema contracts
6. Execute Phase 6 verification lock and readiness signoff

## Risk Watchlist

- descriptor scope expansion beyond bootstrap keys can increase migration overhead
- artifact placement can increase failure surface unless recovery behavior is defined early
- consumer schema drift risk increases if canonical contracts are not finalized before workflow runtime work
