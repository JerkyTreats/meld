# Turn Manager Generalized Spec

Date: 2026-03-01

## Intent

Define a runtime turn manager that executes user defined multi turn workflows from configuration.

This replaces hard coded workflow logic with profile driven behavior.

## Related Specs

- none yet

## Scope

- user defined workflow configuration
- runtime workflow loading and validation
- generic turn execution model
- agent to workflow binding with zero or one workflow per agent

## Domain Ownership And Boundaries

Ownership:
- `src/workflow` owns workflow registry resolver executor and state orchestration
- `src/context` owns frame and context retrieval
- `src/prompt_context` owns prompt render and context payload artifact storage
- `src/metadata` owns metadata key validation size budgets and mutability rules

Boundary rules:
- turn manager components live in `src/workflow` and delegate storage and validation by contract
- context reads are requested through `src/context` contracts only
- prompt render and context payload writes are requested through `src/prompt_context` contracts only
- metadata and budget enforcement is requested through `src/metadata` contracts only
- tooling and api adapters parse route and format only, then delegate to `src/workflow`

## Non Goals

- multi workflow binding per agent
- remote workflow distribution
- dynamic code execution from configuration

## Core Requirements

1. workflows are defined by user owned config files
2. turn manager loads workflow definitions at runtime
3. turns are ordered by declared sequence
4. each turn references prompt source through path or artifact id
5. gates are declared in config and evaluated after each turn
6. thread state and turn state are persisted durably
7. agent can bind to zero or one workflow

## Configuration Domain

Suggested directory layout:

- `config/workflows/<workflow_id>.yaml`
- `config/workflows/prompts/<workflow_id>/<prompt_name>.md`

Suggested load order:

1. workspace scoped workflow files
2. user config scoped workflow files
3. built in fallback profiles

Conflict rule:

- first match by workflow id wins by load order priority

## Workflow Definition Model

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

## Profile Extension Fields

Some workflow profiles may define optional top level identity fields for routing and targeting.
These fields do not change generalized turn manager execution semantics.

Optional fields:

- `thread_profile`
- `target_agent_id`
- `target_frame_type`
- `final_artifact_type`

## Prompt Reference Model

`prompt_ref` supports:

- local file path reference
- CAS artifact id reference

Runtime behavior:

- resolve prompt source
- render prompt template with turn inputs
- create prompt render artifact when policy allows
- provide in memory prompt payload to provider call

## Agent Binding Model

Agent binding is explicit and optional.

Agent config extension:

- `workflow_id` optional field

Rules:

- agent with no workflow uses one shot generation behavior
- agent with workflow id runs through turn manager
- invalid workflow id fails validation for write capable agents

## Turn Manager Runtime Components

### Registry

- loads workflow config files
- validates schemas and references
- exposes workflow lookup by id

### Resolver

- resolves agent workflow binding
- resolves prompt references
- resolves artifact inputs for each turn

### Executor

- executes turns in declared order
- writes turn artifacts
- runs declared gate
- records turn and gate state

### State Store

- thread record store
- turn record store
- gate record store
- artifact link store

## Determinism Rules

- turn order must be stable by `seq`
- gate evaluation must be pure for same inputs
- artifact ids must be content addressed
- retries must preserve turn input snapshot

## Validation Rules

Workflow load must fail when:

- duplicate `workflow_id` with same priority source
- missing prompt reference
- duplicate turn sequence
- unknown gate type
- missing gate reference from turn

Runtime execution must fail when:

- turn output is missing required fields
- gate fails with strict fail policy
- declared size budgets are exceeded

## Minimal Config Example

```yaml
workflow_id: docs_writer_thread_v1
version: 1
title: Docs Writer Turned Workflow
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

## Bootstrap Adoption Plan

1. add workflow registry and schema validation
2. add optional `workflow_id` field to agent config
3. route generation through turn manager when workflow is bound
4. migrate `docs_writer` to workflow profile config
5. keep one shot generation for agents without workflow binding

## Future Work

Post feature exploration items live in [Workflow Bootstrap Future Work Backlog](../future_work.md).
