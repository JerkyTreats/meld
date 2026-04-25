# Execution Crate Migration

Date: 2026-04-25
Status: working plan
Purpose: make [CRATE.md](CRATE.md) true through code changes in `src`

## Intent

`meld-execution` should own deliberate action.
That includes control, task, capability, workflow runtime, planning policy, and provider execution ports.

This document names the code changes needed for that split.

## Ground Truth In `src`

Execution owned logic is spread across:

- [src/control.rs](../../../src/control.rs)
- [src/task.rs](../../../src/task.rs)
- [src/capability.rs](../../../src/capability.rs)
- [src/workflow.rs](../../../src/workflow.rs)
- [src/merkle_traversal.rs](../../../src/merkle_traversal.rs)

The main blockers are explicit in the current imports:

- capability invocation depends on `ContextApi` in [src/capability/invocation.rs](../../../src/capability/invocation.rs#L3) and [src/capability/invocation.rs](../../../src/capability/invocation.rs#L59)
- task runtime depends on `ContextApi` and telemetry workflow event types in [src/task/runtime.rs](../../../src/task/runtime.rs#L3) and [src/task/runtime.rs](../../../src/task/runtime.rs#L13)
- workflow facade still loads config and registry in [src/workflow/facade.rs](../../../src/workflow/facade.rs#L4) and [src/workflow/facade.rs](../../../src/workflow/facade.rs#L29)
- workflow commands resolve workspace node ids and validate provider existence in [src/workflow/commands.rs](../../../src/workflow/commands.rs#L113) through [src/workflow/commands.rs](../../../src/workflow/commands.rs#L176)
- workflow executor reaches into metadata, prompt context, provider helpers, and world model query in [src/workflow/executor.rs](../../../src/workflow/executor.rs#L10) through [src/workflow/executor.rs](../../../src/workflow/executor.rs#L51)
- task package preparation still resolves workspace nodes through root workspace code in [src/task/package/prepare.rs](../../../src/task/package/prepare.rs#L24) and [src/task/package/prepare.rs](../../../src/task/package/prepare.rs#L230)
- `ProviderExecutionBinding` lives under provider in [src/provider/generation.rs](../../../src/provider/generation.rs#L68)

## Target State

When `CRATE.md` becomes declarative truth, `meld-execution` should:

- define execution contracts and policy
- consume world model reads through public query contracts
- request provider work through an execution owned port
- request context and prompt artifact access through execution owned ports
- publish outcomes through an event publication port
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

- define execution owned traits for the required ports
- add adapter implementations in root `meld`
- change capability, task, and workflow entrypoints to depend on those traits
- keep `ContextApi` only as a compatibility wrapper while call sites move

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

- either move provider execution request contracts into `meld-execution`
- or define a small shared contract surface that provider and execution both depend on
- avoid forcing `meld-execution` to import provider registry and client management code

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

## Exit Criteria

`CRATE.md` is ready to become declarative once all of the following are true:

- capability, task, and workflow runtime code no longer depend on `ContextApi`
- execution imports world model through public query contracts only
- execution does not import root CLI, workspace internals, or telemetry internals
- root `meld` provides adapters for execution ports during runtime wiring

## Non Goals For This Migration

- creating a crate per execution subdomain
- moving provider registry management into execution
- moving all context and prompt storage into execution
