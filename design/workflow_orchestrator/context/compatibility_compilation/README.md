# Compatibility Compilation

Date: 2026-03-14
Status: active

## Parent Requirements

- [Context Refactor Requirements](../README.md)
- [Migration Plan](../../migration_plan/README.md)

## Intent

Preserve current recursive `context generate` behavior while the implementation shifts from hidden domain orchestration to workflow compilation.
Compatibility should preserve the public command contract until the workflow substrate is complete.

## Current Pressure

- `src/context/generation/run.rs` still binds CLI level recursion decisions directly to target planning and queue execution
- current bottom up behavior is valuable product behavior and cannot regress during HTN preparation
- workflow aware execution already exists through `TargetExecutionProgram`, but the command path still carries transitional logic

## Requirements

- legacy command entry must compile into the same workflow aware execution contract used by later HTN methods
- recursive directory behavior must stay stable during migration
- compatibility translation must be explicit, testable, and removable after parity is proven
- no parallel hidden orchestration model should emerge beside workflow compilation

## Planned Deliverables

- one compatibility adapter for current `context generate` command inputs
- parity tests that compare legacy behavior to workflow backed behavior
- migration notes that define when direct recursive planning can be removed from context

## Verification Focus

- recursive directory runs preserve current level order and failure semantics
- command output remains stable for users and scripts
- workflow backed and non workflow backed agents share the same compatibility surface

## Related Code

- `src/context/generation/run.rs`
- `src/context/generation/program.rs`
- `src/context/generation/selection.rs`
- `src/workflow`
