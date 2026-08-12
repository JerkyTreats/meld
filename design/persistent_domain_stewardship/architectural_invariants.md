# Persistent Domain Stewardship Architectural Invariants

Date: 2026-08-08  
Status: proposed architectural constraints derived from the current runtime and the expanded PDS example corpus  
Scope: cross-expression architectural concerns that should remain stable while PDS declarations, packages, and application-specific facets evolve

## Purpose

The PDS examples now span software quality, documentation freshness, CVE exposure, service reliability, learner mastery, portfolio thesis/risk, roleplay continuity, lore/canon stewardship, game strategy, and physical maintenance.

The examples differ substantially in vocabulary and intervention style. The useful commonality is therefore not a universal ontology. It is a set of architectural boundaries that keep the runtime general while allowing durable domain semantics to vary.

This document records those recurring invariants and relates them to the current codebase. It should be read with [PDS Code Expression Map](code_expression_map.md), which distinguishes current implementation from design gaps.

## Invariant 1 — PDS Is A Linked Composition, Not A New Runtime

A persistent domain steward is assembled from domain-owned contracts:

```text
selection / declaration
+ operational domain theory
+ physical bindings
+ existing runtime domains
```

PDS must not become a second event ledger, graph, belief engine, Agent runtime, planner, goal store, task runtime, or capability system.

**Current code:** held. The docs-freshness proof composes root configuration with world-model, Agent, meld-lang, execution, task-package, and event contracts rather than introducing a PDS engine.

**Pressure from new designs:** roleplay and lore require additional domain semantics, but they do not justify a PDS-owned memory database or ontology runtime.

## Invariant 2 — Application Semantics Must Not Branch In Root Runtime Code

Application vocabulary belongs in installed theory or a domain-owned facet, not in root runtime `match` arms.

The current docs proof contains explicit compatibility branches mapping `docs_freshness` to `docs_writer`. Those branches are transitional, not the extension model.

A generic PDS architecture should satisfy:

```text
new expression
    => new declaration / theory / facet / binding
    != new root semantic branch
```

**Current code:** partially held. Belief, evidence mapping, methods, actions, and realizations are data-driven, but root package selection remains docs-specific.

**Continuation:** the next declaration/linking layer should remove expression-name dispatch as the way new PDS applications enter runtime composition.

## Invariant 3 — Operational Theory Is State-Free; Runtime State Is Domain-Owned

Theory declares meaning:

- belief families;
- evidence routes;
- curation rules;
- planning methods;
- available actions;
- method realizations;
- task-package semantics;
- maintained conditions;
- authority/governance declarations.

Theory does not own current observations, graph anchors, belief revisions, Agent decisions, goals, task state, capability attempts, or measured outcomes.

**Current code:** strongly held for the implemented slice. Durable runtime state remains in the owning domains.

**Pressure from new designs:** a lore package may declare what a canon claim means, but the current claims, disputes, and reconciliations remain runtime/domain state; a roleplay package may declare knowledge semantics, but current character knowledge remains runtime state.

## Invariant 4 — Exact Semantic Revision Must Be Recoverable

Any installed theory that changes runtime meaning must have stable identity and exact revision lineage.

The desired pattern is:

```text
semantic identity
+ content hash
+ append-only revisions
+ current selection
+ historical resolution
```

**Current code:** held for belief-family theory and the curation rule bound to an Agent record; incomplete for evidence mappings, methods, actions, realizations, and package theory.

**Continuation:** each owning domain should gain durable revision resolution where replay depends on that theory. A single central PDS registry is not required.

## Invariant 5 — Domain Identity Is Explicit; Domain Ontology Remains Open

Subjects and relationships must have stable identity, but PDS must not freeze one universal object taxonomy.

The runtime should continue to carry generic object, relation, dimension, predicate, action, and artifact identities as data.

**Current code:** held through generic domain object references, relation records, belief dimensions, methods, and capability identities.

**Pressure from new designs:** `character`, `place`, `CVE`, `service`, `portfolio_position`, and `learning_concept` should not become core Rust variants merely because several packages use them.

## Invariant 6 — Perspective And Branch Are Part Of Meaning

A statement can be true or current for one perspective or branch and not another.

Perspective and branch must therefore be explicit in durable identity rather than hidden process context.

**Current code:** held in `PerspectiveKey`, `BranchScope`, belief keys, and graph anchor selection.

**Pressure from new designs:** roleplay makes the distinction especially visible:

```text
world/canon truth
!= character knowledge
!= another character's knowledge
!= what a response may reveal
```

The first three can build on current perspective machinery. Reveal/visibility authority still needs domain theory and governance.

## Invariant 7 — Provenance Is Preserved, Not Flattened Into Current State

A steward must be able to reconstruct why a belief, relationship, decision, or action existed.

Current state is a projection over historical evidence; it must not erase its sources.

**Current code:** strongly held across event ids, graph anchor provenance, evidence records, belief revision lineage, curation input references, goal sources, planning frame identity, and task/outcome records.

**Pressure from new designs:** lore definitions and roleplay continuity make provenance user-visible, but do not require a new provenance subsystem.

## Invariant 8 — Contradiction, Correction, And Supersession Are Non-Destructive

Conflicting evidence must remain independently inspectable. New information should not silently rewrite history.

The architecture should distinguish at least:

```text
support
counterevidence
supersession
invalidation/correction
current projection
```

**Current code:** held at belief level through evidence roles and contradiction state; current anchors also retain ended history.

**Continuation:** domains that require claim-level correction, alias merge/split, or canon dispute semantics should define those as domain-owned records rather than overloading the generic belief contradiction type.

## Invariant 9 — Runtime Sequence, Reference Time, Freshness, And Domain Valid Time Are Different

PDS designs repeatedly require temporal continuity, but there is not one universal timestamp.

Keep separate:

- runtime/event sequence;
- source/reference time;
- freshness/decay state;
- current-selection interval;
- domain semantic valid time, when a domain needs it.

**Current code:** the first four have concrete representation across events, beliefs, and graph anchors.

**Pressure from new designs:** lore may require a claim such as "X ruled from year A to year B." That is domain valid time and should not be encoded as a runtime transaction sequence.

## Invariant 10 — Standing Responsibility Is Separate From Transient Goals

A steward remains responsible after one goal or task is satisfied.

Therefore:

```text
standing maintained condition
    causes zero or more transient goals over time
```

A goal is not the stewardship mandate itself.

**Current code:** gap. Goals are strongly typed and durable, and satisfied goals can reopen through lifecycle epochs, but the maintained condition that explains why the goal family continues to exist is not a first-class runtime contract.

**Continuation:** this is a priority design area because every PDS example depends on it.

## Invariant 11 — Observation Is An Action Choice, Not Merely Missing Data

Partial observability is a defining PDS property. The steward may choose to acquire evidence rather than immediately intervene.

The architecture should preserve the distinction:

```text
missing/uncertain evidence
→ observation opportunity
→ optional observation action
→ new evidence
```

**Current code:** partial. Belief exposes typed observation opportunities; generic observation-policy declaration and realization are not yet a complete installed PDS theory path.

**Pressure from new designs:** roleplay may hydrate a prior episode; lore may retrieve neighboring source passages; service reliability may collect a trace; software stewardship may run a targeted test. These are different actions over one common observation-choice concept.

## Invariant 12 — Capability Availability Is Not Authority

The runtime may know how to perform an action without granting a steward permission to perform it.

The required relationship is:

```text
available capability
∩ package requested authority
∩ principal grant
∩ runtime policy
∩ current restrictions
= effective authority
```

**Current code:** capability and effect contracts are typed; the generic PDS grant/governance intersection is not implemented.

**Continuation:** authority is a first-class gap and must be designed before PDS is used for materially consequential interventions.

## Invariant 13 — Task Completion Is Not Stewardship Success

An execution completing mechanically does not prove that the maintained condition was restored.

The architecture must preserve:

```text
expected effect
!= execution completion
!= observed outcome
!= verified restoration
```

**Current code:** held in the docs-freshness vertical through canonical package outcomes, config-driven outcome evidence mapping, belief reassessment, and Agent satisfaction curation.

**Continuation:** generalize outcome-contract installation and revision lineage without collapsing verification into task state.

## Invariant 14 — Context Hydration Is Bounded And Derived

Persistent stewardship does not require one ever-growing prompt or one authoritative narrative blob.

The durable source remains event, graph, belief, decision, goal, task, artifact, and outcome history. A model receives a bounded projection or hydration of the material needed for the current decision.

**Current code:** partially held. Belief exposes hydration references and the context domain provides immutable frames and heads.

**Pressure from new designs:** roleplay and lore make selective episodic/entity hydration central. The missing concern is a generic projection-selection contract, not a requirement to keep all history in active model context.

## Invariant 15 — Storage Mechanism Is Not A PDS Semantic Invariant

Durability, deterministic identity, replay, and provenance are semantic requirements. A specific storage algorithm is not.

In particular:

```text
FrameMerkleSet is implemented context infrastructure
but
Merkle tree is not part of the canonical PDS theory/runtime contract
```

A PDS may use Merkle-backed context, sled registries, an event ledger, source-system records, or other domain-owned storage as appropriate.

**Current code:** held by architecture; PDS contracts span several storage mechanisms already.

**Continuation:** do not promote context-frame or Merkle implementation details into the PDS declaration grammar unless a future semantic requirement specifically demands them.

## Invariant 16 — Domain-Specific Reconciliation Belongs In Domain Theory Or Facets

Some concerns recur across examples but have incompatible semantics:

- entity alias/merge/split;
- narrative canon;
- CVE applicability;
- service causal diagnosis;
- learner misconception;
- portfolio thesis invalidation.

The common runtime should supply identity, evidence, belief, perspective, planning, outcomes, and authority boundaries. The interpretation itself remains domain-owned.

**Current code:** mostly held. Runtime vocabulary is generic and family semantics live in configuration.

**Continuation:** new lore/roleplay support should pressure-test a facet boundary before adding universal PDS schema fields for claim or character semantics.

## Invariant 17 — Compatibility Forms Must Remain Explicitly Transitional

The first proof necessarily has hand-authored and root-composed forms:

- `DocsFreshnessSelection`;
- `StewardshipTheoryBindings` injection;
- built-in `docs_writer` package selection;
- root-derived first-proof perspective/branch/session conventions.

These are valuable because they prove real runtime seams. They should not silently become the canonical declaration schema merely because they exist in code.

**Continuation:** every compatibility form should either lower from the future declaration layer or be retired when its owning durable registry/linking contract lands.

## Invariant 18 — A New Expression Must Falsify The Need For Kernel Change

Every new PDS expression should first attempt to fit the existing runtime grammar.

A successful expression should normally add:

```text
domain theory
+ domain-owned adapters/facets
+ physical bindings
+ tests
```

It should not normally add:

```text
new event-loop states
new planner branches
new belief subsystems
new task lifecycle states
new root semantic switches
```

If a new expression truly requires a new generic kernel mechanism, that mechanism should be justified independently of the application's vocabulary.

## Cross-Design Concern Register

| Concern | Docs / software | CVE / reliability | Roleplay | Lore/canon | Current common runtime | Main gap |
|---|---|---|---|---|---|---|
| bounded subject identity | yes | yes | yes | yes | graph/object identity | none at common layer |
| persistent mandate | yes | yes | yes | yes | indirect through curation/goals | first-class maintained condition |
| partial observability | yes | yes | yes | yes | belief/evidence | observation-policy installation |
| provenance | yes | yes | yes | yes | event/graph/belief lineage | domain claim presentation only |
| contradiction | yes | yes | yes | yes | belief contradiction | claim-level domain reconciliation when needed |
| perspective | moderate | moderate | central | useful | perspective + branch | reveal/knowledge policy for roleplay |
| temporal continuity | yes | yes | central | central | seq/reference/freshness/anchor history | domain valid time where required |
| intervention alternatives | yes | yes | yes | yes | methods/actions/capabilities | durable planning-theory installation |
| verification | yes | yes | conversational feedback | user/source reconciliation | outcome -> evidence loop | generic outcome-contract registry |
| authority | yes | critical | output/canon boundary | canon-edit boundary | capability effects only | principal grants/governance |
| context hydration | useful | diagnostic | central | central | hydration refs/context domain | generic projection owner |
| domain reconciliation semantics | software-specific | vulnerability/diagnosis-specific | narrative-specific | entity/claim-specific | generic evidence/belief substrate | domain facets, not kernel ontology |

## Continuation Priorities

The invariant set produces a smaller architecture agenda than simply implementing every field in the example documents.

### Priority A — Generic declaration and linking

Create an upper-layer declaration/linking target that can express more than docs freshness without root semantic branches.

### Priority B — Standing maintained conditions

Settle the authoritative representation of the persistent responsibility that survives goal completion.

### Priority C — Authority and governance

Define principal grants, action classes, approvals, budgets, prohibitions, escalation, and effective authority lineage separately from capability availability.

### Priority D — Complete theory revision installation

Bring outcome mappings, planning methods, available actions, method realizations, and package/outcome theory to the same exact-revision standard already established by belief families where replay requires it.

### Priority E — Generic outcome verification declaration

Make action outcome meaning installable and replayable without application-specific root code.

### Priority F — Context projection contract

Define bounded hydration/projection only after identifying what runtime domain should own it. Reuse context frames where useful; do not make them mandatory PDS state.

### Priority G — Domain facets for new examples

Implement lore claim/identity semantics and roleplay knowledge/reveal semantics as domain facets or theory after the common boundaries above are stable.

## Acceptance Rule

A proposed architecture change belongs in the common PDS/runtime layer only if at least two dissimilar expressions need the same semantic mechanism **and** the mechanism can be stated without application vocabulary.

Otherwise it belongs in a domain package, facet, adapter, or application-specific capability.

That rule is the principal defense against turning the expanding PDS example corpus into an expanding kernel.