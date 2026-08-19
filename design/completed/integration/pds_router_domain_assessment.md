# PDS Router Assessment By Domain

Date: 2026-08-15
Status: design input
Scope: route external PDS package components to domain-owned theory installers, use dependency security as a new consumer, and realign docs freshness as an existing consumer

> This assessment remains authoritative for router structure and domain ownership. [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) supersedes its package-carried exact capability and authority components. Those routes now describe compatibility migration only.

## Concern

Meld needs one package attachment boundary through which external user-defined PDS packages can contribute state-free operational domain theory without making PDS the owner of domain semantics. One package may contain several components, several packages may be installed and assigned, and every component must enter through one route published by its owning domain. The assessment traces package loading, route publication, owner installation, exact receipt composition, physical activation, capability binding, and runtime consumption across the complete current domain set.

## Direct Behavior

A caller supplies one PDS package entry point. The PDS router verifies the package envelope, resolves every named component route, asks each owning domain to validate and install its own body, links declared component requirements, and commits one exact package installation receipt only after every owner result is resolvable. Runtime activation resolves that receipt, asks owning domains to contribute activation behavior and exact capability implementations, and then uses the existing Meld event, belief, Agent, Strategy, Goal, and execution path.

## In Scope

- one package entry point with many route-addressed components
- installation of more than one package
- domain-published route contracts and route handlers
- state-free package source and exact installed owner revisions
- deterministic package closure and complete installation receipts
- package imports pinned to exact package revisions
- capability contract publication and exact implementation binding
- assignment and physical activation as concepts separate from package identity
- dependency-security as a new PDS consumer
- docs freshness migration to the same router and adapter pattern
- compatibility characterization and parity requirements for the docs migration
- event-mediated cooperation among independently assigned stewards
- placement-independent activation isolation requirements
- assignment-local catalogs, bindings, activation generations, participant incarnations, and result admission lineage
- separate prepared closure, runtime readiness, and expected-prior current-generation publication
- linked, subprocess, sidecar, remote-service, and persistent-controller pressure cases

## Out Of Scope

- conflict policy among several stewards
- a package marketplace or repository service
- dynamic native code loading
- one mandatory process-isolation topology
- remote transport selection
- package signing and trust distribution
- user-facing package authoring syntax beyond a canonical manifest shape
- automatic authority delegation between stewards
- a global workflow that sequences security, developer, and docs work
- implementation sequencing inside this assessment

## Maturity Envelope

The router pattern is design input. It does not make the proposal canonical cognitive architecture and does not authorize implementation. Dependency security is a consumer design, not an approved scanner selection. Docs freshness migration must preserve existing behavior until parity is demonstrated.

## Evidence Basis

Evidence date: 2026-08-15

The top-level source domain snapshot was regenerated with:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

Snapshot:

```text
agent
api
branches
capability
cli
compat
concurrency
config
context
control
docs
error
events
execution
harness
heads
ignore
init
lib
logging
merkle_traversal
metadata
prompt_context
provider
runtime
serve
session
store
task
telemetry
tree
types
views
workflow
workspace
world_state
```

Independently owned workspace domains on the product path:

```text
meld-events
meld-execution
meld-lang
meld-world-model
```

Prospective domains assessed explicitly:

```text
theory
dependency-security
```

Primary evidence:

- [Canonical PDS layer](../../cognitive_architecture/persistent_domain_stewardship.md)
- [PDS runtime layer map](pds_theory_runtime_layer.md)
- [External attachment proposition](external_domain_theory_attachment_proposition_brief.md)
- [Capability ownership assessment](capability_ownership_and_pds_theory_routing_domain_assessment.md)
- [CVE discovery ground](cve_freshness_step_5_discovery_ground_map.md)
- [Current capability facade](../../../src/capability.rs)
- [Current fixed installation receipt](../../../src/runtime/theory.rs)
- [Current docs-specific activation](../../../src/runtime/assembly.rs)
- [Current fixed theory bundle](../../../src/init/world/pipeline.rs)
- [Current fixed declaration selection](../../../src/config/stewardship/selection.rs)
- [Current docs domain](../../../src/docs.rs)
- [Current capability contracts](../../../crates/meld-execution/src/capability/contracts.rs)

Applicable policy:

- [Assessment By Domain Policy](../../../governance/assessment_by_domain_policy.md)
- [Compatibility Policy](../../../governance/compatibility_policy.md)
- [Compatibility Shim Policy](../../../governance/compatibility_shim_policy.md)
- [Storage Policy](../../../governance/storage_policy.md)
- [Semantic Unit Preservation Policy](../../../governance/semantic_unit_preservation_policy.md)
- repository domain architecture rules in `AGENTS.md`

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | legacy root Agent and context APIs | `not needed` | `src/agent.rs` | world-model Agent owns PDS curation and standing responsibility | none |
| `api` | `none` | context API adapter | `not needed` | `src/api.rs` | package attachment does not require a context API route | none |
| `branches` | `none` | branch identity is already available to runtime meaning | `not needed` | `src/branches.rs` | router installation is branch-neutral and assignments may carry existing branch refs | none |
| `capability` | `own` | shared facade publishes only docs contracts | `partial` | `src/capability.rs` |  | assemble domain contributors without owning their contracts |
| `cli` | `adapter` | world init accepts authored theory source | `partial` | `src/init/world/tooling.rs` |  | accept one package entry and present router diagnostics |
| `compat` | `adapter` | compatibility is distributed beside current callers | `partial` | `src/config/stewardship/selection.rs` |  | keep one characterized docs lowering seam during migration |
| `concurrency` | `none` | process-local locks | `not needed` | `src/concurrency.rs` | installation ordering belongs to theory and runtime effect arbitration belongs to execution | none |
| `config` | `own` | declarations contain one fixed list of theory identities | `partial` | `src/config/stewardship/selection.rs` |  | select package revision, assignment, scope, principal, and activation bindings |
| `context` | `publish` | owns context capabilities but lacks common product contribution | `partial` | `src/context/capability.rs` |  | publish through the capability contributor contract |
| `control` | `none` | owns explicit orchestration plans | `not needed` | `src/control.rs` | steward cooperation is event-mediated rather than a control plan | none |
| `docs` | `publish` | owns capabilities and claim policy but is directly called by root | `partial` | `src/docs/capability.rs` |  | publish docs theory routes and an activation contributor |
| `error` | `none` | shared legacy error vocabulary | `not needed` | `src/error.rs` | theory owns router errors and adapters translate them | none |
| `events` | `publish` | reexports canonical append and replay authority | `complete` | `src/events.rs` |  | reuse unchanged for domain events and evidence lineage |
| `execution` | `adapter` | root ports expose execution contracts | `partial` | `src/execution.rs` |  | route settled capability and authority contracts only |
| `harness` | `none` | development observation layer | `not needed` | `src/harness.rs` | router inspection through the harness is useful but not required by package correctness | none |
| `heads` | `none` | legacy frame compatibility | `not needed` | `src/heads.rs` | no frame-head behavior participates | none |
| `ignore` | `none` | workspace scan policy | `not needed` | `src/ignore.rs` | package routing does not interpret file exclusion | none |
| `init` | `adapter` | fixed bundle installs known owner types and commits one fixed receipt | `partial` | `src/init/world/pipeline.rs` |  | invoke package routing and commit the generic receipt last |
| `lib` | `adapter` | exports existing product domains | `partial` | `src/lib.rs` |  | expose settled router and dependency-security public contracts |
| `logging` | `none` | operational logs | `not needed` | `src/logging.rs` | logs are not installation evidence | none |
| `merkle_traversal` | `publish` | owns traversal capability implementations | `partial` | `src/merkle_traversal/capability.rs` |  | publish through the capability contributor contract |
| `metadata` | `none` | frame metadata contracts | `not needed` | `src/metadata.rs` | PDS component metadata is not frame metadata | none |
| `prompt_context` | `none` | bounded prompt artifacts and lineage | `not needed` | `src/prompt_context.rs` | only capabilities that need prompt context consume it at runtime | none |
| `provider` | `publish` | owns provider capabilities while root makes provider mandatory for docs | `partial` | `src/provider/capability.rs` |  | publish capabilities and bind providers only when selected contracts require them |
| `runtime` | `adapter` | resolves a fixed receipt and activates docs directly | `partial` | `src/runtime/theory.rs`, `src/runtime/assembly.rs` |  | resolve routed components and call activation contributors without expression dispatch |
| `serve` | `none` | non-authoritative harness transport | `not needed` | `src/serve.rs` | package inspection transport is not required for semantic attachment | none |
| `session` | `none` | command and harness lifecycle | `not needed` | `src/session.rs` | package identity and assignment lifetime do not become command sessions | none |
| `store` | `none` | generic node persistence | `not needed` | `src/store.rs` | theory and domain registries retain their own storage | none |
| `task` | `consume` | task execution consumes capability contracts and artifacts | `complete` | `src/task.rs` |  | reuse unchanged after exact activation |
| `telemetry` | `none` | downstream observability | `not needed` | `src/telemetry.rs` | telemetry must not become correctness evidence | none |
| `tree` | `none` | filesystem Merkle model | `not needed` | `src/tree.rs` | router does not inspect workspace trees | none |
| `types` | `none` | legacy primitive aliases | `not needed` | `src/types.rs` | PDS identities belong to theory contracts | none |
| `views` | `none` | compatibility presentation | `not needed` | `src/views.rs` | no PDS presentation model is required | none |
| `workflow` | `none` | explicit workflow profiles and gates | `not needed` | `src/workflow.rs` | package components do not form a manual cross-steward workflow | none |
| `workspace` | `publish` | owns workspace capabilities and observation behavior | `partial` | `src/workspace/capability.rs` |  | publish through capability contribution and preserve workspace truth |
| `world_state` | `adapter` | root facade reexports world-model contracts | `complete` | `src/world_state.rs` |  | carry generic propositions and refs without domain branches |
| `meld-events` | `publish` | owns generic canonical events, provenance, append, and replay | `complete` | `crates/meld-events/src` |  | reuse unchanged |
| `meld-execution` | `own` | owns capability, authority, Goal, planning, and execution contracts | `partial` | `crates/meld-execution/src` |  | publish theory route handlers for installable execution-owned bodies and preserve runtime behavior |
| `meld-lang` | `consume` | carries open generic propositions and effects | `complete` | `crates/meld-lang/src` |  | reuse unchanged and add no security or docs variants |
| `meld-world-model` | `own` | exact registries exist for belief, Agent, and Strategy bodies | `partial` | `crates/meld-world-model/src` |  | publish route handlers over existing registries |
| prospective `theory` | `own` | fixed root receipt and loaders currently substitute for a router | `not started` | `src/runtime/theory.rs`, `src/init/world/theory.rs` |  | own package envelope, route catalog, link closure, generic receipt, and exact resolution dispatch |
| prospective `dependency-security` | `own` | no product domain exists | `not started` | CVE ground map |  | own dependency, advisory, assessment, remediation-feasibility, and verification meaning |

## Frozen Affected-Domain Set

The affected-domain set is frozen as:

```text
capability
cli
compat
config
context
docs
events
execution
init
lib
merkle_traversal
provider
runtime
task
workspace
world_state
meld-events
meld-execution
meld-lang
meld-world-model
prospective theory
prospective dependency-security
```

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Product capability inventory | `capability` | docs-only product publication | collect all domain contributors and reject duplicate exact identities | `extend existing` | aggregator selects by expression | `src/capability.rs` |
| Package install presentation | `cli` | theory source tooling | pass one entry point to theory and render typed diagnostics | `adapter only` | CLI parses domain component bodies | `src/init/world/tooling.rs` |
| Docs declaration lowering | `compat` | legacy docs table lowers to fixed declaration | lower old selection into one synthetic docs package attachment | `extend existing` | shim becomes the normal path | `src/config/stewardship/selection.rs` |
| Package selection | `config` | fixed theory identity fields | select exact package id and revision plus assignment identity | `extend existing` | config repeats component truth | `src/config/stewardship/selection.rs` |
| Physical activation | `config` | one workspace, subject, provider, and package selection | bind owner-declared requirements without making provider universal | `extend existing` | credentials leak into semantic package identity | `src/config/stewardship/binding.rs` |
| Context capability contribution | `context` | domain-owned invokers | implement shared product contributor surface | `extend existing` | capability contract moves to root | `src/context/capability.rs` |
| Docs theory installation | `docs` | exact claim policy registry called by root | publish a docs claim-policy route handler | `extend existing` | router decodes claim policy | `src/docs/claim_validation/registry.rs` |
| Docs activation | `docs` | root constructs docs config and calls docs registration | contribute exact invokers from docs-owned activation adapter | `extend existing` | root retains docs-specific config knowledge | `src/docs/capability.rs` |
| Canonical domain facts | `events` | generic event authority | carry dependency-security and docs facts intact | `reuse unchanged` | event type enum closes domain vocabulary | `src/events.rs` |
| Root execution facade | `execution` | typed ports and provider binding | expose settled activation inputs without interpreting package bodies | `adapter only` | facade duplicates execution registry logic | `src/execution.rs` |
| Package installation command | `init` | fixed `WorldInitTheoryBundle` | call theory install service and owner handlers | `extend existing` | init becomes an alternate router | `src/init/world/pipeline.rs` |
| Product exports | `lib` | root module list | export approved public theory and dependency-security surfaces | `adapter only` | premature exposure of internal handlers | `src/lib.rs` |
| Traversal capability contribution | `merkle_traversal` | domain-owned invoker | implement shared product contributor surface | `extend existing` | traversal semantics enter shared capability | `src/merkle_traversal/capability.rs` |
| Provider capability contribution | `provider` | domain-owned invoker | implement shared product contributor surface | `extend existing` | every package inherits provider requirement | `src/provider/capability.rs` |
| Package resolution | `runtime` | fixed `ResolvedStewardshipTheory` | freeze one generic resolved package image per composition | `extend existing` | root switches on route or expression | `src/runtime/theory.rs` |
| Activation assembly | `runtime` | direct docs registration | call exact owner activation contributors and verify closure | `extend existing` | contributors receive unrestricted product stores | `src/runtime/assembly.rs` |
| Task execution | `task` | generic exact capability invocation | consume the activated catalog and artifacts unchanged | `reuse unchanged` | task branches on package identity | `src/task.rs` |
| Workspace capability contribution | `workspace` | domain-owned invokers and workspace truth | implement shared product contributor surface | `extend existing` | package adapter interprets workspace internals | `src/workspace/capability.rs` |
| World-state facade | `world_state` | open world-model reexport | carry typed propositions and refs unchanged | `reuse unchanged` | root adds CVE-specific helpers | `src/world_state.rs` |
| Event envelope and replay | `meld-events` | open string identities and provenance | carry new domain products unchanged | `reuse unchanged` | duplicated semantic fields beside canonical products | `crates/meld-events/src` |
| Capability registry route | `meld-execution` | exact append-only capability registry | install and resolve execution-owned capability contract components | `extend existing` | PDS owns capability meaning | `crates/meld-execution/src/capability` |
| Authority policy route | `meld-execution` | exact authority policy registry | install and resolve authority components | `extend existing` | package request treated as effective grant | `crates/meld-execution/src/authority.rs` |
| Goal and execution runtime | `meld-execution` | existing generic behavior | consume exact activated theory unchanged | `reuse unchanged` | new PDS scheduler duplicates behavior | `crates/meld-execution/src` |
| Open proposition language | `meld-lang` | domain-open terms, propositions, and effects | carry docs and dependency-security meaning without variants | `reuse unchanged` | core gains application vocabulary | `crates/meld-lang/src` |
| Belief route handlers | `meld-world-model` | exact belief and mapping registries | publish owner handlers over current registry commands | `extend existing` | router validates epistemic semantics | `crates/meld-world-model/src/belief` |
| Agent route handlers | `meld-world-model` | exact curation and maintained-condition registries | publish owner handlers and assignment activation contribution | `extend existing` | theory creates live beliefs or Goals | `crates/meld-world-model/src/agent` |
| Strategy route handler | `meld-world-model` | exact Strategy theory registry | install and resolve exact Strategy bodies | `extend existing` | route handler binds executable implementation directly | `crates/meld-world-model/src/strategy` |
| Package envelope | prospective `theory` | no generic package contract | own manifest, component envelope, imports, and package digest | `new local behavior` | envelope becomes a universal domain schema | router design requirement |
| Route catalog | prospective `theory` | no route publication contract | register unique owner routes and dispatch opaque bodies | `new local behavior` | route catalog becomes service locator for live behavior | router design requirement |
| Link closure | prospective `theory` | fixed cross-owner checks in root | verify component refs, exact imports, handler validation, and closure | `new local behavior` | router interprets domain symbols | router design requirement |
| Installation receipt | prospective `theory` | fixed fields including docs claim policy | commit sorted owner component receipts and package lineage | `new local behavior` | opaque refs cannot be historically resolved | router design requirement |
| Dependency model | prospective `dependency-security` | absent | own declarations, resolved components, coupling, and scope | `new local behavior` | workspace becomes dependency authority | security consumer requirement |
| Advisory assessment | prospective `dependency-security` | absent | own sources, coverage, applicability, violation, and bounded clean verdicts | `new local behavior` | absence becomes clean | security consumer requirement |
| External runtime adapter | prospective `dependency-security` | absent | translate scanner and monitor products into canonical domain observations | `new local behavior` | adapter becomes alternate cognition runtime | security consumer requirement |
| Security capability contribution | prospective `dependency-security` | absent | publish observation, assessment, feasibility, and verification contracts | `new local behavior` | code mutation leaks from developer ownership | security consumer requirement |

## Ownership And Boundary Synthesis

### Concern owners

The prospective `theory` domain owns package identity, the component envelope, route publication, link closure, installation receipts, and exact owner-resolution dispatch. It owns no docs, dependency, advisory, belief, Agent, Strategy, authority, or capability semantics.

Each route owner owns its component schema, body validation, append-only installation, exact resolution, activation contribution, and diagnostics. `dependency-security` owns security meaning. `docs` owns claim policy and docs capability behavior. `meld-world-model` owns belief, Agent, and Strategy bodies. `meld-execution` owns capability and authority bodies.

### Contract publishers and consumers

Domains publish two independent contract families:

```text
theory route contracts
→ accepted package component kinds and owner handlers

capability contributor contracts
→ executable contracts and exact invoker factories
```

PDS installation consumes route contracts. Runtime activation consumes installed owner refs and capability contributors. Package source never supplies effective authority or live runtime state.

### Adapters

`cli`, `init`, root `execution`, root `world_state`, and root `lib` remain adapters. `runtime` composes exact resolved products but may not interpret owner bodies. External scanner and monitor bindings are dependency-security adapters and are non-authoritative until dependency-security admits their results.

### Reused unchanged

Canonical event append and replay, generic proposition language, Goal storage, task execution, task artifacts, Agent decision behavior, belief revision behavior, and Strategy search remain the cognitive and execution runtime. New package consumers must falsify a need for changes before any of these are generalized.

## Architectural Pattern

The assessment resolves to one pattern:

```text
one package entry
→ common manifest and routed component envelopes
→ route catalog lookup
→ owner validation and installation
→ generic exact owner receipts
→ package receipt commit
→ assignment and physical activation
→ owner activation contributors
→ existing Meld runtime
```

The pattern deliberately has two extension axes. Theory routes extend state-free meaning. Capability contributors extend executable implementation inventory. Neither axis owns the other.

## Independent Requirements Derived From The Assessment

| Requirement | Finding | Required resolution | Design owner |
| --- | --- | --- | --- |
| `AR-01` | package source has no canonical single entry | define one manifest-rooted package entry with embedded or content-addressed components | `theory` |
| `AR-02` | the current receipt has fixed docs-shaped fields | replace fixed slots with sorted exact owner component receipts | `theory` |
| `AR-03` | root currently loads and validates owner bodies | dispatch opaque canonical bodies to published owner handlers | `theory` and route owners |
| `AR-04` | current cross-owner validation is root-authored | let router validate structural refs and let owners validate semantic links | `theory` and route owners |
| `AR-05` | package identity and physical provider binding are coupled by current config | separate package revision, assignment, and activation binding | `config` and `theory` |
| `AR-06` | one declaration assumes one fixed theory shape | allow several assignments to cite several installed package receipts | `config` and `runtime` |
| `AR-07` | capability publication is docs-only | define product capability contributors and exact activation contributors | `capability` and capability-owning domains |
| `AR-08` | direct docs activation remains in root | route docs claim policy and docs invoker activation through docs-owned contributors | `docs` and `runtime` |
| `AR-09` | world-model and execution registries are exact but not routable | expose route handlers that delegate to existing registry commands | `meld-world-model` and `meld-execution` |
| `AR-10` | dependency-security meaning does not exist | create a domain that owns dependency, advisory, assessment, feasibility, and verification semantics | `dependency-security` |
| `AR-11` | a complete external security runtime could become an alternate source of Meld cognition | admit typed domain observations and keep beliefs, Goals, and decisions in Meld | `dependency-security` |
| `AR-12` | direct steward notification risks becoming an imperative workflow | publish actionable facts that independent steward assignments may consume | domain event publishers and consumers |
| `AR-13` | a commit can be mistaken for resolution evidence | require subsequent assessment against exact advisory and inventory revisions | `dependency-security` |
| `AR-14` | docs freshness has compatibility configuration and a hand-composed test fixture | migrate through one characterized shim and prove receipt, capability, and flywheel parity | `compat` and `docs` |
| `AR-15` | owner stores can retain partial installs before root receipt failure | require idempotent owner install and commit the package receipt last | `theory`, `init`, and route owners |
| `AR-16` | owner refs could become untyped opaque data | require route-addressed exact refs and owner resolution verification | `theory` and route owners |
| `AR-17` | router access to product stores would erase ownership | inject narrow handler commands and resolvers, never unrestricted store collections | `theory`, `init`, and `runtime` |
| `AR-18` | package data may be mistaken for live state | make package bodies state-free and keep runtime state in existing domain stores | all owners |
| `AR-19` | process placement does not settle semantic, authority, binding, state, failure, resource, effect, admission, or replay isolation | encode a portable activation requirement set and keep each active runtime image assignment local | `config`, `runtime`, execution, and owner adapters |
| `AR-20` | multi-owner activation can expose partial invokers or let one failure disturb another assignment | prepare into an activation-local draft, commit inert prepared closure, prove runtime readiness, then publish current generation through an expected-prior fence | `runtime`, capability, and activation contributors |
| `AR-21` | timeout, retry, restart, and late callbacks can misattribute external results | separate activation generation, participant incarnation, stable operation, attempt, and passive-delivery lineage, then require destination-domain admission before canonical append | execution, runtime, events, and destination domains |

These requirements are independent. A route catalog does not satisfy exact receipts. Exact receipts do not satisfy activation completeness. Capability contribution does not grant authority. Installing a package does not create a live assignment.

## Explicit Non-Integration Decisions

- PDS does not own domain component schemas.
- PDS does not own capability implementations.
- PDS does not create beliefs, Agents, Goals, tasks, or outcomes.
- Root runtime does not switch on package, expression, route, or owner names.
- `meld-lang` gains no docs or dependency-security variants.
- `workflow` does not chain security, developer, and docs stewards.
- `provider` is not mandatory for deterministic local adapters.
- package installation does not load executable native code.
- package identity does not contain process placement, endpoint, credentials, or workspace path.
- external runtime storage is not copied into Meld merely because an adapter exists.
- canonical events carry domain products intact and do not duplicate their fields.
- installation receipts are not evidence that a maintained condition is satisfied.
- runtime placement is not evidence of isolation or domain truth.
- shared adapter machinery does not share grants, bindings, catalogs, or cognitive state.

## Three Scope Views

### Runtime path domains

```text
config
theory
route owners
capability
init
runtime
events
meld-events
meld-world-model
meld-lang
meld-execution
task
domain adapters
```

### Domains with changed behavior

```text
capability
compat
config
context
docs
init
merkle_traversal
provider
runtime
workspace
meld-execution
meld-world-model
prospective theory
prospective dependency-security
```

### Likely code regions

This is evidence of probable touch points, not an implementation plan.

```text
src/capability.rs
src/config/stewardship
src/docs
src/init/world
src/runtime/theory.rs
src/runtime/assembly.rs
src/context/capability.rs
src/provider/capability.rs
src/workspace/capability.rs
src/merkle_traversal/capability.rs
crates/meld-execution/src/capability
crates/meld-execution/src/authority.rs
crates/meld-world-model/src/belief
crates/meld-world-model/src/agent
crates/meld-world-model/src/strategy
new src/theory.rs and src/theory children if approved
new src/dependency_security.rs and src/dependency_security children if approved
```

## Boundary Risks

- A common component payload schema would recreate the rejected central PDS schema.
- A handler receiving all stores could write foreign state.
- A generic opaque JSON owner ref would make historical resolution unverifiable.
- Activation contributors selected by expression name would preserve hidden application dispatch.
- Package imports that float at activation time would destroy replay.
- Product capability aggregation could accidentally register capabilities not selected by the exact package image.
- A dependency-security adapter could collapse inventory, advisory coverage, applicability, remediation, and verification into one success flag.
- Docs compatibility could become permanent if new packages continue to target the fixed theory selection.
- An aggregate activation could become visible before exact owner and invoker closure.
- An inert prepared closure could be mistaken for live readiness or current-generation authority.
- A stale external callback could be admitted under a new activation generation.
- A dedicated process could be mistaken for credential, state, or effect isolation.
- A shared service could merge assignment lineage even while process isolation appears strong.

## Smallest Missing Connective Behavior

The smallest missing connective behavior is a paired publication boundary:

```text
domain theory route publication
+
domain capability contribution
```

The first installs and resolves state-free meaning. The second binds exact executable behavior during activation. The generic package receipt links their exact identities without owning either body.

## Unresolved Questions

1. Whether the approved top-level name is `theory` with a `router` concern or `pds` with a theory route concern.
2. Whether package imports are required for the first implementation or only exact component references within one package.
3. Whether one owner route may accept several component schema versions concurrently.
4. Whether activation receipts belong to theory, config, or runtime assembly after package installation is complete.
5. Whether external capability adapters are compiled product contributors first or may later arrive through an isolated transport registry.
6. How assignment scope discovery binds one package to several derived dependency subjects.
7. How conflicting stewardship is handled after more than one independent PDS exists.
8. Whether isolation requirements are stored in activation records or derived only from owner contributors.
9. Which domain owns the aggregate activation receipt and current activation head.
10. Which late observation classes remain admissible after activation retirement.

## Assessment Verdict

The PDS Router is justified as a domain-owned package linking and dispatch concern. It must be a semantic router, not a universal compiler and not a plugin runtime. The complete attachment contract is one package entry, many owner-routed components, exact owner installation refs, a package receipt committed last, separate assignment and activation, and exact domain capability contribution.

Dependency security and docs freshness can consume this pattern without sharing domain schemas. Dependency security adds genuinely new domain meaning. Docs freshness moves existing meaning behind the same owner route and activation contracts. Existing Meld cognition and execution remain the runtime for both.
