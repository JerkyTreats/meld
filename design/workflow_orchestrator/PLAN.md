# Capability And Plan Implementation Plan

Date: 2026-03-27
Status: proposed
Scope: refactor current behavior into capability-ready domain contracts and plan-ready compiler inputs

## Objective

Refactor existing code so current behavior can be expressed as validated plans of bound capability instances.

This plan does not add new HTN behavior.
This plan does not make compiler choose capabilities.
This plan does not make execution smarter.

This plan does:

- define durable capability contracts
- make current domain behavior capability-ready
- add plan graph infrastructure on `petgraph`
- add compiler as plan validation and lock-in
- preserve current command behavior while old workflow-shaped internals are removed

## First Slice Outcomes

- `context generate` becomes capability-ready and loses mixed orchestration concerns
- current docs writer behavior becomes representable as a candidate capability graph
- compiler accepts candidate capability graphs and emits locked plan records
- plans are parallel-ready DAGs
- plan graph infrastructure is built on `petgraph`

## Core Architecture

- `src/capability`
- `src/plan`
- `src/plan/compiler`
- `src/plan/graph`
- `src/context`

The durable abstraction is not `workflow`.
The durable abstractions are `capability` and `plan`.

## Scope Guardrails

In scope:

- capability contracts for existing behavior
- context refactors needed to expose `context_generate` cleanly
- plan graph infrastructure
- compiler inputs, validation, and locked plan records
- compatibility lowering from current command paths into candidate capability graphs

Out of scope:

- HTN decomposition
- goal selection
- dynamic capability search
- broad execution redesign beyond what plan readiness requires
- new user-visible orchestration products

## Fixed Decisions

- compiler is goal-agnostic
- compiler validates candidate capability graphs
- compiler emits locked plan records
- plans are DAGs of bound capability instances
- plans are parallel-ready from the first slice
- `petgraph` is the graph substrate
- `context generate` refactor work is required before broad cutover

## Open Decisions

- exact capability catalog record shape
- exact artifact family ids and schema ids
- exact binding set shape and digest inputs
- exact plan record storage layout
- exact checkpoint and repair mapping once execution work begins

## Phases

| Phase | Goal | Status |
|------|------|--------|
| 0 | Freeze capability and plan contracts | proposed |
| 1 | Add capability domain and catalog | proposed |
| 2 | Add plan graph infrastructure on `petgraph` | proposed |
| 3 | Refactor `context generate` to be capability-ready | proposed |
| 4 | Add compatibility lowering into candidate capability graphs | proposed |
| 5 | Add plan compiler and locked plan record emission | proposed |
| 6 | Cut current command paths onto capability graphs and compiler | proposed |

## Phase 0

### Goal

Freeze the minimum contract surface needed for implementation.

### Tasks

- define capability contract fields
- define capability instance fields
- define dependency edge fields
- define artifact handoff fields
- define plan digest inputs
- define graph validation rules
- define first-slice capability families

### Exit

- no implementation task depends on unresolved contract vocabulary

## Phase 1

### Goal

Add the durable capability domain.

### Tasks

- add `src/capability`
- define capability catalog types
- define capability ids and versions
- define artifact contract types
- register first-slice capability families:
- `context_generate`
- `order_execution`
- `compatibility_turn`

### Exit

- existing first-slice behavior can be described as capability contracts without execution detail leaking into compiler

## Phase 2

### Goal

Add plan graph infrastructure on `petgraph`.

### Tasks

- add `src/plan/graph`
- wrap `petgraph` with plan-owned graph types
- add capability instance node types
- add dependency edge types
- add artifact handoff projection types
- add deterministic id generation helpers
- add cycle and disconnected graph validation
- add graph serialization support for locked plan records

### Exit

- compiler has a durable graph substrate that does not leak third-party graph types into domain contracts

## Phase 3

### Goal

Refactor `context generate` to be capability-ready.

### Tasks

- separate target expansion outputs from generation execution
- separate ordering outputs from generation execution
- isolate generation execution behind a capability-facing adapter
- remove hidden orchestration assumptions from `src/context`
- make generation inputs explicit and typed
- make generation outputs explicit and typed
- stop relying on workflow-shaped sequencing inside `context`

### Exit

- `context` owns `context_generate` behavior only
- plan-related concerns no longer live inside `context`

## Phase 4

### Goal

Lower current behavior into candidate capability graphs.

### Tasks

- add lowering from current docs writer behavior into candidate capability graphs
- add lowering from current `context generate` entry paths into candidate capability graphs
- keep lowering outside compiler
- preserve current command behavior while lowering paths are introduced

### Exit

- current command paths can produce candidate capability graphs without invoking execution

## Phase 5

### Goal

Add plan compiler and locked plan record emission.

### Tasks

- add `src/plan/compiler`
- accept candidate capability graph input
- bind scope and policy inputs
- validate capability graph structure
- validate artifact handoffs
- compute plan digests
- emit locked plan records
- emit compile diagnostics

### Exit

- compiler can reject invalid candidate graphs before execution starts
- compiler can emit locked plan records for valid candidate graphs

## Phase 6

### Goal

Cut current command paths onto capability graphs and compiler.

### Tasks

- route current command paths through lowering
- route lowered graphs into compiler
- preserve current command behavior
- remove durable dependence on old workflow-shaped internals

### Exit

- current behavior is plan-ready even if full execution cutover is deferred

## Verification

- contract validation tests for capability records
- graph validation tests on `petgraph` wrapper
- deterministic id and digest tests
- compatibility tests for current `context generate`
- compatibility tests for current docs writer behavior

## Read With

- [Capability And Plan Design](README.md)
- [Capability Model](capability/README.md)
- [Context Capability Readiness](context/README.md)
- [Plan Model](plan/README.md)
- [Plan Compiler](plan/compiler/README.md)
- [Plan Graph Model](plan/compiler/graph_model.md)
- [Petgraph Choice](plan/compiler/petgraph.md)
- [Migration Plan](migration_plan/README.md)
