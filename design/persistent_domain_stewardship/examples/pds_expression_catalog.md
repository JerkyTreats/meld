# Persistent Domain Stewardship Expression Catalog

Date: 2026-08-08  
Status: illustrative  
Scope: normalized definitions of dissimilar PDS applications used to pressure-test the common stewardship shape

## Purpose

Persistent Domain Stewardship should not be validated by one application family.

This catalog normalizes several candidate PDS expressions against a small comparative surface. It does not define a final package schema. The purpose is to expose where the common form holds, where a simpler mechanism is sufficient, and where a candidate pressures a specific Meld runtime capability.

The catalog complements the complete decomposition procedure in [Use-Case Decomposition](../use_case_decomposition.md). Detailed examples may expand an expression into domain, observation, epistemic, charter, operational, outcome, and governance models.

## Expression Card

Each expression is described through the same questions:

```text
mandate
subjects and scope
key observations
key beliefs
maintained conditions
observation choices
intervention choices
verification
bounded authority
simpler baseline
Meld purchase
fit verdict
```

The fields are deliberately application-level. They lower into canonical operational domain theory rather than creating a parallel PDS runtime.

## Comparison Matrix

| Expression | Persistent subject | Primary uncertainty | Typical intervention | Simpler baseline | Fit |
|---|---|---|---|---|---|
| Documentation freshness | source/document relation | whether docs still describe current source | verify, regenerate, draft change | scheduled docs workflow | strong when selective and continuous |
| CVE exposure | component/dependency/exposure | whether a vulnerability applies and remains material | inspect, upgrade, mitigate, escalate | dependency scanner/bot | strong when applicability and remediation are uncertain |
| Roleplay continuity | character/world/session | what is canon, known, remembered, or permitted | respond, hydrate, reconcile, ask | persona prompt + RAG | conditional |
| Lore/canon stewardship | entity/claim/source | identity, support, contradiction, supersession | create/link/merge/split/refresh | extraction + knowledge graph | conditional |
| Codebase quality | module/API/test/change | quality state across conflicting concerns | validate, patch, tolerate, escalate | CI + coding agents | strong |
| Game faction strategy | faction/actor/territory | hidden state, intent, trust, future effects | scout, negotiate, allocate, act | behavior tree/utility AI | strong research fit |
| Service reliability | service/deployment/dependency | diagnosis, recurrence, action risk | diagnose, scale, rollback, remediate | alerts + runbooks + controllers | strong at operational horizon |
| Learner mastery | learner/concept/skill | mastery, misconception, retention | diagnose, teach, practice, defer | spaced repetition/adaptive sequencing | conditional |
| Portfolio thesis and risk | thesis/position/exposure | evidence quality, invalidation, interacting risk | research, recommend, rebalance under external controls | research DB + risk engine | strong for research/risk, weak for autonomous trading |
| Physical asset maintenance | asset/component/inspection | condition, degradation, failure risk | inspect, service, schedule, isolate | CMMS/scheduled maintenance | conditional-to-strong |

## 1. Documentation Freshness Steward

### Mandate

Maintain evidence that documentation accurately describes the active source, API, workflow, or other declared subject.

### Subjects and scope

```text
repository
workspace subtree
source artifact
document
document section
source-document relationship
revision
```

### Key observations

- source change;
- document change;
- generated evidence map;
- verification report;
- human acceptance or correction.

### Key beliefs

```text
content.freshness
source_document_coverage
verification_freshness
```

### Maintained conditions

- declared documentation remains above a freshness threshold;
- required source coverage remains represented;
- stale verification is reacquired before high-confidence claims are made.

### Observation choices

- inspect changed source regions;
- compare source-document lineage;
- run bounded documentation verification;
- hydrate prior accepted documentation decisions.

### Intervention choices

- tolerate a non-material source change;
- regenerate a bounded document;
- update an evidence map;
- open a draft change;
- request review.

### Verification

Task completion is not proof of freshness. Verification requires renewed evidence that the generated artifact covers and accurately describes the declared subject.

### Authority

Read and verification may be automatic. Source/document mutation, pull-request creation, and merge authority remain separately granted.

### Simpler baseline

Scheduled documentation generation or a docs-writer workflow.

### Meld purchase

Selective evidence acquisition, persistent source-document lineage, uncertainty-aware freshness, bounded regeneration, and outcome history.

### Fit verdict

Strong when the responsibility is continuous and selective. Weak when every source change deterministically regenerates a known artifact.

## 2. CVE Exposure Steward

### Mandate

Maintain an evidence-grounded view of vulnerability exposure for an assigned software or infrastructure scope and drive bounded remediation when risk enters a declared breach region.

### Subjects and scope

```text
component
dependency
package version
image
service
runtime exposure
vulnerability advisory
mitigation
exception
```

### Key observations

- advisory publication or revision;
- package-lock, SBOM, or image change;
- dependency-resolution result;
- runtime reachability or configuration evidence;
- exploitability evidence;
- mitigation or patch verification;
- accepted risk exception.

### Key beliefs

```text
security.cve_applicability
security.exploitability
security.exposure
security.remediation_confidence
security.evidence_freshness
```

A scanner match is evidence of possible applicability, not necessarily proof of exploitable exposure.

### Maintained conditions

- critical assigned exposure remains below declared risk thresholds;
- evidence for high-severity findings remains current;
- accepted exceptions retain explicit expiry and rationale;
- remediated findings are independently reverified.

### Observation choices

- inspect resolved dependency graph;
- acquire SBOM or runtime-version evidence;
- evaluate reachability or affected configuration;
- retrieve revised advisory detail;
- run bounded validation after remediation.

### Intervention choices

- tolerate with explicit bounded exception;
- update dependency or base image;
- disable affected feature;
- apply configuration mitigation;
- isolate service;
- escalate when remediation authority is unavailable.

### Verification

A dependency update is not sufficient. Later evidence must establish that the vulnerable version or affected configuration is no longer materially exposed and that protected reliability constraints remain satisfied.

### Authority

Observation may be broad. Repository mutation, deployment, isolation, exception approval, and emergency remediation require separate grants.

### Simpler baseline

Dependency scanner, vulnerability-management platform, and automated update bot.

### Meld purchase

Applicability reasoning, evidence freshness, selective validation, remediation alternatives, cross-objective tradeoffs, exception continuity, and verified outcome history.

### Fit verdict

Strong when scanner findings require contextual applicability and remediation decisions. Weak when the policy is a deterministic "upgrade every matched package" rule.

## 3. Roleplay Character Continuity Steward

Detailed in [Roleplay Character Continuity Steward](roleplay_character.md).

### Mandate

Maintain coherent perspective-bound character state and narrative continuity across an evolving interaction history.

### Subjects and scope

```text
character
world fact
character knowledge
relationship
commitment
secret
scene
narrative thread
canon revision
```

### Key beliefs

```text
narrative.canon_support
character.knowledge
character.relationship_state
character.commitment_state
narrative.thread_state
character.identity_consistency
```

### Maintained conditions

- world truth and character knowledge remain distinct;
- commitments and relationships survive prompt-window turnover;
- explicit retcons supersede rather than erase history;
- response generation respects active knowledge and authority.

### Simpler baseline

Persona prompt plus bounded conversation history and semantic retrieval.

### Meld purchase

Perspective-relative truth, temporal supersession, active context hydration, explicit commitments, explainable continuity constraints, and multi-session replay.

### Fit verdict

Conditional. Strongest for shared, mutable, long-running worlds. A single-session chatbot should remain simpler.

## 4. Lore Transcriber And Canon Steward

Detailed in [Lore Transcriber And Canon Steward](lore_transcriber.md).

### Mandate

Maintain durable entity identity, source-backed claims, conflict visibility, supersession, and current derived definitions for an evolving corpus.

### Subjects and scope

```text
entity
alias
claim
relationship
source
definition
revision
```

### Key beliefs

```text
lore.identity_match
lore.claim_support
lore.definition_completeness
lore.definition_freshness
lore.conflict_state
```

### Maintained conditions

- stable identities remain resolvable across aliases and revisions;
- accepted claims retain provenance;
- ambiguity and contradiction remain explicit;
- summaries are refreshed when their evidence changes materially.

### Simpler baseline

Entity extraction, vector retrieval, deterministic document storage, and optional knowledge graph.

### Meld purchase

Longitudinal identity reconciliation, active disambiguation, merge/split lineage, contradiction management, and selective downstream refresh.

### Fit verdict

Conditional. Transcription alone is not stewardship; canon maintenance is.

## 5. Codebase Quality Steward

Detailed in [Software Quality Stewardship Example](software_quality.md).

### Mandate

Maintain several software-quality concerns over an indefinitely changing repository without collapsing them into one universal quality score.

### Subjects and scope

```text
module
API
persistence boundary
test
benchmark
document
change
```

### Key beliefs

```text
reliability.test_health
persistence.migration_safety
performance.health
usability.health
maintainability.health
content.freshness
action.cost
action.value
```

### Maintained conditions

Separate stewards may maintain reliability, persistence, performance, usability, maintainability, and documentation conditions over overlapping subjects.

### Simpler baseline

CI, static analysis, dependency bots, scheduled coding agents, and fixed remediation workflows.

### Meld purchase

Heterogeneous evidence, selective validation, conflict among quality objectives, persistent decision history, adaptive remediation, and verified downstream outcomes.

### Fit verdict

Strong.

## 6. Game Faction Strategy Steward

### Mandate

Maintain a faction's survival, territory, resources, relationships, and strategic objectives under partial and perspective-dependent information.

### Subjects and scope

```text
faction
actor
territory
resource
relationship
threat
commitment
operation
```

### Key observations

- sightings;
- reports and testimony;
- resource updates;
- diplomacy;
- combat or operation outcomes;
- scouting results;
- world-state changes.

### Key beliefs

```text
faction.threat
faction.intent
faction.trust
faction.resource_security
faction.plan_viability
```

### Maintained conditions

- preserve faction viability and protected resources;
- maintain strategic awareness above configured uncertainty limits;
- preserve commitments and relationship consequences across episodes.

### Observation choices

Scout, interrogate, inspect, wait, or acquire intelligence through bounded sources.

### Intervention choices

Allocate resources, negotiate, form or break commitments, move strategic assets, initiate operations, or tolerate uncertainty.

### Verification

World outcomes supply later evidence about territorial, resource, casualty, relationship, and strategic effects.

### Authority

Strategic decisions are bounded to faction-controlled resources. Per-frame control, animation, aiming, collision, and other real-time mechanics remain external deterministic systems.

### Simpler baseline

Behavior tree, utility AI, GOAP, blackboard, or scripted simulation.

### Meld purchase

Persistent divergent beliefs, provenance-sensitive testimony, active scouting, long-term identity, plan repair, and counterfactual reconstruction.

### Fit verdict

Strong research fit at strategic timescales.

## 7. Service Reliability Steward

### Mandate

Maintain service objectives, dependency health, capacity margin, recovery readiness, and bounded remediation state across recurring operational incidents.

### Subjects and scope

```text
service
deployment
dependency
region
resource
SLO
incident
runbook
remediation
```

### Key observations

- metrics;
- traces;
- logs and structured failure events;
- deployment changes;
- dependency health;
- synthetic checks;
- incident outcomes;
- recovery tests.

### Key beliefs

```text
reliability.slo_risk
reliability.fault_hypothesis
reliability.capacity_risk
reliability.recovery_readiness
reliability.recurrence_risk
```

### Maintained conditions

- SLO risk remains within declared policy;
- recovery evidence stays sufficiently fresh;
- recurring failures retain causal and remediation history;
- uncertain diagnosis triggers evidence acquisition before high-risk intervention where time permits.

### Observation choices

Run targeted diagnostics, collect a trace, compare deployments, inspect dependency health, or trigger a bounded recovery test.

### Intervention choices

Scale, rollback, fail over, disable a feature, restart a bounded component, propose a patch, or escalate.

### Verification

Successful command execution does not close the episode. Later SLO, recurrence, and dependency evidence determines whether the maintained condition was restored.

### Authority

Hard real-time safety controls, circuit breakers, and millisecond-scale protection remain deterministic. High-blast-radius actions require explicit operational authority.

### Simpler baseline

Alerts, autoscaling, deterministic remediation, incident workflows, and runbooks.

### Meld purchase

Selective diagnostics, competing hypotheses, action-cost and blast-radius reasoning, persistent recurrence history, adaptive recovery, and outcome attribution.

### Fit verdict

Strong at the minutes-to-months operational horizon.

## 8. Learner Mastery Steward

### Mandate

Maintain an evidence-grounded model of a learner's mastery, prerequisite readiness, misconceptions, retention, and progression, then select bounded learning interventions.

### Subjects and scope

```text
learner
concept
skill
prerequisite
assessment item
practice episode
artifact
misconception
```

### Key observations

- answers;
- solution steps;
- projects;
- assessment results;
- time-to-response;
- hint use;
- later retention and transfer evidence.

### Key beliefs

```text
learning.mastery
learning.prerequisite_readiness
learning.misconception
learning.retention_risk
learning.transfer
```

### Maintained conditions

- required prerequisite mastery remains above the threshold for assigned learning goals;
- stale mastery evidence is reacquired;
- suspected misconceptions remain explicit until differentiated by evidence;
- progression respects protected load and policy constraints.

### Observation choices

Ask a diagnostic question, request an explanation, select a discriminating exercise, or inspect a prior artifact.

### Intervention choices

Teach, demonstrate, assign practice, vary representation, revisit prerequisite material, defer progression, or escalate to a human educator.

### Verification

Immediate answer correctness is weak evidence of durable mastery. Later retention, transfer, and independent performance provide stronger outcome evidence.

### Authority

The steward may select activities within the assignment but does not acquire authority over grading, credentialing, health, or other unrelated domains.

### Simpler baseline

Spaced repetition, Bayesian knowledge tracing, fixed adaptive sequencing, or an LLM with learner memory.

### Meld purchase

Active diagnostic selection, multimodal provenance, explicit uncertainty, cross-topic continuity, reconstructable instructional decisions, and delayed outcome feedback.

### Fit verdict

Conditional but credible. Outcome evidence is delayed and confounded compared with software or simulation.

## 9. Portfolio Thesis And Risk Steward

### Mandate

Maintain research theses, invalidation conditions, mandate constraints, exposure, liquidity, and interacting risk while keeping research belief separate from execution authority.

### Subjects and scope

```text
thesis
claim
asset
position
exposure
constraint
risk factor
evidence source
```

### Key observations

- filings and source documents;
- market and reference data;
- portfolio state from external systems of record;
- risk measurements;
- thesis evidence and counterevidence;
- mandate or constraint changes.

### Key beliefs

```text
portfolio.thesis_support
portfolio.invalidation_risk
portfolio.exposure_risk
portfolio.liquidity_risk
portfolio.evidence_freshness
```

### Maintained conditions

- thesis evidence remains current enough for declared decisions;
- invalidating evidence remains explicit;
- mandate and exposure constraints remain visible;
- research confidence never substitutes for execution permission.

### Observation choices

Retrieve primary evidence, inspect counterevidence, recalculate exposure, or request additional research.

### Intervention choices

Update thesis state, recommend a change, request review, or submit a bounded action to a separately governed execution system where authority exists.

### Verification

Later outcomes may update action-value evidence but do not retroactively prove a prior thesis correct. Attribution remains uncertain and explicit.

### Authority

Research and risk stewardship should be separable from live execution. Autonomous trading requires external controls and independent validation beyond the PDS expression.

### Simpler baseline

Research database, risk engine, optimizer, fixed rebalancing rules, and alerts.

### Meld purchase

Decision-time thesis reconstruction, counterevidence, belief decay, active research selection, explicit uncertainty, mandate-aware act/tolerate policy, and prospective outcome history.

### Fit verdict

Strong for thesis and risk stewardship; weak initial fit for autonomous live trading.

## 10. Physical Asset Maintenance Steward

### Mandate

Maintain evidence that assigned physical assets remain safe, available, calibrated, and serviceable over an indefinite operating horizon.

### Subjects and scope

```text
asset
component
consumable
inspection
calibration
service interval
fault
work order
replacement part
```

### Key observations

- runtime hours or cycles;
- sensor readings;
- inspection reports;
- fault codes;
- maintenance actions;
- calibration results;
- environmental conditions;
- part replacement history.

### Key beliefs

```text
asset.condition
asset.failure_risk
asset.calibration_confidence
asset.maintenance_urgency
asset.evidence_freshness
```

### Maintained conditions

- protected safety and calibration conditions remain inside declared bounds;
- maintenance evidence remains current;
- repeated faults retain recurrence context;
- uncertain condition triggers inspection before avoidable replacement where policy permits.

### Observation choices

Request inspection, run self-test, collect a measurement, inspect maintenance history, or compare a component against expected degradation.

### Intervention choices

Schedule maintenance, replace consumable, recalibrate, isolate equipment, recommend service, or defer low-risk work.

### Verification

Closing a work order does not prove restoration. Post-service inspection, calibration, or operating evidence must support the maintained condition.

### Authority

The system may recommend or schedule work without necessarily having physical actuation authority. Safety-critical lockout, energized work, and other hazardous actions remain externally governed.

### Simpler baseline

Computerized maintenance management system, fixed service intervals, checklists, and deterministic threshold alerts.

### Meld purchase

Condition inference from heterogeneous evidence, adaptive inspection, recurrence history, maintenance prioritization under uncertainty, and outcome verification.

### Fit verdict

Conditional-to-strong when condition is partially observed and maintenance choices are nontrivial. Weak when fixed intervals and authoritative sensor thresholds are sufficient.

## Cross-Expression Findings

The catalog supports several distinctions that should remain explicit as PDS evolves.

### Persistent memory is insufficient

Roleplay, lore, learner, and thesis applications all need long-term memory, but memory alone does not establish stewardship. The PDS shape appears only when the system has explicit maintained conditions, observation choices, intervention choices, verification, and authority.

### A derived artifact is not domain truth

Documentation, lore definitions, roleplay responses, research summaries, and maintenance reports are projections or artifacts. They do not become authoritative merely because a steward generated them.

### Observation and intervention can be semantic

A PDS action need not be a machine mutation. Asking a discriminating question, hydrating prior context, producing a bounded character response, or requesting an inspection can be an operational action when it changes the evidence or interaction state under a standing mandate.

### Perspective matters in several domains

Roleplay characters, game factions, learners, and portfolio theses demonstrate that the same underlying event can support different perspective-specific beliefs without requiring multiple event ledgers.

### Outcome delay varies by domain

Software tests and simulations can provide rapid verification. Learner retention, reliability recurrence, asset degradation, and portfolio outcomes may take much longer. PDS must not equate task completion with restored domain conditions.

### The baseline remains mandatory

Every expression has a simpler alternative. The PDS package model should be narrowed rather than expanded if the simpler mechanism achieves equivalent correctness, cost, safety, and inspectability.

## Extraction Pressure On The Common Model

| Pressure | Expressions that expose it |
|---|---|
| source/document lineage | docs freshness, lore |
| applicability rather than raw match | CVE exposure |
| perspective-relative knowledge | roleplay, game faction |
| identity merge/split and supersession | lore |
| conflicting standing objectives | codebase quality, reliability |
| selective evidence acquisition | CVE, reliability, learner, portfolio, physical maintenance |
| expressive intervention | roleplay |
| delayed outcome evidence | learner, portfolio, physical maintenance, reliability recurrence |
| external source-of-truth boundary | CVE, reliability, portfolio, physical maintenance |
| bounded context hydration | roleplay, lore, codebase quality, portfolio |
| authority distinct from capability | all expressions |

A future package source language should be considered insufficient if it cannot represent these differences without adding domain-specific runtime grammar.

## Promotion Rule

An expression should receive a dedicated detailed example when it either:

1. exposes a runtime or package-model pressure not already covered by another detailed example; or
2. is selected as an implementation proof.

On that basis, roleplay continuity and lore/canon stewardship receive dedicated examples in this branch because they pressure perspective-relative knowledge, semantic intervention, identity resolution, contradiction, supersession, and bounded context hydration in ways the software-quality example does not.

## Read With

- [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md)
- [Use-Case Decomposition](../use_case_decomposition.md)
- [Software Quality Stewardship Example](software_quality.md)
- [Roleplay Character Continuity Steward](roleplay_character.md)
- [Lore Transcriber And Canon Steward](lore_transcriber.md)
