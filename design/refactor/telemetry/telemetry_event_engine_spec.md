# Telemetry Event Engine Spec

Date: 2026-02-17

## Objective

Define a focused extraction for telemetry event emission so CLI execution stops owning session policy and summary mapping.

Related ownership spec: [God Module Detangling Spec](../god_module_detangling_spec.md).

## Scope

This spec covers telemetry event engine behavior.

- session start and finish policy
- session prune trigger policy
- command summary event mapping
- event routing and fanout
- sink integration for TUI and OpenTelemetry

## Out Of Scope

This spec does not redesign event semantics.

- no change to existing event names in this phase
- no change to CLI parse and help surface
- no change to workspace provider or context behavior

## Naming Model

Use domain sharp subdomains instead of generic layer names.

- `contracts` for event schema and summary schema
- `sessions` for session policy and lifecycle
- `emission` for emission orchestration and mapping
- `routing` for bus and ingest fanout
- `sinks` for TUI store and OpenTelemetry exporters

This model answers where logic lands by behavior, not by generic layer label.

## Current Mix Of Concerns

`src/tooling/cli.rs` currently mixes shell route execution with telemetry policy.

- shell concern that should remain: route command execution and return command result
- orchestration concern to move: command session start in `execute`
- orchestration concern to move: command session finish in `execute`
- orchestration concern to move: session prune trigger in `execute`
- orchestration concern to move: command summary emission in `emit_command_summary`
- orchestration concern to move: typed summary event mapping in `typed_summary_event`

## Target Ownership

### Telemetry event engine owns

- session lifecycle policy
- command summary mapping policy
- best effort emission error policy
- event routing and sink fanout policy

### CLI shell owns

- parse and route execution
- output envelope selection for text and json
- translation from telemetry service errors to CLI error surface

### Other domains own

- business logic that decides what work happens
- provider and context use case orchestration

## Concern Landing Map

### Session lifecycle policy

- current shell area: `execute`
- target home: `src/telemetry/sessions/service.rs`

### Summary mapping policy

- current shell area: `emit_command_summary` and `typed_summary_event`
- target home: `src/telemetry/emission/summary_mapper.rs`

### Emission orchestration

- current shell area: handler level emission scaffolding
- target home: `src/telemetry/emission/engine.rs`

### Event routing and ingest

- current area: `src/progress/bus.rs` and `src/progress/ingestor.rs`
- target home: `src/telemetry/routing/bus.rs` and `src/telemetry/routing/ingestor.rs`

### Event contracts

- current area: `src/progress/event.rs`
- target home: `src/telemetry/contracts/event.rs` and `src/telemetry/contracts/summary.rs`

### Sink integrations

- current area: `src/progress/store.rs` and TUI consumer call paths
- target home: `src/telemetry/sinks/store.rs`, `src/telemetry/sinks/tui.rs`, `src/telemetry/sinks/otel.rs`

## Proposed Domain Shape

- `src/telemetry/mod.rs`
- `src/telemetry/facade.rs`
- `src/telemetry/contracts/mod.rs`
- `src/telemetry/contracts/event.rs`
- `src/telemetry/contracts/summary.rs`
- `src/telemetry/sessions/mod.rs`
- `src/telemetry/sessions/service.rs`
- `src/telemetry/sessions/policy.rs`
- `src/telemetry/emission/mod.rs`
- `src/telemetry/emission/engine.rs`
- `src/telemetry/emission/summary_mapper.rs`
- `src/telemetry/routing/mod.rs`
- `src/telemetry/routing/bus.rs`
- `src/telemetry/routing/ingestor.rs`
- `src/telemetry/sinks/mod.rs`
- `src/telemetry/sinks/store.rs`
- `src/telemetry/sinks/tui.rs`
- `src/telemetry/sinks/otel.rs`

## Migration Plan

1. add characterization tests for current session lifecycle and summary behavior
2. introduce telemetry modules with compatibility wrappers in `src/progress`
3. move summary mapping into `src/telemetry/emission/summary_mapper.rs`
4. move lifecycle policy into `src/telemetry/sessions/service.rs`
5. move bus and ingestor into `src/telemetry/routing`
6. move store and external integrations into `src/telemetry/sinks`
7. keep CLI execute path as route and delegate only
8. remove progress named wrappers after parity suite is green

## Test Plan

### Behavior parity coverage

- parity for session started and session ended ordering
- parity for prune trigger behavior
- parity for typed summary event selection by command
- parity for command summary payload truncation behavior

### Boundary coverage

- route tests confirm CLI does not own telemetry lifecycle policy
- guard tests confirm summary mapping is outside CLI handlers
- error mapping tests for telemetry write failures

### Sink coverage

- TUI sink contract tests
- OpenTelemetry sink exporter tests
- persistent sink read write tests

## Acceptance Criteria

- telemetry event engine is owned by `src/telemetry`
- `src/tooling/cli.rs` does not own session lifecycle policy
- `src/tooling/cli.rs` does not own summary mapping policy
- TUI and OpenTelemetry sinks are first class telemetry sinks
- characterization and parity suites pass

## Risks And Mitigation

- risk: event ordering regressions around session boundaries
- mitigation: integration tests for lifecycle ordering and terminal session events

- risk: sink specific failures causing command failure
- mitigation: best effort emission policy with explicit failure handling contract

- risk: naming drift back to generic layers
- mitigation: keep subdomain naming contract in this spec and enforce in reviews

## Deliverables

- telemetry module split under `src/telemetry`
- compatibility wrappers in `src/progress` during migration
- CLI execute wiring that delegates to telemetry services
- parity and boundary tests for session and summary flows
