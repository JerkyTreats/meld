# Telemetry Model

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Define telemetry for HTN ready workflow execution with enough detail for planning visibility, runtime debugging, durable summaries, and repair.

## HTN Position

- telemetry must cover both planning time and execution time behavior
- telemetry must explain why a task network was compiled the way it was and how execution diverged when it does
- telemetry must align with durable workflow state so repair and resume decisions remain auditable
- compact CLI summaries should be projections from richer durable records rather than a separate truth source
- compiler phase telemetry should be rich enough to explain method choice, schema binding, and capability binding without opening domain specific logs

## Provisional Answers

### Event Layers

- planning layer records top level task selection, method selection, rejected methods, plan digest, and compile validation outcomes
- planning layer should also record compiler phase transitions such as task catalog resolution, artifact schema binding, capability binding, and final compiled plan emission
- task network layer records task instance creation, dependency resolution, artifact handoff wiring, and checkpoint creation
- atomic task layer records start, finish, skip, retry, reuse, failure, and observation outcomes for each task instance
- artifact layer records artifact creation, handoff, materialization, divergence, publish, and compensation outcomes

### Durable Versus Compact Records

- durable records should preserve planning choices, task instance traces, reason codes, and artifact lineage for replay and repair
- durable records should preserve compiled plan digest inputs such as method versions, scope digest, binding digest, and artifact schema bindings
- compact summaries should report workflow result, task counts, retry counts, write outcomes, and key divergence reasons for CLI and logs
- no compact summary should omit the plan identity or the workflow run identity

### Required Outcome Visibility

- skip, reuse, retry, divergence, publish, resume, and repair must each have stable reason codes
- write behavior must clearly distinguish skipped because unchanged, skipped because policy, wrote successfully, and halted because divergence
- task failures must carry enough context to determine whether repair, resume, or full restart is appropriate

### Alignment With Durable State

- every planning event should reference `workflow_run_id` and `plan_id`
- every execution event should reference `task_instance_id` and when relevant `artifact_id`
- checkpoints and repair records should be emitted as both durable state transitions and telemetry events
- telemetry should be derived from workflow owned records rather than inferred loosely from domain log lines
- compile diagnostics should reference `method_id`, `method_version`, and when relevant the rejected or selected capability binding

## Initial Requirements

- unify planning level, workflow level, atomic task level, batch level, and target level visibility
- produce compact summary events for CLI and logs
- preserve enough detail to diagnose invalid configuration, decomposition errors, and partial execution failures
- make write skip versus write change outcomes obvious
- keep telemetry aligned with durable workflow state records
- expose compiler phase events clearly enough to debug invalid task graphs before runtime begins

## Event Families

- `plan_selected`
- `workflow_definition_validated`
- `method_selected`
- `method_rejected`
- `task_catalog_resolved`
- `artifact_schema_bound`
- `capability_bound`
- `plan_compiled`
- `checkpoint_created`
- `task_started`
- `task_finished`
- `task_skipped`
- `task_retried`
- `task_reused`
- `task_failed`
- `artifact_created`
- `artifact_handoff_bound`
- `materialization_diverged`
- `materialization_applied`
- `repair_started`
- `repair_resolved`
- `workflow_resumed`

## Residual Questions

- how much method selection detail should be preserved by default before event volume becomes too costly
- which rejected method reasons deserve durable storage versus short lived diagnostics only
- how should telemetry sampling behave for large batch runs without weakening repair and audit value

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Primitive Task Contract](../primitive_task_contract/README.md)
- [File Write Task](../file_write_task/README.md)
- [Write Policy](../write_policy/README.md)
- [Migration Plan](../migration_plan/README.md)
- [HTN Codebase Structure Report](../../research/htn/README.md)
