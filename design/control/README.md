# Control Design

Date: 2026-03-28
Status: active
Scope: control layer design above capability and task compilation, including refactor-phase compatibility orchestration

![[meld/design/control/screenshot-2026-03-28_11-51-02.png]]

## Unified Task Network Architecture

```mermaid
flowchart LR
    subgraph Goals
        A[goal intent]
        B[repair goal]
    end

    subgraph Task_Network
        C[task network instance]
        D[dispatch]
        E[event stream]
        F[event task reducer]
        G[event repair reducer]
        H[task network state]
        I[plan modify]
        J[continuation]
        K[logical operators]
    end

    subgraph Plan
        L[plan]
        M[task]
        N[compiled task graph]
    end

    subgraph Capability_Domain
        O[atomic capability]
        P[domain execution]
        Q[artifacts and effects]
    end

    A --> C
    B --> I

    C --> D
    C --> K
    D --> M
    K --> H

    L --> M
    M --> N
    N --> O
    O --> P
    P --> Q
    Q --> E

    E --> F
    E --> G
    F --> H
    G --> H
    H --> J

    H -->|repair requested| I
    I -->|repair applied| L
    J --> C
```

## Thesis

This design set defines the layer above `capability` and `task`.

The key control concern is `task_network`.
`task_network` is the stateful orchestrator over tasks.
It owns dispatch, event reduction, repair state, continuation, and ordering through logical operators.

During the refactor phase, `control` also acts as the honest temporary home for orchestration logic extracted from `context` and legacy workflow execution.
That temporary role is documented in [Interregnum Orchestration](interregnum_orchestration.md).

Capabilities remain atomic domain owned contracts.
Tasks are compiled graphs of chained capabilities.
Plans contain only tasks.

## Layer Boundary

`control` is not a capability catalog.
`control` is not domain execution.
`control` is not the full goals layer.

`control` does own:

- task network dispatch
- task network state
- event reduction
- continuation
- repair through `plan::modify`

## Core Architecture

The architecture is layered:

- `goals`
  - why change is needed
- `control`
  - how tasks are dispatched, observed, reduced, and repaired
- `plan`
  - tasks only
- `task`
  - compiled capability graph
- `capability`
  - atomic executable contract

The most important constraint is state ownership.
Running work emits events.
Only the task network reduces those events into durable state.

## Durable Structure

The durable structure is:

- `interregnum_orchestration.md`
  - refactor-phase orchestration ownership before task execution takes over
- `task_network.md`
  - task network model, events, state ownership, repair, and dispatch
- `architecture_diagrams.md`
  - the unified architecture diagram and reading notes
- `htn/`
  - hierarchy and lineage
- `program/`
  - logical operators and control graph details
- `runtime/`
  - continuation and resume
- `repair/`
  - repair entry and repair rules

## Core Decisions

- `task_network` is the primary stateful control artifact
- `control` owns refactor-phase compatibility orchestration when orchestration has left `context` but task execution is not ready yet
- `task` is a compiled artifact, not a state owner
- `plan` contains only tasks
- `plan` and graph execution semantics are one coherent control concern
- capability behavior remains atomic and domain owned
- events are the only path from running work back into task network state
- repair is a control function expressed as `plan::modify`
- the reason for repair belongs to `goals`, not to control

## Compiled Artifacts

The current model assumes these durable artifact families:

- `plan`
- `task`
- task network continuation and state records
- event log records for `event::task` and `event::repair`

## Read Order

1. [Bootstrap Plan](PLAN.md)
2. [Interregnum Orchestration](interregnum_orchestration.md)
3. [Task Network](task_network.md)
4. [Unified Task Network Diagram](architecture_diagrams.md)
5. [HTN Model](htn/README.md)
6. [HTN Lineage Model](htn/lineage_model.md)
7. [Control Program Model](program/README.md)
8. [Control Graph Model](program/control_graph.md)
9. [Runtime Model](runtime/README.md)
10. [Continuation Model](runtime/continuation_model.md)
11. [Repair Model](repair/README.md)
12. [Repair Entry Model](repair/repair_entry.md)

## Read With

1. [Capability And Task Design](../capabilities/README.md)
2. [Domain Architecture](../capabilities/domain_architecture.md)
3. [HTN Turing Plan](../htn_turing.md)
4. [Goals](../goals/README.md)

## Reading Notes

- `goals` provides why change is needed
- `control` temporarily houses extracted orchestration during the refactor window
- `task_network` owns dispatch, event reduction, repair state, and continuation
- `plan` contains only `task`
- `task` encapsulates compiled capability structure
- `capability` remains atomic domain behavior
- events are the only path from running work back into task network state

## Non Goals

- making capabilities stateful orchestrators
- letting tasks mutate task network state directly
- mixing goals and control into one layer
- forcing repair intent to live only inside control
