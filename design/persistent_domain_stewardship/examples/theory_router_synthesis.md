# Theory Router Synthesis Across PDS Examples

Date: 2026-08-18
Status: discovery synthesis aligned to canonical PDS boundary
Scope: implications of the normalized example router fanout for the shared PDS theory set

## Purpose

The example fanout tests whether dissimilar stewardship applications can attach through one `theory::router` contract without moving their vocabulary or runtime state into PDS core.

The goal is not to make every repeated field a shared schema. The goal is to identify:

- the route spine already supported by several examples
- owner-specific meaning that should remain behind domain routes
- possible missing theory fragments exposed by dissimilar cases
- candidates that falsify PDS qualification or a universal route assumption

Every example retains its own [README and router specification](README.md).

[Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) now fixes the interpretation of this fanout. Exact capability contracts, Methods, Strategy construction policy, projection requests, search controls, and effective authority are runtime assembly inputs rather than PDS semantic theory. Older router examples that list exact capability components describe the compatibility migration shape.

## Role In The Next Iteration

The corpus has two different roles.

Documentation freshness and dependency security are the delivery consumers for the next iteration. Their detailed designs decide the required router, migration, activation, admission, isolation, and lifecycle behavior:

- [documentation freshness use case](documentation_freshness/README.md) and [router specification](documentation_freshness/router_spec.md)
- [dependency security use case](dependency_security/README.md) and [router specification](dependency_security/router_spec.md)

The other examples are pressure tests. They help keep the two-consumer design open to dedicated PDS packages later, but they do not add implementation scope now.

| Finding from the corpus | Next-iteration treatment |
| --- | --- |
| owner-routed package spine | implement and prove with both delivery consumers |
| package-local route cardinality | implement and prove with the several components used by both packages |
| provider-free deterministic activation | prove through docs inspection and security scanning |
| request attempt and passive delivery lineage | implement through dependency-security invocation and source advance |
| generation, incarnation, late-result, wake, and recovery fencing | prove through both consumer lifecycle cases |
| source coherence | exercise through security inventory and advisory revisions while keeping sufficiency owner-defined |
| perspective projection and disclosure | preserve owner extension seams and defer implementation |
| graph projection and exact hydration | keep below the router and defer to a future domain consumer |
| candidate evidence-mapping route | keep candidate and do not add to the common route set yet |
| high-consequence external approval | preserve authority and effect separation without implementing a universal approval framework |
| shared observation deduplication | preserve producer lineage and defer a common reuse mechanism |

A wider example may reject a closed abstraction. It does not promote its own vocabulary into the common package or runtime contract. When that example becomes active work, its folder should seed a dedicated PDS package and owner integration rather than a broader first implementation.

## Interpretation Rule

A repeated requirement supports a common theory mechanism only when at least two dissimilar examples need the same semantics and one owner can state the contract without application vocabulary.

Repeated application words do not establish a common route. For example, roleplay secrets, portfolio confidential research, and service credentials all involve restricted information, but they do not necessarily share one disclosure-policy body.

An example router spec therefore classifies each finding as:

- common candidate mechanism
- owner-specific meaning
- unresolved bridge

## Shared Route Spine Under Test

The current router design already provides the strongest common spine.

| Route | Cardinality tendency | Shared responsibility |
| --- | --- | --- |
| domain-owned policy route | one or many by owner | install the application vocabulary and validation policy without exposing it to root |
| `world-model.belief-family.v1` | many | declare falsifiable questions and admissible evidence shape |
| `world-model.outcome-mapping.v1` | many | map owner-admitted domain products into belief evidence without treating task success as truth |
| `world-model.agent-curation-rule.v1` | one or many | declare perspective-bound normative evaluation behavior |
| `world-model.agent-maintained-condition.v1` | one or many | declare standing conditions that survive transient Goals |
| `world-model.strategy-theory.v1` | one or many | declare settlement, prospective evidence, abstract action, outcome, and constraint meaning |
| domain governance policy route | one or many by owner | declare action classes, restrictions, and approval meaning without granting effective authority |

The domain-owned policy route is a pattern, not one universal route id. Docs, security, narrative, learning, reliability, portfolio, and maintenance owners interpret different bodies.

No route in this spine owns current observations, beliefs, Goals, tasks, activation generations, participant incarnations, external cursors, or measured outcomes.

Exact capability contracts arrive through the separate capability-contribution and activation path. Strategy Methods and construction policy arrive through Strategy and Agent contracts. These products may be linked by exact identity during runtime assembly without becoming semantic package components.

The narrative examples repeatedly use candidate `world-model.evidence-mapping.v1` components. That repetition is evidence for a narrative or belief-owner contract, not yet for a universal route. The same domain family supplied the pressure, while dissimilar examples often lower admitted products directly through outcome mappings and owner policy.

## Cross-Example Pressure Map

| Example | Domain-owned semantic center | Distinctive pressure on shared theory |
| --- | --- | --- |
| documentation freshness | claim and source-document policy | exact local theory, selective provider binding, verified artifact outcomes |
| dependency security | inventory, advisory, applicability, coverage, and verification policy | bounded negative evidence, passive source advance, external runtime admission |
| roleplay character continuity | canon, knowledge, relationship, commitment, reveal, and retcon policy | perspective, disclosure authority, context hydration, accepted interaction outcomes |
| lore and canon | identity, claim, provenance, merge, split, and definition policy | claim-level correction, stable identity, conflict visibility, derived projection refresh |
| freeform narrative database | source-backed narrative storage and retrieval | proves that durable data alone does not require PDS activation |
| codebase quality | separate reliability, persistence, performance, usability, maintainability, and docs concerns | several maintained conditions over one subject and protected-floor arbitration |
| game faction strategy | faction priorities, testimony, treaties, resources, and strategic effects | perspective-bound evidence, simulation outcomes, strategic versus real-time control |
| service reliability | SLO, diagnosis, capacity, remediation, and recovery policy | high-rate input, risky action, readiness, safe points, and recurrence outcomes |
| learner mastery | mastery, prerequisite, misconception, progression, and instructional policy | active diagnosis, delayed verification, learner perspective, human escalation |
| portfolio thesis and risk | thesis, mandate, exposure, liquidity, and research policy | external source truth, counterevidence, recommendation versus execution authority |
| physical asset maintenance | condition, degradation, inspection, service, and safety policy | physical observation, scheduling versus actuation, safety-critical external governance |

## Findings Supported Across Dissimilar Examples

### One package needs many owner routes

No substantial example is truthfully represented by one opaque policy body. Each qualifying steward links domain policy, belief, normative evaluation, Strategy semantic theory, governance, and outcome meaning owned by different domains, then combines those semantics with separately activated capabilities.

This supports a router over linked owner fragments rather than one PDS runtime object.

### Route cardinality must remain package local

Codebase quality, service reliability, roleplay continuity, and learner mastery need several belief families and may need several maintained conditions or curation rules. A shared route contract can constrain occurrences within one package without making the route mandatory for every package.

### Domain policy is the semantic anchor

Every qualifying example has at least one owner-specific policy or vocabulary body that other fragments reference. PDS links its exact identity but does not interpret it.

This is the main defense against a universal PDS ontology.

### Observation and intervention remain separate affordances

Every strong or conditional steward chooses among observing, tolerating, acting, and escalating. The action may be a test, scan, diagnostic question, source retrieval, scout operation, inspection, research request, or physical service proposal.

PDS Strategy semantic theory declares the action-class and outcome meaning. The activation capability snapshot supplies current realizations. Strategy constructs the choice. The domain owner retains result admission.

### Outcome meaning cannot be reduced to task completion

Documentation verification, security reassessment, SLO recovery, learner retention, portfolio attribution, and post-service inspection all require later domain evidence. The existing outcome-mapping route is therefore central to the common theory set.

### Assignment meaning is richer than package selection

Roleplay, game faction, learner, portfolio, and overlapping software stewards demonstrate that principal, perspective, subject, branch, requested authority, and grant lineage belong to assignment. They cannot be inferred from the package or process.

### Activation placement remains outside theory identity

The same domain theory may use an in-process library, subprocess, sidecar, remote service, persistent controller, model provider, simulator, learning platform, market-data service, or maintenance system. Placement and isolation requirements belong to activation, activation generation, and participant incarnation.

### Source coherence is common mechanics with owner meaning

Several stewards need a coherent decision-time view across independently revised inputs. Portfolio positions and prices, learner content and rubrics, and asset telemetry and calibration all apply this pressure.

The common carrier may preserve source identity, revision, coherent-cut references, and explicit mixed-snapshot uncertainty. Only the domain owner can decide whether a view is coherent enough for admission or action.

### Late evidence has two independent decisions

Authentic historical admission does not imply eligibility for the current projection. A delayed retention result, corrected filing, late telemetry sample, retired provider response, or old simulation outcome may remain valid evidence about the past while being too stale or out of scope for current stewardship.

This distinction belongs at the owner-admission boundary and must survive activation replacement.

### Action closure has several separate gates

The examples distinguish capability availability, data disclosure, requested authority, principal grant, external approval or interlock, authoritative effect reconciliation, and independent outcome verification.

No single successful capability attempt can collapse those gates. This is especially visible in portfolio execution, physical maintenance, learner data sent to providers, repository mutation, and service remediation.

### Justified idle is an active lifecycle state

Learner retention waits, market sessions, telemetry subscriptions, source monitors, and long-running CI work can all be correctly idle. A healthy process without a durable wake path, valid subscription, or recoverable cursor is not healthy stewardship.

This strengthens durable wait identity and wake viability as runtime readiness concerns rather than package theory.

### Participant restart may be narrower than activation replacement

Several external-runtime examples suggest that a mechanically equivalent connector restart should receive a new participant incarnation without necessarily replacing assignment-wide activation generation. Admission must remain closed until cursor, ambiguous operation, and late-delivery reconciliation completes.

This is a runtime refinement under discovery, not an established package or assignment concept.

### Perspective needs projection and admission

Roleplay and faction strategy show that naming a perspective does not prevent knowledge leakage. An owner-defined projection must bound what is observable, and owner admission must decide what can enter assignment knowledge.

Hidden evaluator truth may support later assessment of a decision. It cannot be injected retroactively into the historical perspective that made the decision.

### Shared observation does not mean shared stewardship

Several quality charters or assignments may reuse one exact repository snapshot, test product, public filing, telemetry summary, or source revision. Reuse is safe only when scope, revision, binding, disclosure, completeness, and producer lineage satisfy every consumer.

Sharing an admitted product or execution attempt never merges beliefs, Goals, authority, curation, or outcome judgment.

### Structural closure and live readiness are separate aggregates

Package compilation can prove that every required owner installed an exact routed component. Activation readiness must separately prove that every selected runtime participant, binding, admission path, and wake route realizes that closure.

Both aggregates need exact membership. Neither aggregate may interpret owner-specific meaning.

### Four orderings must remain distinct

Package declaration order, semantic priority among maintained conditions, lifecycle dependency order, and event order solve different problems. None may be inferred from another.

This is clearest in multi-charter codebase quality, but also applies to service participants, narrative projections, and high-consequence external effects.

### Health has three separate subjects

The maintained subject may be healthy while the steward is stalled. The runtime process may be healthy while a subscription or wake path is lost. The steward may be operational while the maintained subject remains violated.

Subject condition, stewardship liveness, and runtime operational health therefore remain distinct projections.

## Candidate Theory Bridges Exposed By The Fanout

The examples expose several concerns that may need an installable theory fragment or a narrower public owner contract. The fanout does not yet decide which.

| Concern | Examples applying pressure | Candidate ownership question |
| --- | --- | --- |
| source and observation admission | security, reliability, portfolio, maintenance | domain sensory adapter only, or a source-domain theory route plus adapter contract |
| graph identity and projection policy | lore, roleplay, dependency security, maintenance | domain policy referencing graph public contracts, or a graph-owned route |
| perspective and disclosure | roleplay, game faction, learner, portfolio | Agent-owned policy fragment, domain-owned policy, or a linked pair |
| context hydration policy | roleplay, lore, service diagnosis, portfolio research | context capability selection, Strategy theory, or a context-owned theory route |
| temporal currency and validity | docs, security, learner, portfolio, maintenance | owner policy fields, belief-family semantics, or a common temporal contract |
| claim correction and identity reconciliation | lore, roleplay, portfolio research | domain-owned records over generic provenance rather than a common PDS route |
| lifecycle readiness and safe points | security, service reliability, docs, external platforms | runtime activation contributor ports rather than state-free package theory |
| high-consequence approval | security, reliability, portfolio, physical maintenance | authority policy plus external governance rather than domain capability availability |
| coherent source views | learner, portfolio, physical maintenance, security | generic revision carrier plus owner-defined admission, or an owner-specific contract only |
| disclosure authority | roleplay, learner, portfolio, physical maintenance | explicit intersection of assignment perspective, binding policy, and authority grant |
| late evidence eligibility | docs, learner, portfolio, maintenance, faction strategy | owner admission decision distinct from authentic historical storage |
| passive delivery lineage | security, narrative, service reliability, maintenance | runtime subscription and cursor contract rather than fabricated execution claims |
| participant incarnation | learner, portfolio, maintenance, persistent connectors | activation contributor lineage below assignment-wide generation |
| perspective projection and admission | roleplay, faction strategy, learner | owner contract that bounds observable evidence before belief admission |
| reusable observation identity | codebase quality, docs, security, portfolio | exact product reuse below stewardship without shared normative state |
| aggregate runtime closure | codebase quality, service reliability, external platforms | exact readiness and safe-point contributor membership distinct from package closure |
| structural wake viability | service reliability, learner, security, maintenance | runtime-verifiable wake carrier whose domain meaning remains owner-defined |

The test for each concern is whether a common owner contract can be stated without importing the examples' vocabulary.

## Falsification Results

### Freeform narrative database

Durable ingestion, search, entity storage, and retrieval do not by themselves qualify as stewardship. The router should attach a lore or narrative steward only when a standing condition, evidence choice, intervention choice, verification, and authority boundary exist.

This rejects package installation as a way to relabel an ordinary database as PDS.

It also locates graph projection registration and exact event hydration below `theory::router`. Those may be reusable graph and event owner contracts without becoming stewardship theory.

### Deterministic regeneration and control

Documentation that always regenerates, service protection that must react deterministically, fixed learning sequences, threshold maintenance, and fixed portfolio rules remain simpler systems when uncertainty and adaptive choice add no value.

The package model should permit truthful non-use rather than forcing a generic steward around them.

### Universal domain sections

The fanout does not support universal `docs`, `security`, `character`, `service`, `learner`, `portfolio`, or `asset` fields in the package manifest. New application vocabulary continues to enter through owner routes.

## Overall Theory Set Direction

The fanout strengthens this layered reading:

```text
principal-facing declaration
→ package and profile compiler
→ routed domain-owned semantic fragments
→ exact package receipt
→ assignment with principal perspective scope and grant lineage
→ activation with physical bindings isolation and implementation offers
→ Strategy assembly with current planner capability Method policy and authority snapshots
→ existing Meld runtime domains
→ owner-admitted observations and outcomes
```

The shared theory set should describe stable meaning that the existing runtime can execute. It should not absorb exact capability catalogs, Method topologies, construction policy, search controls, physical placement, runtime lifecycle, source-system state, current beliefs, or application ontology.

## Unresolved Threads

- whether every qualifying package needs one explicit domain policy route or may consist only of imported owner fragments
- whether observation admission needs a common theory route beyond domain policy and outcome mapping
- whether graph projection and identity policy are installable theory or domain adapter configuration
- whether perspective and disclosure lower entirely into Agent plus domain policy
- whether context hydration remains a selected capability or gains an owner theory fragment
- whether evidence mapping is a narrative-domain convenience or a broader belief-owner route
- how several maintained conditions and Agents within one package relate to assignment identity
- how exact package imports share base theory without hiding the source of authority
- which lifecycle requirements belong in activation contributor contracts rather than package theory
- how participant incarnation composes with assignment-wide activation generation
- how data disclosure grants intersect action authority without becoming a second ambient authority channel
- which carrier records coherent source cuts without interpreting domain sufficiency
- how durable lineage survives privacy withdrawal or retention duties without retaining forbidden payloads
- whether reusable observation identity belongs to event, task, capability, or a narrow cross-domain contract
- how aggregate readiness and quiescence receipts evolve when package cardinality allows many owner components

These remain resumable discovery questions. The per-example router specs provide the evidence surface for narrowing them.
