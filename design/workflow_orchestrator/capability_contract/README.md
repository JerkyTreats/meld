# Capability Contract

Date: 2026-03-09
Status: active

## Intent

Define one primitive task execution contract that every workflow task network can depend on.
This contract is the foundation for reliable orchestration, reliable configuration, and future HTN aligned decomposition.

## HTN Position

- each capability is the execution substrate for one primitive task family
- capabilities do not own decomposition, method choice, or cross capability orchestration
- workflow owns capability resolution and task network wiring before runtime starts
- capability contracts must declare enough structure for repair, retry, and explicit artifact handoff

## Provisional Answers

### Request Shape

- every capability request should include `workflow_run_id`, `plan_id`, `task_instance_id`, and `capability_id`
- every capability request should include declared target scope, scope digest, input artifact refs, and resolved profile bindings
- every capability request should include resolved agent and provider bindings when the primitive task depends on them
- every capability request should include execution policy such as retry envelope, timeout, and side effect expectations
- every capability request should be fully validated before execution starts except for runtime state checks that depend on live observations

### Result Shape

- every capability result should include structured status and stable reason codes
- every capability result should include named output artifacts and artifact metadata for downstream handoff
- every capability result should include an observation summary that captures runtime facts learned during execution
- every capability result should include side effect summary data so repair and replay logic remain explicit
- every capability result should include telemetry summary data aligned with durable workflow records

### Artifact Handoff

- downstream tasks should consume named output slots rather than implicit in memory values
- artifact handoff should be declared at workflow compile time and validated against input slot contracts
- capability summaries may be retained for operator visibility, but artifacts remain the durable interface between primitive tasks

### Registration And Resolution

- capability registration should live in `src/workflow` because workflow owns task compilation and plan validation
- concrete capability implementations should remain in their home domains such as `src/context` or `src/workspace`
- workflow should resolve a capability id plus profile binding into one concrete primitive task implementation before runtime starts
- resolution should fail at load time when ids, versions, slots, or profile bindings do not compose cleanly

## Initial Requirements

- each capability has a stable capability id and versioned config schema
- each capability exposes typed input slots and typed output slots
- each capability returns structured execution status, artifacts, observation summary data, and telemetry summary data
- each capability declares whether it is target scoped, batch scoped, or workflow scoped
- each capability declares preconditions, side effect class, and idempotency expectations
- capability config validation must happen before workflow execution starts

## Contract Fields

- `capability_id`
- `capability_version`
- `scope_kind`
- `input_slots`
- `output_slots`
- `preconditions`
- `side_effect_class`
- `idempotency_class`
- `observation_schema`
- `retry_guidance`
- `telemetry_fields`

## Design Goal

Make workflow configuration read like primitive task composition rather than a bag of loosely enforced fields.

## Residual Questions

- how expressive should first phase preconditions be before they become a second planning language
- how much observation data should be standardized versus left capability specific
- where should capability compatibility shims live during migration when one primitive task spans older runtime paths

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Workflow Definition](../workflow_definition/README.md)
- [Ordering Capability](../ordering_capability/README.md)
- [File Write Capability](../file_write_capability/README.md)
- [Telemetry Model](../telemetry_model/README.md)
