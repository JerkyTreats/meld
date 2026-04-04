# Unified Task Network Diagram

Date: 2026-03-28
Status: active
Scope: one coherent architecture view for goals, task network, plan, task, capability, and events

## Intent

Collapse the earlier diagram set into one aligned architecture entity.

## Diagram

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

## Reading Notes

- `goals` answers why plan change is needed
- `task_network` is the sole owner of orchestration state
- `plan` contains only tasks
- `task` is the compiled capability graph unit
- `capability` is the atomic execution contract
- events are emitted by running work and consumed only by task network reducers
- repair enters through goal intent and executes as `plan::modify`
