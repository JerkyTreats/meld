# Core Crate Migration

Date: 2026-04-25
Status: working plan
Purpose: make [CRATE.md](CRATE.md) true through code changes in `src`

## Intent

Root `meld` remains the product crate and composition shell.
It should assemble extracted crates, own CLI and config, host compatibility shims, and implement adapter ports for domains that remain product coupled.

This document is a migration plan.
It is not evergreen architecture prose.

## Ground Truth In `src`

The root crate currently exports every top level domain from [src/lib.rs](../../../src/lib.rs).

The current product super facade is [src/api.rs](../../../src/api.rs).
`ContextApi` bundles node access, frame storage, prompt context storage, agent registry, provider registry, telemetry progress context, and graph runtime in [src/api.rs](../../../src/api.rs#L44).

Root owned adapter and product surfaces already exist in:

- [src/cli.rs](../../../src/cli.rs)
- [src/config.rs](../../../src/config.rs)
- [src/init.rs](../../../src/init.rs)
- [src/logging.rs](../../../src/logging.rs)
- [src/session.rs](../../../src/session.rs)
- [src/telemetry.rs](../../../src/telemetry.rs)
- [src/provider/tooling.rs](../../../src/provider/tooling.rs)
- [src/workspace/tooling.rs](../../../src/workspace/tooling.rs)
- [src/workflow/tooling.rs](../../../src/workflow/tooling.rs)

Root still directly hosts code that should become adapter implementations rather than cross domain authority:

- graph runtime storage on `ContextApi` in [src/api.rs](../../../src/api.rs#L64)
- provider registry exposure on `ContextApi` in [src/api.rs](../../../src/api.rs#L56)
- prompt context storage exposure on `ContextApi` in [src/api.rs](../../../src/api.rs#L52)
- progress emission context on `ContextApi` in [src/api.rs](../../../src/api.rs#L61)

## Target State

When `CRATE.md` becomes declarative truth, root `meld` should:

- wire `meld-events`, `meld-world-model`, and `meld-execution`
- own CLI parsing, presentation, config loading, init, logging, and operator facing status
- own compatibility re exports while migrations are active
- host context, provider, prompt context, workspace, telemetry, and session only where they remain product coupled
- implement execution owned ports without exposing root internals to extracted crates

Root `meld` should not define event truth, world model truth, or execution policy.

## Required Code Changes

### 1. Break up `ContextApi`

`ContextApi` is the main reason root `meld` still behaves like the whole product.
It must stop being the ambient dependency for execution and workflow code.

Required work:

- define narrow root owned adapter traits for context reads, context writes, prompt artifact reads, provider registry lookup, provider execution, and event publication
- add adapter structs in root `meld` that wrap current services
- stop passing `ContextApi` into execution owned code paths
- keep `ContextApi` as a compatibility facade only while migrations are active

Primary files:

- [src/api.rs](../../../src/api.rs)
- [src/context](../../../src/context)
- [src/prompt_context](../../../src/prompt_context)
- [src/provider](../../../src/provider)

### 2. Move runtime assembly into explicit wiring

Root should construct concrete runtimes for extracted crates rather than letting root facades smuggle them through shared state.

Required work:

- create one runtime assembly layer that builds event runtime, world model runtime, execution runtime, and root adapters
- remove direct graph runtime storage from `ContextApi`
- inject world model and execution ports through constructor wiring rather than mutable setters

Primary files:

- [src/api.rs](../../../src/api.rs)
- [src/init.rs](../../../src/init.rs)
- [src/bin/meld.rs](../../../src/bin/meld.rs)

### 3. Keep adapter code in root and move authority out

The command and tooling surfaces should remain in root only if they are adapters.
Core domain logic should not stay behind them.

Required work:

- keep CLI and presentation code in root
- move domain logic out of adapter files that currently reach into execution or event internals
- make root command handlers call crate public APIs rather than internal module functions

Primary files:

- [src/provider/tooling.rs](../../../src/provider/tooling.rs)
- [src/workflow/tooling.rs](../../../src/workflow/tooling.rs)
- [src/workflow/commands.rs](../../../src/workflow/commands.rs)
- [src/workspace/tooling.rs](../../../src/workspace/tooling.rs)

### 4. Shrink the root public surface

`src/lib.rs` currently exports every domain module.
That is fine for a monolith, but not for a crate split with explicit authority.

Required work:

- stop re exporting extracted crate internals from root by default
- add temporary compatibility re exports with clear migration names
- route new code to extracted crate APIs rather than old root module paths

Primary files:

- [src/lib.rs](../../../src/lib.rs)

## Ordered Migration Plan

### Step 1

Define root adapter contracts that `meld-execution` will consume.

Deliverables:

- root adapter traits and concrete implementations
- no new execution code may take `ContextApi`

### Step 2

Introduce one runtime assembly layer that constructs extracted crate runtimes and adapters.

Deliverables:

- constructor wiring replaces mutable runtime storage on `ContextApi`
- root owns product composition explicitly

### Step 3

Move command and tooling logic to public crate APIs.

Deliverables:

- CLI handlers call stable crate interfaces
- root adapter code no longer reaches into crate internals

### Step 4

Reduce root exports and add compatibility shims.

Deliverables:

- `src/lib.rs` reflects crate authority rather than folder layout
- compatibility paths are temporary and named as such

## Exit Criteria

`CRATE.md` is ready to become declarative once all of the following are true:

- root `meld` can assemble the product without extracted crates depending on root internals
- `ContextApi` is no longer required by execution owned code paths
- root command handlers call public crate APIs only
- root re exports are compatibility shims rather than the primary architecture

## Non Goals For This Migration

- moving context into its own crate now
- moving provider into its own crate now
- splitting binaries now
- changing user facing CLI behavior unless a port boundary requires it
