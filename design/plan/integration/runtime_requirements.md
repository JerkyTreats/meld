# Runtime Requirements Index

Date: 2026-06-17
Status: domain requirement index with superseded vertical support links
Scope: detailed runtime requirement map for the durable flywheel

## Purpose

This document links the detailed runtime requirements for the first durable flywheel and records the shared boundary rules they must all preserve.

Current runtime completion authority is [Runtime Completion Ground Map](runtime_completion_ground_map.md), followed by [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md). Those documents replace the earlier one-turn proof, activation-heavy configuration, synthetic patch, and production-closure assumptions. Domain requirement documents remain useful contract evidence where they do not conflict with the current completion authority.

Update 2026-08-12: the runtime-completion ground map closed with the strategy landing. Current forward authority is [Theory Elevation Program](theory_elevation_program.md).

The product flywheel remains:

```text
world model runtime
-> execution runtime
-> event runtime
-> world model runtime
```

Root `meld` assembles stores and ports, starts runtime handles, supervises lifecycle, coordinates shutdown, flushes storage, and reports diagnostics. Root `meld` does not own semantic progress inside the flywheel.

## Requirement Documents

| Runtime Area | Requirement Document | Owner |
| --- | --- | --- |
| world model runtime | [World Model Runtime Requirements](world_model_runtime_requirements.md) | `meld-world-model` |
| execution runtime | [Execution Runtime Requirements](execution_runtime_requirements.md) | `meld-execution` |
| event runtime | [Event Runtime Requirements](event_runtime_requirements.md) | `meld-events` |
| root supervisor runtime | [Supervisor Runtime Requirements](supervisor_runtime_requirements.md) | root `meld` |

## Vertical Support Documents

| Concern | Requirement Document | Owner |
| --- | --- | --- |
| current runtime completion | [Runtime Completion Ground Map](runtime_completion_ground_map.md) | integration |
| current implementation workstreams | [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) | integration |
| product runtime assembly | [Product Runtime Assembly Requirements](product_runtime_assembly_requirements.md) | root `meld` |
| historical vertical proof | [Durable Flywheel Vertical Proof Requirements](durable_flywheel_vertical_proof_requirements.md) | integration |
| historical physical config proposal | [Docs Freshness Physical Configuration Requirements](docs_freshness_physical_configuration_requirements.md) | integration |
| historical next iteration proposal | [Docs Freshness Flywheel Next Iteration Report](docs_freshness_flywheel_next_iteration_report.md) | integration |
| phase detail audit | [Runtime Phase Design Detail Audit](runtime_phase_design_detail_audit.md) | integration |

## Shared Commitments

- Runtime behavior lives in the owning crate or root supervisor domain.
- Cross domain momentum passes through explicit ports and durable stores.
- Root assembly wires direct handoff ports once and does not mediate every semantic transition.
- Supervisor records are operational lifecycle records, not semantic progress records.
- Each runtime owns its own cursors, journals, leases, revisions, or lifecycle state according to its domain meaning.
- Bounded worker reports are diagnostics and must not become correctness state.
- Restart resumes from domain stores, not from supervisor memory.
- Deterministic provider, clock, and observation fixtures may enter through real product assembly, but proof fixtures must not call semantic handoffs directly after startup.

## Direct Handoff Map

| Handoff | Producer Runtime | Consumer Runtime | Durable Boundary |
| --- | --- | --- | --- |
| curated goal command | world model agent goal curation | execution goal set | execution goal API |
| active goal planning | execution planning | world model planner projection | projection query port |
| task network mutation | execution planning | execution task network command | task network command API |
| task outcome | execution task dispatch | execution task network command | fenced task outcome command |
| execution fact publication | execution publication | event append | idempotent event append API |
| event replay | event replay | world model graph and evidence ingestion | bounded replay source |
| satisfaction mutation | world model satisfaction curation | execution goal set | execution goal mutation API |

## Supervisor Boundary

The supervisor owns:

- runtime ids
- desired runtime set
- runtime instances
- leases
- heartbeats
- health snapshots
- shutdown state
- restart records
- operator status
- flush coordination

The supervisor does not own:

- event sequence
- graph replay cursor
- belief evidence cursor
- belief view
- agent delivery cursor
- curation decision
- goal lifecycle
- planning result
- task network revision
- dispatch claim
- task outcome
- publication state
- satisfaction decision

## Implementation Use

Use [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) as the current implementation sequence. The earlier domain-by-domain sequence is historical and does not direct the operational-parity workstream.

## Acceptance

The detailed requirements are consistent when:

- every domain handoff goes through a named port
- every durable cursor remains in the owning domain store
- supervisor lifecycle records never replace domain progress
- execution publishes outcomes into events
- world model consumes outcomes from events
- world model submits goal and satisfaction commands into execution
- proof fixtures do not call semantic handoffs after startup and can be deleted without changing product runtime semantics
