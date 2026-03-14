# File Write Task

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Define file materialization as a separate atomic task with explicit contracts and explicit write scope.

## HTN Position

- file write is a side effecting atomic task and not an incidental consequence of context generation
- workflow owns when file write is invoked and which artifact is eligible for materialization
- file write must expose enough structure for repair, retry, divergence handling, and watch safety
- file write should consume explicit workflow artifacts and produce explicit materialization records
- file write should declare managed scope, capability requirements, and effect summaries at compile time rather than rely on local runtime convention

## Provisional Answers

### Required Inputs

- file write should require a content artifact ref rather than an implicit in memory string
- file write should require a repo relative target path binding
- file write should require a write policy id and declared write scope
- file write should require provenance metadata that links the content artifact to workflow execution state
- file write should require artifact type and schema bindings for the content it will materialize
- file write should optionally consume a prior materialization record when change suppression or divergence checks need it

### Scope Declaration

- write scope should be declared in workflow config and validated before runtime starts
- workflow should reject any plan whose file write task falls outside declared managed scope
- first phase scope can be narrow and explicit, such as folder level `README.md` materialization only

### Relation To Frames And Artifacts

- file writes should consume workflow artifacts that may have been derived from context frames, but they should not mutate context frame heads as part of file materialization
- context frames remain immutable lineage sources while file writes produce repository side effects and materialization records
- materialization records should preserve plan digest, binding digest, and managed scope information for later repair and audit

### Self Churn Avoidance

- file write should report managed output metadata so watch and publish logic can suppress self caused churn
- workflow should pair file write with explicit write policy and divergence handling rather than relying on local file comparison alone

## Initial Requirements

- file writing is separate from context frame head mutation
- workflow steps must declare target path policy before runtime
- file writes should consume explicit content artifacts rather than implicit in memory strings
- initial scope should support folder level `README.md` materialization
- task output should report whether content changed, wrote, skipped, or failed
- task output should include effect summary fields strong enough for reuse, halt, or compensation decisions

## Expected Output States

- `wrote`
- `skipped_same_content`
- `skipped_policy`
- `divergence_detected`
- `failed`

## Residual Questions

- should first phase file write support only one content artifact input or allow a small merge contract for later publishing cases
- what minimum materialization record is needed so repair can decide between retry, halt, or operator review

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Task Model](../task_model/README.md)
- [Write Policy](../write_policy/README.md)
- [Telemetry Model](../telemetry_model/README.md)
- [Migration Plan](../migration_plan/README.md)
- [Publish Arbiter Idea](../../workflow_ideas/publish_arbiter_spec.md)
- [HTN Codebase Structure Report](../../research/htn/README.md)
