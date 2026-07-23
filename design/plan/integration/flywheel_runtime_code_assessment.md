# Flywheel Runtime Code Assessment

Date: 2026-06-16
Status: historical code assessment
Scope: current codebase assessment for domain embedded flywheel runtime implementation

Current runtime-completion authority is [Runtime Completion Ground Map](runtime_completion_ground_map.md), followed by [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md). This dated assessment remains implementation evidence only where it agrees with that authority.

## Purpose

This assessment identifies what the current code already supports, what must be added, and what must be removed or avoided so the runtime follows the cognitive architecture flywheel.

The intended shape is:

```text
root assembly
-> world model runtime
-> execution runtime
-> event spine
-> world model runtime
```

Root `meld` wires ports and supervises process lifecycle. It must not become the global event loop.

## Current Supportive Surfaces

### Product Storage

[runtime storage](../../../src/runtime/storage.rs) already provides `ProductStorageLayout` and `OpenProductStores`.

This is compatible with the flywheel model. Root owns physical store layout while each domain owns record meaning.

Keep this surface.

### Supervisor Diagnostics

[runtime contracts](../../../src/runtime/contracts.rs) provides `WorkBudget` and `WorkerTickReport`.

This is compatible when treated as supervisor diagnostics. It must not become a shared root cursor table or the authority for progress.

Keep this surface after the comment rename to supervisor facing language.

### Runtime Supervisor Domain

Root `meld` has supervisor facing contracts, but it does not yet have a durable supervisor domain.

Add a focused supervisor implementation for lifecycle, leases, heartbeats, restart policy, shutdown, and status. The detailed plan is [Runtime Supervisor Domain Plan](runtime_supervisor_domain_plan.md).

Suggested owner:
root `meld`

Likely paths:

```text
src/runtime/supervisor.rs
src/runtime/supervisor/contracts.rs
src/runtime/supervisor/store.rs
src/runtime/supervisor/leases.rs
src/runtime/supervisor/health.rs
src/runtime/supervisor/shutdown.rs
src/runtime/supervisor/restart.rs
```

The supervisor store should not contain event cursors, belief cursors, goal lifecycle state, task network revisions, or publication state.

### Event Publication Port

[task network publication](../../../crates/meld-execution/src/task_network/publication.rs) already defines `EventAppendSink`.

This is the right direction. Execution can publish through an event owned append API without root mediating each outcome.

Keep and reuse this surface.

### Execution Goal Boundary

[goal API](../../../crates/meld-execution/src/goals/api.rs) already defines producer neutral goal acceptance through `GoalAcceptanceRequest` and `GoalSetApi`.

This is the right execution owned boundary. World model agent output can feed it through a goal sink.

Keep and extend this surface.

### World Model Runtime Pieces

[belief runtime](../../../crates/meld-world-model/src/belief/runtime.rs), [belief ingestion](../../../crates/meld-world-model/src/belief/ingestion.rs), and [agent curation](../../../crates/meld-world-model/src/agent/curation.rs) already hold real runtime behavior inside `meld-world-model`.

This validates the user model that world model is itself a runtime.

Keep and extend these surfaces.

## Required Additions

### Goal Command Sink

The code needs a direct world model to execution handoff.

Add an execution owned sink contract for agent goal commands or producer neutral requests. Root assembly can provide the concrete adapter, but the world model agent runtime should call the sink directly after persisting its decision.

Suggested owner:
`meld-execution`

Likely integration point:
[goal API](../../../crates/meld-execution/src/goals/api.rs)

### World Model Agent Runtime

`AgentCuration` currently persists decisions and returns commands. It does not yet own the handoff to execution.

Add a world model agent runtime facade that:

- reads durable belief delivery state
- runs curation
- persists `AgentCurationDecision`
- calls the injected goal sink
- advances the subscription cursor only after the handoff contract is satisfied

Suggested owner:
`meld-world-model`

Likely path:
`crates/meld-world-model/src/agent/runtime.rs`

### Execution Planning Runtime

Planning currently exists, but the durable goal to task network loop is still assembled in tests.

Add an execution runtime that:

- reads active goals from execution storage
- reads planner projection through an injected world model query port
- calls `PlanningRuntime`
- lowers compositions
- submits task network mutation commands through execution owned stores

Suggested owner:
`meld-execution`

Likely path:
`crates/meld-execution/src/planning/runtime.rs`

### Execution Task Network Runtime

Task network command acceptance, dispatch, outcome recording, artifact persistence, and publication exist as pieces. The runtime should own the continuous handoff between them.

Add an execution runtime facade that:

- opens task network stores through the factory
- claims ready tasks
- executes tasks through task and capability surfaces
- persists artifacts
- records outcomes
- publishes pending publications through `EventAppendSink`

Suggested owner:
`meld-execution`

Likely path:
`crates/meld-execution/src/task_network/runtime.rs`

### Event Replay Source

World model should consume the event spine through a bounded replay or subscription port.

The graph runtime already consumes event records and keeps its own cursor. The missing shape is the stable event replay source interface for assembly.

Suggested owner:
`meld-events`

Likely path:
`crates/meld-events/src/events/subscription.rs`

### Outcome Evidence Runtime Path

[outcome evidence](../../../src/execution/outcome_evidence.rs) currently maps docs writer success to promoted evidence at the root boundary.

That is acceptable as a first slice adapter, but product runtime should not have root manually handing each event to world model. The mapper should be wired into world model event intake or exposed as a configured promoted evidence mapper.

Suggested direction:
keep the mapper thin, then call it from world model evidence ingestion after event replay selects an applicable execution fact.

## Surfaces To Remove Or Avoid

### Production Root Run Loop

Do not add a production one shot root loop API.

Do not add production modules named:

```text
src/runtime/turn.rs
src/runtime/actors.rs
src/runtime/root_loop.rs
```

Those names encode the wrong center of gravity. If a deterministic proof driver is needed, keep it in tests or name it explicitly as proof support.

### Root Mediated Goal Handoff

Avoid root code that performs this in the product path:

```text
read world model curation output
map command
call execution goal API
```

That mapping belongs behind a goal sink used by the world model agent runtime. Root may build the sink, but it should not be the runtime step.

### Root Mediated Outcome Handoff

Avoid root code that performs this in the product path:

```text
read execution outcome
build event
append event
call world model ingestion
```

Execution should publish to events. World model should consume events.

### Manual Integration Tests As Architecture

[docs freshness reopen contract](../../../tests/integration/docs_freshness_reopen_contract.rs) correctly proves durable state, but it manually sequences every handoff.

Keep it as a contract test. Do not promote its structure into production runtime code.

### Legacy Workflow Authority

Legacy workflow paths still provide useful compatibility and docs writer execution support.

Do not delete them now. Do not let them become the authority for the cognitive flywheel. The cognitive path should flow through execution goals, planning, task network, task, publication, and event replay.

## Code That Does Not Need Deletion Now

- [runtime storage](../../../src/runtime/storage.rs)
- [runtime contracts](../../../src/runtime/contracts.rs)
- [CLI runtime assembly](../../../src/cli/runtime_assembly.rs)
- [goal mutation adapter](../../../src/execution/goal_mutation.rs)
- [outcome evidence mapper](../../../src/execution/outcome_evidence.rs)

These are compatible if they remain assembly surfaces, adapters, or compatibility paths. They become a problem only if they are used to create a root centered event loop.

## Immediate Cleanup Already Applied

- Replaced old root runtime comments with supervisor facing language.
- Reframed the durable runtime phase design around embedded domain runtimes.
- Reframed durable first slice from root loop to assembly plus proof driver.
- Reframed minimal flywheel work items around handoffs instead of root driven operations.

## Implementation Gate

Before writing runtime code, assert these conditions:

- no production one shot root loop is introduced in root `meld`
- world model agent runtime owns the goal handoff
- execution runtime owns planning to publication flow
- world model runtime owns event replay to belief update flow
- root assembly owns stores, ports, lifecycle, and diagnostics only

## Verification Commands

```sh
rg -n "central coordinator|one shot root loop" src crates design/plan/integration
cargo test runtime::contracts
cargo test --test integration_tests docs_freshness_reopens
```
