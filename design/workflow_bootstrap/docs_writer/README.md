# Docs Writer Thread Turn Configuration Spec

Date: 2026-03-01

## Intent

Define a concrete thread turn configuration for `docs_writer` that can be implemented with minimal new surface area.

This spec is normative for the first workflow bootstrap milestone.

This configuration is a workflow profile instance consumed by the generalized turn manager.
Execution behavior is owned by `../turn_manager/README.md`.

## Related Specs

- none yet

## Domain Ownership And Boundaries

Ownership:
- this file defines profile data for `docs_writer` workflow execution
- `src/workflow` owns thread turn gate orchestration for this profile
- `src/context` owns input frame retrieval for this profile
- `src/prompt_context` owns prompt render and context payload artifacts
- `src/metadata` owns metadata key contracts and validation

Boundary rules:
- profile data must remain declarative and must not require direct cross domain calls
- execution must use explicit contracts defined by turn manager and metadata specs

## Configuration Identity

- `workflow_id`: `docs_writer_thread_v1`
- `thread_profile`: `docs_writer_default`
- `target_agent_id`: `docs-writer`
- `target_frame_type`: `context-docs-writer`
- `final_artifact_type`: `readme_final`

## Generalized Schema Conformance

This profile uses the generalized workflow schema from `../turn_manager/README.md`.
Profile identity keys are optional extension fields.
Execution keys must match the generalized schema contracts.

## Thread Start Policy

A new thread starts when all conditions pass.

- target node exists
- target node is directory
- target frame type head exists
- no active thread with same run key

Run key fields:

- `workflow_id`
- `target_node_id`
- `target_frame_id`

Generalized field mapping:

- start conditions map to `thread_policy.start_conditions`
- run key fields map to `thread_policy.dedupe_key_fields`

## Turn Sequence

The thread has four turns in fixed order.

1. `evidence_gather`
2. `verification`
3. `readme_struct`
4. `style_refine`

No turn reordering is allowed in v1.

## Turn Config

### Turn 1 evidence_gather

Inputs:

- directory frame content for target node
- child frame contents used by directory frame

Prompt objective:

- extract concrete claims
- attach one evidence reference per claim

Output artifact:

- type `evidence_map`
- schema fields
- `claims`
- `claim_id`
- `statement`
- `evidence_path`
- `evidence_symbol`
- `evidence_quote`

Gate `evidence_gate`:

- every claim has `evidence_path`
- every claim has at least one symbol or quote
- no duplicate `claim_id`

### Turn 2 verification

Inputs:

- `evidence_map` artifact from turn 1

Prompt objective:

- validate each claim against cited evidence
- mark unsupported claims as rejected

Output artifact:

- type `verification_report`
- schema fields
- `verified_claims`
- `rejected_claims`
- `reasons`

Gate `verification_gate`:

- at least one verified claim exists
- every rejected claim has a reason code
- no claim appears in both verified and rejected sets

### Turn 3 readme_struct

Inputs:

- `verification_report` verified claims only

Prompt objective:

- build README structure from verified claims
- avoid style polish in this turn

Output artifact:

- type `readme_struct`
- schema fields
- `title`
- `sections`
- `scope`
- `purpose`
- `api_surface`
- `behavior_notes`
- `usage`
- `caveats`
- `related_components`

Gate `struct_gate`:

- all required sections present
- no rejected claim text present
- no evidence map section present

### Turn 4 style_refine

Inputs:

- `readme_struct` artifact from turn 3

Prompt objective:

- improve clarity and flow
- preserve technical meaning

Output artifact:

- type `readme_final`
- payload markdown for `README.md`

Gate `style_gate`:

- required sections preserved
- semantic drift score below threshold
- no evidence map section added

## Thread State Model

Thread statuses:

- `pending`
- `running`
- `completed`
- `failed`
- `aborted`

Turn statuses:

- `pending`
- `running`
- `completed`
- `failed`
- `skipped`

Failure policy:

- fail on first gate failure
- allow retry from failed turn only
- preserve prior completed turn artifacts

## Artifact Linking

Each turn output artifact must include:

- `artifact_id`
- `thread_id`
- `turn_id`
- `turn_seq`
- `input_artifact_ids`
- `created_at_ms`

Each thread must include ordered artifact chain:

- `evidence_map` to `verification_report` to `readme_struct` to `readme_final`

## Metadata Contract Binding

The thread configuration relies on metadata contracts from [Workflow Metadata Contracts Spec](../metadata_contracts/README.md).

Required frame metadata keys used by this workflow:

- `prompt_digest`
- `context_digest`
- `prompt_link_id`

Raw prompt text and raw context payload are forbidden in frame metadata.

## Minimal Execution Config Example

```yaml
workflow_id: docs_writer_thread_v1
version: 1
title: Docs Writer Turned Workflow
description: Four turn docs generation with deterministic gates
thread_profile: docs_writer_default
target_agent_id: docs-writer
target_frame_type: context-docs-writer
final_artifact_type: readme_final
thread_policy:
  start_conditions:
    require_directory_target: true
    require_target_head: true
  dedupe_key_fields:
    - workflow_id
    - target_node_id
    - target_frame_id
  max_turn_retries: 1
turns:
  - turn_id: evidence_gather
    seq: 1
    title: Gather Evidence
    prompt_ref: config/workflows/prompts/docs_writer/evidence_gather.md
    input_refs:
      - target_context
    output_type: evidence_map
    gate_id: evidence_gate
    retry_limit: 1
    timeout_ms: 60000
  - turn_id: verification
    seq: 2
    title: Verify Claims
    prompt_ref: config/workflows/prompts/docs_writer/verification.md
    input_refs:
      - evidence_map
    output_type: verification_report
    gate_id: verification_gate
    retry_limit: 1
    timeout_ms: 60000
  - turn_id: readme_struct
    seq: 3
    title: Build Readme Structure
    prompt_ref: config/workflows/prompts/docs_writer/readme_struct.md
    input_refs:
      - verification_report
    output_type: readme_struct
    gate_id: struct_gate
    retry_limit: 1
    timeout_ms: 60000
  - turn_id: style_refine
    seq: 4
    title: Refine Style
    prompt_ref: config/workflows/prompts/docs_writer/style_refine.md
    input_refs:
      - readme_struct
    output_type: readme_final
    gate_id: style_gate
    retry_limit: 1
    timeout_ms: 60000
gates:
  - gate_id: evidence_gate
    gate_type: schema_required_fields
    required_fields:
      - claims
    fail_on_violation: true
  - gate_id: verification_gate
    gate_type: schema_required_fields
    required_fields:
      - verified_claims
    fail_on_violation: true
  - gate_id: struct_gate
    gate_type: required_sections
    required_fields:
      - scope
      - purpose
      - api_surface
      - behavior_notes
      - usage
      - caveats
      - related_components
    fail_on_violation: true
  - gate_id: style_gate
    gate_type: no_semantic_drift
    required_fields:
      - readme_markdown
    fail_on_violation: true
artifact_policy:
  store_output: true
  store_prompt_render: true
  store_context_payload: true
  max_output_bytes: 262144
failure_policy:
  mode: fail_fast
  resume_from_failed_turn: true
  stop_on_gate_fail: true
```

## Verification Checklist

- thread starts only on valid thread policy
- exactly four turns execute in order
- each turn writes exactly one artifact
- each gate emits deterministic result record
- final output is markdown suitable for `README.md`
- no forbidden metadata keys present

## Future Work

Post feature exploration items live in [Workflow Bootstrap Future Work Backlog](../future_work.md).
