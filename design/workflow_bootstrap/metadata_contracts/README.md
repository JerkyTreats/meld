# Workflow Metadata Contracts Spec

Date: 2026-03-01

## Intent

Define the minimal metadata contracts required to bootstrap thread managed workflows without broad platform redesign.

This spec is focused on foundation quality for workflows.

## Related Specs

- [Phase Technical Specification](technical_spec.md)
- [Boundary Cleanup Foundation Spec](../foundation_cleanup/README.md)
- [Metadata Contract Ready Cleanup](../foundation_cleanup/metadata_contract_ready/README.md)
- [Post Cleanup Findings](code_path_findings.md)

## Scope

- metadata contracts required for thread and turn workflows
- relocation of context payload from frame metadata to local CAS artifacts
- conversation metadata objects for thread and turn state

## Domain Ownership And Boundaries

Ownership:
- `src/metadata` owns metadata key registry mutability classes and write validation
- `src/prompt_context` owns prompt render and context payload artifacts
- `src/context` owns frame metadata emission with typed identifiers and digests only
- `src/workflow` owns thread turn gate and artifact link records

Boundary rules:
- metadata contract enforcement runs in `src/metadata` only
- `src/context` and `src/workflow` must not define private metadata key semantics
- cross domain calls must use explicit domain contracts
- raw prompt text and raw context payload must not cross the frame metadata boundary

## Out Of Scope

- full encryption rollout
- provider specific orchestration
- cross workspace workflow orchestration

## Core Decision

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
4. write prompt link record

### read path

1. default context read returns frame content and digest references
2. privileged prompt query resolves raw prompt and context payloads by artifact id

## Size Budgets

Mandatory budgets for initial rollout:

- frame metadata total bytes limit
- per metadata key bytes limit
- prompt artifact max bytes
- context artifact max bytes

Write attempts above budget must fail with typed error.

## Verification Rules

- every artifact read must verify digest and size
- every turn record must reference valid artifact ids
- every gate record must reference existing turn id
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
