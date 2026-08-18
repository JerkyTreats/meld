# PDS Examples Across Compilation Layers

Date: 2026-08-18
Status: illustrative architecture analysis aligned to canonical PDS
Scope: reinterpret the PDS example corpus through the distinction between user intent, canonical declaration, compiled semantic IR, runtime assembly inputs, and runtime-owned state

## Purpose

The current PDS examples were intentionally developed from the runtime upward. They mix stable belief and settlement semantics with Methods, exact affordances, runtime policy, actions, outcome contracts, provenance, and authority because those are all inputs the Meld runtime ultimately consumes.

That makes the examples useful as a **compiler-target corpus**, but it also creates a presentation hazard: the detailed examples can be mistaken for candidate end-user syntax.

This document makes the layering explicit across the example set.

The normalized use-case and route-oriented counterparts are indexed in [PDS Example Router Corpus](README.md).

The architectural refinement is:

```text
user intent surface
    ↓ author / configure
canonical PDS declaration
    ↓ compile and link
compiled stewardship image / semantic IR
    ↓ install stable meaning through domain-owned runtime contracts
operational domain theory
    ↓ combine with activation and cognitive inputs
runtime assembly snapshots
    ↓ execute against
runtime-owned state
```

The user-facing surface and the compiler-facing representation are different products with different stability requirements.

## Layer Definitions

### Layer A — User intent surface

Human-oriented expression of the stewardship outcome and policy.

Typical content:

```text
steward type
scope
desired conditions
sensitivity
autonomy
budget
escalation
verification posture
```

The surface may be a form, conversational editor, concise configuration, SDK, or another product interface. It is not required to expose runtime vocabulary.

### Layer B — Canonical PDS declaration

The durable, diffable, approvable record of what the principal authorized.

It resolves user-facing choices into stable package/profile symbols but should still represent **intent**, not runtime mechanics.

Typical content:

```text
selected steward family/profile
concrete assignment scope
selected package-defined objective presets
policy selections
requested authority
principal identity
activation requirements
package/profile revision constraints
```

This is the primary approval boundary. A principal should be able to understand what this declaration authorizes without reading belief-engine or task-network internals.

### Layer C — Compiled PDS semantic image

The normalized, linked, content-addressed representation emitted by a compiler or facet linker.

Typical content:

```text
resolved package and facet revisions
normalized subject/object identities
belief-family registrations
evidence mapping registrations
curation-rule bindings
maintained-condition lowering
domain propositions and abstract action, outcome, and constraint references
outcome contracts
abstract action-class, constraint, and governance meaning
activation requirements
content hashes and lineage
```

Goals, Methods, exact operators, exact capability contracts, action graphs, and task-package references are not compiled PDS semantics. Stable propositions and action-class meaning may use shared language forms, but situated or reusable cognitive procedure retains its own owner and identity.

### Layer D — Runtime assembly inputs

Independently owned inputs combined only for a current assignment, Goal, and Strategy problem.

Typical content:

```text
current planner snapshot
activation capability snapshot
separately admitted Method snapshot
Agent-selected construction policy
derived projection request
authority request, grant, and restrictions
search engine and traversal controls
physical implementation bindings
```

These inputs do not become PDS theory merely because a compiler or runtime adapter resolves their exact identities.

### Layer E — Domain-owned runtime state

State created while the steward operates.

Examples:

```text
observations and events
graph anchors
accepted evidence
belief revisions
Agent decisions
active goals
planning results
task-network state
capability attempts
approvals
outcomes
current projections
```

This state is never part of the PDS declaration or operational theory body.

## Reading Rule For Existing Examples

The existing examples should now be interpreted as follows:

- `mandate`, `scope`, high-level desired conditions, autonomy, escalation, and verification posture are evidence for **Layers A and B**;
- domain vocabulary, observation models, belief questions, proof semantics, curation semantics, maintained conditions, settlement meaning, abstract action and outcome meaning, and governance classifications are primarily **package/facet semantics and Layer C compiler targets**;
- exact capabilities, Methods, construction policy, projection requests, search controls, and current authority context are **Layer D runtime assembly inputs**;
- example episodes, evidence revisions, current relationships, current exposures, current mastery, current service health, and current goal/task state are **Layer E runtime examples**;
- YAML-like objective fragments and explicit theory IDs are illustrative lowering forms unless explicitly promoted to the canonical declaration contract.

A future user interface should not be judged by how closely it resembles the current detailed examples. A compiler should be judged by whether it can lower a user-approved declaration into the semantics those examples require.

Per-example Layer D details are summarized in the cross-example matrix. The longer cards focus on principal intent, semantic compilation, and resulting runtime state.

## Cross-Example Matrix

| Example | User intent should express | Canonical declaration should select or bind | PDS semantic image must emit or link | Runtime assembly and state own |
|---|---|---|---|---|
| Documentation freshness | keep selected documentation current | repository scope, freshness policy, autonomy, verification | claim proof, freshness question, curation, maintained condition, settlement and outcome meaning | exact docs catalog, Methods, policy, source changes, beliefs, Goals, artifacts, verification outcomes |
| CVE exposure | keep assigned systems within vulnerability-risk policy | system scope, risk policy, exception posture, remediation authority | applicability, coverage, negative proof, curation, maintained condition, settlement and outcome meaning | exact security catalog, Methods, advisories, inventory, beliefs, exceptions, Goals, outcomes |
| Roleplay continuity | keep this character and world coherent across sessions | character scope, canon policy, continuity sensitivity, reveal rules | canon, knowledge, relationship, commitment, disclosure, settlement, and outcome meaning | context capabilities, response Methods, current knowledge, commitments, turns, and responses |
| Lore and canon stewardship | maintain coherent identities and source-backed definitions | corpus scope, identity policy, source authority, merge authority | identity, claim, conflict, provenance, settlement, and verification meaning | extraction catalog, reconciliation Methods, claims, merges, definitions, and projections |
| Codebase quality | maintain selected quality conditions | repository scope, concern profiles, action authority, protected floors | concern questions, protected conditions, settlement, conflict, and verification meaning | validation catalog, repair Methods, evidence, beliefs, Goals, patches, and outcomes |
| Game faction strategy | maintain faction viability and strategic objectives | faction scope, priorities, risk posture, delegated authority | testimony, trust, resource, treaty, settlement, and strategic outcome meaning | simulation catalog, strategic Methods, current reports, beliefs, plans, and operations |
| Service reliability | keep service within SLO and recovery policy | service scope, SLO preset, sensitivity, remediation authority | SLO, fault, capacity, recovery, settlement, and verification meaning | telemetry and control catalog, remediation Methods, incidents, hypotheses, Goals, and outcomes |
| Learner mastery | maintain mastery and prerequisite readiness | learner scope, mastery policy, autonomy, escalation | mastery, prerequisite, misconception, progression, settlement, and verification meaning | assessment and teaching catalog, instructional Methods, mastery beliefs, Goals, and outcomes |
| Portfolio thesis and risk | maintain thesis evidence and risk bounds | portfolio scope, mandate, research policy, execution separation | thesis, counterevidence, risk, liquidity, settlement, and governance meaning | research catalog, recommendation Methods, source evidence, beliefs, approvals, and outcomes |
| Physical maintenance | maintain assets within condition and reliability policy | asset scope, maintenance policy, downtime and authority limits | condition, degradation, failure, safety, service, settlement, and verification meaning | inspection and service catalog, maintenance Methods, telemetry, beliefs, work orders, and outcomes |

## 1. Documentation Freshness

### Layer A — user intent

A user-facing expression could be approximately:

```text
Keep documentation current for /services/payments.
Use normal sensitivity.
You may inspect source, regenerate docs, and open draft changes.
Do not merge changes.
Escalate when source intent is ambiguous.
Require source-backed verification after regeneration.
```

The user does not need to select `content.freshness`, an evidence mapping ID, a comparator, a curation threshold struct, `docs_writer`, or an aggregate outcome contract.

### Layer B — canonical declaration

The canonical declaration should preserve the approved semantics in stable package vocabulary:

```text
steward: software.documentation
scope: repository subtree /services/payments
objective preset: current_for_declared_audience
sensitivity preset: normal
autonomy: observe + verify + draft
merge authority: denied
escalation: ambiguous_source_intent
verification: source_backed
```

Package/profile revisions and the principal/assignment belong here even if the authoring UI hides them.

### Layer C — compiled semantic IR

The compiler/linker resolves the declaration into the detailed forms already exercised by the runtime proof:

```text
subject identity
content-freshness belief-family revision
source and package-outcome evidence mappings
curation rule revision
maintained-condition proposition(s)
settlement and prospective evidence meaning
documentation action-class and outcome meaning
outcome contract and verification mapping
capability and activation requirements
```

The current hand-authored docs-freshness package also contains exact capabilities, policy, bounds, projection dimensions, and an action chain. Those are compatibility inputs for Layer D rather than canonical Layer C output.

### Layer E — runtime

The runtime owns source observations, graph anchors, evidence items, belief revisions, curation decisions, goals, planning results, package runs, generated files, and verification outcomes.

## 2. CVE Exposure

### Layer A — user intent

```text
Watch the production payments services for material CVE exposure.
Treat critical remotely exploitable findings as high sensitivity.
You may inspect dependency and runtime evidence and prepare remediation changes.
Do not deploy or approve risk exceptions automatically.
Escalate unresolved critical exposure.
```

This is a security/risk policy statement, not a declaration of advisory event schemas or Bayesian factors.

### Layer B — canonical declaration

```text
steward: software.vulnerability_exposure
scope: production payments services
risk profile: critical-exposure-strict
exception policy: principal approval required
autonomy: observe + diagnose + draft remediation
deployment authority: denied
verification: independent post-remediation exposure check
```

### Layer C — compiled semantic IR

Compiler output may include:

```text
security.cve_applicability
security.exploitability
security.exposure
security.remediation_confidence
security.evidence_freshness

advisory -> evidence mappings
SBOM/dependency -> evidence mappings
runtime reachability -> evidence mappings
accepted-exception semantics
breach/restore propositions
observation and remediation action-class meaning
verification outcome mappings
governance requirement for deployment/exception approval
```

Exact scanners, observation Methods, remediation Methods, construction policy, and authority decisions remain Layer D inputs even if today's example describes them beside the semantic image.

### Layer E — runtime

The runtime owns currently observed advisories, component versions, applicability beliefs, active exposure, exceptions, remediation goals, package/task state, and post-remediation evidence.

## 3. Roleplay Character Continuity

### Layer A — user intent

```text
Play Mara as a persistent character in the North Road campaign.
Preserve her knowledge, relationships, promises, and unresolved threads.
Use campaign canon as authoritative over generated embellishment.
Never reveal secrets Mara has not learned.
Ask before applying a retcon that changes established canon.
```

The user should not have to author separate belief families for canon support, character knowledge, relationship state, commitment state, and thread state.

### Layer B — canonical declaration

```text
steward: narrative.character_continuity
character: Mara
world/campaign: North Road
canon authority profile: campaign_authoritative
continuity profile: persistent
reveal policy: character_knowledge_only
retcon authority: approval required
response authority: draft/interactive
```

### Layer C — compiled semantic IR

The package compiler or narrative facets may lower that declaration into stable semantic theory:

```text
narrative.canon_support family
character.knowledge family
character.relationship_state family
character.commitment_state family
narrative.thread_state family
perspective identities for world and character
source-authority evidence routes
context-hydration, response, correction, and state-transition action-class meaning
retcon/reveal governance constraints
verification routes for accepted/corrected turns
```

Exact context capabilities, response Methods, and model bindings remain Layer D inputs. The detailed roleplay example spans both layers and must not be read as one PDS body.

### Layer E — runtime

Current relationships, promises, knowledge, secrets learned, open threads, scene state, accepted responses, corrections, and generated response artifacts remain runtime state.

## 4. Lore / Canon Stewardship

### Layer A — user intent

```text
Maintain a living reference for this project.
Track people, places, organizations, objects, and concepts from my notes and conversations.
Preserve source citations.
You may add obvious aliases automatically, but ask before merging ambiguous identities.
Keep generated definitions current when source claims change.
```

### Layer B — canonical declaration

```text
steward: knowledge.canon
scope: project corpus
entity profile: project_reference
provenance: required
alias policy: strong-evidence automatic
identity merge policy: ambiguous requires approval
source authority profile: user_material
summary freshness: maintained
```

### Layer C — compiled semantic IR

```text
entity/claim source vocabulary
lore.identity_match family
lore.claim_support family
lore.definition_completeness family
lore.definition_freshness family
lore.conflict_state family
source-to-claim evidence routes
identity candidate, alias, merge, split, reconciliation, and refresh action-class meaning
merge/split governance requirements
definition outcome verification
```

The current lore example's ontology-like vocabulary is package/facet vocabulary. It should not become a universal PDS language merely because it is detailed.

### Layer E — runtime

Entity records, candidate matches, accepted aliases, claims, contradictions, merge/split decisions, definition artifacts, and current projections are runtime/domain state.

## 5. Codebase Quality

### Layer A — user intent

```text
Maintain this repository's reliability, persistence safety, performance, maintainability, usability, and documentation.
Prioritize persistence and reliability over performance.
You may run validation and open draft fixes.
Do not merge or deploy automatically.
```

### Layer B — canonical declaration

Selects package-defined concern profiles, protected floors, repository scope, observation budgets, draft authority, and escalation requirements.

### Layer C — compiled semantic IR

Links the multiple quality belief questions, proof routes, curation rules, protected-condition propositions, abstract action and outcome meaning, and verification contracts described by the existing software-quality example. Exact validation and repair capabilities, Methods, and construction policy remain Layer D inputs.

### Layer E — runtime

Tests, benchmarks, incidents, belief revisions, conflicting goals, patches, task networks, reviews, and measured outcomes remain runtime state.

## 6. Game Faction Strategy

### Layer A — user intent

```text
Run the Northern Alliance at strategic timescales.
Preserve the capital and food supply.
Avoid breaking treaties without explicit cause.
Scout when threat confidence is low rather than assuming hostile intent.
```

### Layer B — canonical declaration

Binds the faction scope, strategic objective presets, risk posture, treaty constraints, resource authority, and escalation rules.

### Layer C — compiled semantic IR

Links threat, intent, trust, and resource questions with testimony and scouting proof semantics, strategic action-class meaning, protected-condition propositions, and outcome semantics. Exact simulation capabilities and strategic Methods remain Layer D inputs.

### Layer E — runtime

Current sightings, beliefs, treaties, resource state, plans, operations, and territorial outcomes remain simulation/world runtime state.

## 7. Service Reliability

### Layer A — user intent

```text
Keep checkout within its SLO and maintain recovery readiness.
Diagnose before risky remediation when time permits.
You may restart bounded components and prepare rollbacks.
Require approval for region failover or destructive actions.
```

### Layer B — canonical declaration

Binds service scope, SLO/recovery presets, sensitivity, remediation autonomy, blast-radius policy, and escalation.

### Layer C — compiled semantic IR

Links SLO, fault, capacity, and recovery questions with telemetry proof semantics, diagnostic and remediation action-class meaning, verification routes, and governance boundaries. Exact telemetry, control, and remediation capabilities and Methods remain Layer D inputs.

### Layer E — runtime

Metrics, traces, current hypotheses, incidents, goals, remediation tasks, and later SLO/recurrence outcomes remain runtime state.

## 8. Learner Mastery

### Layer A — user intent

```text
Help this learner master algebra through quadratic equations.
Do not advance when prerequisites are uncertain.
Use diagnostic questions before reteaching when the misconception is unclear.
Escalate persistent difficulty to the instructor.
```

### Layer B — canonical declaration

Binds learner/course scope, mastery and progression presets, diagnostic sensitivity, instructional autonomy, budget, and escalation.

### Layer C — compiled semantic IR

Links mastery, prerequisite, misconception, and retention questions with assessment proof semantics, diagnostic and instructional action-class meaning, maintained-condition propositions, and delayed verification contracts. Exact assessment and teaching capabilities and Methods remain Layer D inputs.

### Layer E — runtime

Answers, artifacts, mastery revisions, current misconception hypotheses, active learning goals, activities, and later retention/transfer evidence remain runtime state.

## 9. Portfolio Thesis And Risk

### Layer A — user intent

```text
Maintain the investment theses and mandate-risk view for this portfolio.
Keep invalidating evidence visible.
Refresh stale research before recommendations.
You may recommend changes but may not trade.
```

### Layer B — canonical declaration

Binds portfolio/thesis scope, mandate constraints, research freshness policy, recommendation autonomy, and explicit separation from execution authority.

### Layer C — compiled semantic IR

Links thesis support, invalidation, exposure, liquidity, and freshness questions with primary-evidence semantics, research and recommendation action-class meaning, mandate propositions, outcome history semantics, and governance gates. Exact research capabilities and recommendation Methods remain Layer D inputs.

### Layer E — runtime

Observed sources, thesis revisions, current portfolio/exposure state from external systems, recommendations, approvals, and later outcomes remain runtime state.

## 10. Physical Asset Maintenance

### Layer A — user intent

```text
Maintain these pumps within the approved condition and failure-risk envelope.
Inspect early when degradation evidence is uncertain.
Schedule reversible maintenance automatically within the maintenance window.
Escalate shutdown decisions.
```

### Layer B — canonical declaration

Binds asset scope, condition/risk profiles, inspection sensitivity, maintenance-window policy, action authority, downtime budget, and escalation.

### Layer C — compiled semantic IR

Links condition, degradation, and failure-risk questions with telemetry and inspection proof semantics, inspection and service action-class meaning, maintained-condition propositions, verification routes, and shutdown governance. Exact inspection and service capabilities and maintenance Methods remain Layer D inputs.

### Layer E — runtime

Telemetry, inspections, belief revisions, work orders, maintenance execution, downtime, and subsequent condition/failure evidence remain runtime/external state.

## What The Examples Now Validate

With this refinement, every example should answer **two separate questions**.

### Question 1 — can the user intent be small?

For each domain, can a principal express the desired stewardship configuration without understanding runtime internals?

A useful profile should generally avoid requiring the user to author:

- evidence schema IDs;
- comparator weights;
- graph-anchor coupling;
- belief lease behavior;
- event cursor semantics;
- `meld-lang` ASTs;
- method-to-action realization tables;
- task-network structure;
- runtime actor topology;
- storage layout.

If routine user intent requires those concepts, the abstraction has failed upward even if the runtime representation is complete.

### Question 2 — can the compiler target be general?

Can the declaration compile into the required operational theory without adding application-specific branches to the root runtime?

A new example should normally require:

```text
new package/facet semantics
+ new declaration/profile symbols
+ possibly new domain adapters/capabilities
```

but not:

```text
new PDS runtime state machine
new core belief variant
new core object ontology variant
new root match arm on expression name
```

If every new example changes runtime grammar, the IR is not general enough.

## Compiler Architecture Consequences

### The canonical declaration is not the compiled image

The declaration is what the principal approved. The compiled image is what the runtime can execute.

Both need durable identity and lineage, but for different reasons:

```text
declaration identity
    answers: what did the principal authorize?

compiled image identity
    answers: exactly what semantics did the runtime execute?
```

A runtime record should eventually be traceable to both.

### Compilation is more than serialization

The compiler/linker may need to:

- resolve package/profile/facet imports;
- expand user presets into concrete semantic parameters;
- bind package-defined objectives to concrete subjects;
- lower desired conditions into propositions;
- link observation schemas to installed evidence mappings;
- select or instantiate belief-family configurations;
- emit compatibility requirements for separately owned Methods and activation capabilities;
- validate outcome-contract compatibility;
- intersect requested authority with assignment and activation constraints;
- emit content-hash revisions and lineage;
- reject unresolved symbols or impossible bindings before activation.

The compiled image is therefore an IR/link product, not merely the user's YAML normalized into JSON.

### Domain-owned IR fragments remain preferable to one universal AST

The compiler does not need to own every semantic type centrally.

A viable model is:

```text
canonical PDS declaration
    ↓
PDS linker / compiler coordinator
    ├── world-model facet compiler -> belief/evidence registrations
    ├── Agent facet compiler -> curation/authority bindings
    ├── meld-lang lowering -> stable propositions and semantic references
    ├── execution facet compiler -> outcome and governance meaning
    └── activation requirement linker -> required action classes and connectors
    ↓
compiled stewardship image
```

After semantic compilation, activation resolves exact capabilities and physical bindings. Strategy separately resolves Method and policy snapshots before constructing a current problem.

The image can be a content-addressed manifest over domain-owned compiled fragments rather than a monolithic PDS AST.

## Example Acceptance Test

For each future PDS example, record four artifacts conceptually even if only two are written initially:

1. **Intent sketch** — what a normal principal would say or configure.
2. **Canonical declaration sketch** — stable, approvable product semantics.
3. **Compiler-target decomposition** — the domain theory and linked IR required by the runtime.
4. **Runtime-state scenario** — evidence that the compiled semantics produce the intended persistent behavior.

An example is incomplete if it only has #3. It may prove runtime expressiveness while saying nothing about whether PDS is a usable application abstraction.

An example is also incomplete if it only has #1. It may be a good product idea while having no demonstrated lowering path into the runtime.

The example corpus is strongest when both directions meet:

```text
human intent
    ↓
small canonical declaration
    ↓
complete generic compilation
    ↓
existing domain runtime contracts
    ↓
observable stewardship behavior
```

## Current Interpretation Of The Corpus

Under this model:

- docs freshness is the most complete **runtime-lowering proof**;
- CVE exposure is a strong **semantic-IR pressure test** for uncertain applicability and authority;
- software quality pressures **multi-objective linking**;
- roleplay pressures **perspective, temporal knowledge, and expressive actions**;
- lore pressures **identity reconciliation, provenance, and supersession**;
- service reliability pressures **diagnostic/action risk and delayed verification**;
- learner stewardship pressures **delayed and confounded outcomes**;
- portfolio stewardship pressures **belief/action authority separation**;
- game strategy pressures **perspective and repeated adaptive planning**;
- physical maintenance pressures **sensor evidence, temporal degradation, and consequential action authority**.

None of these examples currently establishes the final user-facing syntax. Collectively, they establish the semantic range that a future PDS declaration compiler must cover.

## Read With

- [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md)
- [Steward Profile Abstraction](../profile_abstraction.md)
- [Stewardship Package Model](../package_model.md)
- [PDS Code Expression Map](../code_expression_map.md)
- [PDS Architectural Invariants](../architectural_invariants.md)
- [PDS Expression Catalog](pds_expression_catalog.md)
- [Roleplay Character Continuity Steward](roleplay_character.md)
- [Lore Transcriber And Canon Steward](lore_transcriber.md)
