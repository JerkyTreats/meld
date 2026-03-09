# Write Policy

Date: 2026-03-09
Status: active

## Intent

Define non regressive write policy so workflow outputs do not create unnecessary churn.

## Primary Questions

- when should a workflow write a file versus skip the write
- how should intended content be compared to current repository state
- when should workspace state be refreshed after writes
- what should happen when the repository has diverged from the last workflow artifact

## Initial Requirements

- compare intended file content to current file content before writing
- write only when resulting content is materially different
- keep write policy deterministic for the same inputs and repository state
- report write reason in telemetry and workflow summaries
- define how workflow writes interact with workspace updates and future watch behavior

## Related Areas

- [File Write Capability](../file_write_capability/README.md)
- [Telemetry Model](../telemetry_model/README.md)
- [Migration Plan](../migration_plan/README.md)
