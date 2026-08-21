# Events

Events is Meld's neutral durable append and replay spine. It gives domains one ordered history for facts that must survive process boundaries and restart.

An Event envelope carries identity, producer, domain, event type, schema identity, causal lineage, activation generation, timestamp, and payload bytes. The producing domain owns payload meaning. Events does not impose Goal, Strategy, Curation, or Execution grammar.

## Authority

Append authority is explicit. A producer may append only event families granted by its owner route. Consumers advance durable cursors only after their owned transition commits.

## Replay

Replay is the ordinary recovery mechanism. Equal histories and equal consumer revisions produce equal domain state. Consumers remain responsible for idempotency and schema interpretation.

## Connectivity

Events can wake graph admission, belief settlement, Agent reconciliation, Curation result handling, Execution progression, product projections, and operator views. A wake signals available durable work. It does not declare the work semantically complete.

## Boundaries

Events does not own graph currentness, evidence admission, Goal satisfaction, Task state, activation readiness, or telemetry interpretation. Those facts are published and consumed through explicit owner contracts.

- [Multi-Domain Spine](multi_domain_spine.md)
