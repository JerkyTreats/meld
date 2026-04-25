# Cognitive Architecture Crate Split Implementation Plan

Date: 2026-04-25
Status: active
Scope: phased implementation plan for the authority crate split under `design/cognitive_architecture`

## Overview

This plan converts the per crate migration guides into one dependency ordered execution plan.

The order is authority driven, not folder driven.

Primary objective:
- define and cut stable public APIs for a small set of authority crates
- preserve one coherent product shell in root `meld`
- complete contract extraction before physical crate extraction
- update each `CRATE.md` only after its migration work is complete

Target crate outcome:
- `meld-events` owns canonical event ledger truth
- `meld-world-model` owns graph materialization and planner facing world model reads
- `meld-execution` owns control, task, capability, workflow runtime, and execution policy
- root `meld` remains the product shell, composition root, compatibility layer, and adapter host

Implementation posture:
- no crate per top level domain
- no direct crate extraction from current concrete runtime types
- compatibility shims remain temporary and must not become the architecture

## Related Docs

- [Architecture Overview](README.md)
- [Crate Split Dependency Checklist](dependency_checklist.md)
- [Phase 0 Baseline](phase0_baseline.md)
- [Public Surface Inventory](public_surface_inventory.md)
- [Core Crate](core/CRATE.md)
- [Core Migration](core/MIGRATION.md)
- [Initial Port Inventory](core/port_inventory.md)
- [Events Crate](events/CRATE.md)
- [Events Migration](events/MIGRATION.md)
- [World Model Crate](world_state/CRATE.md)
- [World Model Migration](world_state/MIGRATION.md)
- [Execution Crate](execution/CRATE.md)
- [Execution Migration](execution/MIGRATION.md)
- [Crate Boundary Assessment By Domain](microarchitecture_assessment_by_domain.md)
- [Complex Change Workflow Governance](../../governance/complex_change_workflow.md)

## Implementation Progress

Current working branch:
- `feat/cognitive-architecture-crate-split`

Committed progress:

| Commit | Scope | Result |
|--------|-------|--------|
| `25b13cc` | Phase 0 baseline | plan, migration guides, dependency checklist, public surface inventory, and gate baseline published |
| `037eab6` | Phase 1 events boundary | session lifecycle removed from `EventStore` and runtime idempotent append fixed |
| `7eb71d4` | Phase 2 world model boundary | source intent moved inside world model and root reads routed through `WorldModelQueries` |
| `f78708c` | Phase 3 execution boundary | execution ports added, provider request contracts moved under execution, and capability plus task runtime signatures removed direct `ContextApi` dependence |
| `this commit` | Phase 4 workflow and assembly boundary | workflow runtime moved onto execution ports and root assembly now owns CLI runtime wiring plus profile adapters |

Next active phase:
- Phase 5 root composition seal and public surface cleanup

## CLI Path Default Exception List

Project direction is path first targeting.
Current command surfaces that still include non default path behavior:

- `merkle context generate` accepts `--node` as an alternate selector
- `merkle context regenerate` accepts `--node` as an alternate selector
- `merkle context get` accepts `--node` as an alternate selector
- `merkle workspace delete` accepts `--node` as an alternate selector
- `merkle workspace restore` accepts `--node` as an alternate selector

This crate split plan does not expand non default path behavior.

## Verification Strategy

Run formatter gates before compile and test gates for every phase that touches Rust source.

| Gate | Purpose | Minimum evidence |
|------|---------|------------------|
| F0 | Formatter gate | `cargo fmt --check` |
| F1 | Compile gate | `cargo check` |
| F2 | Focused unit and integration gates | targeted `cargo test` commands for the active phase |
| F3 | Dependency boundary gate | `rg` based import audit for forbidden dependency directions |
| F4 | Public surface gate | verification that extracted public APIs do not expose temporary store or runtime internals |
| F5 | Full regression gate | `cargo test` before phase close when runtime code changed broadly |

## Boundary Rules

Apply these rules in every phase.

| Rule | Statement |
|------|-----------|
| Contract first | define public contracts and adapters before moving modules into crates |
| Authority over layout | split by owner of truth and policy, not by folder count |
| Root shell remains | root `meld` keeps product composition, CLI, config, compatibility, and adapters |
| No ambient facades | replace shared super facades such as `ContextApi` with narrow ports |
| No internal source hooks | world model and execution may not import source domain internals after boundary cleanup |
| Compatibility must shrink | temporary re exports and wrappers must be reduced phase by phase |

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 0 | Baseline lock and boundary inventory | None | completed |
| 1 | Events authority boundary cleanup | Phase 0 | completed |
| 2 | World model ingestion and query boundary cleanup | Phase 0 and Phase 1 | completed |
| 3 | Execution port foundation and contract extraction | Phase 0 and Phase 1 and Phase 2 | completed |
| 4 | Workflow runtime split and root adapter cutover | Phase 2 and Phase 3 | completed |
| 5 | Root composition seal and public surface cleanup | Phase 1 and Phase 2 and Phase 3 and Phase 4 | proposed |
| 6 | Physical crate extraction and declarative doc cutover | Phase 1 through Phase 5 | proposed |

---

### Phase 0 — Baseline lock and boundary inventory

| Field | Value |
|-------|--------|
| Goal | Freeze current behavior and publish the dependency gates needed for the split |
| Dependencies | None |
| Docs | all `MIGRATION.md` docs in this folder and this plan |
| Status | completed |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Add one crate split dependency checklist that captures the forbidden directions for events, world model, execution, and root `meld`. | Completed |
| 2 | Add characterization coverage for event append and replay, world model catch up, workflow execution, task execution, and provider backed generation. | Completed |
| 3 | Record the concrete public surfaces that must disappear from long term APIs, especially `ContextApi`, `EventStore`, `GraphRuntime`, `TraversalStore`, and `WorldStateStore`. | Completed |
| 4 | Freeze the initial port inventory for execution and root adapters. | Completed |

| Exit criterion | Completion |
|----------------|------------|
| Behavior and dependency baselines are explicit enough to support safe boundary work. | Completed |
| Every later phase has a dependency gate and focused verification target. | Completed |

| Key seams |
|-----------|
| [src/api.rs](../../src/api.rs) |
| [src/events/store.rs](../../src/events/store.rs) |
| [src/world_state.rs](../../src/world_state.rs) |
| [src/workflow/executor.rs](../../src/workflow/executor.rs) |
| [src/task/runtime.rs](../../src/task/runtime.rs) |

| Verification gate | Evidence target |
|-------------------|-----------------|
| F0 | `cargo fmt --check` |
| F1 | `cargo check` |
| F2 | targeted characterization tests added in this phase |
| F3 | boundary checklist published and used by later phases |

| Dependency closure solved |
|---------------------------|
| Creates the baseline needed to change boundaries without silent behavioral drift |

| Phase 0 artifacts |
|-------------------|
| [Crate Split Dependency Checklist](dependency_checklist.md) |
| [Phase 0 Baseline](phase0_baseline.md) |
| [Public Surface Inventory](public_surface_inventory.md) |
| [Initial Port Inventory](core/port_inventory.md) |

| Phase 0 gate evidence | Result |
|-----------------------|--------|
| F0 | passed on 2026-04-25 with `cargo fmt --check` |
| F1 | passed on 2026-04-25 with `cargo check` |
| F2 | passed on 2026-04-25 with focused `event_spine`, `traversal_graph`, `workflow_task_compatibility`, `task_executor`, and `generation_parity` integration coverage |
| F3 | checklist published on 2026-04-25 in [Crate Split Dependency Checklist](dependency_checklist.md) and baseline audits captured remaining Phase 1 through Phase 3 violations |

| Phase 0 completion notes |
|--------------------------|
| Baseline audit confirms event authority still imports `session`, world model reducer still imports `workspace`, `context`, and `task` reducers, and execution core still depends on `ContextApi` plus telemetry internals. Later phases must remove those exact hits without weakening characterization coverage. |

---

### Phase 1 — Events authority boundary cleanup

| Field | Value |
|-------|--------|
| Goal | Make the event ledger independent from session lifecycle and telemetry compatibility |
| Dependencies | Phase 0 |
| Docs | [Events Migration](events/MIGRATION.md) |
| Status | completed |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Remove `SessionStore` ownership and session lifecycle methods from `EventStore`. | Completed |
| 2 | Move session retention and interruption policy fully into `src/session` and root telemetry adapters. | Completed |
| 3 | Fix runtime idempotent append so the public runtime honors `record_id` behavior. | Completed |
| 4 | Move telemetry event aliases and sink compatibility out of the event authority boundary. | Completed |
| 5 | Normalize public event ledger naming and keep old spine names as temporary compatibility only. | Completed |

| Exit criterion | Completion |
|----------------|------------|
| `meld-events` public APIs have no session lifecycle dependency. | Completed |
| world model and execution can consume event contracts without importing telemetry or session code. | Completed |

| Key seams |
|-----------|
| [src/events.rs](../../src/events.rs) |
| [src/events/store.rs](../../src/events/store.rs) |
| [src/events/runtime.rs](../../src/events/runtime.rs) |
| [src/session/runtime.rs](../../src/session/runtime.rs) |
| [src/telemetry/sessions/service.rs](../../src/telemetry/sessions/service.rs) |
| [src/telemetry/contracts.rs](../../src/telemetry/contracts.rs) |

| Verification gate | Evidence target |
|-------------------|-----------------|
| F0 | `cargo fmt --check` |
| F1 | `cargo check` |
| F2 | focused event store and runtime tests for append, replay, and idempotent append |
| F3 | import audit confirms no session dependency remains under extracted event authority code |
| F4 | event public surface exposes contracts and runtime only, not telemetry aliases |

| Dependency closure solved |
|---------------------------|
| Unblocks world model and execution extraction on top of one stable event authority crate |

| Phase 1 gate evidence | Result |
|-----------------------|--------|
| F0 | passed on 2026-04-25 with `cargo fmt --check` |
| F1 | passed on 2026-04-25 with `cargo check` |
| F2 | passed on 2026-04-25 with focused `event_spine` plus telemetry session retention coverage in `progress_observability` |
| F3 | passed on 2026-04-25 with `rg -n 'crate::session|crate::telemetry' src/events` returning no matches |
| F4 | passed on 2026-04-25 after removing `src/events/query.rs` and keeping telemetry compatibility outside `src/events` |

| Phase 1 completion notes |
|--------------------------|
| `EventStore` now owns only ledger state. Non idempotent append and explicit idempotent append have distinct runtime semantics. Session reads used by telemetry verification now flow through `ProgressRuntime` rather than through the event store. |

---

### Phase 2 — World model ingestion and query boundary cleanup

| Field | Value |
|-------|--------|
| Goal | Remove source domain imports from world model reduction and replace store shaped APIs with public query surfaces |
| Dependencies | Phase 0 and Phase 1 |
| Docs | [World Model Migration](world_state/MIGRATION.md) |
| Status | completed |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Define the source intent boundary for graph materialization and remove direct imports of workspace, context, and task reducers from world model code. | Completed |
| 2 | Hide `TraversalStore`, `WorldStateStore`, and `GraphRuntime` from the long term public world model surface. | Completed |
| 3 | Publish query services for anchor reads, traversal reads, provenance reads, and legacy claim compatibility reads. | Completed |
| 4 | Remove direct `GraphRuntime` storage from root facades and move world model runtime access to explicit wiring. | Completed |

| Exit criterion | Completion |
|----------------|------------|
| world model no longer imports source domain internals to interpret events. | Completed |
| execution and root code consume world model reads through public query contracts rather than raw stores and runtime types. | Completed |

| Key seams |
|-----------|
| [src/world_state/graph/reducer.rs](../../src/world_state/graph/reducer.rs) |
| [src/workspace/reducer.rs](../../src/workspace/reducer.rs) |
| [src/context/reducer.rs](../../src/context/reducer.rs) |
| [src/task/reducer.rs](../../src/task/reducer.rs) |
| [src/world_state.rs](../../src/world_state.rs) |
| [src/world_state/query.rs](../../src/world_state/query.rs) |
| [src/world_state/graph/runtime.rs](../../src/world_state/graph/runtime.rs) |
| [src/api.rs](../../src/api.rs) |

| Verification gate | Evidence target |
|-------------------|-----------------|
| F0 | `cargo fmt --check` |
| F1 | `cargo check` |
| F2 | focused replay, catch up, and query tests for graph and legacy claim paths |
| F3 | import audit confirms no direct `workspace`, `context`, or `task` reducer imports remain in world model authority code |
| F4 | public API audit confirms store and runtime internals are not the primary surface |

| Dependency closure solved |
|---------------------------|
| Creates the world model public contracts that execution can depend on without reaching into internals |

| Phase 2 gate evidence | Result |
|-----------------------|--------|
| F0 | passed on 2026-04-25 with `cargo fmt --check` |
| F1 | passed on 2026-04-25 with `cargo check` |
| F2 | passed on 2026-04-25 with focused `traversal_graph`, `world_state_graph`, `workflow_task_compatibility`, and `branches_query` integration coverage |
| F3 | passed on 2026-04-25 with `rg -n 'crate::workspace::reducer|crate::context::reducer|crate::task::reducer' src/world_state` returning no matches |
| F4 | passed on 2026-04-25 after demoting top level `GraphRuntime`, `TraversalStore`, and `WorldStateStore` exports and wiring root reads through `WorldModelQueries` |

| Phase 2 completion notes |
|--------------------------|
| World model now decodes traversal source intent from event contracts inside `src/world_state`, not by reaching into source reducers. Root read paths use `WorldModelQueries` instead of storing raw `GraphRuntime` inside `ContextApi`. Top level world model exports now emphasize query contracts over store and runtime types. |

---

### Phase 3 — Execution port foundation and contract extraction

| Field | Value |
|-------|--------|
| Goal | Replace `ContextApi` based execution flows with execution owned ports and stabilize execution owned request contracts |
| Dependencies | Phase 0 and Phase 1 and Phase 2 |
| Docs | [Execution Migration](execution/MIGRATION.md) and [Core Migration](core/MIGRATION.md) |
| Status | completed |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Define execution owned ports for context reads, context writes, prompt artifact reads, node resolution, provider execution, provider validation, event publication, world model queries, and workflow profile loading. | Completed |
| 2 | Migrate capability invocation and task runtime off `ContextApi` onto those ports. | Completed |
| 3 | Decide ownership of `ProviderExecutionBinding` and any other provider execution request contracts that execution must own. | Completed |
| 4 | Remove direct telemetry compatibility type dependence from execution runtime code. | Completed |

| Exit criterion | Completion |
|----------------|------------|
| capability and task execution no longer require `ContextApi`. | Completed |
| execution owned request contracts are stable enough to extract without dragging provider registry and CLI code with them. | Completed |

| Key seams |
|-----------|
| [src/capability/invocation.rs](../../src/capability/invocation.rs) |
| [src/task/runtime.rs](../../src/task/runtime.rs) |
| [src/task/package/prepare.rs](../../src/task/package/prepare.rs) |
| [src/provider/generation.rs](../../src/provider/generation.rs) |
| [src/workflow/events.rs](../../src/workflow/events.rs) |
| [src/api.rs](../../src/api.rs) |

| Verification gate | Evidence target |
|-------------------|-----------------|
| F0 | `cargo fmt --check` |
| F1 | `cargo check` |
| F2 | focused task runtime, capability invocation, and task package tests |
| F3 | import audit confirms no new execution runtime code depends on `ContextApi` or root telemetry internals |
| F4 | port and contract audit confirms execution public APIs expose traits and contracts rather than root facades |

| Dependency closure solved |
|---------------------------|
| Establishes the contract layer required to split workflow runtime and root adapter code safely |

| Phase 3 gate evidence | Result |
|-----------------------|--------|
| F0 | passed on 2026-04-25 with `cargo fmt --check` |
| F1 | passed on 2026-04-25 with `cargo check` |
| F2 | passed on 2026-04-25 with focused `capability_invocation`, `task_executor`, and `workflow_task_compatibility` coverage |
| F3 | passed on 2026-04-25 with targeted `rg` audits showing runtime execution seams no longer import `ContextApi` outside tests and task runtime plus workflow event contracts no longer depend on telemetry workflow turn payloads |
| F4 | passed on 2026-04-25 after adding `src/execution` traits and execution owned provider request contracts with `ContextApi` limited to compatibility adapters |

| Phase 3 completion notes |
|--------------------------|
| `src/execution` now owns the first extraction safe trait surface and provider request contracts. Capability invocation, task runtime, traversal expansion, workflow resolver, package preparation, and prompt lineage helpers now consume execution ports instead of the root facade type. `ContextApi` remains only as the compatibility adapter host for these paths. |

---

### Phase 4 — Workflow runtime split and root adapter cutover

| Field | Value |
|-------|--------|
| Goal | Separate execution owned workflow runtime from CLI, config, and root product routing code, then cut root adapters over to public crate APIs |
| Dependencies | Phase 2 and Phase 3 |
| Docs | [Execution Migration](execution/MIGRATION.md) and [Core Migration](core/MIGRATION.md) |
| Status | completed |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Split workflow runtime authority files from workflow command and tooling adapters. | Completed |
| 2 | Move config loading, workspace node resolution, provider existence checks, and product routing behind root adapters and ports. | Completed |
| 3 | Introduce one runtime assembly layer in root `meld` that wires events, world model, execution, and root adapters explicitly. | Completed |
| 4 | Stop storing graph runtime and progress emission state as ambient mutable dependencies inside root facades. | Completed |
| 5 | Route root command handlers to public execution and world model APIs rather than internal module paths. | Completed |

| Exit criterion | Completion |
|----------------|------------|
| workflow runtime is execution owned and depends on ports only. | Completed |
| root `meld` is the explicit product shell and adapter host rather than an ambient dependency source. | Completed |

| Key seams |
|-----------|
| [src/workflow/executor.rs](../../src/workflow/executor.rs) |
| [src/workflow/resolver.rs](../../src/workflow/resolver.rs) |
| [src/workflow/facade.rs](../../src/workflow/facade.rs) |
| [src/workflow/commands.rs](../../src/workflow/commands.rs) |
| [src/workflow/tooling.rs](../../src/workflow/tooling.rs) |
| [src/provider/tooling.rs](../../src/provider/tooling.rs) |
| [src/workspace/commands.rs](../../src/workspace/commands.rs) |
| [src/api.rs](../../src/api.rs) |
| [src/bin/meld.rs](../../src/bin/meld.rs) |

| Verification gate | Evidence target |
|-------------------|-----------------|
| F0 | `cargo fmt --check` |
| F1 | `cargo check` |
| F2 | focused workflow execution, CLI route, and provider validation tests |
| F3 | import audit confirms workflow runtime does not load config or workspace internals directly |
| F4 | root command and wiring code calls public crate APIs only |
| F5 | `cargo test` |

| Dependency closure solved |
|---------------------------|
| Completes the split between execution authority and root product composition |

| Phase 4 gate evidence | Result |
|-----------------------|--------|
| F0 | passed on 2026-04-25 with `cargo fmt --check` |
| F1 | passed on 2026-04-25 with `cargo check` |
| F2 | passed on 2026-04-25 with focused `workflow_cli`, `workflow_task_compatibility`, and `provider_cli` coverage |
| F3 | passed on 2026-04-25 with targeted `rg` audits showing workflow runtime no longer imports `ConfigLoader`, `WorkflowRegistry::load`, workspace node resolution, or provider registry validation hooks |
| F4 | passed on 2026-04-25 after routing workflow target execution through workflow profile load ports and introducing explicit CLI runtime assembly wiring |
| F5 | passed on 2026-04-25 with full `cargo test` regression coverage |

| Phase 4 completion notes |
|--------------------------|
| Root CLI runtime ownership now sits in `src/cli/runtime_assembly.rs`. `RunContext` delegates to that assembly instead of carrying raw runtime fields directly. Workflow facade no longer loads config or registries on its own, and workflow executor plus task path execution now consume execution ports and world model query ports rather than the root facade type directly. |

---

### Phase 5 — Root composition seal and public surface cleanup

| Field | Value |
|-------|--------|
| Goal | Reduce root exports, formalize compatibility shims, and ensure each authority crate surface is narrow and declarative |
| Dependencies | Phase 1 and Phase 2 and Phase 3 and Phase 4 |
| Docs | [Core Migration](core/MIGRATION.md), [Events Migration](events/MIGRATION.md), [World Model Migration](world_state/MIGRATION.md), and [Execution Migration](execution/MIGRATION.md) |
| Status | proposed |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Reduce `src/lib.rs` to authority aligned exports and temporary compatibility re exports only. | Proposed |
| 2 | Remove long term public exposure of temporary runtime and store types from root and extracted surfaces. | Proposed |
| 3 | Add clear compatibility wrappers for any old import paths that still need a transition window. | Proposed |
| 4 | Audit all direct crate internal reach through from root adapter code and replace them with public API calls. | Proposed |

| Exit criterion | Completion |
|----------------|------------|
| root `meld` surface reflects composition and compatibility, not domain authority leakage. | Proposed |
| authority crate public APIs are stable enough for physical extraction. | Proposed |

| Key seams |
|-----------|
| [src/lib.rs](../../src/lib.rs) |
| [src/world_state.rs](../../src/world_state.rs) |
| [src/events.rs](../../src/events.rs) |
| [src/workflow.rs](../../src/workflow.rs) |
| [src/task.rs](../../src/task.rs) |

| Verification gate | Evidence target |
|-------------------|-----------------|
| F0 | `cargo fmt --check` |
| F1 | `cargo check` |
| F3 | import audit confirms dependency directions match crate intent |
| F4 | public surface audit confirms no temporary store or runtime types remain primary |
| F5 | `cargo test` |

| Dependency closure solved |
|---------------------------|
| Establishes the stable public surfaces needed for the final crate extraction move |

---

### Phase 6 — Physical crate extraction and declarative doc cutover

| Field | Value |
|-------|--------|
| Goal | Move implementation into actual crates, cut root `meld` over to those crates, and then rewrite each `CRATE.md` as declarative truth |
| Dependencies | Phase 1 through Phase 5 |
| Docs | all `CRATE.md` and `MIGRATION.md` docs in this folder |
| Status | proposed |

| Order | Task | Completion |
|-------|------|------------|
| 1 | Create workspace crate layout for `meld-events`, `meld-world-model`, and `meld-execution`. | Proposed |
| 2 | Move implementation modules into their owning crates with minimal behavior change. | Proposed |
| 3 | Update root `meld` to depend on extracted crates through public APIs only. | Proposed |
| 4 | Run full dependency and regression gates. | Proposed |
| 5 | Rewrite each `CRATE.md` so it describes completed architecture only and remove migration language from those files. | Proposed |

| Exit criterion | Completion |
|----------------|------------|
| physical crates match the authority split defined in this plan. | Proposed |
| each `CRATE.md` is declarative and accurate for the completed codebase. | Proposed |

| Key seams |
|-----------|
| crate manifests and workspace metadata |
| extracted crate roots and public APIs |
| root `meld` runtime assembly and compatibility shims |
| all `CRATE.md` docs in this folder |

| Verification gate | Evidence target |
|-------------------|-----------------|
| F0 | `cargo fmt --check` |
| F1 | `cargo check` |
| F3 | final import audit across crates |
| F4 | final public surface audit across crates |
| F5 | `cargo test` |

| Dependency closure solved |
|---------------------------|
| Completes the authority split and makes the declarative crate docs true |

## Cross Phase Gates

| Gate | Applied phases | Requirement |
|------|----------------|-------------|
| Characterization parity | Phase 0 through Phase 6 | no targeted behavior drift without an intentional contract update |
| Forbidden dependency audit | Phase 1 through Phase 6 | events do not depend on session or telemetry authority, world model does not import source domain internals, execution does not depend on root internals, extracted crates do not depend on root `meld` |
| Public surface audit | Phase 2 through Phase 6 | stores and runtimes do not become long term public crate APIs |
| Adapter discipline | Phase 3 through Phase 6 | root adapters call public crate APIs and do not reintroduce ambient shared facades |

## Implementation Order Summary

| Order | Phase | Summary |
|-------|-------|---------|
| 1 | Phase 0 | Lock behavior and publish dependency gates |
| 2 | Phase 1 | Separate event truth from session and telemetry concerns |
| 3 | Phase 2 | Separate world model truth from source domain internals and store shaped APIs |
| 4 | Phase 3 | Extract execution ports and remove `ContextApi` from execution core |
| 5 | Phase 4 | Split workflow runtime from root adapters and make root wiring explicit |
| 6 | Phase 5 | Seal root and public surfaces for physical extraction |
| 7 | Phase 6 | Move code into crates and rewrite `CRATE.md` as declarative truth |

## Final Readiness Condition

This plan is complete when all of the following are true:

- root `meld` is the only product composition shell
- `meld-events`, `meld-world-model`, and `meld-execution` each own one clear source of truth
- no extracted crate depends on root `meld`
- current migration docs are no longer describing future work for the completed split
- each `CRATE.md` is purely declarative and accurate for the resulting codebase
