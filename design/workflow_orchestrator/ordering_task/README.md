# Ordering Task

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Extract target ordering into a reusable atomic task that workflow can call directly.
Ordering should not remain hidden inside `context generate`.

## HTN Position

- ordering is an atomic planning support task and not the planner itself
- workflow decides when ordering is needed and how its output feeds later tasks
- ordering produces a stable target plan artifact that downstream task instances consume directly
- ordering must stay reusable across workflow execution, compatibility commands, and future hierarchical methods
- ordering output should support explicit dependency graph semantics so later methods are not limited to one linear traversal model

## Provisional Answers

### First Phase Policies

- first class support should start with the current bottom up policy used by recursive generation
- the next safe extensions are single target, leaves only, and folders only
- broader policies should be added only when a real workflow method needs them

### Result Shape

- ordering should describe levels, target sets, dependency edges, and deterministic target identity mapping
- ordering output should include a `scope_digest`, `artifact_type_id`, and `artifact_schema_version` so downstream execution can detect stale or mismatched target assumptions
- ordering output should be stored as a workflow artifact rather than reconstructed ad hoc by later tasks

### Downstream Consumption

- downstream tasks should receive ordering output as an explicit input artifact
- generation and publish paths should not recompute traversal logic once ordering has been compiled and recorded
- workflow should validate that each downstream task expects the same target identity model exposed by ordering

### Ownership

- `src/tree` is the best fit if ordering remains primarily about traversal and dependency shape
- `src/workflow` should own ordering invocation, artifact binding, and compile time validation
- `src/context` should consume ordering results rather than own ordering policy

## Initial Requirements

- support bottom up ordering for current recursive context generation behavior
- leave room for top down, leaves only, folders only, and future policies
- produce a stable ordered target plan that multiple commands can consume
- represent dependency shape explicitly enough to support later partially ordered task networks
- separate ordering policy from context artifact production
- keep target identity and path mapping deterministic

## Residual Questions

- should first phase ordering expose only level groups or also explicit edge lists for every target relationship that matters
- when should watch driven updates reuse a prior ordering artifact versus request a fresh one

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Task Model](../task_model/README.md)
- [Context Generate Task](../context_generate_task/README.md)
- [Workflow Definition](../workflow_definition/README.md)
- [Migration Plan](../migration_plan/README.md)
- [HTN Codebase Structure Report](../../research/htn/README.md)
