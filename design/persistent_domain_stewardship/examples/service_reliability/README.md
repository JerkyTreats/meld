# Service Reliability Stewardship

Date: 2026-08-15
Status: discovery example
Scope: persistent stewardship of service objectives, recovery readiness, and bounded operational remediation

## Working Model

A service reliability steward maintains an evidence-grounded view of service objectives, dependency health, capacity margin, recovery readiness, and recurrence risk across recurring operational incidents.

It operates at the minutes-to-months horizon. Hard real-time protection, circuit breakers, request admission, and millisecond-scale fail-safe behavior remain deterministic systems outside the steward.

This example pressures continuous passive observation, competing causal hypotheses, high-blast-radius authority, ambiguous external effects, delayed recurrence outcomes, and durable wake viability.

## Three Separate Truths

The design must not collapse these states:

```text
subject health
    whether the assigned service satisfies its reliability policy

steward semantic liveness
    whether standing obligations have viable observation and wake paths

runtime operational health
    whether the Meld process and participant incarnations hold leases and report
```

A healthy Meld process can steward a breached service. A healthy service does not prove that its steward can wake on the next incident. A restarted telemetry participant does not by itself change the service assessment.

## Domain Boundary

The service-reliability domain candidate owns:

- service, dependency, region, deployment, and incident vocabulary
- SLO, error-budget, capacity, and recovery policy
- fault-hypothesis and recurrence semantics
- diagnostic and remediation result admission
- reliability assessment, restoration, and recurrence products
- blast-radius and verification meaning for reliability actions

Telemetry producers own raw metric, trace, log, and synthetic products. Deployment and infrastructure domains own mutation effects and receipts. Execution owns claims, effect arbitration, and authority enforcement. Theory owns inert package routing.

The reliability steward does not become the service control plane or the source of raw operational truth.

## Subjects And Scope

Primary subjects include:

```text
service and endpoint
deployment and version
dependency and region
resource and capacity pool
SLO and error budget
incident and fault hypothesis
runbook and recovery procedure
remediation and recurrence window
```

Assignment scope names exact services, regions, dependencies, and policy profiles. Topology discovery may produce candidate relationships, but it does not silently expand operational authority.

## Standing Conditions

Representative conditions include:

```text
SLO risk remains within declared policy
capacity margin remains adequate for declared demand uncertainty
recovery evidence remains current
critical dependency uncertainty remains below its threshold
recurring failures retain causal and remediation history
high-risk intervention waits for sufficient diagnosis when time permits
```

Emergency policy may permit action under weak diagnosis. That is an explicit authority and risk posture, not an inference from urgency.

## Observations And Evidence

Useful evidence includes:

- metrics with exact source, labels, windows, and completeness
- traces and structured failure events
- logs admitted through typed interpretation
- deployment, configuration, and feature-flag changes
- dependency and regional health
- synthetic checks and bounded recovery tests
- capacity, saturation, and queue behavior
- incident commands and effect receipts
- later SLO restoration and recurrence windows
- human incident assessment and accepted root-cause revision

Transport success, command acceptance, and a temporary metric improvement do not by themselves prove restoration.

## Beliefs

Candidate belief families include:

```text
reliability.slo_risk
reliability.fault_hypothesis
reliability.capacity_risk
reliability.recovery_readiness
reliability.recurrence_risk
action.cost
action.blast_radius
```

Fault hypotheses may conflict while sharing evidence. They retain provenance and revision history rather than collapsing into one hidden diagnosis.

## Observation Choices

The steward may:

- run a targeted diagnostic
- collect or compare a trace
- compare deployment and configuration revisions
- inspect dependency or regional health
- acquire higher-resolution telemetry for a bounded window
- trigger a bounded synthetic or recovery test
- wait for discriminating evidence when policy permits
- reconcile an ambiguous prior operation before retry

Observation cost and incident urgency shape method choice without becoming router semantics.

## Actions

Candidate interventions include:

- scale a bounded resource
- restart a bounded component
- disable a feature
- roll back a deployment
- fail over under explicit policy
- isolate a dependency path
- propose a durable code or configuration repair
- escalate to an operator

Each action has exact effect scope, expected blast radius, reversibility, and verification obligations. An external operations platform may implement several actions, but activation exposes only exact selected contracts.

## Outcomes

An accepted command receipt proves only that an operational request was accepted. Later observations establish whether service risk entered its restore region, protected dependencies remained healthy, and recurrence risk changed.

An episode may remain open after immediate SLO recovery while recurrence verification is pending. A later relapse revises the prior action outcome rather than erasing the original effect history.

## Authority

Read-only diagnostics, bounded synthetic checks, component restart, scaling, rollback, failover, feature disablement, and destructive intervention are separate grants.

High-blast-radius actions require explicit operational authority or approval. Hard safety controls remain deterministic and may deny or preempt the steward regardless of package intent.

Two principals observing the same service may share telemetry products while retaining different action grants, incident policies, budgets, and perspectives.

## Why PDS

The simpler baseline is alerts, autoscaling, deterministic remediation, incident workflows, and runbooks. It remains preferable for known signals with one safe fixed response.

PDS earns its cost when reliability requires selective diagnostics, competing hypotheses, action-cost and blast-radius reasoning, recurring incident memory, adaptive recovery, explicit abstention, and later outcome attribution.

It should collapse back toward simpler controllers when those capabilities do not improve restoration time, recurrence, unnecessary intervention rate, or operator trust.

## Physical Runtime Variants

| Placement | Useful behavior | Required proof |
| --- | --- | --- |
| linked local adapter | deterministic policy and telemetry transformation | source scope, resource bound, result admission |
| owned subprocess | diagnostic or analysis tool | executable identity, network and secret grants, termination |
| shared sidecar | telemetry query or topology service | tenant namespace, query budget, cache revision, crash sharing |
| remote request service | observability or control-plane API | endpoint identity, credential scope, operation correlation |
| persistent external controller | continuous telemetry and operational callbacks | subscription, cursor, command authority, callback admission |

The package may remain identical across equivalent placements. Activation binds exact implementations and isolation claims.

## Interaction With Other Stewards

A dependency-security violation may support a reliability fault hypothesis or mitigation choice. An incident may provide evidence to codebase-quality and dependency-security assignments. A reliability-authorized rollback or config change may wake docs freshness and codebase quality.

These interactions flow through canonical facts. The reliability steward does not command developer, security, or docs stewards and does not grant their authority.

## Explicit Non Commitments

This example does not approve:

- autonomous high-blast-radius remediation
- one SLO or incident ontology
- a generic circuit-breaker runtime
- replacement of deterministic safety controls
- one telemetry or control-plane provider
- service health inferred from process health
- semantic liveness inferred from heartbeat success
- a final lifecycle or package schema

## Router Exercise

The package fanout and theory pressure are developed in [Router Specification](router_spec.md).

## Source Grounding

- [Service Reliability Catalog Entry](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [Software Quality Stewardship](../software_quality.md)
- [PDS Router Design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Activation Lifecycle Fanout](../../../plan/integration/pds_activation_lifecycle_fanout_exercise.md)
- [Runtime Lifecycle And Quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
