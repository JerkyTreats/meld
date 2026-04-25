# Core Crate

Date: 2026-04-23
Status: active experiment
Scope: root `meld` crate as the product orchestrator and compatibility shell

## Intent

The root `meld` crate remains the product crate.
It wires domain crates together and owns user-facing application concerns.

The root crate is not the owner of event truth, world-model truth, or execution policy once those crates are extracted.

## Target Crate

`meld`

## Owns

- CLI command routing and presentation
- config loading and path resolution
- storage root discovery and database opening
- runtime assembly and dependency injection
- compatibility shims for migrated module paths
- app bootstrap, logging, init, and operator-facing status
- workspace and context compatibility paths until they have clearer crate homes
- context storage, context query, prompt context, and generation compatibility APIs for now
- provider configuration, registry, diagnostics, and concrete model client management for now
- public adapter implementations for execution ports

## Does Not Own

- canonical event append and replay
- graph materialization
- belief revision
- planner policy
- task execution internals
- provider execution policy inside `meld-execution`

## Current Code Areas

- `src/bin/meld.rs`
- `src/cli`
- `src/config`
- `src/api.rs`
- `src/init`
- `src/logging.rs`
- `src/session`
- `src/telemetry`
- `src/workspace`
- `src/context`
- `src/agent`
- `src/provider` configuration and command surfaces

## Context And Provider Posture

Context and provider remain in root `meld` for now.

They are too coupled to the current product shell to extract cleanly, but they are also too broad to fold into `meld-execution`.

Root `meld` should expose explicit public APIs and adapters for:

- context reads
- context writes
- generated artifact and frame publication
- provider registry lookup
- provider model invocation
- provider diagnostics used by CLI

`meld-execution` consumes those capabilities through ports.
It does not import root `meld` internals.

## Deferred Crate

Do not create `meld-provider` yet.

A later crate may be justified if generation stabilizes as its own meta-domain.
That future crate would be closer to `meld-generation` than `meld-provider`.
It would own prompt assembly, provider call orchestration, generated frame metadata, and generation-specific adapters.

It would not own all context storage or all provider configuration.

## Target Dependencies

| From | To | Reason |
| --- | --- | --- |
| `meld` | `meld-events` | append, replay, and event runtime wiring |
| `meld` | `meld-world-model` | graph and belief query wiring |
| `meld` | `meld-execution` | planner and task runtime wiring |

## Forbidden Direction

No extracted crate may depend on `meld`.

If a subcrate needs a capability from root `meld`, that capability must be moved into the owning crate or passed through a narrow contract.

`meld-execution` may depend on context and provider capabilities only through execution-owned ports implemented by root `meld`.

## Migration Notes

The root crate may temporarily re-export migrated crates to preserve old imports.

Compatibility re-exports must be marked as temporary and should not become the permanent architecture.
