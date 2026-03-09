# Turn Manager Functional Specification

Date: 2026-03-06

## Intent

Define the desired behavior for a generalized turn manager that executes user defined multi turn workflows from configuration.

This document is a target state specification.

## Related Specs

- [Workflow Bootstrap Roadmap](../README.md)
- [Workflow Metadata Contracts Spec](../metadata_contracts/README.md)
- [Metadata Contracts Phase Technical Specification](../metadata_contracts/technical_spec.md)
- [Docs Writer Thread Turn Configuration Spec](../docs_writer/README.md)

## Scope

- workflow profile configuration format
- runtime workflow loading and validation
- generic turn execution semantics
- explicit agent to workflow binding with zero or one workflow per agent
- deterministic state persistence for thread, turn, gate, and artifact linkage

## Non Goals

- multi workflow binding per agent
- remote profile distribution
- dynamic code execution from profile content

## Domain Ownership And Boundaries

Ownership:
- `src/workflow` owns workflow profile registry, resolution, execution orchestration, and workflow state records
- `src/context` owns context read and frame retrieval contracts
- `src/prompt_context` owns prompt render and context payload artifact storage contracts
- `src/metadata` owns metadata key descriptors, mutability policy, and budget policy contracts

Boundary rules:
- workflow orchestration must live in `src/workflow`
- workflow domain may consume only public contracts from other domains
- workflow domain must not redefine metadata schemas owned by metadata contracts
- tooling and api adapters may parse and format only, then delegate to workflow orchestration

## Core Requirements

1. Workflow profiles are user owned configuration artifacts
2. Runtime loads workflow profiles from declared configuration sources
3. Turn execution order is stable and deterministic by declared sequence
4. Each turn resolves prompt input from file path or artifact id reference
5. Gates are declared in profile config and evaluated after each turn
6. Thread state and turn state are durably persisted as workflow records
7. Agent binding is explicit and supports zero or one workflow id per agent

## Metadata Contract Requirements

- Workflow state records must align to canonical thread turn gate and prompt link contracts
- Workflow domain must consume canonical validators and must not clone schema definitions
- Prompt and context artifact linkage must use digest and id references
- Retry behavior must preserve deterministic linkage semantics for all artifact references

## Configuration Model

Suggested profile layout:

- `config/workflows/<workflow_id>.yaml`
- `config/workflows/prompts/<workflow_id>/<prompt_name>.md`

Suggested load priority:

1. workspace scoped workflow profiles
2. user config scoped workflow profiles
3. built in default workflow profiles

Conflict rule:

- first profile by workflow id wins according to load priority

## Workflow Profile Schema

Top level fields:

- `workflow_id`
- `version`
- `title`
- `description`
- `thread_policy`
- `turns`
- `gates`
- `artifact_policy`
- `failure_policy`

Thread policy fields:

- `start_conditions`
- `dedupe_key_fields`
- `max_turn_retries`

Turn fields:

- `turn_id`
- `seq`
- `title`
- `prompt_ref`
- `input_refs`
- `output_type`
- `gate_id`
- `retry_limit`
- `timeout_ms`

Gate fields:

- `gate_id`
- `gate_type`
- `required_fields`
- `rules`
- `fail_on_violation`

Artifact policy fields:

- `store_output`
- `store_prompt_render`
- `store_context_payload`
- `max_output_bytes`

Failure policy fields:

- `mode`
- `resume_from_failed_turn`
- `stop_on_gate_fail`

## Optional Profile Extension Fields

Optional identity and routing fields may be present in selected profiles.

Optional fields:

- `thread_profile`
- `target_agent_id`
- `target_frame_type`
- `final_artifact_type`

These fields may specialize profile routing but must not change core execution semantics.

## Prompt Reference Model

`prompt_ref` supports:

- local file path reference
- content addressed artifact id reference

Required runtime behavior:

- resolve prompt source
- render prompt template with turn inputs
- persist prompt render artifact when policy allows
- provide rendered prompt payload to provider calls

## Agent Binding Model

Agent binding is explicit and optional.

Agent config extension:

- `workflow_id` optional field

Rules:

- agent with no workflow id uses one shot generation
- agent with workflow id runs the configured turn workflow
- invalid workflow id fails validation for write capable agents

## Runtime Components

### Registry

- load and index workflow profiles
- validate schema and cross reference integrity
- expose lookup by workflow id

### Resolver

- resolve agent workflow binding
- resolve prompt and input references per turn
- resolve artifact references for downstream turn inputs

### Executor

- execute turns in declared sequence
- persist turn output artifacts under policy rules
- evaluate gates after turn completion
- persist turn and gate record state

### State Store

- thread record storage
- turn record storage
- gate record storage
- prompt and output link record storage

## Determinism Rules

- turn ordering is stable by `seq`
- gate evaluation is pure for identical inputs
- artifact ids are content addressed
- retries reuse the same turn input snapshot contract

## Validation Rules

Profile load must fail on:

- duplicate `workflow_id` inside the same priority layer
- unresolved prompt reference
- duplicate turn sequence values
- unknown gate type
- unresolved gate reference from a turn

Runtime execution must fail on:

- turn output missing required fields
- gate failure under strict stop policy
- declared metadata or artifact budgets exceeded

## Minimal Profile Example

```yaml
workflow_id: docs_writer_thread_v1
version: 1
title: Docs Writer Turn Workflow
thread_policy:
  start_conditions:
    require_directory_target: true
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

## Future Work

Future exploration items live in [Workflow Bootstrap Future Work Backlog](../future_work.md).
