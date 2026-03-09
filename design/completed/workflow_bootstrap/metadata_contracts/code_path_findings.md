# Metadata Contracts Code Path Findings

Date: 2026-03-04
Status: active
Scope: entry analysis for metadata contracts after foundation cleanup completion

## Intent

Capture current implementation truth for metadata contracts so decomposition and technical execution stay tied to real code seams.

## Governance Context

Large workflow governance is defined in [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md).

Current governance implications:
- complex workflow artifacts are required only after explicit user activation
- decomposition and synthesis specs can be prepared before activation
- when complex workflow mode is active, verification evidence and phase gate status must be tracked in one PLAN document under the active design scope

## Evidence Set

- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Phase Technical Specification](technical_spec.md)
- [Core Context API](../../../src/api.rs)
- [Frame Metadata Key Registry](../../../src/metadata/frame_key_registry.rs)
- [Frame Metadata Write Contract](../../../src/metadata/frame_write_contract.rs)
- [Frame Metadata Types](../../../src/metadata/frame_types.rs)
- [Generation Prompt Collection](../../../src/context/generation/prompt_collection.rs)
- [Generation Orchestration](../../../src/context/generation/orchestration.rs)
- [Context CLI Presentation](../../../src/cli/presentation/context.rs)
- [Context API Integration Tests](../../../tests/integration/context_api.rs)
- [Frame Queue Integration Tests](../../../tests/integration/frame_queue.rs)
- [Context CLI Integration Tests](../../../tests/integration/context_cli.rs)
- [Generation Parity Fixtures](../../../tests/fixtures/generation_parity/file_success.json)

## Findings

### F1 Shared write boundary is live and enforced

Observed state:
- runtime frame writes call metadata validation at `src/api.rs:246`
- queue orchestration writes through `api.put_frame` at `src/context/generation/orchestration.rs:71`
- bypass guard exists at `tests/integration/context_api.rs:563`

Impact:
- the core metadata enforcement seam is stable and test protected

### F2 Registry exists but descriptor depth is still minimal

Observed state:
- key registry provides `write_policy` and `visibility_policy` in `src/metadata/frame_key_registry.rs:16`
- registry does not yet carry owner, mutability class, retention, redaction, or per key schema descriptors

Impact:
- R2 mutability envelope and redaction requirements are not fully representable in one descriptor model yet

### F3 Forbidden raw payload keys are blocked with typed errors

Observed state:
- validator rejects forbidden keys in `src/metadata/frame_write_contract.rs:44`
- typed failure classes exist in `src/error.rs:60`
- direct and queue tests cover forbidden and unknown key behavior at `tests/integration/context_api.rs:409` and `tests/integration/frame_queue.rs:815`

Impact:
- readiness hardening outcomes remain in place and deterministic

### F4 Generated metadata path does not emit `context_digest` yet

Observed state:
- generated metadata builder writes `prompt_digest` and `prompt_link_id` only in `src/metadata/frame_write_contract.rs:27`
- parity fixtures confirm no `context_digest` on current generated frames in `tests/fixtures/generation_parity/file_success.json`

Impact:
- required bootstrap key set is only partially emitted by the default generation path

### F5 Prompt and context payload are still inline assembly values

Observed state:
- prompt assembly composes raw prompt and context text into provider messages in `src/context/generation/prompt_collection.rs:48`
- no `src/prompt_context` domain exists yet

Impact:
- R1 artifact placement contract is not implemented
- default runtime still depends on in memory prompt and context payload composition only

### F6 Default read visibility is centralized and registry driven

Observed state:
- read projection delegates to registry policy in `src/metadata/frame_types.rs:83`
- CLI text and json surfaces use shared projection in `src/cli/presentation/context.rs:57`
- CLI metadata projection integration coverage exists at `tests/integration/context_cli.rs:269`

Impact:
- read visibility behavior has one policy owner for default outputs

### F7 Privileged prompt and context retrieval path is missing

Observed state:
- no dedicated prompt artifact lookup API exists
- no prompt link record storage implementation exists

Impact:
- R1 read contract for privileged artifact resolution is still open

### F8 Metadata owned workflow schema contracts are not implemented

Observed state:
- no metadata domain package exists for canonical thread turn gate and prompt link schema validators
- no consumer contract surface exists yet for turn manager to import these schemas and validators

Impact:
- turn manager and future workflow domains remain blocked on metadata contract schema delivery

### F9 Error model still contains legacy policy bucket

Observed state:
- `ApiError::FrameMetadataPolicyViolation` remains present in `src/error.rs:57`
- queue retry classification still branches on this legacy variant at `src/context/queue.rs:1083`

Impact:
- typed variants exist and are used, but cleanup of legacy bucket is incomplete

## Synthesis Summary

Stable baseline:
- write boundary centralization
- forbidden key enforcement
- typed unknown and budget error classes
- default read visibility projection
- queue and direct path parity coverage

Primary gap set:
- descriptor depth for mutability and redaction
- `context_digest` emission on generated metadata
- prompt and context artifact placement domain
- privileged artifact retrieval contract
- conversation metadata record domain
