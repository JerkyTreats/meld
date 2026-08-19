# Dependency Security Router Specification

Date: 2026-08-18
Status: discovery router exercise aligned to canonical PDS boundary
Scope: fan dependency security through `theory::router` and record pressure on the common theory set

## Purpose

This exercise asks whether dependency security can attach one state-free semantic package while its physical implementation ranges from a local scanner to a persistent external monitor.

It also tests whether the router can preserve exact package closure without interpreting security bodies, advisory sources, runtime cursors, or results.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Installing owner | Owner meaning |
| --- | --- | --- | --- |
| `dependency-policy` | `dependency-security.policy.v1` | dependency-security domain | source set, coverage, currency, severity, applicability, and verification rules |
| `security-coverage-belief` | `world-model.belief-family.v1` | belief domain | adequacy of current inventory and advisory knowledge |
| `security-posture-belief` | `world-model.belief-family.v1` | belief domain | bounded dependency-security posture |
| `security-outcomes` | `world-model.outcome-mapping.v1` | belief domain | admitted security products as typed evidence |
| `security-curation` | `world-model.agent-curation-rule.v1` | Agent domain | response to uncertainty, breach, and source conflict |
| `security-condition` | `world-model.agent-maintained-condition.v1` | Agent domain | adequate coverage and permitted posture |
| `security-strategy` | `world-model.strategy-theory.v1` | Strategy domain | settlement, prospective evidence, action-class, outcome, and constraint meaning |
| `security-capability-inventory` | activation capability contribution | capability domain | observe exact dependency inventory |
| `security-capability-advisories` | activation capability contribution | capability domain | acquire exact advisory knowledge |
| `security-capability-assess` | activation capability contribution | capability domain | produce bounded assessment |
| `security-capability-feasibility` | activation capability contribution | capability domain | evaluate candidate remediation graph |
| `security-capability-verify` | activation capability contribution | capability domain | produce post-change resolution assessment |
| `security-authority` | assignment governance selection | authority domain | requested acquisition, scan, assessment, and verification posture |

The package does not contain endpoints, credentials, executable paths, current advisories, inventory, findings, cursors, exceptions, goals, or tasks.

## Router-Owned Semantics

The common router owns route addressing, structural dependency closure, owner dispatch, generic exact revision references, package receipt closure, and historical exact resolution.

It does not understand:

- package ecosystems or advisory formats
- coverage or currency policy
- applicability and exploitability
- scanner transport and readiness
- component discovery from inventory
- bounded negative assessment
- risk exception meaning
- result or callback admission

Adding any of these meanings to the router would turn one consumer into common theory grammar.

## Owner Semantics And Cross Links

The dependency-security route validates and installs a state-free policy revision. It owns advisory source requirements, ecosystem coverage, severity policy, currency rules, applicability posture, remediation policy, and verification policy.

World-model owners install belief, outcome, curation, condition, and Strategy semantic components. Capability providers publish exact atomic contracts independently. Assignment and execution resolve requested and effective authority.

Expected exact cross links include:

```text
security assessment
→ exact inventory snapshot, advisory snapshot, and policy revision

outcome mapping
→ exact security product schemas and belief families

maintained condition
→ exact coverage and posture families

strategy theory
→ abstract security action and outcome meaning

activation contribution
→ exact policy, compatible capability contracts, and owner bindings
```

Structural closure proves that named components exist. Each destination owner validates the semantic references in its body.

## Assignment And Activation

One assignment binds an exact package receipt to one principal and bounded software or infrastructure scope. Component subjects derived from inventory retain that assignment lineage.

Activation derives requirements from selected policy, abstract action classes, and compatible capability offers. Potential owner-scoped bindings include:

```text
workspace or image source
security runtime
advisory source credentials
resolver executable
verification profile
repository host only when selected
```

Preparation creates exact policy refs, subject derivation state, durable advisory and workspace cursors, capability selectors, admission policy, and draft invokers. No scanner, sidecar, or remote watcher becomes live during preparation.

The activation generation starts the selected implementation through expected participants, proves exact adapter and source identity, and publishes current status only after readiness closure. A spawned process with an unknown source database is not ready.

## Capability And Authority Closure

Each security behavior is an atomic exact capability contract even when one external product implements several behaviors.

```text
required action-class semantics
∩ activation contract offer
∩ assignment request
∩ principal grant
∩ runtime restriction
→ activation-local invoker
```

The security contributor cannot expose manifest mutation, pull-request creation, deployment, or exception approval through an observation or assessment contract. Those effects require their own owner route and authority.

Provider-free or model-free activation remains valid when the selected contracts do not require such a provider.

## Request Result Admission

Every request-response result carries at least:

```text
assignment and package receipt
activation and activation generation
exact security policy revision
capability contract
durable operation key
attempt id
participant incarnation and adapter identity
observed scope
external source revisions
completeness
typed value
```

The durable operation key remains stable across retry. Attempt, activation generation, and participant incarnation identify one physical realization. An ambiguous timeout retains unresolved effect truth and either reuses the operation key under a new attempt or performs capability-specific reconciliation.

The dependency-security adapter rejects mismatched scope, policy, source, generation, operation, attempt, adapter, or completeness before canonical append.

## Passive Source Admission

A persistent monitor delivery is not a capability invocation. Its envelope carries assignment subscription, package receipt, exact policy, retired or current activation generation, adapter identity, delivery id, cursor, source revision, and authenticated payload.

The adapter validates callback authenticity and monotonic source lineage before publishing a source-advance product. Source advance creates an observation opportunity. It does not directly set a belief to clean or violated.

A shared vendor cursor cannot replace assignment-scoped Meld lineage. A callback from a retired generation is rejected or historical-only under explicit owner policy.

## Domain Product Admission

The security domain admits distinct products:

| Product | Domain assertion |
| --- | --- |
| inventory snapshot | exact components observed in one bounded scope |
| advisory snapshot | exact source knowledge and coverage acquired |
| security assessment | policy interpretation over exact evidence |
| actionable violation | current assessment identifies bounded actionable risk |
| remediation feasibility | candidate dependency graph posture |
| resolution assessment | later evidence supports restored or unresolved posture |

One product never substitutes for another merely because the same tool produced both.

## Isolation Pressure

Dependency security requires the full isolation vector to remain explicit:

- semantic isolation between exact policy revisions
- normative isolation between principals and assignments
- binding isolation for workspace, source, and credentials
- state isolation for inventories, cursors, and current assessments
- failure isolation for shared scanner or sidecar outages
- resource isolation for scans, network, and advisory rate budgets
- effect isolation for resolver or repository actions
- admission isolation for external responses and callbacks
- replay isolation for historical source and policy lineage

Process placement proves only some axes. A remote multi-tenant service may have strong host separation but weak tenant or cursor separation. A local subprocess may isolate crashes while sharing broad credentials or filesystem access.

Required hard filesystem or network isolation must fail closed when an offered linked implementation provides only cooperative enforcement.

## Lifecycle Pressure

Preparation and live readiness are distinct. The scanner or watcher may outlive one Meld tick and may advance sources while the assignment is otherwise idle.

Quiescence closes new scans, drains or suspends accepted claims, checkpoints assignment-scoped advisory and workspace cursors, and fences callbacks. Retirement never relabels an old-generation result as current.

Interrupted scans preserve operation keys, attempts, source lineage, and ambiguous-effect state. Restart reconstructs Meld beliefs, goals, and task state from Meld stores. External caches may accelerate reobservation but do not contain hidden canonical decisions.

## Interaction With Other Stewards

The event network carries cooperation:

```text
actionable dependency violation fact
→ independently observed by developer or reliability assignment

independently authorized code or deployment change
→ workspace or service fact

new inventory and advisory observation
→ security reassessment

relevant accepted source change
→ independent docs freshness observation opportunity
```

No edge is a direct steward command. A missing consumer assignment leaves the producer fact durable without inventing follow-on work.

## Router Falsification Findings

Reject or narrow the common router design if:

- the router must decode security policy to install the package
- a scanner endpoint or credential participates in package identity
- external monitor delivery must fabricate an execution claim
- missing findings can satisfy a maintained condition without explicit coverage proof
- one opaque capability must span observation through code mutation
- current source or policy heads reinterpret historical assessments
- assignments sharing one security service must share authority, cursors, or current posture
- historical interpretation replay requires the old external runtime to be online
- readiness can prove only process existence and not adapter plus source identity

The strongest pressure finding is that request attempts and passive deliveries need different lineage while converging on one owner-admission boundary.

## Theory-Set Implications

### Common Candidate Mechanism

The example supports exact routed components, owner-installed theory revisions, explicit capability components, assignment-local activation, generation fencing, operation and attempt lineage, passive delivery lineage, source-revision preservation, and owner result admission.

It also suggests a common transport-neutral place for completeness and adapter identity. The exact carrier remains unresolved and should not become security-specific router syntax.

### Owner-Specific Meaning

Dependency identity, advisory normalization, coverage, currency, applicability, bounded negative proof, remediation feasibility, and security resolution remain dependency-security semantics.

The common theory set should carry exact references and owner products without interpreting these concepts.

### Unresolved Bridge

Open bridges include:

- the common representation of passive subscriptions and delivery lineage
- the owner of activation-generation, participant-incarnation, and source-cursor receipts
- how semantic time creates observation opportunities without hidden wall-clock belief mutation
- how domain-derived subjects become assignment-eligible and later retract
- how hard isolation requirements are declared and attested across placements
- which retired-generation results may remain useful as historical evidence

These remain discovery questions.

## Non Commitments

This exercise does not approve route schemas, provider selection, process topology, cursor storage, ecosystem identity, sandbox technology, or automated remediation.

## Source Grounding

- [Use-Case Definition](README.md)
- [Dependency Security Consumer Design](../../../completed/integration/dependency_security_pds_consumer_design_spec.md)
- [PDS Router Design](../../../completed/integration/pds_router_design_spec.md)
- [Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Activation Lifecycle Fanout](../../../completed/integration/pds_activation_lifecycle_fanout_exercise.md)
