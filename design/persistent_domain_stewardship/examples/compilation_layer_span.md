# PDS Examples Across Compilation Layers

Date: 2026-08-08  
Status: illustrative architecture analysis  
Scope: reinterpret the PDS example corpus through the distinction between user intent, canonical declaration, compiled semantic IR, and runtime-owned state

## Purpose

The current PDS examples were intentionally developed from the runtime upward. They describe belief families, evidence routes, maintained conditions, methods, actions, outcome contracts, provenance, and authority because those are the semantic forms the Meld runtime must ultimately consume.

That makes the examples useful as a **compiler-target corpus**, but it also creates a presentation hazard: the detailed examples can be mistaken for candidate end-user syntax.

This document makes the layering explicit across the example set.

The architectural refinement is:

```text
user intent surface
    ↓ author / configure
canonical PDS declaration
    ↓ compile and link
compiled stewardship image / semantic IR
    ↓ lower into domain-owned runtime contracts
operational domain theory
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

### Layer C — Compiled stewardship image / semantic IR

The normalized, linked, content-addressed representation emitted by a compiler or facet linker.

Typical content:

```text
resolved package and facet revisions
normalized subject/object identities
belief-family registrations
evidence mapping registrations
curation-rule bindings
maintained-condition lowering
meld-lang propositions, goals, methods, operators, and effects
available-action bindings
method-to-action realizations
task-package references
outcome contracts
governance requirements
activation requirements
content hashes and lineage
```

Most of the detailed material in the current PDS examples belongs at this layer or in the expert-authored package/facet sources that compile into it.

### Layer D — Domain-owned runtime state

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
- domain vocabulary, observation models, belief families, curation semantics, methods, action meanings, outcome routes, and governance requirements are primarily **package/facet semantics and Layer C compiler targets**;
- example episodes, evidence revisions, current relationships, current exposures, current mastery, current service health, and current goal/task state are **Layer D runtime examples**;
- YAML-like objective fragments and explicit theory IDs are illustrative lowering forms unless explicitly promoted to the canonical declaration contract.

A future user interface should not be judged by how closely it resembles the current detailed examples. A compiler should be judged by whether it can lower a user-approved declaration into the semantics those examples require.

## Cross-Example Matrix

| Example | User intent should express | Canonical declaration should select/bind | Compiler must emit or link | Runtime owns |
|---|---|---|---|---|
| Documentation freshness | keep selected documentation current | repository/subtree, freshness policy, autonomy, verification | content-freshness family, evidence routes, curation, docs action, package route, outcome verification | source changes, freshness revisions, goals, docs runs, generated artifacts, verification outcomes |
| CVE exposure | keep assigned systems within vulnerability-risk policy | software/infrastructure scope, severity/risk policy, exception posture, remediation authority | applicability/exposure families, advisory/SBOM mappings, remediation methods/actions, exception semantics, verification routes | advisories observed, dependency state, exposure beliefs, active exceptions, remediation goals/tasks/outcomes |
| Roleplay continuity | keep this character/world coherent across sessions | character/world scope, canon source policy, continuity sensitivity, reveal/autonomy rules | canon/knowledge/relationship/commitment families, perspective bindings, context-hydration actions, response and correction methods | current character knowledge, relationships, commitments, threads, accepted turns, responses |
| Lore/canon stewardship | maintain coherent people/places/things and source-backed definitions | corpus scope, entity classes/presets, source-authority policy, merge/split authority | identity/claim/freshness/conflict families, extraction mappings, reconciliation actions/methods, definition verification | entity candidates, claims, conflicts, merges/splits, current definitions, provenance history |
| Codebase quality | maintain selected quality conditions | repository scope, selected concern profiles, action authority, protected floors | reliability/performance/etc. families, validation and patch methods, cross-concern action contracts, verification | test/benchmark evidence, concern beliefs, goals, patches, reviews, outcomes |
| Game faction strategy | maintain faction viability and strategic objectives | faction scope, strategic priorities, risk posture, delegated authority | threat/intent/trust/resource families, scouting and strategic actions, methods, outcome semantics | sightings, beliefs, commitments, plans, operations, territory/resource outcomes |
| Service reliability | keep service within SLO/recovery policy | service scope, SLO preset, incident sensitivity, remediation authority | SLO/fault/capacity/recovery families, diagnostic mappings, remediation methods/actions, verification | metrics/traces/incidents, hypotheses, active goals, remediation tasks, SLO outcomes |
| Learner mastery | maintain mastery and prerequisite readiness | learner/course scope, mastery policy, instructional autonomy, escalation | mastery/misconception/retention families, diagnostic and teaching actions, progression methods, outcome semantics | answers, assessed mastery, active misconceptions, instructional goals, activities, retention outcomes |
| Portfolio thesis/risk | maintain thesis evidence and mandate/risk bounds | portfolio/thesis scope, mandate, research policy, execution separation | thesis/risk/freshness families, research mappings, recommendation actions, governance gates | source evidence, thesis revisions, exposure state, recommendations, approvals, later outcomes |
| Physical maintenance | maintain assigned assets within condition/reliability policy | asset scope, inspection/maintenance policy, downtime/authority limits | condition/degradation/failure-risk families, sensor/inspection mappings, service actions/methods, verification | telemetry, inspections, condition beliefs, work orders, service outcomes, failures |

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
planning methods
available action: documentation refresh
method realization -> docs_writer package/workflow route
outcome contract and verification mapping
required capability/provider/activation bindings
```

This is close to what the current hand-authored docs-freshness package represents today.

### Layer D — runtime

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
observation methods for dependency/reachability evidence
remediation action affordances
verification outcome mappings
governance requirement for deployment/exception approval
```

Those are compiler-facing semantics, even if today's example describes them directly.

### Layer D — runtime

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

The package compiler or narrative facets may lower that declaration into:

```text
narrative.canon_support family
character.knowledge family
character.relationship_state family
character.commitment_state family
narrative.thread_state family
perspective identities for world and character
source-authority evidence routes
context-hydration observation actions
respond-with-continuity method
continuity-conflict method
response action and state-transition action as separate affordances
retcon/reveal governance constraints
verification routes for accepted/corrected turns
```

The detailed roleplay example is therefore primarily a specification of what the narrative package/facets and compiler must be able to produce.

### Layer D — runtime

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
identity candidate / alias / merge / split action affordances
reconciliation and refresh methods
merge/split governance requirements
definition outcome verification
```

The current lore example's ontology-like vocabulary is package/facet vocabulary. It should not become a universal PDS language merely because it is detailed.

### Layer D — runtime

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

Links the multiple quality belief families, evidence routes, curation rules, methods, action affordances, protected-condition propositions, and verification contracts described by the existing software-quality example.

### Layer D — runtime

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

Links threat/intent/trust/resource belief families, testimony and scouting evidence routes, scouting and negotiation actions, strategic methods, protected-condition propositions, and outcome semantics.

### Layer D — runtime

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

Links SLO/fault/capacity/recovery belief families, metrics/traces/outcome evidence routes, diagnostic actions, remediation methods, action costs, verification routes, and governance boundaries.

### Layer D — runtime

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

Links mastery/prerequisite/misconception/retention families, assessment evidence routes, diagnostic and instructional actions, progression methods, maintained-condition propositions, and delayed verification contracts.

### Layer D — runtime

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

Links thesis-support/invalidation/exposure/liquidity/freshness families, primary-evidence routes, research actions, recommendation methods, mandate propositions, outcome history semantics, and governance gates.

### Layer D — runtime

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

Links condition/degradation/failure-risk families, telemetry and inspection evidence routes, inspection/service actions, maintenance methods, maintained-condition propositions, verification routes, and shutdown governance.

### Layer D — runtime

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
- resolve methods against available actions and capabilities;
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
    ├── meld-lang lowering -> propositions/methods/operators
    ├── execution facet compiler -> actions/packages/outcomes
    └── activation linker -> providers/capabilities/connectors
    ↓
compiled stewardship image
```

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
