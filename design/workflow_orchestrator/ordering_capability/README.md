# Ordering Capability

Date: 2026-03-09
Status: active

## Intent

Extract target ordering into a reusable primitive capability that workflow can call directly.
Ordering should not remain hidden inside `context generate`.

## HTN Position

- ordering is a primitive planning support capability and not the planner itself
- workflow decides when ordering is needed and how its output feeds later primitive tasks
- ordering produces a stable target plan artifact that downstream task instances consume directly
- ordering must stay reusable across workflow execution, compatibility commands, and future hierarchical methods

## Provisional Answers

### First Phase Policies

- first class support should start with the current bottom up policy used by recursive generation
- the next safe extensions are single target, leaves only, and folders only
- broader policies should be added only when a real workflow method needs them

### Result Shape

- ordering should describe levels, target sets, dependency edges, and deterministic target identity mapping
- ordering output should include a `scope_digest` so downstream execution can detect stale or mismatched target assumptions
- ordering output should be stored as a workflow artifact rather than reconstructed ad hoc by later tasks

### Downstream Consumption

- downstream primitive tasks should receive ordering output as an explicit input artifact
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
- separate ordering policy from context artifact production
- keep target identity and path mapping deterministic

## Residual Questions

- should first phase ordering expose only level groups or also explicit edge lists for every target pair that matters
- when should watch driven updates reuse a prior ordering artifact versus request a fresh one

## Candidate Ownership

- `src/tree` is a strong fit if ordering is primarily about traversal and dependency shape
- `src/workspace` is a fit if ordering must remain close to workspace targeting and watch behavior
- `src/context` should consume ordering results rather than own ordering policy

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Context Generate Integration](../context_generate_integration/README.md)
- [Workflow Definition](../workflow_definition/README.md)
- [Migration Plan](../migration_plan/README.md)
