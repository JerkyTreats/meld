# Execution Contract Extraction

Date: 2026-04-25
Status: completed for the execution contract extraction slice
Purpose: make [Execution Crate](../execution/CRATE.md) true through code changes in `src`

## Intent

`meld-execution` should own the root-independent contract boundary for deliberate action.
That includes provider execution contracts and the ports that current execution runtime code uses to request context, prompt artifacts, provider work, event publication, world-model reads, and workflow profiles.

This document records the migration slice that extracted those contracts while keeping concrete runtime adapters in root `meld`.

## Starting Point In `src`

Execution owned logic is spread across:

- [src/control.rs](../../../src/control.rs)
- [src/task.rs](../../../src/task.rs)
- [src/capability.rs](../../../src/capability.rs)
- [src/workflow.rs](../../../src/workflow.rs)
- [src/merkle_traversal.rs](../../../src/merkle_traversal.rs)

The main blockers were explicit in the imports at the start of this slice:

- capability invocation depends on `ContextApi` in [src/capability/invocation.rs](../../../src/capability/invocation.rs#L3) and [src/capability/invocation.rs](../../../src/capability/invocation.rs#L59)
- task runtime depends on `ContextApi` and telemetry workflow event types in [src/task/runtime.rs](../../../src/task/runtime.rs#L3) and [src/task/runtime.rs](../../../src/task/runtime.rs#L13)
- workflow facade still loads config and registry in [src/workflow/facade.rs](../../../src/workflow/facade.rs#L4) and [src/workflow/facade.rs](../../../src/workflow/facade.rs#L29)
- workflow commands resolve workspace node ids and validate provider existence in [src/workflow/commands.rs](../../../src/workflow/commands.rs#L113) through [src/workflow/commands.rs](../../../src/workflow/commands.rs#L176)
- workflow executor reaches into metadata, prompt context, provider helpers, and world model query in [src/workflow/executor.rs](../../../src/workflow/executor.rs#L10) through [src/workflow/executor.rs](../../../src/workflow/executor.rs#L51)
- task package preparation still resolves workspace nodes through root workspace code in [src/task/package/prepare.rs](../../../src/task/package/prepare.rs#L24) and [src/task/package/prepare.rs](../../../src/task/package/prepare.rs#L230)
- `ProviderExecutionBinding` lived under provider-facing paths before moving to `meld-execution`

## Target State

For this extraction slice, `meld-execution` should:

- define provider execution contracts
- define execution-owned ports without importing root `meld`
- let root execution runtime code consume context, prompt artifacts, providers, workflow profiles, and world-model reads through those ports
- publish outcomes through an event publication port contract
- avoid direct imports of root `meld`, CLI, workspace internals, or telemetry internals

## Required Public Ports

The current codebase shows that the initial public execution boundary needs more than the port list in `CRATE.md`.
The minimum useful set is:

- context read port
- context write port
- prompt artifact read port
- node resolution port
- provider execution port
- provider registry validation port
- event publication port
- world model query port
- workflow profile load port

## Required Code Changes

### 1. Replace `ContextApi` with execution ports

This is the central migration.
Without it, `meld-execution` remains a view into root `meld`.

Required work:

- define execution owned traits for the required ports: completed in [crates/meld-execution/src/execution/ports.rs](../../../crates/meld-execution/src/execution/ports.rs)
- add adapter implementations in root `meld`: completed in [src/execution/ports.rs](../../../src/execution/ports.rs)
- change capability, task, and workflow entrypoints to depend on those traits: completed through root compatibility wrappers that bind to the extracted crate contracts
- keep `ContextApi` only as a compatibility wrapper while call sites move: completed for execution runtime boundaries

Primary files:

- [src/capability/invocation.rs](../../../src/capability/invocation.rs)
- [src/task/runtime.rs](../../../src/task/runtime.rs)
- [src/workflow/facade.rs](../../../src/workflow/facade.rs)
- [src/workflow/executor.rs](../../../src/workflow/executor.rs)
- [src/task/package/prepare.rs](../../../src/task/package/prepare.rs)

### 2. Split workflow core from workflow adapters

Not all of `src/workflow` belongs in the extracted crate.
Some files are runtime authority.
Some are CLI and root integration adapters.

Execution owned candidates:

- [src/workflow/executor.rs](../../../src/workflow/executor.rs)
- [src/workflow/resolver.rs](../../../src/workflow/resolver.rs)
- [src/workflow/gates.rs](../../../src/workflow/gates.rs)
- [src/workflow/profile.rs](../../../src/workflow/profile.rs)
- [src/workflow/registry.rs](../../../src/workflow/registry.rs)
- [src/workflow/state_store.rs](../../../src/workflow/state_store.rs)
- [src/workflow/events.rs](../../../src/workflow/events.rs)
- [src/workflow/record_contracts.rs](../../../src/workflow/record_contracts.rs)

Root adapter candidates:

- [src/workflow/tooling.rs](../../../src/workflow/tooling.rs)
- [src/workflow/commands.rs](../../../src/workflow/commands.rs)
- parts of [src/workflow/facade.rs](../../../src/workflow/facade.rs) that load config or resolve product routing

Required work:

- separate workflow runtime APIs from CLI and config adapter code
- move root owned config loading and workspace path resolution behind ports
- make command code call execution crate APIs

### 3. Decouple execution from root telemetry types

Execution currently emits workflow turn compatibility events through task runtime.
That is an observer concern, not execution authority.

Required work:

- move execution event payload types into execution owned contracts
- or replace direct telemetry coupling with an observer port
- keep telemetry formatting and summaries in root `meld`

Primary files:

- [src/task/runtime.rs](../../../src/task/runtime.rs)
- [src/workflow/events.rs](../../../src/workflow/events.rs)
- [src/telemetry](../../../src/telemetry)

### 4. Decide ownership of provider execution binding contracts

`ProviderExecutionBinding` currently lives under provider even though execution needs to own provider request shape.

Required work:

- move provider execution request contracts into `meld-execution`: completed in [crates/meld-execution/src/execution/contracts.rs](../../../crates/meld-execution/src/execution/contracts.rs)
- avoid forcing `meld-execution` to import provider registry and client management code: completed

Primary files:

- [src/provider/generation.rs](../../../src/provider/generation.rs)
- [src/provider/executor.rs](../../../src/provider/executor.rs)
- [src/context/generation/contracts.rs](../../../src/context/generation/contracts.rs)
- [src/task/package/contracts.rs](../../../src/task/package/contracts.rs)

## Ordered Migration Plan

### Step 1

Define execution ports and migrate capability plus task runtime off `ContextApi`.

Deliverables:

- no new capability or task runtime APIs take `ContextApi`
- root adapters implement execution ports

Status: completed for the extracted port authority boundary.

### Step 2

Split workflow runtime code from root adapter code.

Deliverables:

- workflow runtime depends on ports only
- CLI and config loading stay in root

### Step 3

Decouple execution from root telemetry compatibility types.

Deliverables:

- execution event contracts live with execution
- telemetry consumes them as an observer

### Step 4

Finalize provider execution request contract ownership.

Deliverables:

- execution owns the provider request shape it depends on
- provider registry and CLI remain in root

Status: completed.

## Exit Criteria

`CRATE.md` is ready to become declarative once all of the following are true:

- capability, task, and workflow runtime code depend on execution port contracts instead of direct `ContextApi` signatures
- execution imports world model through public query contracts only
- `meld-execution` does not import root CLI, workspace internals, provider internals, or telemetry internals
- root `meld` provides adapters for execution ports during runtime wiring

All criteria are satisfied for the contract extraction slice.

## Remaining Runtime Movement

Concrete task, capability, workflow, and provider execution implementations remain in root `meld`.
That is intentional until context, generation, and provider ownership are also crate-ready.
Moving those modules before their dependent contracts are extracted would turn `meld-execution` into a backdoor owner of root storage, config, and provider registry concerns.

## Non Goals For This Migration

- creating a crate per execution subdomain
- moving provider registry management into execution
- moving all context and prompt storage into execution
