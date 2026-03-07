# Capability Orchestration TODO

Date: 2026-03-07
Status: active

## Intent

Capture the next design subjects so progress can stay outcome focused while the team ships CLI feedback first.

This file is a lightweight holding area and can later split into separate plans and specifications.

## Working Direction

- workflows orchestrate capabilities
- context generate is a capability
- write to file is a capability
- merkle ordering is not owned by context
- context consumes ordered target plans rather than computing ordering policy itself

## Subject List

### Workflow as meta domain

- define workflow as the orchestrator of capabilities rather than only turn management
- define how workflow sequences capability runs and passes artifacts between them
- define resume semantics for multi capability workflows

### Capability contract

- define one capability execution contract with request result telemetry and artifact handoff
- decide where capability registration lives
- decide how workflow resolves capability ids to implementations

### Merkle ordering capability

- extract reusable ordering from context generation
- support bottom up top down leaves only folders only and future policies
- ensure ordering can be consumed by multiple commands and workflows

### Context generate as capability consumer

- refactor `context generate` to consume submitted target order rather than own ordering logic
- keep frame generation focused on context artifact production
- preserve current recursive behavior through an ordering input adapter during migration

### File write capability

- define write to file as a separate capability
- scope initial use to folder level README materialization
- separate file writes from context frame head mutation

### Non regressive write policy

- define how generated files avoid unnecessary merkle churn
- decide when file writes should update workspace state
- decide how workflows can compare intended file content to current file content before writing

### Agent chaining across capabilities

- decide whether workflow names agents directly or capability profiles bind agents internally
- support sequences such as merkle ordering then context generation then file write

### Telemetry model for capability workflows

- unify workflow level capability level batch level and target level visibility
- define compact summary events for capability workflows
- keep telemetry sufficient for CLI feedback and debugging

### CLI feedback

- implement compact `stderr` live rendering first
- validate that the renderer consumes telemetry only
- use this work as the first user facing outcome from the broader architecture

### Migration plan

- define compatibility path from current workflow turn runtime to capability orchestration runtime
- keep existing `context generate` and `workflow execute` behavior stable during transition

## Suggested Next Outcome Order

1. ship CLI feedback
2. design capability contract
3. design merkle ordering capability
4. design file write capability
5. design migration path from turn workflows to capability workflows

## Notes For Later Refinement

- this list should likely split into a roadmap file and several technical specs
- merkle ordering may belong under `src/tree` or a dedicated traversal domain
- file write policy needs a deeper discussion about workspace updates and head semantics
