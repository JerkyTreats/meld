# Physical Asset Maintenance Router Pressure Specification

Date: 2026-08-18

Status: discovery router exercise aligned to canonical PDS boundary

Scope: fan physical asset maintenance through the proposed `theory::router` contracts while preserving external safety and operational control

## Purpose

Test the router against heterogeneous sensor evidence, long-lived edge and external adapters, asset and work-order systems of record, consequential physical effects, delayed restoration evidence, and strict safety authority.

## Layering

The principal approves asset scope, condition and calibration policy, inspection sensitivity, maintenance autonomy, downtime budget, escalation, and verification. The package lowers this intent into exact owner components. Assignment binds principal, asset scope, operational perspective, and grants. Activation binds telemetry, maintenance, diagnostic, inventory, and notification systems. Measurements, work orders, physical effects, condition beliefs, and outcomes remain runtime or external state.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Owner meaning | Structural requirements |
| --- | --- | --- | --- |
| `asset-maintenance-policy` | `asset-maintenance.policy.v1` | asset identity, units, telemetry classes, inspection, calibration, service, safety, and restoration policy | none |
| `condition-belief` | `world-model.belief-family.v1` | current physical condition under admitted evidence | `asset-maintenance-policy` |
| `failure-risk-belief` | `world-model.belief-family.v1` | prospective failure risk under load and environment | `asset-maintenance-policy` |
| `calibration-belief` | `world-model.belief-family.v1` | calibration confidence and validity | `asset-maintenance-policy` |
| `urgency-belief` | `world-model.belief-family.v1` | maintenance urgency under safety, availability, and cost policy | `asset-maintenance-policy` |
| `freshness-belief` | `world-model.belief-family.v1` | adequacy and recency of condition evidence | `asset-maintenance-policy` |
| `asset-outcomes` | `world-model.outcome-mapping.v1` | admitted inspection, work, calibration, and operating evidence | all belief components and `asset-maintenance-policy` |
| `asset-curation` | `world-model.agent-curation-rule.v1` | inspect, tolerate, request work, isolate, or escalate posture | all belief components |
| `asset-condition` | `world-model.agent-maintained-condition.v1` | protected condition, evidence freshness, recurrence, and restoration semantics | `asset-curation` and all belief components |
| `asset-strategy` | `world-model.strategy-theory.v1` | settlement, prospective evidence, maintenance action, outcome, and constraint meaning | `asset-condition` and `asset-maintenance-policy` |
| `asset-capability-*` | activation capability contribution | exact contracts for telemetry, self-test, inspection, work request, scheduling, or notification | compatible maintenance action classes |
| `asset-authority` | assignment governance selection | requested bounded actions and explicit hazardous-action denials | `asset-maintenance-policy` |

`asset-maintenance.policy.v1` is a candidate domain-owned route. It keeps units, equipment identity, procedure revisions, safety classes, and restoration semantics out of generic router logic.

An activation profile may include reading telemetry and drafting or scheduling approved work. It need not include direct control. An externally governed shutdown request and direct physical actuation are distinct contracts.

## Owner Semantics And Cross Links

The asset-maintenance owner defines asset and component identity, measurement classes, units, operating modes, service procedure revisions, work semantics, and restoration evidence. Device and maintenance adapters preserve source claims. Belief owns exact revisions, Agent owns standing response mechanics, Strategy owns choices, and execution owns contracts, effects, claims, and dispatch fences.

Required public cross-links include:

- belief families cite exact measurement and inspection evidence classes
- calibration belief cites accepted procedure, reference instrument, tolerance, and validity
- maintained condition cites protected safety and availability floors
- Strategy cites exact observation and intervention contracts
- outcome mapping distinguishes administrative closure from physical restoration
- authority policy marks hazardous actuation and safety bypass as absent, denied, or independently gated

The router validates references and closure. It never interprets sensor values, safety thresholds, or maintenance procedures.

## Assignment And Activation

A candidate assignment binds:

```text
exact package receipt
+ owner or delegated maintenance principal
+ exact asset and component scope
+ operating perspective and branch
+ requested authority ref
+ principal grant ref
```

Asset ids from the authoritative asset master are standing semantic bindings. Gateway endpoints, credentials, and site-local protocol addresses belong to activation.

Candidate activation bindings include telemetry historian, edge gateway, maintenance system, vendor diagnostic implementation, inspection service, inventory query, scheduling system, and notification sink. Each contributor receives only owner-declared refs.

Readiness for a persistent connector must prove authenticated source scope, recovered cursor, measurement schema and unit compatibility, admission closed, and a viable durable wake path. Process spawn or protocol connection alone is insufficient.

## Capability And Authority Closure

Every action follows exact capability and authority closure. Scheduling a work order does not authorize the technician, energize equipment, or declare return to service.

A high-consequence action needs more than the common grant chain:

```text
exact externally governed request contract
+ current asset and operating-mode scope
+ approved safety procedure and interlock path
+ principal or operator authorization
+ effect conflict arbitration
+ authoritative execution acknowledgment
+ independent post-effect observation
```

The PDS package cannot supply device credentials, executable paths, endpoint identity, or owner identity. Product or administrator composition supplies implementations.

## Observation And Result Admission

Polled diagnostics and self-tests use request-response lineage with assignment, package receipt, activation generation, participant incarnation where available, exact capability ref, durable operation key, claim, and attempt id.

Telemetry and maintenance-system changes use passive delivery lineage with admitted subscription, source cursor, delivery identity, authentication lineage, assignment, activation generation, and participant incarnation.

The asset owner validates asset scope, sensor or work-order identity, units, calibration basis, operating mode, timestamp quality, source sequence, completeness, and evidence class. A gateway success status or work-order close code does not establish restored condition.

Source correction and late telemetry retain original lineage. Owner policy decides whether they revise historical interpretation, qualify current evidence, or are rejected. A retired connector cannot silently relabel a late measurement as current-generation input.

## Isolation And Lifecycle Pressure

Physical maintenance pressures all isolation axes. Shared gateways and historians need asset namespaces, cursor ownership, queue fairness, source-revision lineage, and failure attribution. Credentials and control networks require binding and network isolation. Concurrent work orders require exact effect identity and conflict arbitration.

A telemetry participant may be justifiably idle while waiting past a durable source cursor. A connector that emits heartbeats but has lost its subscription or wake route is not healthy stewardship.

Assignment-wide generation should change when credentials, endpoints, implementation, authority, or binding meaning changes. A mechanically equivalent connector restart may instead create a new participant incarnation after admission-closed cursor and delivery reconciliation. This split is still a proposed refinement.

Retirement closes admission, fences passive subscriptions and outstanding effects, aggregates owner safe-point receipts, and preserves unresolved work. It cannot prove that a device or remote maintenance system will never emit another callback.

## Interaction With Other Stewards

Potential peers include operations, safety, inventory, energy, scheduling, and compliance stewards. Interaction occurs through explicit public claims and requests such as asset unavailable, inspection required, part unavailable, maintenance window granted, or external safety approval recorded.

No steward may reach into another owner store or infer authority from a recommendation. An operations preference for availability cannot override a protected safety floor. An inventory reservation does not prove a part was installed. Cross-steward conflicts remain explicit and governed.

## Router Falsification Findings

The example falsifies the design if:

- the router interprets units, fault codes, procedures, or safety policy
- an activation-selected capability can bypass external interlocks
- telemetry silence is flattened into healthy state
- a closed work order becomes restored condition without verification
- shared gateway state crosses asset, principal, or assignment scope
- participant restart loses cursor or duplicate-delivery lineage
- an ambiguous physical effect is retried as a fresh semantic operation
- historical explanation depends on a still-running vendor service
- hard real-time control is pulled into the PDS runtime

The example narrows to fixed intervals, thresholds, checklists, and deterministic controllers when those mechanisms provide equivalent safety and maintenance value.

## Theory-Set Implications

### Common Candidate Mechanism

The routed package shape survives without physical-domain branches. Exact receipt closure, assignment and activation separation, passive and active lineage, owner admission, effect reconciliation, readiness, durable wakes, and fenced retirement appear reusable.

This example strengthens the case for generic unit-bearing observation envelopes, source cursor lineage, effect identity, and independently verified outcomes. Their carrier mechanics may be common even though their meanings are owner-specific.

### Owner-Specific Meaning

Asset identity, component topology, units, operating modes, fault interpretation, degradation, calibration validity, maintenance procedures, safety classes, and restoration evidence belong to asset-maintenance and external control contracts.

### Unresolved Bridge

The theory set does not yet settle:

- whether `asset-maintenance.policy.v1` is one route or several owner facets
- how device, asset-master, and work-order identities are reconciled and superseded
- how unit and calibration metadata enter generic admitted evidence without a universal asset ontology
- how external interlock and permit receipts participate in effective authority
- how participant incarnation relates to activation generation
- how passive source fencing composes across edge gateway, historian, and maintenance system
- where reconciliation of ambiguous physical effect belongs when no idempotent external command exists

## Source Trail

- [Use-case narrative](README.md)
- [PDS Router design](../../../completed/integration/pds_router_design_spec.md)
- [Isolation and runtime portability](../../isolation_and_runtime_portability.md)
- [Runtime lifecycle and quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
- [Use-case decomposition](../../use_case_decomposition.md)
- [Compilation layer span](../compilation_layer_span.md)
