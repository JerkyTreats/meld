# Telemetry Model

Date: 2026-03-09
Status: active

## Intent

Define telemetry for capability workflows with enough detail for CLI feedback, debugging, and durable summaries.

## Primary Questions

- what events belong at workflow scope versus capability scope versus target scope
- what summary data should survive after large batch runs
- how should telemetry represent skip, reuse, retry, publish, and resume behavior
- how should telemetry stay compact without losing debugging value

## Initial Requirements

- unify workflow level, capability level, batch level, and target level visibility
- produce compact summary events for CLI and logs
- preserve enough detail to diagnose invalid configuration and partial execution failures
- make write skip versus write change outcomes obvious
- keep telemetry aligned with durable workflow state records

## Related Areas

- [Capability Contract](../capability_contract/README.md)
- [File Write Capability](../file_write_capability/README.md)
- [Write Policy](../write_policy/README.md)
- [Migration Plan](../migration_plan/README.md)
