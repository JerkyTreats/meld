# Ordering Capability

Date: 2026-03-09
Status: active

## Intent

Extract target ordering into a reusable capability that workflow can call directly.
Ordering should not remain hidden inside `context generate`.

## Primary Questions

- what ordering policies need first class support
- how does ordering describe levels, target sets, and dependency edges
- what data should downstream capabilities receive from ordering
- which domain should own the implementation

## Initial Requirements

- support bottom up ordering for current recursive context generation behavior
- leave room for top down, leaves only, folders only, and future policies
- produce a stable ordered target plan that multiple commands can consume
- separate ordering policy from context artifact production
- keep target identity and path mapping deterministic

## Candidate Ownership

- `src/tree` is a strong fit if ordering is primarily about traversal and dependency shape
- `src/workspace` is a fit if ordering must remain close to workspace targeting and watch behavior
- `src/context` should consume ordering results rather than own ordering policy

## Related Areas

- [Context Generate Integration](../context_generate_integration/README.md)
- [Workflow Definition](../workflow_definition/README.md)
- [Migration Plan](../migration_plan/README.md)
