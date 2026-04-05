# Telemetry Refactor

Date: 2026-04-05
Status: active
Scope: migration path from the current telemetry centered implementation to a dedicated event management design

## Objective

Move from the current blended telemetry runtime to a design where event management is a durable control runtime concern and telemetry is a downstream consumer.

This document explains how to get there.
It does not redefine the target requirements.

## Starting Point

The current implementation already provides useful building blocks:

- durable event envelope storage
- session lifecycle handling
- in process bus and ingestor
- sink oriented telemetry structure

The current implementation also mixes concerns:

- telemetry manages event transport and storage
- telemetry defines many emitted event names
- task execution keeps a separate local event list
- emit calls pay immediate ingest and flush cost

## Migration Thesis

The safest path is evolutionary, not revolutionary.

Keep the current durable envelope and storage path early.
Introduce explicit event manager ownership next.
Then reduce telemetry to downstream consumption.

## Target Ownership Shape

### End state

- `control` owns domain event meaning and reducer rules
- `events` owns canonical ingress, ordering, durability, replay, and subscription contracts
- `telemetry` owns downstream consumers and observability adapters

### During migration

- compatibility wrappers are allowed
- duplicated emit paths are temporarily allowed if one path is clearly marked transitional
- task local event storage may remain for tests until durable parity is proven

## Migration Phases

### Phase 0

Freeze vocabulary and boundary language.

Required outcomes:

- one written statement that telemetry is downstream in the target model
- one written statement that reducers own state transitions
- one written statement that task local events are transitional and not authoritative

### Phase 1

Introduce the `events` design and contract area.

Required outcomes:

- typed control event families are named
- event envelope continuity rules are written
- stream identity and sequence rules are written
- durability classes are written

### Phase 2

Add a dedicated control event contract in code.

Required outcomes:

- introduce typed control event definitions
- provide serde encoding into the durable envelope payload
- keep compatibility with current readers where practical

### Phase 3

Introduce canonical ingress and reducer ownership.

Required outcomes:

- add one bounded ingress queue for canonical control events
- add one reducer loop per runtime instance or equivalent explicit ownership boundary
- make reducer output drive projections and durable append

### Phase 4

Move task events into the shared spine.

Required outcomes:

- `TaskExecutor` stops treating local `Vec<TaskEvent>` as the authoritative event record
- task lifecycle and artifact events flow into canonical ingress
- current task tests gain parity coverage against durable event output or projection state

### Phase 5

Reduce telemetry to downstream consumption.

Required outcomes:

- telemetry storage and export subscribe to the canonical event path
- summary mapping becomes downstream adaptation rather than primary domain emission
- TUI and external sinks read from canonical records or derived projections

### Phase 6

Change durability strategy on the hot path.

Required outcomes:

- remove implicit flush on every low value event from the correctness path
- keep explicit commit rules for events that require durable acknowledgment
- document batching or checkpoint policy

### Phase 7

Remove compatibility wrappers after parity is proven.

Required outcomes:

- legacy direct telemetry emission paths are deleted
- authoritative event ownership is obvious from module boundaries

## Code Path Mapping

### Current code paths that likely move or narrow

- `src/telemetry/sessions/service.rs`
  current lifecycle and emit orchestration
- `src/telemetry/routing/bus.rs`
  current in process bus
- `src/telemetry/routing/ingestor.rs`
  current ingest and sequence assignment
- `src/task/events.rs`
  current task local event contracts
- `src/task/executor.rs`
  current task local event accumulation
- `src/task/runtime.rs`
  current workflow oriented compatibility emission

### Expected landing direction

- control domain event contracts should live under `src/control`
- runtime event manager mechanics may live under `src/control/events` or one equivalent domain first area
- telemetry sinks should remain under `src/telemetry`

The exact Rust module shape can change, but the ownership line should not.

## Required Characterization Coverage

Before behavior moves, preserve these guarantees with tests:

- session boundary ordering
- monotonic sequence assignment
- replay of durable events into final projection state
- current TUI or summary readers continue to function during migration
- task event parity between local behavior and canonical durable output

## Required Design Decisions Before Large Refactor

These decisions should be made before major code movement:

1. stream identity model
2. sequence scope model
3. durability classes
4. reducer granularity
5. projection persistence model
6. slow consumer policy

## Risks

### Hidden dual ownership

The main migration risk is keeping two authoritative event paths longer than intended.

Mitigation:

- mark one path canonical at each phase
- add tests that assert which path drives final state

### Telemetry naming drift

If telemetry continues to define business event meaning, the refactor will rename code without changing architecture.

Mitigation:

- keep domain event definitions outside telemetry
- make telemetry consume typed events or derived projections

### Hot path regression

If the new event manager preserves per event drain and flush on the hot path, the architectural split will be cleaner but runtime behavior will still bottleneck under burst traffic.

Mitigation:

- make durability policy explicit
- benchmark bursty task event traffic before removing transitional code

## Acceptance Shape

This refactor is complete when all of these are true:

- telemetry is no longer the owner of canonical business event semantics
- task and control events share one canonical durable path
- reducers are the explicit owners of runtime state transitions
- durable replay can rebuild required control projections
- telemetry consumers can follow the canonical stream without sitting in the correctness path
