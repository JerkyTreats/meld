# Candidate PDS Implementation Requirements

Date: 2026-08-18
Status: aligned candidate requirements
Scope: candidate implementation requirements derived from the PDS proposal and cognitive-runtime anchor mapping

## Interpretation

These are candidate requirements, not an accepted implementation plan.

Each requirement identifies:

- motivation
- existing anchor
- candidate owner
- alternatives
- acceptance evidence

The requirements should be promoted only after the relevant open decisions are resolved.

## Current Grounding

This register predates Theory Elevation Steps 1 through 4 and the routed package design. The requirement statements remain useful, but their original phase order is no longer current.

| Requirement | Current ground | Forward posture |
| --- | --- | --- |
| `PDS-C01` deterministic package identity | exact owner theory revisions and one fixed complete receipt are delivered | extend to one generic routed package identity and receipt |
| `PDS-C02` customer profile representation | proposal only | wait for the two-expression declaration target |
| `PDS-C03` assignment and activation model | one typed docs physical binding exists | generalize after package attachment |
| `PDS-C04` federated domain facets | owner registries exist, but generic route publication does not | implement as owner-routed components |
| `PDS-C05` domain symbol and type linking | owner-local validation exists | add structural linking first and semantic linking through owner public contracts |
| `PDS-C06` facet activation receipts | fixed theory receipt exists; generic preparation and activation closure do not | split package receipt, prepared closure, and live generation evidence |
| `PDS-C07` standing objectives | Agent-owned maintained-condition revisions and evaluation are delivered | preserve ownership and add upper-layer declaration lineage later |
| `PDS-C08` stewardship episodes | domain records exist without unified projection | begin with a derived projection after two-expression runtime proof |
| `PDS-C09` stewardship lineage | exact theory, authority, Goal, task, and outcome lineage is partial | add package, assignment, activation, generation, and operation refs at public boundaries |
| `PDS-C10` pluggable curation policy | exact owner curation revisions exist with current bounded policy shape | widen only when a second consumer requires new semantics |
| `PDS-C11` declarative projection routes | exact outcome mappings exist; graph and source routes remain specialized | route proven owner theory and defer generic graph projection |
| `PDS-C12` context projection | declaration-selected belief context exists for docs | defer generic projection ownership until another consumer requires it |
| `PDS-C13` separately owned Strategy inputs | current `StrategyTheoryPackage` combines semantic theory, exact capabilities, policy, bounds, projection dimensions, and authority requests | split semantic theory from activation capability snapshots, Strategy Methods, Agent policy, derived projection, request controls, and authority context |
| `PDS-C14` independent governance enforcement | delivered at current maturity across Agent judgment, admission, planning, and dispatch | extend later for approvals, budgets, delegation, and revocation evidence |
| `PDS-C15` outcome verification | delivered for docs through exact outcome mapping and belief settlement | prove domain-owned security admission and later verification |
| `PDS-C16` generic runtime activation | generic actors run one fixed startup composition | add assignment-local activation closure and lifecycle |
| `PDS-C17` semantic diff and inspection | proposal only | begin after canonical declaration identity exists |
| `PDS-C18` conformance scenarios | strong docs tests exist | unify only after docs and dependency-security suites reveal the common envelope |

See [PDS Authorized Implementation Workstreams](../plan/integration/pds_implementation_workstreams.md) for current owners, dependencies, tasks, and delivery gates through `W06`.

## PDS-C01: Deterministic Package Identity

### Motivation

Runtime decisions and replay need exact semantic provenance.

### Candidate requirement

Represent a compiled stewardship package with:

- package id and version
- exact import hashes
- schema and compiler versions
- linked domain facets
- profile surface
- scenario summary
- canonical package hash

### Existing anchor

Every live theory kind resolves through an exact owner revision and the current fixed installation receipt closes the one delivered expression. The missing anchor is one generic cross-owner package identity and receipt.

### Candidate owner

PDS control plane or root composition during the first slice.

### Alternatives

- package id and semantic version only
- content hash per domain facet rather than one linked image hash

### Acceptance evidence

Identical inputs produce identical hashes; historical decisions identify the exact package image.

## PDS-C02: Customer Profile Representation

### Motivation

Customers should curate stewardship intent without editing the full runtime representation.

### Candidate requirement

Provide a typed canonical profile containing:

- steward template
- scope parameters
- selected objectives
- sensitivity presets and overrides
- autonomy
- budget
- escalation
- verification

### Existing anchor

No direct equivalent. Current runtime fixtures combine these concerns with lower-level configuration.

### Candidate owner

PDS control plane.

### Alternatives

- use the full package source directly
- code-first profiles
- natural-language directives as canonical state

### Acceptance evidence

The same profile can be edited through text, form, and conversational proposal without semantic drift.

## PDS-C03: Assignment And Activation Model

### Motivation

Normative responsibility and physical deployment change for different reasons.

### Candidate requirement

Represent:

- `StewardshipAssignment`: profile, principal, scope, requested authority, principal grant lineage, lifecycle
- `StewardshipActivation`: source adapters, credentials, providers, capabilities, runtime placement, quotas

### Existing anchor

The docs-freshness runtime has a typed physical binding and named declaration, but the current selection still combines assignment and activation dimensions around one expression.

### Candidate owner

PDS control plane with root adapters.

### Alternatives

- one unified assignment record for the first slice
- activation remains external application configuration

### Acceptance evidence

One profile can be assigned to multiple scopes or activated in multiple environments without recompiling the package.

## PDS-C04: Federated Domain Facets

### Motivation

PDS should not own foreign domain semantics.

### Candidate requirement

Allow packages to contain domain-owned facet sources compiled into opaque envelopes with:

- exports
- imports
- requirements
- requested authority
- diagnostics
- facet hash

### Existing anchor

Belief-family configuration, Agent registration, methods, capability requirements, and event adapters already form implicit facets.

### Candidate owner

PDS linker plus participating domains.

### Alternatives

- one central package schema
- root-only adapter wiring

### Acceptance evidence

A new game or learner facet can be added without changing PDS core data types.

## PDS-C05: Domain Symbol And Type Linking

### Motivation

String-backed runtime vocabulary requires authoring-time validation.

### Candidate requirement

Link and validate package exports and imports for:

- object types
- relation types
- event schemas
- dimensions
- belief families
- artifacts
- actions
- methods
- context projections
- verification profiles
- authority classes

### Existing anchor

Runtime contracts use stable string identifiers and typed `meld-lang` values.

### Candidate owner

PDS linker for cross-domain resolution; domain compilers for semantic validation.

### Alternatives

- shared universal symbol registry
- runtime-only validation

### Acceptance evidence

Undeclared or version-incompatible links fail before activation.

## PDS-C06: Facet Activation Receipts

### Motivation

Cross-domain activation cannot assume one transaction.

### Candidate requirement

Persist prepare and active receipts per facet, including:

- activation and facet identity
- facet hash
- domain-local registration references
- sequence boundary
- lifecycle status
- diagnostics

### Existing anchor

Domains expose durable exact registrations and the current expression has one complete fixed theory receipt. No generic package receipt, prepared activation closure, or live generation receipt exists.

### Candidate owner

PDS control plane stores receipt identity; domain owns referenced registrations.

### Alternatives

- activation inferred from domain queries
- one root-owned all-or-nothing activation record

### Acceptance evidence

Partial activation, compensation, deactivation, and upgrade are reconstructable.

## PDS-C07: Standing Objectives

### Motivation

Execution goals terminate, but stewardship responsibility persists.

### Candidate requirement

Represent standing objective declarations and runtime bindings with:

- desired region
- breach and restore conditions
- freshness
- hysteresis
- stability window
- inaction cost
- profile and package lineage

### Existing anchor

Agent curation can create and satisfy transient goals from belief state.

### Candidate owner

Agent owns installed maintained-condition meaning and evaluation. PDS may own upper-layer declaration lineage and projection.

### Alternatives

- regenerate objectives from profile on every runtime start
- treat active goals as maintenance objectives

### Acceptance evidence

A later breach opens new work after a previous goal was satisfied, without recreating the assignment.

## PDS-C08: Stewardship Episodes

### Motivation

Users need one narrative over investigation, action, and verification.

### Candidate requirement

Provide an episode lifecycle correlating:

- triggering belief revisions
- observation work
- goals
- methods
- tasks
- approvals
- outcomes
- verification
- closure

### Existing anchor

The information exists across Agent, execution, and event records but is not correlated into one PDS concept.

### Candidate owner

Open: authoritative PDS state, Agent-owned state, or PDS projection.

### Current recommendation

Start with a derived PDS projection unless coordination state requires an authoritative record.

### Acceptance evidence

One episode can be explained without duplicating underlying belief, goal, or task truth.

## PDS-C09: Stewardship Lineage

### Motivation

Outcomes must be attributable to the profile and standing mandate that initiated them.

### Candidate requirement

Propagate package, profile, assignment, activation, objective, and episode references through public domain boundaries.

### Existing anchor

Task lineage already preserves goal, method, operator, capability, and projection provenance.

### Candidate owner

PDS defines the neutral reference; domains choose appropriate attachment points.

### Alternatives

- event relations only
- reconstruct lineage from command records and object relations

### Acceptance evidence

An outcome can be traced to one package revision, profile, assignment, objective, Agent decision, method, and capability execution.

## PDS-C10: Pluggable Curation Policy

### Motivation

The first Agent implementation uses one threshold rule. Steward profiles require richer policies and multiple perspectives over shared beliefs.

### Candidate requirement

Support domain-owned policy implementations that consume:

- belief and planner view
- assignment concern binding
- current goals and episode projection
- observation cost
- action cost and value
- authority and budget state

and return one of:

- tolerate
- observe
- propose action
- modify action
- escalate
- close or restore

### Existing anchor

Agent curation runtime and threshold configuration.

### Candidate owner

Agent domain.

### Alternatives

- compile all profiles to threshold rules in PDS V1
- represent curation as ordinary planning

### Acceptance evidence

Two profiles consume the same belief family and produce different decisions without duplicating the family.

## PDS-C11: Declarative Projection Routes

### Motivation

Current graph and docs evidence interpretation contain specialized mappings.

### Candidate requirement

Allow domain facets to register:

```text
event → graph projection
event/artifact → promoted evidence
outcome event → verification evidence
```

### Existing anchor

Graph reducers, belief ingestion, and docs-specific evidence replay.

### Candidate owner

Owning source/world-model domains, coordinated through PDS facets.

### Alternatives

- code-only adapter registration
- central PDS mapping language

### Acceptance evidence

A new evidence route is installed without adding a profile-specific root product port.

## PDS-C12: Context Projection

### Motivation

Model-backed methods need bounded, reproducible context derived from the world model.

### Candidate requirement

Represent or reference:

- subject selector
- graph traversal
- belief families
- perspective and branch
- evidence and provenance inclusion
- ranking and retention
- item and byte budgets
- output artifact schema

### Existing anchor

The implemented belief-context bundle for documentation generation.

### Candidate owner

Open: world-model facet, context domain, capability input binding, or separate PDS concept.

### Acceptance evidence

Two packages can request different context projections without changing the generic generation runtime.

## PDS-C13: Separately Admitted Strategy Methods

### Motivation

Method visibility and provenance must be explicit without making an action topology part of PDS semantic authority.

### Candidate requirement

Define a Strategy-owned Method snapshot and admission record with:

- Method identity and content hash
- admission source and revision
- namespace and version
- compatible PDS semantic vocabulary refs
- required projection and capability predicates
- authority and outcome obligations

### Existing anchor

Execution method library and serialized `meld-lang::Method`.

### Candidate owner

Strategy owns Method admission and visibility. `meld-lang` owns the shared pure representation. Execution realizes only the selected candidate.

### Alternatives

- one global Method library with naming convention only
- package-scoped Method installation

### Acceptance evidence

Planner inspection explains why a method was visible to one assignment and not another.

## PDS-C14: Independent Governance Enforcement

### Motivation

A capability match is not authorization.

### Candidate requirement

Calculate effective authority as the intersection of:

- assignment request derived from principal declaration and profile selection
- principal grant
- organization/runtime policy
- current restrictions

Check authority during:

- profile validation
- planning selection
- task dispatch

### Existing anchor

The delivered minimal authority model uses a compatibility package request. The canonical model derives the request through the assignment, intersects it with principal grant, runtime policy, and current restrictions, then revalidates at admission, planning, and dispatch.

### Candidate owner

Governance/policy domain plus execution enforcement.

### Alternatives

- external approval system only
- package-defined authority presets without runtime intersection

### Acceptance evidence

A matching method and capability cannot dispatch when effective authority is absent.

## PDS-C15: Outcome Verification

### Motivation

Mechanical task success does not prove restoration.

### Candidate requirement

Link actions to domain-owned outcome contracts defining:

- required observations
- evaluation window
- success, partial, failure, and harmful propositions
- evaluator independence
- attribution assumptions

### Existing anchor

Documentation freshness already separates task publication, exact outcome interpretation, promoted evidence, belief revision, and Agent satisfaction. The remaining proof is a dissimilar domain-owned admission and verification path.

### Candidate owner

Outcome facet plus world-model and Agent domains.

### Alternatives

- Agent directly interprets task outcomes
- workflow gates remain authoritative

### Acceptance evidence

A task may succeed while the episode remains under verification or breached.

## PDS-C16: Generic Runtime Activation

### Motivation

Packages should instantiate existing cognitive actors without profile-specific runtime kinds.

### Candidate requirement

Activation produces owner-scoped work registrations for generic actors:

- event replay
- graph reduction
- evidence ingestion
- belief assessment
- Agent bootstrap and curation
- planning
- dispatch
- publication

### Existing anchor

The runtime supervisor and generic bounded actors run one fixed startup composition. Assignment-local generation fanout, hot activation, and portable participant lifecycle remain proposed.

### Candidate owner

Root assembly and runtime supervisor.

### Alternatives

- one root loop manually invokes domain APIs
- one process per steward profile

### Acceptance evidence

A package activation advances the cognitive loop without integration-test code invoking each stage manually.

## PDS-C17: Semantic Diff And Inspection

### Motivation

Customer changes may alter authority, scope, evidence sensitivity, and resource exposure.

### Candidate requirement

Produce semantic diffs and effective-profile views for:

- objective changes
- sensitivity changes
- scope changes
- authority changes
- budget changes
- verification changes
- package upgrade effects

### Existing anchor

No complete equivalent; domain inspection APIs exist separately.

### Candidate owner

PDS control plane using domain-provided facet status and diff projections.

### Acceptance evidence

A reviewer can determine behavioral and authority impact without reading compiled internals.

## PDS-C18: Conformance Scenarios

### Motivation

A package must prove behavior across several domains and failure modes.

### Candidate requirement

Support scenarios covering:

- nominal restoration
- tolerance
- insufficient and contradictory evidence
- denied authority
- unavailable capability
- failed or harmful action
- external restoration
- concurrent domain change
- exact-hash replay
- package upgrade

### Existing anchor

Imperative integration and reopen tests for the docs-freshness flywheel.

### Candidate owner

Open: PDS scenario runner coordinating domain harnesses, or domain-owned suites linked by package scenarios.

### Acceptance evidence

The docs, performance, and one non-software package run through the same scenario envelope without changing PDS core semantics.

## Historical Phasing

The phase order below records the original proposal sequence. It is superseded for authorized delivery through `W06` by [PDS Authorized Implementation Workstreams](../plan/integration/pds_implementation_workstreams.md).

### Phase A: Proposal validation

- resolve meta-domain and ownership options
- define canonical profile model
- test package/profile/assignment/activation decomposition
- map docs-freshness fixture into candidate facets

### Phase B: Root-composed first slice

- parse one package and profile
- register existing belief, Agent, method, and outcome mappings through current APIs
- add package and assignment lineage
- keep workflows as compatibility methods

### Phase C: Standing stewardship

- add objective and episode semantics
- add semantic inspection
- add authority enforcement
- activate generic runtime actors

### Phase D: Generality proof

- software performance package
- game-faction or learner package
- package upgrade and replay
- assess whether extracted PDS and facet connectors are justified

## Promotion Rule

A candidate requirement becomes authoritative only through a later implementation plan that records:

- selected option
- rejected alternatives
- owning domain
- public contract
- migration impact
- tests and acceptance criteria
- compatibility posture

See [Open Decisions](open_decisions.md).
