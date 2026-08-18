# Physical Asset Maintenance Steward

Date: 2026-08-15

Status: discovery example

Scope: persistent safety, availability, calibration, condition, and serviceability stewardship for assigned physical assets

## Working Intent

Maintain evidence that assigned assets remain inside declared condition, calibration, safety, and availability bounds over an indefinite operating horizon. Choose inspections and bounded maintenance responses under uncertainty while preserving external operational and safety authority.

## Domain Boundary

The bounded domain contains:

- assigned assets, components, consumables, and stable asset identities
- operating hours, cycles, environmental exposure, and degradation questions
- inspections, calibration records, faults, service intervals, work orders, and replacement history
- condition, failure-risk, calibration-confidence, urgency, and evidence-freshness beliefs

The enterprise asset management or maintenance system remains the system of record for the asset master, approved work orders, service records, and schedules within its remit. Telemetry historians and device gateways own delivered measurements and source sequence. Safety controllers, interlocks, lockout systems, and authorized operators retain operational control. Inventory systems own part availability and issue records.

Meld may preserve admitted observations, infer condition, prioritize evidence acquisition, recommend or request work, and evaluate later outcomes. It does not become the authoritative controller, maintenance ledger, calibration certificate issuer, or safety interlock.

Assignments should partition assets or explicitly describe overlap. Two stewards may share telemetry while retaining distinct principals, operational goals, safety policies, and action grants.

## Standing Condition

The candidate maintained condition is:

```text
protected safety and calibration conditions remain inside declared bounds
and maintenance evidence remains sufficiently current
and repeated faults retain recurrence context
and uncertain condition triggers proportionate inspection before avoidable replacement
and restoration claims receive post-service evidence
```

Asset availability may conflict with maintenance urgency. The steward needs declared protected floors and escalation policy rather than one universal optimization target.

## Observations And Evidence

Candidate observations include runtime hours, cycles, sensor readings, inspection reports, fault codes, calibration results, environmental conditions, service actions, part replacement history, work-order status, and later operating behavior.

Admission should preserve:

- asset, component, sensor, and work-order identity
- source system and adapter identity
- measurement unit, calibration basis, and quality flags
- source sequence, event time, receipt time, and valid time
- operating mode and relevant environment
- inspector or service authority where policy permits
- correction, supersession, and missing-data lineage

A device reading is a source observation, not a domain verdict. A completed work order proves administrative completion, not restored condition. Calibration evidence is meaningful only under the exact procedure, instrument, tolerance, and validity window accepted by the owner domain.

Telemetry silence is not healthy operation. It may be planned downtime, connectivity loss, sensor failure, or unknown state. Conflicting sensors should remain visible until an owner-approved comparison or inspection resolves them.

## Observation And Intervention Choices

The steward may choose to:

- request an inspection
- run an approved self-test
- collect a measurement
- inspect maintenance and fault history
- compare degradation against an expected profile
- schedule or recommend maintenance
- request consumable replacement
- request recalibration
- defer low-risk work within policy
- escalate a shutdown or isolation decision

Hazardous physical acts remain external. Even when an activation can submit a request, an authorized control system or person applies interlocks, permits, lockout, and final actuation policy.

## Outcome Semantics

Work dispatch and work-order closure do not prove restoration. Verification may require post-service inspection, calibration, a stable operating window, recurrence-free cycles, or independent sensor evidence.

Outcomes are delayed and confounded by duty cycle, environment, operator behavior, replacement-part quality, service quality, hidden damage, and later faults. Absence of recurrence during low utilization is weaker evidence than stable behavior under representative load.

The steward should preserve the proposed diagnosis, authorized work, external execution report, post-service observations, restored-condition judgment, and attribution uncertainty as separate records.

## Authority And Safety

The asset owner or delegated maintenance authority grants the assignment. Typical authority may allow read-only telemetry, inspection requests, work-order drafting, scheduling inside a window, or consumable replacement requests.

Equipment isolation, energized work, safety bypass, protection-setting changes, destructive tests, emergency shutdown, and return-to-service approval remain externally governed unless an exact high-consequence grant and independent enforcement path exist. A capability implementation or device credential does not itself grant permission.

Safety controls must remain effective if Meld is unavailable, stale, compromised, or wrong. Deterministic interlocks and hard real-time controllers do not move into PDS.

## Why PDS

A maintenance management system, fixed service intervals, checklists, condition thresholds, and deterministic alerts remain the baseline.

PDS is justified when condition is partially observed, evidence is heterogeneous or stale, inspection has meaningful cost, faults recur across long horizons, maintenance actions compete for resources, and restoration needs independent verification. If authoritative thresholds and fixed intervals provide equivalent safety and effectiveness, PDS should defer to them.

## Physical Runtime Variants

The same package could activate through:

- an in-process deterministic measurement normalizer
- a subprocess for vendor diagnostics
- an edge sidecar shared by several assets
- a remote maintenance or analytics service
- persistent telemetry, historian, or maintenance-system connectors

An edge boundary may improve latency while weakening central visibility. A remote service may improve failure containment while sharing hidden tenant state. Every variant must preserve asset and assignment scope, credentials, source cursors, measurement units, runtime lineage, effect identity, and result admission.

Long-lived connectors must be able to become justifiably idle with durable wake paths. On retirement they close admission, fence subscriptions, preserve safe-point receipts, and treat later device or webhook delivery as retired lineage rather than current truth.

## Explicit Non Commitments

This example does not commit:

- hard real-time control through Meld
- replacement of safety controllers or interlocks
- autonomous hazardous actuation
- one asset ontology or degradation model
- one maintenance platform or telemetry protocol
- a final profile syntax
- a universal runtime topology
- work-order closure as outcome success
- causal certainty about a service action and later reliability

The companion route map remains proposed and may expose missing owner contracts.

## Read With

- [Router pressure specification](router_spec.md)
- [Expression catalog](../pds_expression_catalog.md)
- [Compilation layer span](../compilation_layer_span.md)
- [Use-case decomposition](../../use_case_decomposition.md)
- [PDS Router design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation and runtime portability](../../isolation_and_runtime_portability.md)
- [Runtime lifecycle and quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
