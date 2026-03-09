# Migration Plan

Date: 2026-03-09
Status: active

## Intent

Define a compatibility path from the current turn workflow runtime to a broader capability orchestrator.

## Primary Questions

- how does the current turn runtime fit inside the orchestrator model
- what compatibility layer keeps `context generate` and `workflow execute` stable during transition
- what config changes are safe to introduce first
- how do existing docs writer profiles and bindings migrate without breaking stored state assumptions

## Initial Requirements

- keep current command behavior stable during the transition period
- treat the current turn runtime as a compatibility capability behind the new orchestrator contract
- allow new capability workflows to coexist with current turn profiles during migration
- define cutover rules for profile ids, frame types, and resume state
- require characterization tests before removing old execution paths

## Related Areas

- [Workflow Definition](../workflow_definition/README.md)
- [Context Generate Integration](../context_generate_integration/README.md)
- [Completed Workflow Bootstrap](../../completed/workflow_bootstrap/README.md)
