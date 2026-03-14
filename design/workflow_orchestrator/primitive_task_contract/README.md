# Primitive Task Contract

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Define one primitive task contract that every workflow task network can depend on.
This contract is the foundation for reliable task orchestration, reliable configuration, and future HTN aligned decomposition.

## HTN Position

- each primitive task contracts with one domain operation or one small domain owned execution surface
- primitive tasks do not own decomposition, method choice, or cross task orchestration
- workflow owns task resolution and task network wiring before runtime starts
- task contracts must declare enough structure for repair, retry, and explicit artifact handoff
- task contracts should also declare artifact schemas, capability requirements, and effect summaries so plan compilation can validate more than simple slot names

## Provisional Answers

### Request Shape

- every primitive task request should include `workflow_run_id`, `plan_id`, `task_instance_id`, and `task_type_id`
- every request should include declared target scope, scope digest, input artifact refs, artifact schema bindings, and resolved profile bindings
- every request should include resolved agent, provider, and capability bindings when the task depends on them
- every request should include the compiled plan digest and binding digest that authorized the task instance
- every request should include execution policy such as retry envelope, timeout, and side effect expectations
- every request should be fully validated before execution starts except for live state checks that depend on runtime observations

### Result Shape

- every result should include structured status and stable reason codes
- every result should include named output artifacts and artifact metadata for downstream handoff
- every result should include an observation summary that captures runtime facts learned during execution
- every result should include side effect summary data so repair and replay logic remain explicit
- every result should include effect summary data shaped for compensation, reuse, and audit decisions
- every result should include telemetry summary data aligned with durable workflow records

### Artifact Handoff

- downstream tasks should consume named output slots rather than implicit in memory values
- artifact handoff should be declared at workflow compile time and validated against input slot contracts plus artifact type ids and schema versions
- task summaries may be retained for operator visibility, but artifacts remain the durable interface between primitive tasks

### Capability And Effect Contracts

- every primitive task should declare capability requirements such as agent role, provider class, managed write scope, or storage access before runtime starts
- every primitive task should declare an effect summary shape so repair and compensation logic reason from structured outcomes rather than log text
- side effecting tasks should declare whether they support retry only, reuse, explicit compensation, or halt for operator review

### Registration And Resolution

- task registration should live in `src/workflow` because workflow owns task compilation and plan validation
- concrete execution adapters should remain in their home domains such as `src/context` or `src/workspace`
- workflow should resolve a `task_type_id` plus profile binding into one concrete primitive task implementation before runtime starts
- resolution should fail at load time when ids, versions, slots, artifact schemas, capability bindings, or profile bindings do not compose cleanly

## Initial Requirements

- each task type has a stable id and versioned config schema
- each task type exposes typed input slots and typed output slots
- each task type returns structured execution status, typed artifacts, observation summary data, effect summary data, and telemetry summary data
- each task type declares whether it is target scoped, batch scoped, or workflow scoped
- each task type declares preconditions, side effect class, and idempotency expectations
- each task type declares artifact schemas, capability requirements, and compensation posture
- task config validation must happen before workflow execution starts

## Contract Fields

- `task_type_id`
- `task_type_version`
- `scope_kind`
- `input_slots`
- `output_slots`
- `artifact_input_contracts`
- `artifact_output_contracts`
- `preconditions`
- `side_effect_class`
- `idempotency_class`
- `determinism_class`
- `capability_requirements`
- `effect_summary_schema`
- `compensation_policy`
- `observation_schema`
- `retry_guidance`
- `telemetry_fields`

## Design Goal

Make workflow configuration read like primitive task composition rather than a bag of loosely enforced fields.

## Residual Questions

- how expressive should first phase preconditions be before they become a second planning language
- how much observation data should be standardized versus left task specific
- where should compatibility shims live during migration when one primitive task spans older runtime paths

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Task Model](../task_model/README.md)
- [Ordering Task](../ordering_task/README.md)
- [File Write Task](../file_write_task/README.md)
- [Telemetry Model](../telemetry_model/README.md)
- [HTN Codebase Structure Report](../../research/htn/README.md)
