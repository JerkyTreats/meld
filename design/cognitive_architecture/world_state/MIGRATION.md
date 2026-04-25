# World Model Crate Migration

Date: 2026-04-25
Status: working plan
Purpose: make [CRATE.md](CRATE.md) true through code changes in `src`

## Intent

`meld-world-model` should own graph materialization, provenance, current anchor selection, legacy claim compatibility during migration, and later belief.

This document records the code changes needed for that boundary to be real.

## Ground Truth In `src`

Current world model code lives in:

- [src/world_state.rs](../../../src/world_state.rs)
- [src/world_state/contracts.rs](../../../src/world_state/contracts.rs)
- [src/world_state/query.rs](../../../src/world_state/query.rs)
- [src/world_state/store.rs](../../../src/world_state/store.rs)
- [src/world_state/reducer.rs](../../../src/world_state/reducer.rs)
- [src/world_state/graph.rs](../../../src/world_state/graph.rs)
- [src/world_state/graph](../../../src/world_state/graph)

The main blockers are explicit:

- graph reduction imports source domain reducers from workspace, context, and task in [src/world_state/graph/reducer.rs](../../../src/world_state/graph/reducer.rs#L195)
- public world state exports include `GraphRuntime`, `TraversalStore`, and `WorldStateStore` in [src/world_state.rs](../../../src/world_state.rs#L15)
- `GraphRuntime` constructs `EventStore` directly from `sled::Db` in [src/world_state/graph/runtime.rs](../../../src/world_state/graph/runtime.rs#L17)
- `GraphRuntime` also appends envelopes itself in [src/world_state/graph/runtime.rs](../../../src/world_state/graph/runtime.rs#L48)
- root `ContextApi` stores `GraphRuntime` directly in [src/api.rs](../../../src/api.rs#L64) and [src/api.rs](../../../src/api.rs#L139)
- legacy claim query APIs still expose raw store shaped access in [src/world_state/query.rs](../../../src/world_state/query.rs)

## Target State

When `CRATE.md` becomes declarative truth, `meld-world-model` should:

- consume canonical events through event public APIs
- materialize graph and legacy claim projections without importing source domain internals
- expose query APIs and stable world model contracts
- hide storage and runtime assembly details behind the crate boundary
- avoid direct dependency on root `meld`, provider, task internals, context internals, and workspace internals

## Required Code Changes

### 1. Invert graph reducer source hooks

This is the core extraction blocker.
The reducer currently asks source domains how to interpret events.
That makes the world model depend on source domain internals.

Required work:

- replace direct calls to source reducer helpers with one public source intent contract
- choose one of two shapes and commit to it
- shape one is source domains publish traversal intents as event data
- shape two is root wiring registers source intent providers through a narrow interface
- keep source domain specific interpretation out of `meld-world-model`

Primary files:

- [src/world_state/graph/reducer.rs](../../../src/world_state/graph/reducer.rs)
- [src/workspace/reducer.rs](../../../src/workspace/reducer.rs)
- [src/context/reducer.rs](../../../src/context/reducer.rs)
- [src/task/reducer.rs](../../../src/task/reducer.rs)

### 2. Hide storage and runtime internals

The extracted crate should not freeze `sled` backed storage types and in process runtime construction into its public API.

Required work:

- stop publicly re exporting `TraversalStore` and `WorldStateStore`
- stop publicly re exporting `GraphRuntime` as the main world model surface
- expose query services and contracts instead
- keep runtime assembly and storage constructors either crate private or behind root wiring

Primary files:

- [src/world_state.rs](../../../src/world_state.rs)
- [src/world_state/graph/runtime.rs](../../../src/world_state/graph/runtime.rs)
- [src/world_state/graph/store.rs](../../../src/world_state/graph/store.rs)
- [src/world_state/store.rs](../../../src/world_state/store.rs)

### 3. Separate query API from materialization API

Current query code is store centric.
The extracted crate needs public reads that do not expose internal storage layout.

Required work:

- define query services for anchor reads, traversal reads, provenance reads, and legacy claim compatibility reads
- keep store types out of public signatures
- give execution a planner facing read model rather than raw stores

Primary files:

- [src/world_state/query.rs](../../../src/world_state/query.rs)
- [src/world_state/graph/query.rs](../../../src/world_state/graph/query.rs)
- [src/world_state/graph/compat.rs](../../../src/world_state/graph/compat.rs)

### 4. Remove root owned runtime embedding

The world model should be wired into root, not stored inside root facades as ambient mutable state.

Required work:

- remove direct `GraphRuntime` storage from `ContextApi`
- make root `meld` obtain world model query interfaces through explicit wiring
- ensure event append and world model catch up are orchestrated by root runtime assembly rather than hidden mutable state

Primary files:

- [src/api.rs](../../../src/api.rs)
- [src/world_state/graph/runtime.rs](../../../src/world_state/graph/runtime.rs)

## Ordered Migration Plan

### Step 1

Define the source intent boundary and remove source domain imports from graph reduction.

Deliverables:

- world model reducer no longer imports workspace, context, or task internals
- source event interpretation crosses a public contract

### Step 2

Hide storage and runtime internals from the public world model surface.

Deliverables:

- public API returns query services and contracts
- store and runtime types are no longer the main entrypoint

### Step 3

Move root integrations to explicit wiring.

Deliverables:

- root no longer stores `GraphRuntime` inside `ContextApi`
- root uses explicit world model service construction

### Step 4

Stabilize the planner facing read model.

Deliverables:

- execution consumes world model public queries only
- legacy claim compatibility remains available during migration

## Exit Criteria

`CRATE.md` is ready to become declarative once all of the following are true:

- graph reduction does not import source domain internals
- world model public APIs do not expose `TraversalStore`, `WorldStateStore`, or `GraphRuntime` as primary surfaces
- root `meld` uses explicit world model wiring rather than ambient mutable runtime state
- execution reads world model state through public query contracts only

## Non Goals For This Migration

- fully implementing belief now
- removing legacy claim compatibility before parity is proven
- changing event ownership out of `meld-events`
