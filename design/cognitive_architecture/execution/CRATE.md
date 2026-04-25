# Execution Crate

Date: 2026-04-23
Status: active experiment
Scope: `meld-execution` crate boundary for planning, control, tasks, capability, workflow, and execution ports

## Intent

`meld-execution` owns deliberate action.
It reads planner-facing world-model views, chooses work, dispatches task and capability execution, requests provider work through a port, and publishes outcomes back into events.

The crate groups execution concerns instead of splitting every current top-level module into a crate.

## Target Crate

`meld-execution`

## Owns

- planning contracts and policy
- control state and repair
- task compilation and runtime
- capability contracts and invocation
- workflow execution
- provider execution port
- outcome fact publication
- missing evidence requests from planning

## Does Not Own

- canonical event append and replay
- graph materialization
- belief revision
- workspace source truth
- context storage truth
- provider registry and concrete client ownership
- CLI formatting
- app config loading

## Current Code Areas

- `src/control.rs`
- `src/control`
- `src/task.rs`
- `src/task`
- `src/capability.rs`
- `src/capability`
- `src/workflow.rs`
- `src/workflow`
- `src/merkle_traversal.rs`
- `src/merkle_traversal`
- provider execution contracts that execution needs

## Provider Posture

Provider execution policy belongs with execution.
Concrete provider registry, provider configuration, provider diagnostics, and provider CLI management remain in root `meld` for now.

Execution should depend on a provider execution port, not on root `meld`.

## Context And Provider Reliance

`meld-execution` will rely on context and provider capabilities in the current product shape.

That reliance should be explicit.
Execution owns the ports it needs:

- context read port
- context write port for produced artifacts and frames
- provider execution port
- event publication port
- world-model query port

Root `meld` supplies adapters for those ports during runtime wiring.

This keeps the dependency direction from becoming `meld-execution` to root `meld`.
It also lets context and provider remain in root `meld` while their long-term crate homes stay unresolved.

## Rejected Near Term Shapes

Do not create `meld-provider` now.
Provider and context are coupled by generation, but that does not make them one clean crate responsibility.

Do not move context and provider wholesale into `meld-execution`.
That would make execution own memory, frame storage, prompt assembly, provider configuration, and model backend management.

If a later crate becomes useful, prefer a generation-focused crate over a provider-focused crate.

## Extraction Blockers

- many execution paths accept `ContextApi` directly
- provider modules mix execution, configuration, diagnostics, concrete clients, and CLI tooling
- workflow currently reaches into context, prompt context, metadata, provider, task, and capability internals
- task and capability contracts are useful but still tied to context runtime

## Target Dependencies

| From | To | Reason |
| --- | --- | --- |
| `meld-execution` | `meld-events` | outcome publication and event contracts |
| `meld-execution` | `meld-world-model` public contracts | planner-facing belief and graph views |

## Forbidden Direction

`meld-execution` must not depend on root `meld`, world-model internals, CLI, or workspace internals.

Execution may request context, workspace, or provider capabilities only through explicit ports.

## Migration Path

1. Define execution ports for context reads, artifact writes, provider execution, and event publication.
2. Make planner inputs consume world-model views through public contracts.
3. Move task and capability contracts first.
4. Move workflow and control after `ContextApi` coupling is reduced.
5. Keep concrete provider and context adapters in root `meld` until a stable generation crate boundary appears.
