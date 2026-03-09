# Agent Chaining

Date: 2026-03-09
Status: active

## Intent

Define how workflows coordinate agent usage across multiple capabilities.

## Primary Questions

- should workflow steps name agents directly
- should capabilities bind agents internally through profiles
- how should agent selection work when one workflow mixes ordering, generation, review, and publish steps
- how should agent identity appear in workflow configuration and telemetry

## Initial Requirements

- support workflows that chain multiple capabilities with one or more agents
- keep agent selection explicit enough for debugging and policy checks
- avoid forcing every workflow config to repeat low value agent wiring
- allow capability profiles to provide safe defaults where that improves simplicity
- keep role based authorization clear across chained steps

## Related Areas

- [Capability Contract](../capability_contract/README.md)
- [Telemetry Model](../telemetry_model/README.md)
- [Migration Plan](../migration_plan/README.md)
