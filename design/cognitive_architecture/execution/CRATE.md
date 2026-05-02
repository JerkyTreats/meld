# Execution Crate

Date: 2026-05-02
Status: completed authority crate
Scope: `meld-execution` crate boundary for execution-owned contracts, task and capability authority, workflow runtime, and runtime ports

## Intent

`meld-execution` owns the public contract boundary and runtime algorithms for deliberate action.
It defines the provider execution request shape, task and capability contracts, workflow execution runtime, and the ports that let execution code read context, dispatch provider work, query the world model, load workflow profiles, write generated frames, and publish outcomes without depending on root `meld`.

Root `meld` remains the product shell and adapter host for current concrete runtime implementations.

## Target Crate

`meld-execution`

## Owns

- provider execution binding contracts
- provider runtime override validation
- context read and write port contracts
- prompt artifact read and write port contracts
- node resolution port contract
- provider validation and execution port contracts
- event publication port contract
- world-model query port contract
- workflow profile load port contract
- workflow execution request, summary, progress, and deterministic identity contracts
- live workflow executor algorithm
- task authority contracts, compiler, artifact repo, initialization, runtime loop, and task-package lowering
- capability authority contracts, catalog, invocation payloads, and executor registry contracts
- combined execution context contract

## Does Not Own

- canonical event append and replay
- graph materialization
- belief revision
- workspace source truth
- context storage truth
- provider registry and concrete client ownership
- root task-path capability registration for workspace, traversal, context, and provider product adapters
- concrete provider client ownership
- CLI formatting
- app config loading

## Current Code Areas

- `crates/meld-execution/src/execution/contracts.rs`
- `crates/meld-execution/src/execution/ports.rs`
- `crates/meld-execution/src/capability.rs`
- `crates/meld-execution/src/task.rs`
- `crates/meld-execution/src/workflow/executor.rs`
- `crates/meld-execution/src/workflow/progress.rs`
- root adapter bindings in `src/execution/ports.rs`
- root workflow runtime input adapter in `src/workflow/executor.rs`

## Provider Posture

Provider execution policy belongs with execution.
Concrete provider registry, provider configuration, provider diagnostics, provider clients, and provider CLI management remain in root `meld`.

Execution code should depend on the provider execution port, not on root `meld`.

## Context And Provider Reliance

`meld-execution` relies on context and provider capabilities in the current product shape.

That reliance should be explicit.
The extracted crate owns the ports it needs:

- context read port
- context write port for produced artifacts and frames
- provider execution port
- event publication port
- world-model query port
- workflow profile load port
- generated frame construction supplied by root runtime input
- task-path capability execution supplied by a root runtime bundle

Root `meld` supplies adapters for those ports during runtime wiring.

This keeps the dependency direction from becoming `meld-execution` to root `meld`.
It also lets context and provider remain in root `meld` while their long-term crate homes stay unresolved.

## Rejected Near Term Shapes

Do not create `meld-provider` now.
Provider and context are coupled by generation, but that does not make them one clean crate responsibility.

Do not move context and provider wholesale into `meld-execution`.
That would make execution own memory, frame storage, prompt assembly, provider configuration, and model backend management.

If a later crate becomes useful, prefer a generation-focused crate over a provider-focused crate.

## Root Adapter Posture

Root `meld` binds the associated-type port contracts to the product's current concrete types.
Those wrappers are compatibility adapters, not the long-term authority surface.

The extraction intentionally leaves root-owned storage, config, CLI, provider registry, provider clients, and workspace-specific capability registration in root `meld`.
Root compatibility adapters provide those concrete product concerns to `meld-execution` through explicit ports and runtime inputs.

## Target Dependencies

| From | To | Reason |
| --- | --- | --- |
| `meld-execution` | none of the root `meld` crate | contract boundary must remain root independent |

## Forbidden Direction

`meld-execution` must not depend on root `meld`, world-model internals, CLI, or workspace internals.

Execution may request context, workspace, or provider capabilities only through explicit ports.

## Public Contract Shape

The port traits are associated-type contracts so `meld-execution` can define the execution boundary without importing root data types.
Root `meld` supplies the concrete bindings for `ContextApi`, prompt artifact storage, provider execution, workflow profile loading, and world-model query access.

This keeps the dependency direction stable while allowing root `meld` to remain the product composition shell.
