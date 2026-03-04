# Workflow Metadata Contracts Spec

Date: 2026-03-01

## Intent

Define the minimal metadata contracts required to bootstrap thread managed workflows without broad platform redesign.

This spec is focused on foundation quality for workflows.

## Related Specs

- [Phase Technical Specification](technical_spec.md)
- [Requirements Decomposition](decomposition.md)
- [Code Path Findings](code_path_findings.md)
- [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)
- [Metadata Contract Ready Cleanup](../foundation_cleanup/metadata_contract_ready/README.md)

## Scope

- metadata contracts required for thread and turn workflows
- relocation of context payload from frame metadata to local CAS artifacts
- canonical schema contracts for conversation metadata objects used by thread and turn workflows

## Canonical Domain Contract

Metadata contracts are defined in this workload as canonical domain contracts.
Conversation workflows and future features consume these contracts as stable inputs.

Canonical outputs for this phase:

- metadata key registry descriptor model and extension rules
- write boundary validation contract and typed error taxonomy
- read visibility and redaction contract for default and privileged paths
- digest and link contract for prompt and context artifact lineage
- canonical schema contracts for thread turn gate and prompt link records
- compatibility rules for legacy and current metadata states

Explicit non outputs for this phase:

- workflow runtime persistence implementation
- workflow executor orchestration implementation
- turn sequencing and gate blocking runtime behavior
- durable workflow record lifecycle implementation

## Phase Boundary

- metadata contracts phase defines canonical schema contracts and validation behavior for workflow records
- conversation metadata feature phase implements durable stores and runtime record lifecycle using those canonical contracts

## Connected Workflow Requirements

Connected workflow requirements are part of this metadata contract workload.

### RQ1 Atomic lineage write unit

- one lineage contract unit must define prompt artifact write context artifact write frame write with digest keys and prompt link contract payload
- success state requires all contract unit members represented in one committed lineage contract
- failure state must keep recovery deterministic through idempotent retry and explicit orphan cleanup policy
- runtime persistence of lineage unit members is implemented by downstream workflow features that consume this contract

### RQ2 Canonical digest inputs

- `prompt_digest` must hash canonical rendered prompt bytes
- `context_digest` must hash canonical context payload bytes
- canonical encoding and normalization rules must be defined in one metadata owned contract before runtime rollout

### RQ3 Compatibility and migration behavior

- new writes require full bootstrap digest key set
- reads must handle legacy frames that predate full digest key set with deterministic compatibility behavior
- migration and optional backfill strategy must be explicit and test covered

### RQ4 Workflow record schema binding

- thread turn gate and prompt link records must use one canonical schema set
- schema versioning must be explicit so turn manager and docs writer consume identical record contracts
- record payload references to frame and artifact ids must be validated by metadata contract validators

## Domain Ownership And Boundaries

Ownership:
- `src/metadata` owns metadata key registry mutability classes write validation and canonical workflow record schemas
- `src/prompt_context` owns prompt render and context payload artifacts
- `src/context` owns frame metadata emission with typed identifiers and digests only
- `src/workflow` owns runtime persistence and lifecycle for thread turn gate and artifact link records using metadata contracts

Boundary rules:
- metadata contract enforcement runs in `src/metadata` only
- `src/context` and `src/workflow` must not define private metadata key semantics
- `src/context` and `src/workflow` must not define private workflow record schemas
- cross domain calls must use explicit domain contracts
- raw prompt text and raw context payload must not cross the frame metadata boundary

## Consumer Usage Boundary

Metadata domain publishes canonical contract surfaces for consumers:

- key descriptor registry contract for metadata writes and reads
- write validation contract with typed deterministic errors
- digest and link lineage contract for prompt and context artifacts
- canonical schema contracts plus validators for thread turn gate and prompt link payloads
- compatibility contract for legacy and current metadata states

Consumer domains including turn manager and future workflow entities must:

- import canonical schema and validator contracts from metadata domain
- persist and query runtime records using metadata owned schema contracts
- treat metadata contract version as the source of compatibility truth

Consumer domains are not allowed to:

- redefine workflow record schemas in private modules
- bypass metadata validators at write boundaries
- reinterpret digest and link semantics outside canonical metadata contracts

## Out Of Scope

- full encryption rollout
- provider specific orchestration
- cross workspace workflow orchestration

## Core Contract

Context payload is not a metadata value.

Context payload must live as a local CAS artifact.
Metadata stores only typed identifiers and digests.

## Mutability Envelope

Every metadata key must declare one mutability class.

### identity

- hash critical
- immutable
- typed
- strict size limit

### attested

- not hash critical
- immutable after write
- digest referenced
- strict size limit

### annotation

- mutable
- append only event log
- materialized read view
- ttl and compaction required

### ephemeral

- runtime only
- not persisted in content addressed stores
- not emitted in user facing context output by default

## Metadata Key Registry Contract

Each key must include:

- key name
- owning domain
- schema type
- mutability class
- hash impact
- max bytes
- retention policy
- redaction policy
- default visibility policy

Unknown keys must be rejected on write for identity and attested classes.

## Required Key Set For Workflow Bootstrap

### Frame metadata keys

- `agent_id` as identity
- `provider` as attested
- `model` as attested
- `provider_type` as attested
- `prompt_digest` as attested
- `context_digest` as attested
- `prompt_link_id` as attested

Forbidden:

- raw prompt text in frame metadata
- raw context payload in frame metadata

## Conversation Metadata Objects

### Thread record

- `thread_id`
- `workflow_id`
- `target_node_id`
- `status`
- `created_at_ms`
- `updated_at_ms`
- `last_turn_seq`

### Turn record

- `thread_id`
- `turn_id`
- `turn_seq`
- `turn_type`
- `status`
- `input_artifact_ids`
- `output_artifact_id`
- `started_at_ms`
- `ended_at_ms`
- `error_code`
- `error_text`

### Turn gate record

- `thread_id`
- `turn_id`
- `gate_name`
- `outcome`
- `reasons`
- `evaluated_at_ms`

### Prompt link record

- `prompt_link_id`
- `thread_id`
- `turn_id`
- `node_id`
- `frame_id`
- `system_prompt_artifact_id`
- `user_prompt_template_artifact_id`
- `rendered_prompt_artifact_id`
- `context_artifact_id`
- `created_at_ms`

## Context Placement Contract

### write path

1. assemble prompt and context payload in memory
2. write prompt and context payloads to local CAS as immutable artifacts
3. write frame with digest references only
4. construct prompt link contract payload using canonical schema and validator

### read path

1. default context read returns frame content and digest references
2. privileged prompt query resolves raw prompt and context payloads by artifact id

## Privileged Access Contract

- default read surfaces never return raw prompt payload or raw context payload
- raw prompt and context payload reads require one explicit privileged route

Authorization contract:

- caller identity must be authenticated
- caller must hold scoped capabilities `prompt_payload_read` and `context_payload_read`
- grant scope must include workflow id thread id node id and requested turn range
- grant must include expiry timestamp and reason code

Audit contract:

- every privileged read allow and deny outcome emits one immutable audit event
- audit event fields include event id timestamp requester id grant id reason code scope artifact ids result outcome and policy version
- audit event persistence must complete before raw payload is returned

Access constraints:

- privileged reads resolve targeted artifact ids only
- size budgets and rate limits apply to privileged payload reads

## Size Budgets

Mandatory budgets for initial rollout:

- frame metadata total bytes limit
- per metadata key bytes limit
- prompt artifact max bytes
- context artifact max bytes

Write attempts above budget must fail with typed error.

## Verification Rules

- every artifact read must verify digest and size
- every turn gate and prompt link contract payload must pass canonical schema validator checks
- every validated record payload reference to artifact and frame ids must satisfy reference integrity rules
- default context output must not include raw prompt text

## Migration Plan

Phase R1 context placement refactor
- remove raw prompt metadata writes
- add digest keys and prompt link id
- add CAS artifact writes for rendered prompt and context payload

Phase R2 metadata contract refactor
- add metadata key registry and validation
- enforce mutability classes and size budgets
- enforce default visibility rules

## Future Work

Post feature exploration items live in [Workflow Bootstrap Future Work Backlog](../future_work.md).
