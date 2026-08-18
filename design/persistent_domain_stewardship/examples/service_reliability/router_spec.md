# Service Reliability Router Specification

Date: 2026-08-18
Status: discovery router exercise aligned to canonical PDS boundary
Scope: fan service reliability through `theory::router` and record pressure on the common theory set

## Purpose

This exercise tests whether service reliability can use the same inert package router while depending on continuous telemetry, durable external operations, high-risk authority, delayed verification, and long-lived runtime participants.

It also tests whether lifecycle mechanics remain separate from the health semantics of the service being stewarded.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Installing owner | Owner meaning |
| --- | --- | --- | --- |
| `reliability-policy` | `service-reliability.policy.v1` | service-reliability domain candidate | SLO, recovery, diagnostic, remediation, and verification rules |
| `slo-risk-belief` | `world-model.belief-family.v1` | belief domain | current bounded SLO risk |
| `fault-hypothesis-belief` | `world-model.belief-family.v1` | belief domain | competing causal support |
| `capacity-risk-belief` | `world-model.belief-family.v1` | belief domain | capacity margin and demand uncertainty |
| `recovery-readiness-belief` | `world-model.belief-family.v1` | belief domain | current recovery evidence |
| `recurrence-risk-belief` | `world-model.belief-family.v1` | belief domain | later relapse risk |
| `reliability-outcomes` | `world-model.outcome-mapping.v1` | belief domain | admitted operational products as typed evidence |
| `reliability-curation` | `world-model.agent-curation-rule.v1` | Agent domain | incident sensitivity and uncertainty response |
| `reliability-condition` | `world-model.agent-maintained-condition.v1` | Agent domain | protected service and recovery conditions |
| `reliability-strategy` | `world-model.strategy-theory.v1` | Strategy domain | settlement, prospective evidence, reliability action, outcome, and constraint meaning |
| each atomic diagnostic or action | activation capability contribution | capability domain | one exact observe, diagnose, scale, restart, rollback, failover, or verify behavior |
| `reliability-authority` | assignment governance selection | authority domain | requested observation and operational action posture |

`service-reliability.policy.v1` is a candidate owner route, not a settled router route. Telemetry source schemas, topology, current SLO windows, incidents, and runtime credentials remain outside the package body.

## Router-Owned Semantics

The router owns exact package structure, route lookup, owner dispatch, structural requirement closure, receipts, and historical exact resolution.

It does not interpret SLOs, metrics, incidents, fault hypotheses, topology, blast radius, operational commands, or restoration. It never contacts telemetry or a service control plane during installation.

Router success proves inert semantic closure only. It says nothing about service health, observer readiness, or the availability of a safe remediation path.

## Owner Semantics And Cross Links

The candidate reliability policy owner validates service objectives, evidence windows, diagnostic posture, action constraints, recovery readiness, and verification rules.

Belief owners install separate risk and hypothesis families. Agent owns curation, maintained conditions, and construction policy. The Strategy route owns settlement and abstract reliability action meaning. Strategy separately owns Method admission and current candidates. Capability and execution own exact contracts, authority enforcement, claims, and effect arbitration.

Expected links include:

```text
reliability assessment
→ exact telemetry products, deployment facts, topology revision, and policy

outcome mapping
→ exact reliability product schemas and belief families

maintained condition
→ exact risk, readiness, and recurrence families

strategy theory
→ exact diagnostic, action, and verification capabilities

activation contribution
→ exact policy, subscriptions, implementations, and bindings
```

The router checks structural presence. Owners validate semantic compatibility.

## Assignment And Activation

One assignment binds an exact package receipt to one principal and exact service scope. It may include several regions and dependencies without granting authority outside the declared operational boundary.

Potential owner-scoped bindings include:

```text
telemetry sources
service topology source
deployment and configuration history
diagnostic runtime
service control plane
runbook and recovery-test catalog
incident and escalation target
```

Preparation creates exact policy refs, participant plans, telemetry subscriptions and cursors, capability selectors, authority closure inputs, admission policies, and inert invoker drafts. It performs no live remediation.

Realization starts participant incarnations with admission closed. Readiness proves authenticated telemetry and control-plane identity, source scope, cursor recovery, and exact implementation identity before the assignment activation generation becomes current.

## Activation Generation And Participant Incarnation

Service reliability exposes why assignment-wide activation generation and participant incarnation must differ.

An implementation, binding, authority, package, isolation realization, or selected catalog change advances the activation generation. A telemetry client process reconstructed with the same exact realization may create a new participant incarnation inside the same generation after admission-closed recovery.

Late results validate both identities. One flaky telemetry participant should not force unrelated Agent or execution participants into a new semantic realization.

The exact incarnation carrier and durable owner remain unresolved.

## Capability And Authority Closure

Atomic contracts preserve the gap between technical availability and authorized behavior.

```text
selected exact capability
∩ compatible implementation offer
∩ assignment request
∩ principal grant
∩ runtime and safety restriction
→ activation-local invoker
```

An external operations product may implement query, restart, scale, rollback, failover, and feature disablement. Selecting telemetry query must not make any mutation reachable.

Effects name exact service, region, deployment, resource, and mutation class. Deterministic safety systems may reject dispatch even when PDS authority is otherwise sufficient.

## Passive Observation Admission

Metrics, traces, log interpretations, deployment notifications, dependency health, and synthetic results arrive through source subscriptions or admitted domain products.

Passive deliveries carry assignment subscription, activation generation, participant incarnation, adapter identity, delivery id, cursor or watermark, source revision, subject scope, observation time, and completeness.

High-rate raw telemetry need not become one canonical event per sample. The owning telemetry or reliability adapter may admit bounded summary products when exact aggregation policy, window, source, completeness, and revision remain visible.

That aggregation policy is owner meaning. It is not router grammar.

## Operation And Result Admission

Long diagnostics and operational actions use durable operations outside a synchronous supervisor tick.

```text
bounded dispatch
→ persist operation, attempt, generation, incarnation, effect, and authority lineage
→ hand request to adapter

later completion
→ resolve exact operation and attempt
→ reconcile effect when response is ambiguous
→ owner validates result
→ append admitted domain product
```

Retry after ambiguous transport preserves the durable operation key and creates a new attempt, or first runs an action-specific reconciliation. Lack of response never proves lack of effect.

Command receipt, effect observation, service assessment, and recurrence outcome remain separate products.

## Isolation Pressure

Two assignments may observe the same service through one telemetry platform while retaining distinct policy revisions, principals, grants, query budgets, current hypotheses, action catalogs, subscriptions, cursors, and outcomes.

Credentials are owner-scoped binding handles. A shared topology cache retains exact source revision and cannot become assignment truth. High-rate observation cannot starve remediation verification without explicit resource policy. Operational effects use exact conflict identity.

A remote control plane provides host separation but not automatically tenant, credential, authority, callback, or admission isolation.

## Lifecycle And Quiescence Pressure

Operational health, service health, and steward liveness remain separate projections.

Active idle means only that the last bounded step found no work. Justified idle additionally requires complete expected participation and a viable wake route for every standing obligation. A missing telemetry subscription can make the steward stalled even while every process heartbeat is healthy.

Recovery keeps admission closed while it reconciles exact activation generation, participant incarnations, cursors, open operations, prior effects, and readiness. Handle reconstruction is not recovery closure.

Fenced quiescence aggregates exact owner receipts:

```text
admission fence
+ owner safe points
+ unresolved operation summary
+ passive subscription fences
+ participant stop receipts
→ aggregate quiescence receipt
```

This is not a distributed store snapshot and cannot prove that no remote callback will arrive. It proves that a late delivery retains retired lineage and cannot gain current admission silently.

## Wake Pressure

Continuous stewardship needs explicit viable wake contracts. Candidate common carrier classes may include event past cursor, owner record revision, durable deadline, passive subscription delivery, and explicit operator action.

The carrier vocabulary is candidate and unresolved. Reliability owns the meaning of which metric window, incident, dependency change, or deadline satisfies a wait. Runtime may verify structural wake viability without interpreting that meaning.

## Interaction With Other Stewards

Cooperation remains event mediated:

```text
security assessment
→ possible reliability hypothesis evidence

service incident and admitted diagnosis
→ possible codebase-quality or developer evidence

independently authorized service or code change
→ new telemetry and deployment facts

verified service outcome
→ reliability restoration or recurrence evidence
```

No producer commands the consumer or confers authority.

## Router Falsification Findings

Reject or narrow the router and activation model if:

- router installation must contact telemetry or interpret an SLO
- process heartbeat is required to stand for service or semantic health
- passive telemetry must fabricate capability attempts
- long operations must remain inside one bounded supervisor tick
- participant restart always invalidates the whole activation generation
- a command receipt can directly prove reliability restoration
- quiescence requires one simultaneous cross-store flush
- two assignments sharing telemetry must share authority or hypotheses
- root lifecycle code must understand reliability wait conditions

The strongest finding is that runtime coordination may compare participant, operation, and wake closure, but domain owners must retain the meaning of health, waiting, safe point, and result validity.

## Theory-Set Implications

### Common Candidate Mechanism

The example supports exact routed components, owner-scoped activation, atomic capabilities, effective authority closure, durable operation and attempt lineage, passive delivery lineage, participant incarnation, owner readiness and safe-point receipts, and structural wake viability.

Participant incarnation, wake carrier, and aggregate quiescence are candidate runtime mechanisms. They are not settled `theory::router` routes.

### Owner-Specific Meaning

SLO windows, telemetry aggregation, topology, causal hypotheses, capacity risk, recovery readiness, blast radius, remediation verification, and recurrence remain reliability semantics.

The common theory set carries exact owner references and admitted facts without evaluating them.

### Unresolved Bridge

Open bridges include:

- the owner and schema of `service-reliability.policy.v1`
- durable high-rate telemetry aggregation and bounded canonical products
- participant incarnation ownership and late-result checks
- extensible wake resolution without root domain matching
- semantic time and delayed recurrence episodes
- safe coordination between PDS authority and deterministic operational controllers
- topology discovery without silent scope or authority expansion

These remain discovery questions.

## Non Commitments

This exercise does not approve route schemas, SLO representation, telemetry providers, automatic remediation, wake contracts, lifecycle ports, or runtime topology.

## Source Grounding

- [Use-Case Definition](README.md)
- [PDS Router Design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Activation Lifecycle Fanout](../../../plan/integration/pds_activation_lifecycle_fanout_exercise.md)
- [Runtime Lifecycle And Quiescence](../../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
