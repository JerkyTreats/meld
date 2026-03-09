# Metadata Contracts Phase Technical Specification

Date: 2026-03-04
Status: active

## Intent

Provide one synthesis execution specification for metadata contracts using governance rules for large workflows plus live codebase findings.

## Source Synthesis

This specification synthesizes:

- [Workflow Metadata Contracts Spec](README.md)
- [Metadata Contracts Code Path Findings](code_path_findings.md)
- [Metadata Contracts Requirements Decomposition](decomposition.md)
- [Complex Change Workflow Governance](../../../governance/complex_change_workflow.md)
- [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)

## Governance Profile

Large workflow governance decisions for this scope:
- complex workflow mode is optional and user triggered
- this technical specification is valid in default mode
- if complex workflow mode is activated, maintain one PLAN artifact in this folder with phase status and verification evidence
- CI does not enforce workflow artifacts, so readiness gates in this document are reviewer and author contracts

## Phase Boundary

Start condition:
- C0 foundation cleanup through metadata contract readiness is complete
- current write boundary and read projection safeguards are active

End condition:
- R1 context placement and R2 metadata contract enforcement are complete
- write and read gates pass with deterministic behavior on direct and queue paths
- downstream workflow runtime can consume canonical metadata contracts with no metadata contract redesign

## Entry Criteria

All of the following must remain true before execution:

1. [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)
2. [Metadata Contract Ready Cleanup](../foundation_cleanup/metadata_contract_ready/README.md)
3. shared write boundary remains at `ContextApi::put_frame`
4. queue and adapter runtime paths continue to call `put_frame`

## Goals

1. complete R1 artifact placement by moving prompt and context payload off frame metadata
2. complete R2 registry and policy enforcement with typed deterministic failures
3. preserve strict default read visibility and add explicit privileged retrieval for prompt and context artifacts
4. define canonical metadata schema contracts for workflow record consumers in conversation workflows and future features

## Connected Design Requirements

These requirements are inferred from the workflow bootstrap roadmap plus turn manager and docs writer specs.

### CDR1 Atomic lineage commit contract

- metadata contracts must define one lineage contract unit for prompt artifact write context artifact write frame metadata digest write and prompt link payload
- partial commit behavior must be deterministic and recoverable through idempotent replay and orphan cleanup rules
- runtime persistence of lineage unit members is implemented by downstream workflow runtime features that consume this contract

### CDR2 Canonical digest contract

- `prompt_digest` and `context_digest` must be derived from canonical byte streams
- canonical encoding and normalization rules must be declared before implementation so retries preserve digest identity

### CDR3 Compatibility contract for mixed states

- runtime read paths must support legacy frames without full digest key set
- runtime write paths must enforce full bootstrap key set for new writes
- compatibility behavior must be explicit and covered by characterization tests

### CDR4 Workflow record schema handshake

- metadata contracts and workflow runtime must share one canonical schema for thread turn gate and prompt link records
- schema version and field compatibility rules must be explicit before workflow runtime persistence implementation
- workflow runtime must consume metadata owned validators and must not redefine these schemas

## Non Goals

- encryption rollout
- remote blob services
- multi workspace orchestration
- generalized non docs workflow profiles

## Domain Contracts

Ownership:
- `src/metadata` owns key registry descriptors mutability policy visibility policy write validation and canonical workflow record schemas with validators
- `src/prompt_context` owns prompt and context artifact write read and digest verification
- `src/context` owns frame generation and query orchestration through explicit metadata contracts
- `src/workflow` owns runtime persistence and lifecycle of thread turn gate and prompt link records that consume metadata contracts

Boundary rules:
- frame metadata policy executes at one write boundary
- default read output must use registry visibility policy only
- privileged prompt and context retrieval is explicit and separate from default context output
- cross domain calls use explicit contracts and do not reach internal modules of other domains

## Code Reality Snapshot

Stable baseline from findings:
- shared write boundary is active and test guarded
- forbidden payload keys are rejected with typed errors
- queue and direct paths enforce the same write policy
- default metadata projection is centralized and registry driven

Open gaps from findings:
- generated metadata does not emit `context_digest`
- key descriptor model is missing mutability class retention and redaction fields
- prompt and context artifact domain is not implemented
- privileged artifact retrieval path is not implemented
- metadata owned canonical workflow schema and validator package does not exist

## Execution Tracks

### Track T1 Registry Descriptor Expansion

Deliverables:
- extend key descriptor model to include mutability class hash impact retention redaction and per key budget metadata
- keep bootstrap key set explicit with deterministic descriptor lookup

Acceptance:
- descriptor contract supports R2 policy evaluation without side tables
- unknown and forbidden key behavior remains deterministic

### Track T2 Write Boundary Upgrade

Deliverables:
- upgrade write validation to enforce mutability transitions and budget contracts from T1 descriptors
- ensure generated metadata includes required digest key set including `context_digest`
- keep queue and direct path behavior aligned through shared validation

Acceptance:
- generated writes include `prompt_digest` `context_digest` and `prompt_link_id`
- invalid writes fail with typed deterministic errors on direct and queue paths

### Track T3 Prompt Context Artifact Placement

Deliverables:
- add `src/prompt_context` domain for local content addressed prompt and context artifacts
- write rendered prompt and context payload into artifacts before frame write
- emit artifact references and digests in frame metadata and canonical prompt link contract payloads
- define logical commit and recovery behavior for the full lineage write unit

Acceptance:
- frame metadata never stores raw prompt text or raw context payload
- artifact reads verify digest and size
- lineage write unit behavior is deterministic on retry and failure

### Track T4 Read Contract Upgrade

Deliverables:
- preserve default read projection for visible keys only
- add explicit privileged query contract to resolve prompt and context payload by artifact reference
- keep CLI default output behavior unchanged for non privileged reads
- define compatibility read behavior for legacy frames without full digest key set
- enforce privileged authorization using authenticated identity scoped grants expiry and reason code
- emit immutable audit events for privileged allow and deny outcomes before payload return

Acceptance:
- default `context get` output remains safe by default
- privileged query path is explicit, test covered, and digest verified
- compatibility reads are deterministic for legacy and current frames
- unauthorized privileged reads fail deterministically and are audit logged
- authorized privileged reads are scope bound and audit logged

### Track T5 Workflow Consumer Schema Contracts

Deliverables:
- publish canonical metadata contracts for thread turn gate and prompt link schemas in metadata domain
- define versioning and compatibility rules for consumer domains
- define consumer integration seams for turn manager and future feature domains
- provide metadata owned validators for schema payload reference integrity

Acceptance:
- canonical schema contracts are stable and test covered
- record reference rules are explicit and validated
- consumer domains use metadata owned schema contracts and validators directly
- workflow persistence behavior remains out of scope for this phase

### Track T6 Verification Lock

Deliverables:
- extend parity and integration coverage for new descriptor, artifact, read, and workflow record behavior
- refresh fixtures where contract output intentionally changes
- complete readiness checklist for downstream turn manager and docs writer work

Acceptance:
- all gate classes in this spec pass
- no unresolved high risk migration issues remain

## Verification Strategy

Characterization gates:
- preserve deterministic valid frame write behavior
- preserve queue retry semantics for retryable provider failures

Contract gates:
- unknown key, forbidden key, mutability, and budget failures are typed and deterministic
- required digest key set is present on generated frame metadata
- default read output excludes hidden and forbidden keys
- canonical digest behavior is stable for replay with same logical input
- privileged read authorization behavior is deterministic for allow and deny paths

Artifact gates:
- prompt and context artifact writes are content addressed and digest verified
- privileged retrieval requires explicit access path
- lineage write unit recovery path is deterministic and idempotent
- privileged read events are immutable and persisted before payload release

Workflow record gates:
- canonical workflow record schema contracts are deterministic and versioned
- gate and prompt link reference rules are validated by metadata contract tests
- consumer conformance tests show turn manager contract import path with no schema redefinition

## Gate Matrix

Data safety:
- no raw prompt text in frame metadata
- no raw context payload in frame metadata
- digest and size checks pass for artifact reads

Write correctness:
- one shared write boundary enforces metadata contracts
- direct and queue writes share identical contract behavior

Read correctness:
- default read policy uses registry visibility only
- privileged retrieval is explicit and audited
- privileged retrieval enforces authenticated scoped grant checks
- privileged allow and deny outcomes are both audit logged

Workflow readiness:
- canonical record contracts for thread and turn state are available
- downstream docs writer and turn manager specs consume contracts without redesign

## Completion Criteria

The metadata contracts phase is complete when all statements are true:

1. T1 through T6 deliverables are implemented and verified
2. R1 and R2 requirements are enforced on all runtime paths
3. default and privileged read behavior are both deterministic and test covered
4. canonical workflow record contracts are available for downstream feature tracks
5. no additional foundation cleanup scope is needed to start turn manager execution work
