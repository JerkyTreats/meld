# PDS Code Expression Map

Date: 2026-08-08  
Status: analysis  
Evidence base: `runtime-completion` as inspected on 2026-08-08  
Scope: how Persistent Domain Stewardship is actually represented by the current Meld codebase, with explicit separation between enforced runtime contracts, compatibility wiring, reusable substrate, and design-only concepts

> Evidence status: historical pre-elevation snapshot. Read with the [fresh review through Theory Elevation Step 4](../completed/integration/theory_elevation_steps_1_through_4_fresh_review.md) and the [Current Architecture Overhauls](../plan/README.md). This map provides evidence only and carries no current delivery authority.

> Canonical boundary status: [Persistent Domain Stewardship](../cognitive_architecture/persistent_domain_stewardship.md) supersedes the earlier interpretation that Methods, exact available actions, realizations, and task-package selections are PDS operational theory. This file preserves the historical code map.

## Purpose

Before extending PDS with additional applications, establish what PDS means in code today.

The central finding is that there is **no single PDS runtime object**. The current docs-freshness proof expresses one stewardship application by linking contracts owned by several domains:

```text
root configuration selection
    ↓
physical binding
    ↓
root composition bindings
    ↓
world-model theory and state
    ↓
Agent curation
    ↓
meld-lang goals / methods / propositions
    ↓
execution planning / actions / packages
    ↓
canonical outcomes
    ↓
world-model evidence and revised belief
```

This is consistent with the canonical theory-to-runtime design: PDS supplies state-free operational domain theory and selection; existing runtime domains own observations, beliefs, decisions, goals, execution, and outcomes.

The purpose of this map is therefore not to ask whether "PDS is implemented." It asks, concept by concept:

1. Does a Rust contract exist?
2. Is its semantic meaning generic or docs-freshness-specific?
3. Is the contract durable and replayable where persistence matters?
4. Is the PDS meaning enforced by the owning domain, or only wired by root composition?
5. What remains design work before another PDS expression can use the same architecture without adding application branches to the runtime?

## Classification

This document uses five implementation classes.

| Class | Meaning |
|---|---|
| `enforced` | A typed runtime/domain contract exists, validates its invariants, and has the durability or lineage required by its semantics. |
| `typed-partial` | A typed contract exists, but the generic semantics, durability, or enforcement boundary are incomplete. |
| `compatibility` | A typed first-proof form exists but is docs-specific, process-injected, shipped/built-in, or rooted in explicit transitional wiring. |
| `substrate` | Implemented infrastructure that a PDS may use, but which is not itself part of the PDS semantic contract. |
| `design` | The PDS concept has no runtime representation that currently enforces it. |

`strongly typed` is not treated as a synonym for `implemented`. Many current contracts have Rust struct/enum shapes while their semantic identities are still `String`, their revisions are not durably installed, or their expression selection is hard-coded to docs freshness.

## Current Concrete Expression

The current root-level expression is docs freshness.

### Selection

`src/config/stewardship/selection.rs`

```text
StewardshipConfig
└── docs_freshness: Option<DocsFreshnessSelection>
    ├── expression
    ├── target_root
    ├── subject
    ├── agent_id
    ├── provider_id
    └── TheorySelection
        ├── belief_family_id
        ├── evidence_mapping_id
        └── curation_rule_id
```

The schema is typed and validated, but it is explicitly one expression shape. Validation rejects any expression other than `docs_freshness`.

### Physical binding

`src/config/stewardship/binding.rs`

`PhysicalBinding` resolves the selected target, subject, agent, provider, storage root, and selected theory identities without creating semantic state. This is a real stage-0 contract, but still a docs-freshness-specific compatibility form.

### Composition

`src/runtime/assembly.rs`

`StewardshipActorBindings` derives the actor-facing identities and physical scope. `StewardshipTheoryBindings` carries the theory kinds that do not yet have durable registries:

```text
StewardshipTheoryBindings
├── outcome_mapping: Option<OutcomeMappingSetConfig>
├── planning: Option<PlanningTheoryBinding>
└── dispatch: Option<DispatchRouteBindings>
```

`PlanningTheoryBinding` contains:

```text
methods
capability_catalog
available_actions
method_realizations
requested_dimensions
```

This is explicitly an injection seam, not a canonical PDS package representation.

### Docs-specific root knowledge

Root assembly still contains docs-freshness-specific application knowledge. In particular:

- `StewardshipActorBindings::derive` fixes the first proof to the default perspective and `main` branch.
- package-derived helper functions match `expression == "docs_freshness"` and select the built-in `docs_writer` package.
- network, session, frame-type, and package-route identities are derived from first-proof conventions.

These are acceptable compatibility forms for the first vertical slice. They are **not** a viable extension point for additional PDS expressions. A second expression should not require another root `match expression` arm.

## Concept Map

### PDS control plane and linking

| Concern | Current code | Shape | Generality / durability | Class | Assessment |
|---|---|---|---|---|---|
| Expression selection | `StewardshipConfig`, `DocsFreshnessSelection` in `src/config/stewardship/selection.rs` | typed struct | docs-only; config-resident | `compatibility` | The selection boundary is real, but the type itself names one application. |
| Physical scope | `PhysicalBinding` in `src/config/stewardship/binding.rs` | typed struct | docs-only binding; pure derivation | `compatibility` | Correct stage-0 separation, not yet a general assignment/activation model. |
| Selected theory identities | `TheorySelection`, `SelectedStewardshipPackage` | typed fields, string identities | only belief family, evidence mapping, curation rule | `typed-partial` | Planning theory, task package, outcome contract, authority, and maintained-condition identities are not part of the selection. |
| Actor-facing binding | `StewardshipActorBindings` in `src/runtime/assembly.rs` | typed struct | process-derived; first-proof conventions | `compatibility` | Useful adapter, but it currently contains application-specific derivations. |
| Aggregate theory binding | `StewardshipTheoryBindings` | typed struct | process-local injection | `typed-partial` | It proves the seams but is intentionally not durable package/image state. |
| Canonical PDS declaration | none | none | none | `design` | No generic declaration exists above the current docs selection. |
| PDS package/image compiler | none | none | none | `design` | Current JSON/config/package inputs are hand-authored compatibility antecedents. |
| Assignment/principal record | none | none | none | `design` | `PhysicalBinding` is not a normative assignment with principal and authority grant. |
| PDS activation record | none | none | none | `design` | Agent runtime activation is a different concept; PDS activation remains proposal-tier. |
| Unified PDS projection | none | none | none | `design` | Runtime truth exists across domains, but no generic stewardship projection is implemented. |

### Domain identity, graph, and perspective

| Concern | Current code | Shape | Generality / durability | Class | Assessment |
|---|---|---|---|---|---|
| Subject identity | `DomainObjectRef` consumed throughout graph/belief/runtime | typed object reference | generic; durable in events and world-model records | `enforced` | PDS expressions can name bounded subjects without adding Rust variants. |
| Relations | `EventRelation`, graph traversal records | typed relation records with runtime relation ids | generic; durable facts/indexes | `enforced` | Relation vocabulary remains data, which is appropriate for dissimilar domains. |
| Current anchor | `AnchorSelectionRecord` | typed append/history record | generic; durable | `enforced` | Current subject-to-target selection has explicit start/end sequence and provenance. |
| Perspective | `PerspectiveKey` | typed struct | generic; durable | `enforced` | Different current views can be represented without application-specific graph types. |
| Branch scope | `BranchScope` on `BeliefKey` | typed struct | generic; durable in belief identity | `enforced` | Belief replay is not dependent on hidden process branch state. |
| Generic entity/canon claim | no dedicated claim contract | graph objects/relations plus evidence only | no claim lifecycle/authority semantics | `design` | Lore-style claims need a domain-owned claim model if graph facts alone are insufficient. |
| Alias / merge / split identity | none as generic domain semantics | none | none | `design` | Generic graph identity does not define entity reconciliation policy. |

### Belief and evidence

| Concern | Current code | Shape | Generality / durability | Class | Assessment |
|---|---|---|---|---|---|
| Belief family theory | `BeliefFamilyConfig` | typed config | generic runtime ids | `enforced` | Family semantics are data rather than Rust subsystems. |
| Belief family installation | `BeliefFamilyRegistry`, `BeliefFamilyRegistryStore` | typed trait + sled implementation | durable content-hash revisions | `enforced` | This is the most complete PDS theory registry today. |
| Theory lineage | `TheoryRevisionRef` on belief revisions/views | typed revision reference | durable for belief families | `enforced` | Exact family revision can be reconstructed from durable state. |
| Evidence value | `EvidenceValue` | typed generic enum | generic | `enforced` | Values are intentionally generic; domain-specific schemas remain configuration. |
| Evidence role | `EvidenceRole` | typed enum | generic | `enforced` | Support, contradiction, context, calibration, and supersession are explicit. |
| Evidence provenance | `EvidenceItem`, `BeliefProvenanceSummary`, hydration refs | typed records | durable | `enforced` | Source facts, anchors, objects, relations, and revisions remain inspectable. |
| Freshness | `FreshnessState`, `FreshnessReason` | typed state/reasons | durable belief state | `enforced` | Current epistemic freshness is a first-class belief concern. |
| Contradiction | `ContradictionState`, `ContradictionReason` | typed state/reasons | durable belief state | `enforced` | Counterevidence is preserved rather than silently overwritten. |
| Observation opportunity | `ObservationOpportunity`, `ObservationReason` | typed output | durable/projected belief state | `typed-partial` | The world model can say what evidence is missing; generic stewardship observation-policy selection is not separately modeled. |
| Outcome evidence mapping | `OutcomeMappingSetConfig`, `ConfiguredOutcomeMappingSet` | typed config + runtime implementation | generic mapping semantics, process injection | `typed-partial` | Semantics are data-driven, but there is no durable mapping registry/revision lineage yet. |
| Evidence-mapping revision | selected mapping id only | string identity | no durable mapping revision registry | `design` | Replay can cite mapping identity but cannot resolve an installed historical content hash through a domain registry. |

### Temporal semantics

The codebase already has several kinds of time, but they should not be conflated.

| Temporal concern | Current representation | Class | Boundary |
|---|---|---|---|
| Event/transaction ordering | ledger and world-model sequence fields | `enforced` | Runtime causal order. |
| Evidence reference time | `EvidenceItem.reference_time`, promoted evidence reference time | `enforced` | Source-reported time when evidence is about. |
| Belief freshness | `FreshnessState` and reasons | `enforced` | Whether the current belief needs reassessment. |
| Anchor current interval | `selected_at_seq`, `ended_at_seq` | `enforced` | Runtime interval during which an anchor was current. |
| Semantic valid-time claim | no generic claim validity interval | `design` | Needed by domains such as lore when "true in-world from year X to Y" differs from runtime sequence. |
| Correction vs supersession semantics | belief evidence roles and anchor replacement exist | `typed-partial` | Generic claim-level correction/retraction/supersession is not a shared runtime contract. |

The roleplay and lore examples therefore do **not** justify replacing the existing time model. They expose a possible domain-level valid-time claim contract above the already implemented event sequence, reference time, freshness, and anchor history mechanisms.

### Agent, mandate, and curation

| Concern | Current code | Shape | Generality / durability | Class | Assessment |
|---|---|---|---|---|---|
| Durable agent identity | `AgentRecord`, `AgentStore` | typed record/store | generic; durable | `enforced` | The acting perspective has a stable durable identity. |
| Agent perspective and subject | fields on `AgentRecord` | typed references | generic; durable | `enforced` | Agent scope is explicit. |
| Directive/mandate | `AgentRecord.directive: String` | unstructured string | generic text, durable | `typed-partial` | Intent is recorded but not represented as maintained conditions or authority. |
| Curation rule | `AgentCurationRuleConfig` | typed threshold rule | one current rule family | `typed-partial` | Strongly typed for the first slice, not a general curation theory language. |
| Curation rule revision | `AgentCurationRuleBinding` content hash on `AgentRecord` | typed + durable | exact installed rule on agent | `enforced` | Current curation semantics have a durable home and integrity check. |
| Curation decision | `AgentCurationDecision` and input refs | typed + durable | generic decision record | `enforced` | The runtime can reconstruct why a goal/mutation was produced. |
| Standing objective / maintained condition | no first-class stewardship objective record | indirect through curation rule + goals | not separately durable | `design` | This is one of the main gaps exposed by all PDS examples. |
| Episode | no generic PDS episode authority | runtime goals/tasks/outcomes only | projected/implicit | `design` | Whether episodes should exist as authoritative state remains unresolved. |

### Goals and planning

| Concern | Current code | Shape | Generality / durability | Class | Assessment |
|---|---|---|---|---|---|
| Transient goal | `meld_lang::Goal` | typed struct | generic | `enforced` | Target, source, priority, agent, and lifecycle are explicit. |
| Goal lifecycle | `GoalLifecycle` plus execution goal stores | typed enum + durable store | generic; durable | `enforced` | Active/satisfied/suspended/abandoned work is runtime state. |
| Goal provenance | `GoalSource` | typed enum | generic | `enforced` | Maintenance and belief-divergence sources exist, but do not replace a standing objective record. |
| Planning method | `meld_lang::Method`, `MethodLibrary` | typed IR + verification | generic; loaded/injected | `typed-partial` | Method semantics are strongly typed but no durable theory registry exists. |
| Available action | `AvailableActionBinding`, `AvailableActionSet` | typed affordance shape | generic; injected | `typed-partial` | Correct generic shape, not yet installed theory. |
| Method-to-action realization | `MethodRealizationBinding` | typed association | generic; injected | `typed-partial` | Runtime selection is data-driven, but association revisions are not durable theory. |
| Capability resolution | capability catalog + operator resolution | typed contract | generic; process/runtime catalog | `enforced` as execution substrate | Availability is represented; authority is not implied. |
| Planning world-state provenance | `PlanningWorldStateFrameRef`, execution composition lineage | typed | generic | `enforced` | Plan identity includes exact projection frame identity. |

### Task packages and outcomes

| Concern | Current code | Shape | Generality / durability | Class | Assessment |
|---|---|---|---|---|---|
| Task package | `TaskPackageSpec` | typed authored contract | generic shape; built-in/external loader | `typed-partial` | Strong package structure, but it is not a durable PDS theory registry. |
| Package selection | root helper maps docs expression to `docs_writer` | root match arm | docs-only | `compatibility` | This must disappear as the extension mechanism for new expressions. |
| Package execution | task compiler/network/progress/dispatch | typed + durable | generic execution substrate | `enforced` | Execution owns task state as required by the PDS authority boundary. |
| Action outcome contract reference | `ActionOutcomeContractRef` | typed wrapper containing string id | generic identity | `typed-partial` | The action can name an outcome contract, but there is no generic installed outcome-contract registry. |
| Canonical package outcome | task-network publication records/contracts | typed + durable event publication | package route | `enforced` for current route | Mechanical completion has a canonical event boundary. |
| Verification loop | outcome mapping -> evidence -> belief -> Agent satisfaction curation | typed across several domains | proven for docs slice | `typed-partial` | The loop exists, but reusable outcome-policy declaration and installation remain incomplete. |

### Authority and governance

| Concern | Current code | Shape | Generality / durability | Class | Assessment |
|---|---|---|---|---|---|
| Capability side-effect declaration | `EffectSpec`, `EffectKind`, `CapabilityTypeContract` | typed | generic | `substrate` | Describes what an executable capability may do, not whether this steward is authorized to do it. |
| Policy reference binding | `BindingValueKind::PolicyRef` | typed enum variant | generic hook | `substrate` | A place to reference policy is not an implemented PDS authority model. |
| Principal grant | none | none | none | `design` | No durable grant intersects assignment request, principal authority, and runtime policy. |
| Approval requirement | none as generic PDS contract | none | none | `design` | Domain workflows may contain gates, but that is not PDS governance. |
| Budget / rate / intervention limits | scattered execution/config controls | no unified PDS authority semantics | none | `design` | PDS-governed limits remain proposal-tier. |
| Prohibition | none as stewardship grant semantics | none | none | `design` | Capability presence still must not be treated as permission. |
| Escalation boundary | curation can remain indeterminate; workflow gates exist | indirect | no generic authority semantics | `design` | A first-class governance result is still needed for consequential applications. |

Authority is therefore the clearest place where future PDS work must **not** infer implementation from nearby typed capability machinery.

## New-Design Pressure Tests

The roleplay-continuity and lore/canon examples exercise existing mechanisms in new combinations. They do not automatically imply new kernel concepts.

### Roleplay continuity

| Required concept | Existing support | Gap |
|---|---|---|
| durable subject/persona identity | `DomainObjectRef`, `AgentRecord` | domain identity policy still belongs to the application/domain facet |
| multiple perspectives | `PerspectiveKey`, belief perspective, branch scope | no generic visibility/reveal policy or character-knowledge authority boundary |
| contradictory memories/reports | evidence roles + contradiction state | no narrative-specific reconciliation policy, which should remain domain theory |
| unresolved threads/commitments | graph/object relations can carry data | no maintained-condition model that says which commitments remain standing responsibilities |
| cross-session continuity | durable events, graph, belief, goals, context storage | no PDS-owned context projection contract |
| response generation | provider/capability/workflow machinery | not itself PDS semantics |

### Lore/canon stewardship

| Required concept | Existing support | Gap |
|---|---|---|
| entities and relations | generic graph object/relation model | no canon-claim object with domain-valid-time semantics |
| provenance | event, graph, and belief provenance | strong existing substrate |
| contradiction | belief contradiction and supersession roles | claim-level canonical/disputed/retracted semantics remain domain-specific |
| alias resolution | generic identity references | no generic merge/split/alias decision contract |
| derived definitions | belief/projection machinery and task generation | no declared standing freshness condition for derived entity definitions |
| active disambiguation | observation opportunities | no generic observation-policy declaration linking ambiguity to acquisition actions |

The main architectural conclusion is that alias resolution, character visibility, narrative canon, and similar vocabulary should **not** be promoted into the PDS kernel merely because these examples need them. They are candidate domain-owned theory/facet semantics implemented over common identity, evidence, belief, planning, and outcome contracts.

## Context Frames And Merkle Trees

The context domain is implemented and strongly typed:

- `Frame` is an immutable, content-addressed context record.
- `FrameMerkleSet` maintains a deterministic set of frame ids and a BLAKE3 Merkle root.
- frame storage, heads, generation, query, and context capabilities exist under `src/context/`.

This does **not** make Merkle structure a PDS invariant.

Current canonical PDS theory-to-runtime contracts do not mention Merkle trees. A PDS may use context frames as a context/hydration or artifact substrate when a domain integration needs them. The belief domain also exposes `HydrationRefs` without requiring a context-frame implementation.

Therefore:

```text
PDS requires durable identity, provenance, replayable state, and bounded hydration

PDS does not require
    Frame
    FrameMerkleSet
    a Merkle tree
    filesystem context frames
```

If roleplay continuity later benefits from content-addressed episodic context, the context domain may be a useful adapter. That is an implementation choice at the context boundary, not a new PDS semantic dependency.

## Codebase Domain Map

| Codebase domain | Current PDS relationship | Strength | Main continuation |
|---|---|---|---|
| Root config / composition | owns first-proof selection, physical binding, actor composition | `compatibility` | general declaration/linking; remove application-specific root branches |
| Events | canonical append/replay and durable consumer progress | `substrate`, strongly implemented | no PDS-specific change unless a new required event contract is discovered |
| World-model graph | generic objects, relations, current anchors, provenance | `enforced` substrate | add only domain-owned claim semantics when a use case proves graph facts insufficient |
| World-model belief | family theory, durable registry, evidence, freshness, contradiction, observation opportunities | strongest current PDS semantic implementation | durable evidence-mapping registry; additional family semantics as data |
| World-model Agent | durable identity, subscriptions, curation decisions, installed threshold rule | `typed-partial` | structured maintained conditions, broader curation theory, governance boundary |
| `meld-lang` | propositions, conditions, goals, methods, operators, effects, world state | `enforced` generic IR | preserve domain independence; avoid PDS-specific syntax unless proven generic |
| Execution goals | durable transient-goal lifecycle | `enforced` | keep separate from standing stewardship objectives |
| Execution planning | verified methods, afforded actions, realization binding | `typed-partial` | durable installed planning-theory revisions |
| Task package / task network | typed package authoring and durable execution | strong execution substrate | package registry/selection as installed theory rather than root expression branch |
| Capabilities | typed executable affordance contracts and side effects | strong execution substrate | authority/grant enforcement remains separate work |
| Context | immutable frames, heads, content-addressed/Merkle frame sets | `substrate` | optional context projection/hydration integration; not a PDS dependency |
| Workspace / provider | physical target and execution adapters | `substrate` | remain application/environment bindings, not PDS semantic owners |

## Highest-Value Continuation Work

The code map suggests the following sequence before treating the new example corpus as implementation-ready.

### 1. Generalize declaration and linking

Replace the docs-only selection as the conceptual upper-layer target with a generic declaration that lowers to domain-owned theory identities and physical requirements.

Success criterion:

> Adding a new PDS expression does not require a new `match expression` branch in root config or runtime assembly.

The existing docs selection can remain a compatibility reader that lowers into the new form.

### 2. Finish theory durability symmetry

Belief families already establish the desired pattern: identity + content hash + append-only revisions + exact resolution.

Apply the same installation discipline, where needed, to:

- outcome evidence mappings;
- method libraries;
- available action sets;
- method realizations;
- task-package selection or package theory;
- any future outcome-contract declaration.

Not every kind must share one registry. Each owning domain should expose its own registry contract and revision identity.

### 3. Make standing responsibility first-class

All PDS examples depend on a maintained condition that survives individual goals. Current code has durable goals and curation rules but no durable semantic object for the standing condition itself.

The next design should settle:

- identity of a maintained condition;
- desired/breach/restore propositions;
- hysteresis/stability semantics;
- relationship to Agent curation;
- relationship to repeated goal epochs;
- whether the authoritative record is Agent-owned, PDS-owned, or an explicit projection over domain truth.

### 4. Define authority separately from capability

Before consequential PDS expressions, introduce a typed authority/grant model that can answer:

```text
this action exists
!= this steward is allowed to invoke it
```

The model should preserve principal, scope, action class, approval, budget, prohibition, escalation, and effective-policy lineage without moving capability ownership into PDS.

### 5. Complete generic outcome verification

The docs slice proves the causal loop but not a universal declaration model.

A future package must be able to declare how an action's canonical outcome becomes verification evidence without root application code and with exact theory revision lineage.

### 6. Design context projection only if required

Roleplay and lore make bounded context hydration visible, but the implementation should first decide what projection contract is needed. It should not begin by making `FrameMerkleSet` part of PDS.

### 7. Add domain facets for lore/roleplay semantics

Only after the common gaps above are separated should domain-specific work define:

- canon claims and semantic valid time;
- entity alias/merge/split policy;
- character knowledge and reveal policy;
- narrative commitment/thread semantics.

Those should compile to or operate over existing generic runtime contracts whenever possible.

## Architectural Tests For The Next Expression

A second implemented PDS expression should be rejected as a generality proof if any of these are required:

- a new root `match` arm on expression name to select runtime semantics;
- a new belief Rust subsystem named after the application;
- a new planner branch for application vocabulary;
- a PDS-owned copy of event, graph, belief, goal, task, or outcome truth;
- capability discovery being treated as authorization;
- context-frame or Merkle storage becoming mandatory without a semantic requirement;
- a standing objective represented only as prompt prose;
- task completion being treated as proof that the maintained condition was restored.

A successful second expression should instead add or install domain theory, bind physical adapters, and reuse the same runtime grammar.

## Conclusion

The current codebase already contains a substantial PDS runtime substrate and several real theory seams. The implementation is uneven by design:

```text
strong and durable
    belief-family theory
    belief/evidence/provenance/conflict state
    graph identity and perspective
    transient goals and execution state

strongly typed but not durably installed as PDS theory
    outcome mappings
    planning methods
    available actions
    method realizations
    task-package selection

first-proof compatibility
    docs-specific selection
    root expression-to-package wiring
    process-local aggregate theory injection

design gaps
    generic declaration/compiler
    standing maintained conditions
    authority and governance
    assignment/activation
    generic outcome-policy installation
    domain-specific canon/knowledge semantics where required
```

The next architecture work should preserve this distinction. New PDS examples are most useful when they expose a missing generic seam; they should not cause application vocabulary or storage choices to become runtime invariants.
