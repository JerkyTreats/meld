# Metadata Contracts Requirements Decomposition

Date: 2026-03-04
Status: active

## Intent

Decompose metadata contract requirements into dependency ordered work packages with explicit gates and ownership seams.

## Source Inputs

- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Code Path Findings](code_path_findings.md)
- [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md)
- [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)

## Governance Alignment For Large Workflows

- complex workflow mode is user triggered and not automatic
- this decomposition is valid in default workflow mode and can be promoted to complex mode without rework
- if complex workflow mode is activated, maintain one PLAN artifact in this folder with phase status and verification evidence per governance rules

## Dependency Graph

```mermaid
flowchart LR
    D1[Registry Descriptor Expansion] --> D2[Write Boundary Contract Upgrade]
    D2 --> D3[Prompt Context Artifact Placement]
    D2 --> D4[Read Visibility and Privileged Query Contract]
    D3 --> D4
    D4 --> D5[Workflow Consumer Schema Contracts]
    D5 --> D6[Verification Lock and Readiness Signoff]
```

## Work Package Breakdown

### D1 Registry Descriptor Expansion

Goal:
- extend key descriptor model so each key has mutability class hash impact retention redaction and size budget metadata

Primary seams:
- `src/metadata/frame_key_registry.rs`
- `src/metadata/frame_types.rs`

Exit gates:
- registry exposes full descriptor contract for bootstrap keys
- unknown key and forbidden key behavior remains deterministic
- descriptor lookup remains stable for read and write paths

### D2 Write Boundary Contract Upgrade

Goal:
- enforce mutability and size policy at one shared write boundary and close generated metadata key set gaps

Primary seams:
- `src/metadata/frame_write_contract.rs`
- `src/api.rs`
- `src/context/generation/metadata_construction.rs`
- `src/context/queue.rs`

Exit gates:
- generated metadata emits required digest key set including `context_digest`
- mutability and budget failures map to typed deterministic errors
- queue and direct write parity tests pass

### D3 Prompt Context Artifact Placement

Goal:
- move raw prompt and context payload off frame metadata into local content addressed artifacts

Primary seams:
- `src/prompt_context` new domain
- `src/context/generation/prompt_collection.rs`
- `src/context/generation/orchestration.rs`

Exit gates:
- frame metadata stores typed digest and link identifiers only
- prompt and context payload writes produce immutable artifact ids
- digest verification runs on artifact read
- prompt link payloads are emitted through metadata owned canonical schema contracts

### D4 Read Visibility And Privileged Query Contract

Goal:
- preserve strict default visibility while adding explicit privileged retrieval for prompt and context artifacts

Primary seams:
- `src/metadata/frame_types.rs`
- `src/context/query/service.rs`
- `src/cli/presentation/context.rs`
- new privileged query surface under `src/context/query/`

Exit gates:
- default `context get` output keeps forbidden and hidden keys out
- privileged path is explicit and opt in
- privileged path resolves prompt and context payload by artifact reference with digest verification

### D5 Workflow Consumer Schema Contracts

Goal:
- publish canonical metadata schema contracts for thread turn gate and prompt link records so conversation workflows and future features consume one stable contract set

Primary seams:
- `src/metadata` contract domain
- `src/workflow` consumer contract seam
- `src/context` consumer contract seam

Exit gates:
- canonical schema version contract is explicit
- reference integrity rules are explicit and validated
- consumer domains use metadata contracts and metadata validators
- workflow runtime domains do not redefine workflow record schemas

### D6 Verification Lock And Readiness Signoff

Goal:
- lock deterministic behavior and document full phase readiness for downstream turn manager and docs writer work

Primary seams:
- `tests/integration/context_api.rs`
- `tests/integration/frame_queue.rs`
- `tests/integration/context_cli.rs`
- `tests/integration/generation_parity.rs`

Exit gates:
- write and read policy gates pass
- parity fixtures are updated only for intended contract deltas
- downstream specs consume metadata contracts with no additional redesign request

## Execution Order

1. D1 registry descriptor expansion
2. D2 write boundary contract upgrade
3. D3 prompt context artifact placement
4. D4 read visibility and privileged query contract
5. D5 workflow consumer schema contracts
6. D6 verification lock and readiness signoff

## Risk Watchlist

- key descriptor schema churn can create migration overhead if D1 scope grows beyond bootstrap keys
- artifact placement can increase failure surface if digest verification and retry semantics are not aligned early
- workflow consumer integration can sprawl if canonical schema and validator boundaries are not finalized before workflow runtime implementation
