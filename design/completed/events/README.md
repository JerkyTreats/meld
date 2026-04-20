# Completed Events

Date: 2026-04-20
Status: completed implementation archive
Scope: completed event spine extraction and refactor history

## Purpose

This directory holds implementation history for extracting canonical event ownership from `telemetry` into `events`.

Active declarative design remains under [Events Design](../../cognitive_architecture/events/README.md).

## Completed Baseline

The completed events slice landed:

- canonical `events` domain ownership
- runtime-wide event sequence
- append-only spine history
- legacy event read compatibility
- idempotent derived fact append through `record_id`
- graph objects and relations on canonical events
- telemetry as downstream compatibility and reporting
- session lifecycle separated from canonical event history

## Documents

- [Event Extraction Plan](PLAN.md)
  completed phased extraction tracker
- [Event Domain Extraction Spec](event_domain_extraction_spec.md)
  historical extraction boundary and migration spec
- [Event Spine Refactor](telemetry_refactor.md)
  historical refactor plan from telemetry-centered storage to spine ownership
- [Event Management Research](research.md)
  research notes that motivated the event spine

## Active Design

- [Events Design](../../cognitive_architecture/events/README.md)
- [Event Spine Requirements](../../cognitive_architecture/events/event_manager_requirements.md)
- [Multi-Domain Spine](../../cognitive_architecture/events/multi_domain_spine.md)
