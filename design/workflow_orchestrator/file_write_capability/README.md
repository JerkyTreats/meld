# File Write Capability

Date: 2026-03-09
Status: active

## Intent

Define file materialization as a separate capability with explicit contracts and explicit write scope.

## Primary Questions

- what inputs are required for a file write step
- how should write scope be declared and validated
- how should materialized files relate to context frames and workflow artifacts
- how should workflows publish to repository files without causing avoidable self churn

## Initial Requirements

- file writing is separate from context frame head mutation
- workflow steps must declare target path policy before runtime
- file writes should consume explicit content artifacts rather than implicit in memory strings
- initial scope should support folder level `README.md` materialization
- capability output should report whether content changed, wrote, skipped, or failed

## Related Areas

- [Write Policy](../write_policy/README.md)
- [Telemetry Model](../telemetry_model/README.md)
- [Migration Plan](../migration_plan/README.md)
- [Publish Arbiter Idea](../../workflow_ideas/publish_arbiter_spec.md)
