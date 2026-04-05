# Event Manager Requirements

Date: 2026-04-05
Status: active
Scope: declarative requirements for the canonical control event manager

## Objective

Define the requirements for an event manager that becomes the canonical feedback spine for control and task execution.

This document states what must be true.
It does not describe migration sequencing.

## Core Thesis

The application should have one canonical domain event spine for control relevant facts.

That spine must:

- accept events from concurrent producers
- preserve deterministic reduction order
- support durable replay
- drive state projections
- allow downstream consumers such as telemetry without making telemetry the owner of domain semantics

## Required Ownership

### Event manager owns

- event ingress
- event ordering
- durability policy
- replay contract
- subscription contract
- reduction handoff boundary

### Control owns

- domain event vocabulary
- reducer rules
- projection rules
- command to event rules

### Telemetry owns

- observability adapters
- summary mapping
- metrics export
- sink integrations

## Canonical Event Rules

### One spine

There must be one canonical event path for control relevant domain facts.

A task local event list may exist as a cache or test helper, but it must not be authoritative.

### Facts, not views

The canonical stream must contain domain facts, not presentation summaries.

Examples of facts:

- `task_requested`
- `task_started`
- `task_progressed`
- `task_succeeded`
- `task_failed`
- `task_blocked`
- `task_artifact_emitted`
- `repair_requested`
- `repair_applied`
- `dispatch_requested`
- `dispatch_started`
- `dispatch_completed`

Examples of derived views:

- task progress percentage
- active task count
- retry dashboard summary
- TUI lane state

Derived views must come from projections, not primary event emission.

### Reducer order

For one runtime instance, the reducer must observe one deterministic order.

Parallel work is allowed.
State transition ownership is not.

### Typed contracts

Canonical domain events must have typed contracts.

The stored envelope may still use one stable generic shape such as timestamp, stream identity, sequence, event type, and payload.
But the business meaning must not depend on ad hoc JSON blobs alone.

## Durability Requirements

### Replay

The event manager must support rebuilding runtime projections from durable event history after restart.

### Append only

Canonical event records must be append only.
Correction must happen through later events or projection repair, not in place mutation of prior facts.

### Sequence

Each stream must have monotonic sequence assignment.

The design may choose global sequence or per runtime sequence.
Whichever model is selected must be explicit and stable.

### Durability classes

The design must define at least two durability classes:

- events that must become durable before the runtime treats them as committed facts
- events that may be buffered briefly before durable commit

This distinction is required so the hot path does not pay full sync cost for every low value update.

## Ingress Requirements

### Multi producer

The event manager must support concurrent producers.

Expected producers include:

- task executors
- repair logic
- dispatch logic
- operator commands
- compatibility adapters from existing workflow paths

### Backpressure

Ingress must expose bounded backpressure semantics.

An unbounded queue is not acceptable for the canonical runtime path because control load must fail or slow in explicit ways under sustained pressure.

### Cancellation

Ingress and reduction must support explicit cancellation and shutdown semantics.

## Reduction Requirements

### One owner for state transitions

Reducers must be the only owners of canonical runtime state transitions.

Workers may emit events.
Workers must not mutate task network state directly.

### Idempotence posture

The design must define how reducers handle duplicate delivery, restart replay, or retry of boundary actions.

### Projection families

The first projection set should be sufficient to derive:

- active task set
- blocked task set
- completed task set
- failed task set
- artifact availability
- dispatch state
- repair state
- continuation checkpoint state

## Consumer Requirements

### Telemetry is downstream

Telemetry must consume the event stream.
Telemetry must not be the authoritative owner of domain event semantics.

### Multiple consumers

The event manager must allow multiple downstream consumers without making them part of the correctness path for control state.

Expected consumers include:

- telemetry storage adapters
- TUI live views
- metrics exporters
- debugging taps

### Slow consumer isolation

One slow consumer must not break reducer correctness.

The design must define whether slow consumers are dropped, caught up from durable replay, or served from separate projection materialization.

## Diagnostic Requirements

### Separate diagnostics

Diagnostic tracing must remain separate from canonical business events.

Tracing may annotate execution and include correlation fields.
Tracing must not become the sole replay source for task and control semantics.

### Correlation

Canonical domain events should carry enough identifiers to correlate with tracing spans, task runs, artifacts, repair scopes, and operator actions.

## Compatibility Requirements

### Existing envelope continuity

The design should preserve compatibility with the current stored event envelope where practical so existing readers can continue during migration.

### Compatibility adapters

Existing telemetry emit paths may be wrapped during migration, but the target ownership model must stay visible.

## Non Requirements

This document does not require:

- an external message broker
- distributed consensus
- a full CQRS framework as the first implementation step
- merging diagnostic tracing and domain events into one contract

## Acceptance Conditions

The event manager design is acceptable when all of these are true:

- there is one canonical domain event path for control relevant facts
- reducer ownership of state transitions is explicit
- durable replay can rebuild required runtime projections
- telemetry is downstream of the event spine
- hot path durability cost is governed by explicit policy rather than implicit flush on every event
