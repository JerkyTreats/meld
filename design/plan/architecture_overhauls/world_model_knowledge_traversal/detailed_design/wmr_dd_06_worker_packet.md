# WMR-DD-06 Worker Packet

Date: 2026-08-22

Slice: `WMR-DD-06`

Status: active

Implementation authorization: none

## Objective

Close the activation-local lifecycle vertical from one accepted inert prepared closure through exact participant realization, owner readiness, current publication, order-independent work and waiting, interrupted recovery, generation replacement, fenced quiescence, and retirement.

## Product Boundary

```text
prepared activation closure
-> lifecycle acceptance and preparing generation
-> exact participant realization and incarnation
-> native-owner readiness
-> conditional current publication and admission
-> owner work or durable wait with viable wake
-> interruption or replacement fence
-> owner drain and safe points
-> fenced quiescence and retired generation
```

## Required Inputs

- accepted [WMR-DG-05 receipt](../delivery_gates/wmr_dg_05_acceptance_receipt.md)
- [runtime initialization lifecycle assessment](../runtime_initialization_lifecycle/runtime_initialization_lifecycle_assessment.md)
- [runtime lifecycle and supervision review](../runtime_initialization_lifecycle/reviews/runtime_lifecycle_and_supervision_review.md)
- [producer-consumer connectivity review](../runtime_initialization_lifecycle/reviews/producer_consumer_connectivity_review.md)
- `WMR-H22` through `WMR-H25` and every accepted owner position they aggregate

## Required Outputs

- exact lifecycle authority and identity chain
- participant-plan to runtime-registration parity position
- native-owner readiness, checkpoint, wait, wake, safe-point, and stop receipts
- current publication, admission, interruption, recovery, replacement, and retirement rules
- exact distinction among operational health, active idle, quiescent, stalled, and interrupted
- crash and restart reconstruction from every lifecycle transition
- fresh activation, lag, missed wake, uncertain effect, replacement, late delivery, and retirement traces
- frozen `WMR-DG-06` definition

## Stop Conditions

- root interprets owner semantic products or owns Execution, Curation, Belief, or source outcome meaning
- participant order becomes causal correctness or semantic scheduling
- an opaque receipt reference, process handle, heartbeat, clean tick, empty queue, or polling stands in for owner readiness or wake closure
- two assignment heads or unrelated registration and participant sets remain simultaneous authorities
- current publication and admission can expose different generation truth
- retirement ignores unresolved effects, passive delivery, owner checkpoints, or safe points
- a storage engine, schema, wire protocol, service topology, crate, migration, or source change is selected

## Review Contract

Integrated review and Gate Acceptance use distinct read-only subagents against frozen candidate digests. Reviewers may identify only active-slice coherence defects and may not design implementation topology, choose persistence, amend the gate, or authorize `WMR-DD-07`.
