# Capability Refactor Implementation Plan

Date: 2026-04-03
Status: proposed
Scope: phased refactor of traversal, control, provider execution, and workflow compatibility before task features land

## Overview

This plan defines the implementation order for the current capability-readiness refactor.

The goal is not to add `task` execution yet.
The goal is to make the current system honest about ownership so later capability and task work lands on clean seams.

The dependency order is:

1. freeze current behavior with characterization and parity gates
2. split Merkle traversal out of `context`
3. bootstrap `src/control` and move ordered orchestration into it
4. extract provider execution into `src/provider`
5. rebuild workflow paths as compatibility consumers of the new domains

## Source Docs

- [Capability And Task Design](README.md)
- [Domain Architecture](domain_architecture.md)
- [Context Technical Spec](context/technical_spec.md)
- [Provider Capability Design](provider/README.md)
- [Workflow Refactor](workflow_refactor/README.md)
- [Workflow Cleanup Technical Spec](workflow_refactor/technical_spec.md)
- [Merkle Traversal Capability](capability/merkle_traversal/README.md)
- [Merkle Traversal Technical Spec](capability/merkle_traversal/technical_spec.md)
- [Interregnum Orchestration](../control/interregnum_orchestration.md)

## Execution Rules

- preserve end-to-end workflow-triggered execution during the refactor window
- do not remove old ownership until the new owner is wired and covered by tests
- prefer compatibility adapters over mixed ownership
- move one concern at a time and seal the boundary before the next phase
- expand characterization tests before deleting any legacy orchestration path

## Development Phases

| Phase | Goal | Dependencies | Status |
|------|------|--------------|--------|
| 0 | Characterization and baseline gates | None | proposed |
| 1 | Merkle traversal extraction | Phase 0 | proposed |
| 2 | Control bootstrap and orchestration cutover | Phase 0 and Phase 1 | proposed |
| 3 | Provider executor extraction | Phase 0 and Phase 2 | proposed |
| 4 | Workflow compatibility rebuild | Phase 0 through Phase 3 | proposed |
| 5 | Legacy seal and refactor closeout | Phase 0 through Phase 4 | proposed |

## Phase 0

### Goal

Freeze the current recursive generate and workflow-triggered behavior before ownership moves.

### Tasks

- document the current recursive bottom-up behavior that must remain true during the interregnum
- document the current docs writer execution path that must continue to run
- add characterization coverage for bottom-up release and parent-after-child behavior
- add characterization coverage for workflow-triggered docs writer execution through the CLI path
- add characterization coverage for provider request telemetry and response correlation as observed today

### Expanded Test Work

- extend [progress_observability.rs](/home/jerkytreats/meld/tests/integration/progress_observability.rs) with assertions around level release order and barrier completion
- extend [context_cli.rs](/home/jerkytreats/meld/tests/integration/context_cli.rs) with recursive directory cases that prove parent execution waits for lower levels
- extend [workflow_cli.rs](/home/jerkytreats/meld/tests/integration/workflow_cli.rs) with end-to-end docs writer compatibility coverage
- extend [frame_queue.rs](/home/jerkytreats/meld/tests/integration/frame_queue.rs) with ordering and dedupe assertions that capture current queue behavior

### Exit Criteria

- current recursive generation order is characterized in tests
- current workflow-triggered docs writer behavior is characterized in tests
- current provider request lifecycle signals are characterized well enough to detect regressions

### Verification Gates

- compile gate: `cargo check`
- integration gate: `cargo test --test integration_tests integration::context_cli::`
- integration gate: `cargo test --test integration_tests integration::workflow_cli::`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`
- integration gate: `cargo test --test integration_tests integration::frame_queue::`

## Phase 1

### Goal

Split Merkle traversal out of `context` into its own domain seam with structural batch output.

### Tasks

- extract traversal derivation from `src/context/generation/run.rs`
- introduce traversal contracts that emit `ordered_merkle_node_batches`
- preserve current `bottom_up` behavior as the baseline strategy
- add `top_down` as the second supported strategy if it is cheap to carry during extraction
- remove direct traversal derivation from `context` public execution shape

### Key Files And Seams

- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)
- [plan.rs](/home/jerkytreats/meld/src/context/generation/plan.rs)
- the new traversal domain path under `src`
- [README.md](/home/jerkytreats/meld/design/capabilities/capability/merkle_traversal/README.md)
- [technical_spec.md](/home/jerkytreats/meld/design/capabilities/capability/merkle_traversal/technical_spec.md)

### Expanded Test Work

- add a new integration surface for traversal output shape, likely `tests/integration/merkle_traversal.rs`
- characterize bottom-up batch output against current recursive behavior
- add deterministic ordering checks for repeated traversal over the same tree
- add strategy coverage for `top_down` if the variant lands in this phase

### Exit Criteria

- traversal lives outside `context`
- traversal output is batch-shaped and structural
- `context` no longer derives traversal internally for the future path
- bottom-up behavior still matches current recursive ordering

### Verification Gates

- compile gate: `cargo check`
- unit gate: traversal domain unit tests
- integration gate: `cargo test --test integration_tests integration::tree_determinism::`
- integration gate: new traversal integration suite
- regression gate: `cargo test --test integration_tests integration::context_cli::`

## Phase 2

### Goal

Bootstrap `src/control` and move ordered orchestration out of `context`.

### Tasks

- add `src/control` as the refactor-phase orchestration home
- move bottom-up release logic and wave progression out of `context`
- move batch barrier coordination out of `context`
- make `control` consume traversal batches and coordinate execution waves
- preserve `context` as preparation and finalization around atomic generation
- keep CLI-triggered behavior stable through compatibility adapters

### Key Files And Seams

- the new `src/control.rs`
- the new `src/control/orchestration.rs`
- the new `src/control/traversal_release.rs`
- the new `src/control/batch_barrier.rs`
- [run.rs](/home/jerkytreats/meld/src/context/generation/run.rs)
- [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs)
- [technical_spec.md](/home/jerkytreats/meld/design/capabilities/context/technical_spec.md)
- [interregnum_orchestration.md](/home/jerkytreats/meld/design/control/interregnum_orchestration.md)

### Expanded Test Work

- add a new integration suite for control-owned orchestration, likely `tests/integration/control_orchestration.rs`
- add assertions that one batch fully completes before the next batch is released
- add parity coverage proving the old recursive CLI flow still emits the same user-visible outcomes through the new control path
- add failure-path coverage proving the barrier does not release parent work after lower-level failure

### Exit Criteria

- ordered orchestration no longer lives in `context`
- `control` is the owner of batch release and batch barriers
- recursive generation still works end to end
- `context` remains the atomic generation seam

### Verification Gates

- compile gate: `cargo check`
- unit gate: control release and barrier unit tests
- integration gate: new control orchestration suite
- integration gate: `cargo test --test integration_tests integration::context_cli::`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`

## Phase 3

### Goal

Extract provider execution into the provider domain and make `control` hand off ready work to a provider executor.

### Tasks

- move provider binding resolution out of context-owned execution paths
- add a provider-native execution contract
- add `ProviderExecutor` under `src/provider`
- move batching, throttling, retry, and backoff into the provider domain
- make `context` hand off provider-ready requests rather than calling provider transport directly
- keep result correlation stable by request identity

### Key Files And Seams

- [provider.rs](/home/jerkytreats/meld/src/provider.rs)
- [generation.rs](/home/jerkytreats/meld/src/provider/generation.rs)
- the new `src/provider/executor.rs`
- [provider_execution.rs](/home/jerkytreats/meld/src/context/generation/provider_execution.rs)
- [orchestration.rs](/home/jerkytreats/meld/src/context/generation/orchestration.rs)
- [README.md](/home/jerkytreats/meld/design/capabilities/provider/README.md)

### Expanded Test Work

- add a new provider executor suite, likely `tests/integration/provider_execution.rs`
- characterize batching behavior for compatible requests
- characterize throttling and retry behavior by provider lane rather than agent identity
- add correlation tests that prove results map back to stable request ids after batch execution
- preserve current provider CLI and model-provider coverage

### Exit Criteria

- provider transport no longer lives under `context`
- batching and throttling live under `provider`
- `context` exposes an explicit provider handoff
- `control` can submit ready work to `provider` without reaching into provider internals

### Verification Gates

- compile gate: `cargo check`
- unit gate: provider executor unit tests
- integration gate: new provider execution suite
- integration gate: `cargo test --test integration_tests integration::model_providers::`
- integration gate: `cargo test --test integration_tests integration::provider_cli::`
- regression gate: `cargo test --test integration_tests integration::generation_parity::`

## Phase 4

### Goal

Rebuild workflow paths so workflow becomes a compatibility consumer of traversal, control, context, and provider instead of a hidden orchestration owner.

### Tasks

- keep the outer workflow trigger path callable
- move ordered workflow execution out of workflow internals into `control`
- reduce workflow to compatibility request mapping and user-facing trigger behavior
- remove workflow-owned provider execution calls
- remove workflow-owned batch release logic
- preserve docs writer execution during the interregnum

### Key Files And Seams

- [facade.rs](/home/jerkytreats/meld/src/workflow/facade.rs)
- [executor.rs](/home/jerkytreats/meld/src/workflow/executor.rs)
- [resolver.rs](/home/jerkytreats/meld/src/workflow/resolver.rs)
- [commands.rs](/home/jerkytreats/meld/src/workflow/commands.rs)
- [route.rs](/home/jerkytreats/meld/src/cli/route.rs)
- [runtime.rs](/home/jerkytreats/meld/src/workspace/watch/runtime.rs)
- [README.md](/home/jerkytreats/meld/design/capabilities/workflow_refactor/README.md)
- [technical_spec.md](/home/jerkytreats/meld/design/capabilities/workflow_refactor/technical_spec.md)

### Expanded Test Work

- extend [workflow_cli.rs](/home/jerkytreats/meld/tests/integration/workflow_cli.rs) with compatibility-trigger coverage that proves delegation into `control`
- add assertions that docs writer still completes through the workflow entry path
- add watch-mode compatibility coverage if workflow-shaped watch execution remains during the interregnum
- add telemetry coverage proving workflow compatibility events and control events remain reconstructable in one session

### Exit Criteria

- workflow no longer owns execution order
- workflow still works end to end as a compatibility trigger
- docs writer remains runnable during the interregnum
- workflow internals no longer reach into context-owned provider transport

### Verification Gates

- compile gate: `cargo check`
- integration gate: `cargo test --test integration_tests integration::workflow_cli::`
- integration gate: `cargo test --test integration_tests integration::workflow_contracts_conformance::`
- integration gate: `cargo test --test integration_tests integration::progress_observability::`
- integration gate: `cargo test --test integration_tests integration::workspace_commands::`

## Phase 5

### Goal

Seal the refactor boundaries and remove the last mixed-ownership paths that would block later task work.

### Tasks

- remove dead traversal logic from `context`
- remove dead provider execution glue from `context`
- remove dead orchestration loops from workflow internals
- reduce compatibility wrappers to clearly named edge adapters
- publish the post-refactor `src` map for the pre-feature state
- confirm the system is ready for `src/task` and `src/capability` introduction

### Expanded Test Work

- add a full regression matrix for touched domains
- add targeted negative tests for batch-failure stop behavior
- add characterization coverage for docs writer result parity before and after the refactor
- add command-surface coverage that proves user-visible workflow and context behavior did not regress during ownership moves

### Exit Criteria

- each concern has one clear owner
- no active end-to-end path depends on mixed context and workflow orchestration
- no active end-to-end path depends on context-owned provider transport
- the codebase is ready for capability and task feature work without another large ownership refactor

### Verification Gates

- compile gate: `cargo check`
- full integration gate: `cargo test --test integration_tests`
- full suite gate: `cargo test`

## Cross-Phase Gates

### Behavior Preservation Gates

- recursive directory generation remains bottom-up
- parent work does not release before lower-level completion
- workflow-triggered docs writer remains runnable during the interregnum
- provider request correlation remains deterministic

### Boundary Gates

- traversal is not owned by `context`
- orchestration is not owned by `context`
- provider transport is not owned by `context`
- workflow does not own durable orchestration

### Observability Gates

- progress events still reconstruct batch release order
- provider lifecycle signals still correlate to request identity
- workflow compatibility events can still be joined with control execution events in one session

## Implementation Order Summary

1. characterize current behavior
2. extract traversal
3. bootstrap `control`
4. extract provider executor
5. rebuild workflow compatibility on top of the new seams
6. remove dead mixed-ownership code

## Read With

- [Capability And Task Design](README.md)
- [Domain Architecture](domain_architecture.md)
- [Context Technical Spec](context/technical_spec.md)
- [Provider Capability Design](provider/README.md)
- [Workflow Cleanup Technical Spec](workflow_refactor/technical_spec.md)
- [Interregnum Orchestration](../control/interregnum_orchestration.md)
