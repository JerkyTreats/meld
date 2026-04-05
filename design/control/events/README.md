# Events Design

Date: 2026-04-05
Status: active
Scope: durable event management for control, task execution, replay, and downstream telemetry consumption

## Intent

This area defines the event management design for control.

The design goal is one durable event spine for task execution and future control runtime behavior.
That spine should support:

- ordered domain facts
- reducer driven state projection
- replay after restart
- downstream telemetry consumption
- live views such as TUI or diagnostics

## Document Split

This area is intentionally split into two documents with different purposes.

- [Event Manager Requirements](event_manager_requirements.md)
  declarative requirements for what should be true
- [Telemetry Refactor](telemetry_refactor.md)
  migration path for how to move from the current telemetry centered implementation to the target design

## Boundary

`events` is not the same concern as `telemetry`.

- `events` owns ingress, ordering, reduction boundaries, durability, replay, and subscription semantics
- `control` owns domain event meaning and projection rules
- `telemetry` consumes the event stream for observability, summaries, metrics, and external export

## Read With

- [Control Design](../README.md)
- [Task Network](../task_network.md)
- [Event Management Research](../event_management_research.md)
- [Runtime Model](../runtime/README.md)
