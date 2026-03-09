# Workflow Definition

Date: 2026-03-09
Status: active

## Intent

Define workflow as the orchestration layer above individual capabilities.
Workflow should no longer mean only turn sequencing for one concrete docs flow.

## Primary Questions

- what is the smallest stable definition of a workflow
- what parts belong to workflow planning versus capability execution
- what are the durable runtime units such as workflow run, step, target batch, and artifact handoff
- how does workflow resume after partial success across multiple capabilities

## Initial Requirements

- workflow config compiles into a validated execution plan with stable step ids
- workflow steps declare dependencies and artifact handoff explicitly
- workflow runtime owns step status, retry policy, and resume checkpoints
- workflow should orchestrate both target local steps and batch scoped steps
- workflow should support capability graphs that are broader than one prompt thread

## Current Code Pressure

- `src/workflow` is currently centered on turn execution and gate checks
- `src/context` still owns major orchestration concerns such as ordering and recursive planning
- the current profile surface looks broader than the runtime semantics it truly enforces

## Related Areas

- [Capability Contract](../capability_contract/README.md)
- [Migration Plan](../migration_plan/README.md)
- [Completed Workflow Bootstrap](../../completed/workflow_bootstrap/README.md)
