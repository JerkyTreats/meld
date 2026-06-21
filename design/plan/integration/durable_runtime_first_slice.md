# Durable Flywheel Runtime First Slice

Date: 2026-06-14
Status: proposed
Scope: durable single process runtime assembly for the first `docs_freshness` flywheel turn

## Purpose

This document defines the first runtime assembly shape after the non assembly gaps are closed.

Pre implementation gaps found during review are tracked in [Durable Runtime Pre Implementation Gaps](durable_runtime_pre_implementation_gaps.md). Resolve those before adding the runtime assembly.

The goal is not to build a central owner for the flywheel. The goal is to wire existing domain runtimes so the output of one runtime becomes durable input to the next runtime.

The first slice must prove one minimal `docs_freshness` turn and also leave behind a durable runtime foundation.

## Architecture Position

Root `meld` owns runtime assembly, storage opening, config loading, adapter construction, CLI routing, and process lifecycle.

Root `meld` does not own event truth, world model truth, execution policy, belief settlement, goal judgment, planning policy, task network state, or publication semantics.

The first runtime assembly should therefore be an initiator and supervisor, not the center of the flywheel.

## Assembly Model

Add root runtime assembly under `src/runtime` or another root owned entrypoint module chosen by implementation.

Root assembly owns:

- store path resolution
- concrete store opening
- adapter construction for context, provider, workspace, and prompt artifacts
- process level cancellation and shutdown
- diagnostic reports for operators and tests

Root assembly must not:

- decide whether a goal should exist
- decide whether a goal is satisfied
- decide how a task outcome becomes belief evidence
- mutate task network state outside the task network command boundary
- mutate execution goal state outside the goal set API
- mutate graph or belief state outside world model runtimes

## Flywheel Model

The product flywheel is direct handoff through durable boundaries:

```text
world model runtime
-> execution goal API
-> execution runtime
-> event append API
-> world model replay runtime
```

Root wires the APIs and starts the runtimes. Root does not mediate each handoff after the runtimes are connected.

## Domain Runtime Actors

The first slice should treat each row below as a durable runtime actor even if all actors run in one process.

| Actor | Owning Crate Or Domain | Durable Input | Durable Output | First Slice Status |
| --- | --- | --- | --- | --- |
| event spine | `meld-events` | event append requests | sequenced event records | existing |
| graph reducer | `meld-world-model` | sequenced event records | graph indexes and anchors | existing |
| belief assessor | `meld-world-model` | graph evidence and promoted evidence | belief revisions and belief views | existing first slice |
| agent curation | `meld-world-model` | belief revisions, planner projection, active goals | curation decisions and goal commands | existing first slice |
| goal set | `meld-execution` | accepted goal commands and mutation commands | durable goal lifecycle records | existing |
| planning loop | `meld-execution` | active goals and planner projection | task network mutation commands | existing contracts |
| task network reducer | `meld-execution` | task network commands | durable task network state and outbox | existing |
| task worker | `meld-execution` plus root adapters | ready task claims | task outcomes and artifacts | existing first slice |
| publication worker | `meld-execution` plus `meld-events` | task network publication outbox | execution outcome facts in event spine | existing |
| evidence ingestion | `meld-world-model` plus root adapter | published execution outcome facts | promoted evidence and belief reassessment | existing first slice |
| satisfaction curation | `meld-world-model` plus `meld-execution` adapter | active goals and updated projection | goal mutation commands and satisfied goals | existing first slice |

The first proof may drive these actors in deterministic order. That driver is a proof harness, not the product architecture. Correctness must come from each actor reading and writing durable state through its owner boundary.

## Worker Contract

Each actor should expose or be wrapped by a bounded work operation.

```rust
pub struct WorkBudget {
    pub max_items: usize,
}

pub enum WorkStatus {
    Idle,
    MadeProgress,
}
```

The concrete API can differ by domain, but every bounded worker operation must follow the same durability rules.

- read from durable input state
- claim work durably when duplicate work would be harmful
- write through the owning domain command or append boundary
- advance cursors only after output is durable
- tolerate duplicate delivery
- return diagnostics that are not semantic authority

## First Slice Proof Driver

The first proof can run a bounded deterministic driver.

```text
event graph catchup
belief assessment
agent goal curation
goal acceptance
planning tick
task network command acceptance
task claim and execution
publication outbox drain
event graph catchup
outcome evidence ingestion
belief reassessment
agent satisfaction curation
goal satisfaction mutation
```

The driver stops at named proof checkpoints, when no actor makes progress, or when the configured budget is exhausted.

The driver must not store correctness state in local variables. Local reports are allowed only for diagnostics. Restart must resume from domain stores.

## Storage Shape

The first proof should use one product storage root with named stores or trees for each durable owner.

Required state:

- event spine
- graph reducer state
- belief store
- agent store
- execution goal store
- task network store
- task artifact store
- context frame and prompt artifact stores

Store layout is root `meld` product assembly. Record meaning remains owned by each crate or domain.

The concrete implementation surface is `ProductStorageLayout` plus `OpenProductStores` under root runtime storage assembly. The first durable proof should resolve one product root, open stores through that layout, and avoid direct `sled::Db` exposure in supervisor-facing runtime fields.

The product layout uses separate sled database groups for the event ledger, workspace records, world model state, execution goals, task artifacts, and one task network database per network id. Context frames and prompt artifacts remain filesystem content-addressed stores below the same product root.

## Cursor And Idempotency Rules

The first slice must make restart safety visible.

- event publication uses stable record ids
- publication marks happen only after event append succeeds
- belief evidence ingestion dedupes by source cursor and source id
- agent curation persists decisions before advancing delivery cursors
- goal acceptance dedupes by source identity and command id
- satisfaction mutation dedupes by goal id and review sequence
- task network claims and outcomes use fenced claim records

No root level cursor may replace these domain cursors.

## Entrypoints

The first implementation should prefer a library assembly entrypoint before adding a user facing CLI command.

Suggested shape:

```rust
pub struct ProductRuntimeAssembly { ... }

impl ProductRuntimeAssembly {
    pub fn load_for_workspace(...) -> Result<Self, RuntimeAssemblyError>;
    pub fn start_supervisor(&self, request: RuntimeStartRequest) -> Result<RuntimeSupervisor, RuntimeAssemblyError>;
}
```

The CLI can later call this assembly. Tests may use a deterministic proof driver built from the same assembly.

## First Proof

Add one end to end test:

```text
minimal_runtime_flywheel_turn_persists_and_satisfies_goal
```

The proof must show:

- seeded or observed evidence reaches the event spine
- graph and belief state update from durable inputs
- agent curation emits one goal command
- execution stores one active goal
- planning emits a task network mutation command
- task network accepts the command and dispatches the docs writer task
- task outcome creates a pending publication
- publication worker appends one execution outcome fact
- outcome evidence ingestion updates docs freshness belief
- updated planner projection satisfies the goal target
- agent satisfaction curation emits one mutation command
- execution records the goal as satisfied

The proof must reopen runtime assembly after at least one intermediate boundary and continue from stores. Preferred reopen points are after goal acceptance and after pending publication creation.

## Non Goals

- no autonomous background daemon
- no multi process worker supervisor
- no broad sensory runtime
- no plan diffing
- no shared task equivalence across goals
- no provider capacity scheduler
- no causal or regime runtime
- no CLI command before the library proof

## Design Risks

The main risk is centralization by convenience. A single process proof driver can quietly become semantic authority if it keeps progress in memory or directly calls helper functions that bypass command boundaries.

The opposite risk is overbuilding. Full leases, push subscriptions, worker pools, and supervisor recovery are not required for the first proof.

The selected compromise is a durable single process assembly with domain embedded runtimes. One process is acceptable. One semantic owner is not.

## Exit Criteria

This design is satisfied when the first flywheel test proves durable progress through domain stores and every semantic transition is owned by the domain that defines it.

After that point, deepening can split individual actors into independent workers without rewriting the cross domain semantics.
