# Execution Domain

Date: 2026-06-02
Status: active
Scope: world-model-aware action through goal-directed planning, task network execution, and outcome publication

## Thesis

Execution is the system's push layer. It reads the world model and acts to change the world.

The world model agent curates a goal set — desired belief states — through execution's public API. The planning loop reads that goal set and the world model view, then submits commands that maintain a task network graph that closes the gap between current belief and desired state. The task network executes tasks in parallel, governed by dependency structure. Outcomes publish back through the event spine for belief revision.

The task network is a single writer event sourced aggregate. Its command boundary serializes graph mutation sets, dispatch claims, task outcomes, artifact availability, and publication marks into one accepted revision stream.

Seed agents are trusted genesis state created by init or configuration. After seed agents exist, additional agents are created through ordinary execution goals curated by authorized agents. Execution owns the agent initialization workflow and runs the required tasks and capabilities.

The foundational pattern is graphs-lower-graphs:

```
Capability      atomic executable contract
    ↑ composed into graph
Task            graph of Capabilities, compiled and dependency ordered
    ↑ composed into graph
Task Network    graph of Tasks, the plan and dependency ordered
```

At each level, the execution model is identical: compute the ready set, dispatch, receive events, update state.

## Boundary

`execution` owns:

- the goal set — desired belief states as data, with a public curation API consumed by world model agents
- agent initialization workflows requested by seed config or curated `CreateAgent` goals
- the planning loop — continuous plan construction reading goal set and world model view
- the task network graph — shared execution substrate across all goals, parallel by dependency
- the task network command boundary — single writer authority for graph and lifecycle state
- dispatch through task and capability execution
- publication of outcomes, failures, and learned facts back into events
- synthesis escalation when the current capability catalog cannot satisfy a goal
- workflow runtime as compatibility layer where cognitive subsystems are not yet built

`goals` owns the goal set data structure, lifecycle state machine, and curation API.
`planning` owns HTN decomposition, graph mutation proposals, task network command construction, cost-aware plan transitions, guard and observation semantics.
`synthesis` owns runtime capability growth.
`task` and `capability` own compiled execution units and atomic contracts.
The world model agent owns normative judgment — deciding which goals should exist — and curates the goal set through execution's public API.
The world model owns graph and belief views that execution reads as `WorldState` — a set of ground propositions in the shared language [`meld-lang`](../meld-lang/README.md).
`meld-lang` owns the shared typed substrate: `Proposition`, `Goal`, `Operator`, `Composition`, `WorldState`, `Effect`, `Method`, and all pure evaluation operations. Execution depends on `meld-lang` for all planning types.

## Architecture

```mermaid
flowchart TD
    AG[world model agent] -->|curates via API| GS[goal set]
    WMV[world model view] --> PL[planning loop]
    GS --> PL
    PL -->|commands| TNC[task network command boundary]
    TNC -->|accepted state| TN[task network graph]
    TN -->|ready set| EX[task execution]
    EX -->|outcome commands| TNC
    EX -->|outcome events| SP[event spine]
    SP --> WM[world model]
    WM -.->|belief revision| WMV
    WM -.->|belief revision| AG
    PL -->|capability missing| SY[synthesis]
    SY -->|catalog updated| PL
```

## Documents

### Core

- [Goals](goals/README.md)
  normative layer — desired belief states, lifecycle, curation API, satisfaction checking
- [Agent Genesis And Activation](../world_model/agent/genesis_and_activation.md)
  seed agent authority, runtime activation, and initialization capability work
- [Execution Planning](planning/README.md)
  planning loop, HTN decomposition, guard and observation semantics
- [Planning Pipeline](planning/planning_pipeline.md)
  graphs-lower-graphs execution model: planning loop and task network, connected by commands
- [Task Network](task_network.md)
  stateful orchestration over compiled tasks, command acceptance, and event-driven dispatch
- [Task Initialization](task_initialization.md)
  seed artifacts, data flow materialization, and validation authority before task dispatch

### Supporting

- [Synthesis Overview](synthesis/README.md)
  runtime capability growth when the catalog cannot satisfy a goal
- [Execution Crate](CRATE.md)
  `meld-execution` crate boundary, owned modules, workflow runtime, task/capability authority

### Open Gaps

- [Execution Gaps](GAPS.md)
  open contracts and undefined seams — world model read interface, outcome publication, workflow integration

### Examples

- [Examples](examples/) — concrete capability designs for git diff summary, bayesian evaluation, and ast change impact

### Research

- [Research](research/) — HTN/GOAP research, system evaluations, and reference implementations

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Lang Domain](../meld-lang/README.md)
- [World Model Domain](../world_model/README.md)
- [World Model Agent](../world_model/agent/README.md)
- [World Model Planner](../world_model/planner/README.md)
- [World Model Belief](../world_model/belief/README.md)
- [Events Domain](../events/README.md)
