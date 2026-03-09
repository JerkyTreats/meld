# Capability Contract

Date: 2026-03-09
Status: active

## Intent

Define one capability execution contract that every workflow step can depend on.
This contract is the foundation for reliable orchestration and reliable configuration.

## Primary Questions

- what request shape does every capability receive
- what result shape does every capability return
- how are artifacts and summaries handed to downstream steps
- where does capability registration live
- how does workflow resolve a capability id into one concrete implementation

## Initial Requirements

- each capability has a stable capability id and versioned config schema
- each capability exposes typed input contracts and typed output contracts
- each capability returns structured execution status, artifacts, and telemetry summary data
- each capability declares whether it is target scoped, batch scoped, or workflow scoped
- capability config validation must happen before workflow execution starts

## Design Goal

Make workflow configuration read like capability composition rather than a bag of loosely enforced fields.

## Related Areas

- [Workflow Definition](../workflow_definition/README.md)
- [Ordering Capability](../ordering_capability/README.md)
- [File Write Capability](../file_write_capability/README.md)
- [Telemetry Model](../telemetry_model/README.md)
