# Foundation Cleanup Implementation Plan

Date: 2026-03-01
Status: complete
Scope: workflow bootstrap foundation cleanup

## Overview

This document defines the phased implementation plan for `completed/workflow_bootstrap/foundation_cleanup`.
The plan follows the same execution style as the completed context plan and maps cleanup work to clear outcomes, dependencies, and verification gates.

Primary objective:
- stabilize metadata and generation boundaries before metadata contracts and turned workflow features

Foundation outcome:
- metadata ownership boundaries are explicit across frame node and agent domains
- frame write validation and policy enforcement run through one shared write boundary
- queue lifecycle is isolated from prompt provider and metadata construction concerns

## CLI Path Default Exception List

Project direction is path first targeting.
Current command surfaces that still include non default path behavior:

- `merkle context generate` accepts `--node` as an alternate selector
- `merkle context regenerate` accepts `--node` as an alternate selector
- `merkle context get` accepts `--node` as an alternate selector
- `merkle workspace delete` accepts `--node` as an alternate selector
- `merkle workspace restore` accepts `--node` as an alternate selector

This foundation cleanup plan does not expand non default path behavior.

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 1 | Domain metadata separation | None | complete |
| 2 | Frame integrity boundary cleanup | Phase 1 | complete |
| 3 | Generation orchestration boundary cleanup | Phase 1 and Phase 2 | complete |
| 4 | Integrated parity and readiness gates | Phase 1 through Phase 3 | complete |
| 5 | Metadata contract readiness hardening | Phase 1 through Phase 4 | complete |

---

### Phase 1 — Domain metadata separation

**Goal**: Separate frame node and agent metadata contracts with explicit types and adapters.

**Source docs**:
- [Domain Metadata Separation Cleanup](domain_metadata/README.md)
- [Domain Metadata Separation Technical Specification](domain_metadata/technical_spec.md)
- [Domain Metadata Separation Spec](domain_metadata/separation_spec.md)
- [Domain Metadata Code Path Findings](domain_metadata/code_path_findings.md)

| Task | Completion |
|------|------------|
| Normalize store module layout to remove `mod.rs` usage in the targeted seam. | Complete |
| Introduce explicit metadata domain types for frame node and agent domains. | Complete |
| Add explicit prompt contract adapter in agent profile domain. | Complete |
| Centralize frame metadata construction and validation contract usage through one write boundary. | Complete |
| Replace read path raw metadata map lookups with typed accessors and projection policy. | Complete |
| Complete node metadata cutover with direct serialization path and no compatibility wrapper track. | Complete |
| Add isolation misuse and adapter parity coverage across integration suites. | Complete |

**Exit criteria**:
- frame node and agent metadata boundaries are explicit and isolated
- context generation no longer depends on private agent metadata key names
- frame metadata write checks are centralized through one shared contract path
- characterization coverage proves metadata isolation and deterministic misuse failures

**Key files and seams**:
- `src/store.rs`
- `src/metadata/frame_types.rs`
- `src/store/node_metadata.rs`
- `src/agent/profile/metadata_types.rs`
- `src/agent/profile/prompt_contract.rs`
- `src/agent/registry.rs`
- `src/context/frame.rs`
- `src/api.rs`
- `src/context/queue.rs`
- `src/context/query/view_policy.rs`

**Implementation evidence**:
- compile gate passed: `cargo check`
- context api integration module passed under `integration_tests`
- frame queue integration module passed under `integration_tests`
- store integration module passed under `integration_tests`
- config integration module passed under `integration_tests`
- context cli integration module passed under `integration_tests`

---

### Phase 2 — Frame integrity boundary cleanup

**Goal**: Enforce deterministic typed frame metadata policy at one write boundary and decouple storage integrity from free form metadata lookup.

**Source docs**:
- [Frame Integrity Boundary Cleanup](frame_integrity/README.md)
- [Frame Integrity Boundary Technical Specification](frame_integrity/technical_spec.md)
- [Frame Integrity Code Path Findings](frame_integrity/code_path_findings.md)

| Task | Completion |
|------|------------|
| Extend shared frame write validator as the single frame write validation service. | Complete |
| Add structural frame identity fields for storage integrity verification. | Complete |
| Remove storage hash dependence on metadata map lookup. | Complete |
| Introduce typed metadata policy error variants for unknown forbidden and budget failures. | Complete |
| Enforce allow list and forbidden key policy at write boundary. | Complete |
| Enforce per key and total metadata size budgets at write boundary. | Complete |
| Add direct and queue parity tests for success and failure behavior. | Complete |

**Exit criteria**:
- all frame writes flow through one shared validator path
- storage hash and integrity checks rely on structural identity fields only
- metadata policy failures are typed deterministic and parity verified across direct and queue write paths

**Key files and seams**:
- `src/metadata/frame_write_contract.rs`
- `src/context/frame.rs`
- `src/context/frame/id.rs`
- `src/context/frame/storage.rs`
- `src/error.rs`
- `src/api.rs`
- `src/context/queue.rs`

**Implementation evidence**:
- compile gate passed: `cargo check`
- storage integrity unit gate passed: `cargo test non_structural_metadata_mutation`
- storage corruption unit gate passed: `cargo test structural_content_corruption`
- context api integration module passed: `cargo test --test integration_tests integration::context_api`
- frame queue integration module passed: `cargo test --test integration_tests integration::frame_queue`

**Phase completion notes**:
- shared frame metadata validation now enforces unknown key forbidden key and budget checks with typed deterministic errors
- frame storage hash checks now recompute from structural identity fields and no longer depend on metadata map lookup for new writes
- direct and queue write paths now assert parity on unknown forbidden and budget policy failures in integration suites
- `prompt` metadata key compatibility allowance in this phase is now closed by Phase 5 readiness hardening

---

### Phase 3 — Generation orchestration boundary cleanup

**Goal**: Split generation orchestration responsibilities from queue lifecycle while preserving generation and retry behavior.

**Source docs**:
- [Generation Orchestration Boundary Cleanup](generation_orchestration/README.md)
- [Generation Orchestration Synthesis Technical Specification](generation_orchestration/technical_spec.md)
- [Generation Orchestration Code Path Findings](generation_orchestration/code_path_findings.md)

| Task | Completion |
|------|------------|
| Extract prompt and context collection logic from queue worker into generation units. | Complete |
| Extract provider execution from queue worker into a dedicated generation unit contract. | Complete |
| Extract frame metadata construction from queue worker and route through metadata contract boundary. | Complete |
| Constrain queue worker to lifecycle dedupe retry ordering and telemetry concerns. | Complete |
| Preserve generate run ownership and level policy seams in generation domain. | Complete |
| Add characterization baseline capture and post split parity suites for generation output and retries. | Complete |

**Exit criteria**:
- queue worker no longer performs inline prompt assembly provider calls or metadata map construction
- generation contracts are explicit and domain scoped
- queue lifecycle behavior remains stable for ordering retry and dedupe
- parity suites confirm no contract drift for targeted scenarios

**Key files and seams**:
- `src/context/queue.rs`
- `src/context/generation/run.rs`
- `src/context/generation/executor.rs`
- `src/context/generation/contracts.rs`
- `src/context/generation/orchestration.rs`
- `src/context/generation/prompt_collection.rs`
- `src/context/generation/provider_execution.rs`
- `src/context/generation/metadata_construction.rs`
- `src/metadata/frame_write_contract.rs`
- `tests/fixtures/generation_parity/`

**Implementation evidence**:
- compile gate passed: `cargo check`
- generation parity integration module passed: `cargo test --test integration_tests integration::generation_parity`
- frame queue integration module passed: `cargo test --test integration_tests integration::frame_queue`
- generation executor unit module passed: `cargo test generation::executor`

**Phase completion notes**:
- queue request processing now delegates generation content execution to `src/context/generation/orchestration.rs`
- prompt and context collection moved to `src/context/generation/prompt_collection.rs` and queue no longer assembles prompts inline
- provider preparation and completion execution moved to `src/context/generation/provider_execution.rs`
- generated frame metadata construction moved to `src/context/generation/metadata_construction.rs` and validated through shared frame metadata contract checks
- parity fixtures for file success directory success retryable failure and non retryable failure are committed under `tests/fixtures/generation_parity/`

---

### Phase 4 — Integrated parity and readiness gates

**Goal**: Run cross phase verification gates and confirm cleanup readiness for downstream metadata contracts and turned workflow features.

| Task | Completion |
|------|------------|
| Run full integration suite for context queue store config and cli surfaces impacted by cleanup. | Complete |
| Run generation parity gates P1 P2 and P3 with committed baseline artifacts. | Complete |
| Verify frame write policy parity for direct and queue paths under identical invalid inputs. | Complete |
| Verify storage integrity checks remain deterministic after metadata policy hardening. | Complete |
| Verify no new non default path behavior appears in CLI docs or specs. | Complete |
| Publish final cleanup completion notes in foundation cleanup readme and workflow bootstrap roadmap. | Complete |

**Exit criteria**:
- all phase gates pass with no unresolved behavioral drift
- cleanup outputs are accepted by downstream metadata contracts and turn manager tracks

**Implementation evidence**:
- compile gate passed: `cargo check`
- integration gate passed: `cargo test --test integration_tests integration::context_api`
- integration gate passed: `cargo test --test integration_tests integration::frame_queue`
- integration gate passed: `cargo test --test integration_tests integration::store_integration`
- integration gate passed: `cargo test --test integration_tests integration::config_integration`
- integration gate passed: `cargo test --test integration_tests integration::context_cli`
- integration gate passed: `cargo test --test integration_tests integration::node_deletion`
- integration gate passed: `cargo test --test integration_tests integration::generation_parity`
- parity fixture gate P1 passed with tracked artifacts under `tests/fixtures/generation_parity/`
- parity gate P2 file passed: `cargo test --test integration_tests integration::generation_parity::generation_parity_file_success_matches_fixture`
- parity gate P2 directory passed: `cargo test --test integration_tests integration::generation_parity::generation_parity_directory_success_matches_fixture`
- parity gate P3 retryable failure passed: `cargo test --test integration_tests integration::generation_parity::generation_parity_retryable_failure_matches_fixture`
- parity gate P3 non retryable failure passed: `cargo test --test integration_tests integration::generation_parity::generation_parity_non_retryable_failure_matches_fixture`
- direct and queue unknown key parity passed
- direct gate command: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_non_frame_metadata_key`
- queue gate command: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_metadata_policy_violation`
- direct and queue forbidden key parity passed
- direct gate command: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_forbidden_metadata_key`
- queue gate command: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_forbidden_metadata_key`
- direct and queue per key budget parity passed
- direct gate command: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_per_key_metadata_budget_overflow`
- queue gate command: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_per_key_budget_overflow`
- direct and queue total budget parity passed
- direct gate command: `cargo test --test integration_tests integration::context_api::test_put_frame_rejects_total_metadata_budget_overflow`
- queue gate command: `cargo test --test integration_tests integration::frame_queue::test_queue_rejects_generated_total_budget_overflow`
- storage integrity determinism gate passed: `cargo test non_structural_metadata_mutation`
- storage corruption determinism gate passed: `cargo test structural_content_corruption`

**Phase completion notes**:
- Phase 4 gates are complete as of 2026-03-04 with no behavioral drift
- generation parity fixtures remain committed and unchanged for all four scenarios
- direct and queue write paths emit matching typed policy failures for unknown forbidden and budget classes
- storage integrity checks remain deterministic for non structural metadata mutation and structural content corruption
- CLI non default path behavior audit now includes `context regenerate` in the exception list and no additional command exceptions were found
- foundation cleanup and workflow bootstrap roadmap docs include Phase 4 completion status and evidence summary

---

### Phase 5 — Metadata contract readiness hardening

**Goal**: Close remaining metadata policy and read visibility gaps so metadata contracts can start with no new foundation scope.

**Source docs**:
- [Metadata Contract Ready Cleanup](metadata_contract_ready/README.md)
- [Metadata Contract Ready Code Review](metadata_contract_ready/code_review.md)
- [Metadata Contract Ready Technical Specification](metadata_contract_ready/technical_spec.md)

| Task | Completion |
|------|------------|
| Enforce forbidden key rejection for raw prompt and raw context metadata payload keys at shared write boundary. | Complete |
| Add forward accepted keys for `prompt_digest` `context_digest` and `prompt_link_id`. | Complete |
| Replace string policy failures with typed metadata policy failures for unknown forbidden and budget classes. | Complete |
| Enforce registry driven metadata visibility projection on default read surfaces. | Complete |
| Add no bypass runtime write gate for shared frame metadata validation entry. | Complete |
| Add direct and queue parity suites for forbidden key unknown key and budget failures. | Complete |

**Exit criteria**:
- default metadata output cannot reveal forbidden payload values
- direct and queue writes emit identical typed metadata policy failures
- runtime frame writes cannot bypass shared metadata validator entry
- metadata contracts phase can start without additional cleanup scope growth

**Implementation evidence**:
- compile gate passed: `cargo check`
- direct write parity gate passed: `cargo test --test integration_tests integration::context_api`
- queue write parity gate passed: `cargo test --test integration_tests integration::frame_queue`
- read projection gate passed: `cargo test --test integration_tests integration::context_cli`
- generation parity gate passed: `cargo test --test integration_tests integration::generation_parity`
- storage determinism gate passed: `cargo test non_structural_metadata_mutation`
- storage corruption gate passed: `cargo test structural_content_corruption`
- direct forbidden key gate passed for raw prompt prompt and raw context
- queue forbidden key gate passed for raw prompt prompt and raw context
- no bypass runtime write guard gate passed: `cargo test --test integration_tests integration::context_api::test_runtime_write_paths_use_shared_put_frame_boundary`

**Phase completion notes**:
- Phase 5 gates are complete as of 2026-03-04 with no unresolved drift
- shared write boundary now rejects raw prompt payload keys through forbidden key policy
- generation metadata writer now emits digest based metadata references and no raw prompt value
- metadata read projection now delegates visibility decisions to one metadata key registry contract
- direct and queue invalid metadata paths keep parity for unknown forbidden and budget failures
- metadata contracts phase can start without additional foundation cleanup scope

---

## Implementation Order Summary

1. Complete Phase 1 domain metadata separation
2. Complete Phase 2 frame integrity boundary cleanup
3. Complete Phase 3 generation orchestration split
4. Complete Phase 4 integrated readiness gates
5. Complete Phase 5 metadata contract readiness hardening

## Verification Strategy

Isolation gates:
- frame metadata policy edits do not alter node metadata behavior
- frame metadata policy edits do not alter agent profile metadata behavior

Write boundary gates:
- direct and queue writes share one validation and policy path
- unknown forbidden and oversize metadata fail deterministically

Storage integrity gates:
- frame integrity checks use structural identity fields only
- metadata map key mutations do not bypass integrity checks

Generation parity gates:
- baseline artifacts exist and are committed before split validation
- post split artifacts match baseline for targeted success scenarios
- retry count backoff class and terminal error class match baseline

Read safety gates:
- default metadata output hides forbidden and non visible key classes
- projection policy is centralized in metadata domain contracts

Readiness gates:
- shared write boundary accepts required forward digest keys
- forbidden payload key writes fail deterministically on all runtime paths
- no runtime write path bypasses shared metadata policy enforcement

CLI direction gates:
- no new non default path command behavior is introduced
- exception list in this plan remains accurate as command surfaces evolve

## Success Criteria

Foundation cleanup is complete when:

1. domain metadata boundaries are explicit and isolated by contract types
2. frame integrity policy and validation are centralized typed and deterministic
3. generation orchestration is split by ownership with queue lifecycle isolation
4. parity and characterization coverage pass for metadata integrity and generation behavior
5. metadata contract readiness hardening gates pass for write and read policy behavior
6. cleanup outputs are ready inputs for metadata contracts and turned docs workflow phases

## Related Documentation

- [Boundary Cleanup Foundation Spec](README.md)
- [Domain Metadata Separation Cleanup](domain_metadata/README.md)
- [Frame Integrity Boundary Cleanup](frame_integrity/README.md)
- [Generation Orchestration Boundary Cleanup](generation_orchestration/README.md)
- [Metadata Contract Ready Cleanup](metadata_contract_ready/README.md)
- [Workflow Bootstrap Roadmap](../README.md)
- [Workflow Metadata Contracts Spec](../metadata_contracts/README.md)
- [Turn Manager Generalized Spec](../turn_manager/README.md)
- [Docs Writer Thread Turn Configuration Spec](../docs_writer/README.md)
