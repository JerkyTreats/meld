# Workflow Metadata Contracts Implementation Plan

Date: 2026-03-04
Status: active with Phase 1 through Phase 5 complete and Phase 4 permanently deferred
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
- privileged prompt and context retrieval is permanently deferred for this milestone
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
| 2 | Write boundary contract upgrade | Phase 1 | complete |
| 3 | Prompt context artifact placement | Phase 2 | complete |
| 4 | Read visibility and privileged query contract | Phase 2 and Phase 3 | permanently deferred |
| 5 | Workflow consumer schema contracts | Phase 3 | complete |
| 6 | Verification lock and readiness signoff | Phase 1 through Phase 5 | in progress |

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

**Completion snapshot**:
- completion date: 2026-03-05
- implementation commit: `24dc847`
- result: all planned Phase 2 tasks are complete and validated

**Source docs**:
- [Metadata Contracts Requirements Decomposition](decomposition.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)

| Task | Completion |
|------|------------|
| Upgrade shared write validator to consume expanded descriptor fields from Phase 1. | Complete |
| Enforce mutability transition rules and size budget checks with typed deterministic failures. | Complete |
| Ensure generated metadata includes `prompt_digest` `context_digest` and `prompt_link_id` on new writes. | Complete |
| Preserve direct and queue parity through shared `put_frame` boundary usage. | Complete |
| Add deterministic failure tests for unknown forbidden mutability and budget classes across direct and queue paths. | Complete |

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

**Implementation evidence**:
- compile gate passed: `cargo check`
- contract gate passed: `cargo test frame_write_contract`
- integration gate passed: `cargo test --test integration_tests integration::context_api::`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue::`
- parity gate passed: `cargo test --test integration_tests integration::generation_parity::`
- full suite gate passed: `cargo test`

**Phase completion notes**:
- shared write validator now enforces required key presence descriptor max bytes global budget and cross frame mutability transitions
- new typed failures are active for missing required keys and immutable transition violations
- generated metadata now emits `context_digest` with `prompt_digest` and `prompt_link_id`
- queue metadata prevalidation now resolves previous head metadata so mutability failures are deterministic before provider execution
- direct and queue metadata failure classes are parity covered by integration gates
- parity fixtures for generation were updated to include full digest and link metadata output

---

### Phase 3 - Prompt context artifact placement

**Goal**: Move prompt and context payload storage to local content addressed artifacts and link lineage metadata through canonical contracts.

**Completion snapshot**:
- completion date: 2026-03-05
- implementation state: local working tree implementation complete with all verification gates passing

**Source docs**:
- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Requirements Decomposition](decomposition.md)

| Task | Completion |
|------|------------|
| Create `src/prompt_context` domain for prompt and context artifact write read and digest verification behavior. | Complete |
| Write rendered prompt payload and context payload into local CAS artifacts before frame write. | Complete |
| Replace raw payload metadata values with typed identifiers digests and prompt link references only. | Complete |
| Define deterministic lineage unit behavior for retry orphan handling and idempotent replay. | Complete |
| Add artifact integrity tests for digest and size verification on read. | Complete |

**Exit criteria**:
- frame metadata stores no raw prompt text and no raw context payload
- artifact ids and digests are emitted for prompt and context payload lineage
- artifact reads fail deterministically on digest or size mismatch

**Key files and seams**:
- `src/prompt_context`
- `src/metadata/prompt_link_contract.rs`
- `src/context/generation/prompt_collection.rs`
- `src/context/generation/orchestration.rs`
- `src/context/generation/metadata_construction.rs`
- `src/config/workspace/storage_paths.rs`
- `src/cli/route.rs`
- `tests/integration/generation_parity.rs`

**Implementation evidence**:
- compile gate passed: `cargo check`
- unit gate passed: `cargo test prompt_context`
- contract gate passed: `cargo test frame_write_contract`
- integration gate passed: `cargo test --test integration_tests integration::generation_parity::`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue::`
- integration gate passed: `cargo test --test integration_tests integration::context_api::`
- integration gate passed: `cargo test --test integration_tests integration::progress_observability::`
- integration gate passed: `cargo test --test integration_tests integration::config_integration::`
- full suite gate passed: `cargo test`

**Phase completion notes**:
- new filesystem CAS prompt context domain is live with content addressed writes and verified reads
- generation now persists system prompt user prompt template rendered prompt and context payload artifacts before frame write
- typed prompt link payload validation is active for lineage assembly and telemetry emission
- generated frame metadata now derives digest and link keys from lineage artifacts and does not require raw payload metadata keys
- lineage partial failure policy is deterministic orphan keep and queue retryability classifies artifact and prompt link contract failures as non retryable

---

### Phase 4 - Read visibility and privileged query contract

**Goal**: Preserve default safe metadata projection and add explicit privileged prompt and context retrieval with authorization and audit behavior.
**Status**: permanently deferred for this milestone.

**Source docs**:
- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Code Path Findings](code_path_findings.md)

| Task | Completion |
|------|------------|
| Keep default `context get` projection registry driven and unchanged for non privileged calls. | Deferred |
| Add explicit privileged query path to resolve prompt and context artifacts by typed reference. | Deferred |
| Enforce authenticated scoped grants expiry and reason code checks for privileged reads. | Deferred |
| Emit immutable audit events for privileged allow and deny outcomes before payload return. | Deferred |
| Add compatibility read behavior for legacy frames without full digest key set. | Deferred |
| Add integration tests for allow deny compatibility and digest verification behavior. | Deferred |

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

**Goal**: Publish canonical workflow owned schema contracts and validators for thread turn gate and prompt link records.

**Completion snapshot**:
- completion date: 2026-03-06
- implementation state: local working tree implementation complete with verification gates passing

**Source docs**:
- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Metadata Contracts Requirements Decomposition](decomposition.md)

| Task | Completion |
|------|------------|
| Define canonical schema contracts and version policy for thread turn gate and prompt link records in workflow domain. | Complete |
| Define reference integrity validation for frame and artifact identifiers in record payloads. | Complete |
| Publish workflow owned validator surfaces for workflow and context consumer domains. | Complete |
| Add consumer conformance tests that verify import and validator usage with no schema redefinition in consumer domains. | Complete |
| Document compatibility contract for legacy and current metadata states in consumer reads. | Complete |

**Exit criteria**:
- canonical record schema contracts are explicit versioned and test covered
- reference integrity rules are enforced by workflow validators
- consumer domains use workflow owned contracts directly

**Key files and seams**:
- `src/workflow.rs`
- `src/workflow/record_contracts.rs`
- `src/workflow/record_contracts`
- `src/metadata/prompt_link_contract.rs`
- `tests/integration/workflow_contracts_conformance.rs`
- `src/error.rs`

**Implementation evidence**:
- contract gate passed: `cargo test record_contracts`
- integration gate passed: `cargo test --test integration_tests integration::workflow_contracts_conformance::`
- full suite gate passed: `cargo test`

**Phase completion notes**:
- workflow domain now publishes canonical workflow record contracts for thread turn gate and prompt link records with strict V1 schema version policy
- workflow validators now enforce semantic identifier checks for thread turn gate and prompt link references with deterministic typed failures
- workflow domain contract seam is explicit and no longer proxies metadata for record schema ownership
- context lineage output now maps to canonical prompt link record builders through workflow conversion helpers that consume metadata prompt link contracts
- compatibility for existing generation and read paths remains unchanged because workflow record contracts are additive in this phase

---

### Phase 6 - Verification lock and readiness signoff

**Goal**: Lock deterministic behavior and produce readiness evidence for downstream turn manager and docs writer tracks.

| Task | Completion |
|------|------------|
| Run characterization gates for deterministic write and queue retry behavior. | Complete |
| Run contract gates for unknown forbidden mutability budget and required digest key checks. | Complete |
| Run artifact gates for placement digest verification and lineage determinism behavior. | Complete |
| Run read gates for default projection privileged allow deny and audit persistence behavior. | Deferred |
| Run workflow record gates for schema versioning and reference integrity validation behavior. | Complete |
| Publish phase completion notes unresolved risks and evidence links in this plan. | Complete |

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

**Implementation evidence**:
- format gate passed: `cargo fmt -- --check`
- compile gate passed: `cargo check`
- contract gate passed: `cargo test frame_write_contract -- --nocapture`
- artifact gate passed: `cargo test prompt_context -- --nocapture`
- workflow record gate passed: `cargo test record_contracts -- --nocapture`
- characterization gate passed: `cargo test --test integration_tests integration::context_api:: -- --nocapture`
- characterization gate passed: `cargo test --test integration_tests integration::frame_queue:: -- --nocapture`
- artifact gate passed: `cargo test --test integration_tests integration::generation_parity:: -- --nocapture`
- workflow record gate passed: `cargo test --test integration_tests integration::workflow_contracts_conformance:: -- --nocapture`
- readiness gate passed: `cargo test --test integration_tests integration::progress_observability:: -- --nocapture`
- readiness gate passed: `cargo test --test integration_tests integration::config_integration:: -- --nocapture`
- full suite gate passed: `cargo test`

**Phase completion notes**:
- Phase 6 is complete for all non deferred gate classes
- read gate scope remains deferred by milestone policy
- no unresolved high risk failures were observed in this gate run

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
- privileged path is deferred for this milestone

Workflow record gates:
- canonical schema contracts are deterministic and versioned
- reference integrity validators enforce payload linkage rules
- consumer domains import metadata contracts without schema redefinition

## Implementation Order Summary

1. Completed Phase 1 registry descriptor expansion
2. Completed Phase 2 write boundary contract upgrade
3. Completed Phase 3 prompt context artifact placement
4. Phase 4 is permanently deferred for this milestone
5. Completed Phase 5 workflow consumer schema contracts
6. Completed Phase 6 verification lock and readiness signoff

## Risk Watchlist

- descriptor scope expansion beyond bootstrap keys can increase migration overhead
- artifact placement can increase failure surface unless recovery behavior is defined early
- consumer schema drift risk increases if canonical contracts are not finalized before workflow runtime work
