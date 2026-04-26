# Crate Split Dependency Checklist

Date: 2026-04-25
Status: active
Scope: forbidden dependency directions for the authority split

## Purpose

This checklist freezes the dependency rules that every crate split phase must preserve.
It is the audit source for gate `F3` in [PLAN](PLAN.md).

## Authority Directions

| Authority | May depend on | Must not depend on |
| --- | --- | --- |
| `meld-events` | storage primitives, serde, event contracts | `session`, `telemetry`, world model reducers, execution runtime, root adapters |
| `meld-world-model` | `meld-events` public contracts, world model storage, world model query contracts | `workspace` reducer internals, `context` reducer internals, `task` reducer internals, provider internals, root facades |
| `meld-execution` | `meld-events` public contracts, `meld-world-model` public queries, execution owned contracts | `ContextApi`, CLI routing, config loading, telemetry internals, root mutable facades |
| root `meld` | all public crate APIs, CLI, config, adapters, compatibility shims | authority crate internals reached through private module paths |

## Concrete Import Audits

Use these audits during phases `F3`.

### Events

- `rg -n 'crate::session|crate::telemetry' src/events`
- expected result after Phase 1: no matches inside extracted event authority files

### World model

- `rg -n 'crate::workspace::reducer|crate::context::reducer|crate::task::reducer' src/world_state`
- expected result after Phase 2: no matches inside world model authority files

### Execution

- `rg -n 'ContextApi|crate::telemetry::' src/capability src/task src/workflow src/control`
- expected result after Phase 3 for execution core paths: no new runtime dependencies on root facade or telemetry internals

### Root shell

- `rg -n 'crate::world_state::graph::|crate::events::store::|crate::task::runtime::|crate::workflow::executor::' src/api.rs src/cli src/provider src/workspace src/workflow`
- expected result after Phase 5: root adapter code reaches authority crates through public surfaces only

## Review Checklist

- event authority changes do not add session retention or telemetry aliases
- world model changes do not add new source reducer hooks
- execution changes do not add new `ContextApi` parameters to execution owned paths
- root adapter changes do not grow ambient runtime storage or private cross domain reach through

## Read With

- [PLAN](PLAN.md)
- [Crate Boundary Assessment By Domain](microarchitecture_assessment_by_domain.md)
- [Core Migration](core/MIGRATION.md)
- [Events Migration](events/MIGRATION.md)
- [World Model Migration](world_state/MIGRATION.md)
- [Execution Contract Extraction](completed/execution_contract_extraction.md)
