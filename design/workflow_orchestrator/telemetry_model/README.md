# Telemetry Model

Date: 2026-03-09
Status: active

## Intent

Define telemetry for HTN ready workflow execution with enough detail for planning visibility, runtime debugging, durable summaries, and repair.

## HTN Position

- telemetry must cover both planning time and execution time behavior
- telemetry must explain why a task network was compiled the way it was and how execution diverged when it does
- telemetry must align with durable workflow state so repair and resume decisions remain auditable
- compact CLI summaries should be projections from richer durable records rather than a separate truth source

## Provisional Answers

### Event Layers

- planning layer records top level task selection, method selection, rejected methods, plan digest, and compile validation outcomes
- task network layer records task instance creation, dependency resolution, artifact handoff wiring, and checkpoint creation
- primitive task layer records start, finish, skip, retry, reuse, failure, and observation outcomes for each task instance
- artifact layer records artifact creation, handoff, materialization, divergence, publish, and compensation outcomes

### Durable Versus Compact Records

- durable records should preserve planning choices, task instance traces, reason codes, and artifact lineage for replay and repair
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

## Initial Requirements

- unify planning level, workflow level, capability level, batch level, and target level visibility
- produce compact summary events for CLI and logs
- preserve enough detail to diagnose invalid configuration, decomposition errors, and partial execution failures
- make write skip versus write change outcomes obvious
- keep telemetry aligned with durable workflow state records

## Event Families

- `plan_selected`
- `method_selected`
- `method_rejected`
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

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Capability Contract](../capability_contract/README.md)
- [File Write Capability](../file_write_capability/README.md)
- [Write Policy](../write_policy/README.md)
- [Migration Plan](../migration_plan/README.md)

## Residual Questions

- how much method selection detail should be preserved by default before event volume becomes too costly
- which rejected method reasons deserve durable storage versus short lived diagnostics only
- how should telemetry sampling behave for large batch runs without weakening repair and audit value
