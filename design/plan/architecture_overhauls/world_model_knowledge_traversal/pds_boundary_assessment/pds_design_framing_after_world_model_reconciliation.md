# PDS Design Framing After World Model Reconciliation

Date: 2026-08-20

Status: discovery framing, contracts and implementation not authorized

## Purpose

This document identifies what can now be treated as structurally true about Persistent Domain Stewardship after the World Model Reconciliation proposal is applied to current Meld.

It does not define Rust types, schemas, storage, source syntax, APIs, migrations, or implementation phases. It establishes the architecture frame within which those contracts can later be drawn.

The direct behavior under assessment is productization:

> A principal selects a durable software stewardship product, binds it to a subject and authority posture, activates it in an environment, and receives one or more persistent Agents whose behavior is supplied by exact Domain Theory and realized by World Model Reconciliation.

In scope are product declaration, semantic packages, owner routing, assignment, activation, Agent genesis, lifecycle lineage, runtime consumption, and inspection meaning.

Out of scope are the exact PDS source language, final crate boundary, API shape, storage engine, Agent cardinality, migration sequence, and every World Model Reconciliation contract still under discovery.

## Primary Finding

World Model Reconciliation gives PDS a stable compilation target.

PDS does not compile to a workflow, one Task, one Strategy candidate, or one opaque Agent prompt. It compiles principal intent into a linked set of exact, owner-installed Domain Theory revisions plus the assignment and activation information needed to materialize configured Agents.

```text
principal intent
-> product declaration or profile
-> linked semantic package
-> exact owner-installed theory revisions
-> assignment
-> activation
-> Agent genesis
-> World Model Reconciliation
```

PDS owns the product definition and its lineage. Native Meld domains own the runtime meanings and state produced from that definition.

This is the framing from which PDS contracts can now be drawn.

## Evidence Classes

The framing rests on three distinct forms of evidence.

Canonical architecture fixes the semantic boundary. [Persistent Domain Stewardship](../../../../cognitive_architecture/persistent_domain_stewardship.md) defines PDS as a semantic program for cognition rather than a cognitive program. It also separates packages, assignment, activation, native domain ownership, live Capability availability, and situated authority.

Current implementation proves the lower mechanics. [`PdsPackageManifestV1`](../../../../../src/theory/package.rs) and [`TheoryRouter`](../../../../../src/theory/router.rs) provide exact structural packages, owner routes, owner validation, installation, exact receipts, and historical resolution. [`StewardshipAssignmentV1`](../../../../../src/config/stewardship/assignment.rs) and [`StewardshipActivationV1`](../../../../../src/config/stewardship/activation.rs) have separate identities. [`pipeline.rs`](../../../../../src/init/world/pipeline.rs) installs theory and materializes an operational Agent. [`activation.rs`](../../../../../src/runtime/activation.rs) records assignment-local runtime generations.

World Model Reconciliation fixes the runtime target. [Strategy Is Bigger Than A Task](../strategy_plan_redesign.md) places mixed causal Plan construction with Strategy, progression with Agent, epistemic realization with Curation, graph reads with Traversal, and executable realization with Execution. The [impact assessment](../impact_assessment/impact_assessment.md) confines the major semantic refactor to world-model and preserves Events, language, Task Network, and most Execution behavior.

The two installed products provide a dissimilarity test. [Docs Freshness](../../../../../theory/docs_freshness/pds-package.json) and [Dependency Security](../../../../../theory/dependency_security/pds-package.json) carry different product semantics through the same routed package mechanics.

## Truth One: PDS Is The Product Program

PDS is the durable answer to what kind of steward the principal has created.

An Endless Horizon software developer is a PDS product expression. Its product definition may select documentation freshness, Dependency Security, commenting governance, test thoroughness, and other maintained concerns. The definition explains which exact semantic program was intended, who selected it, what subject it governs, and how it was activated.

World Model Reconciliation answers what that product currently knows, which desired conditions are breached, which causal Plan is justified, and what happens next.

The distinction is durable:

```text
PDS
-> what product exists
-> what stable domain meaning configures it
-> who and what it is assigned to
-> how it is activated

World Model Reconciliation
-> what is currently known
-> what is currently desired
-> what epistemic or executable work is justified
-> whether the maintained concern is reconciled
```

No PDS contract should need to participate in each belief revision, Plan transition, EpiOp result, Task transition, or Agent judgment merely to preserve this relationship.

## Truth Two: Compilation Is Plural And Owner Routed

A PDS product does not lower to one universal runtime object.

It lowers into a set of native owner products. Current code already installs belief-family revisions, outcome mappings, Agent Curation rules, maintained conditions, Strategy theory, product policies, authority policy compatibility products, and exact Capability compatibility products through separate routes.

World Model Reconciliation adds one decisive target: first-class Curation operation theory. It also changes Strategy theory from executable-chain selection toward heterogeneous Plan semantics.

The stable compilation shape is:

```text
one product declaration
-> several owner-routed semantic components
-> one exact linked installation receipt
-> one assignment-level selection
-> native runtime bindings
```

PDS owns the structural envelope, exact imports, route identities, structural requirements, and linked receipt. Each destination domain owns its component schema, semantic validation, installed revision, and runtime interpretation.

This rules out a central PDS ontology and a central PDS compiler that understands every domain body.

## Truth Three: The Five Conceptual Identities Are Distinct

Five identities answer different questions and should remain distinguishable even if an early implementation stores some together.

| Concept | Question answered | Runtime authority |
| --- | --- | --- |
| semantic package | what reusable Domain Theory exists | PDS structure plus routed semantic owners |
| product declaration or profile | what product choices did the principal select | PDS product model |
| assignment | who, which subject, which perspective, which branch, and which grant lineage | PDS assignment identity plus referenced owners |
| activation | which environment, bindings, offers, placement, isolation, and limits realize the assignment | activation and runtime composition |
| Agent generation | which situated cognitive actor is operating from that installed definition | Agent and native runtime owners |

Current code directly proves package, assignment, activation, and Agent identities. The customer-facing profile remains proposed. Its conceptual role is nevertheless distinct because reusable product meaning and one principal selection answer different audit questions.

The final record count, storage ownership, and names remain open. The identity separation does not.

## Truth Four: PDS Causes Agent Genesis But Does Not Own Agent Cognition

PDS must be able to explain why an Agent exists and which exact product definition caused its materialization.

Agent owns the durable runtime record. Current [`AgentRecord`](../../../../../crates/meld-world-model/src/agent/contracts.rs) already owns perspective, subject, branch, directive, installed Curation theory, maintained condition, lifecycle status, and runtime sequence. Current [`AgentRegistration`](../../../../../crates/meld-world-model/src/agent/registration.rs) idempotently creates and transitions that record.

PDS may declare the desired Agent topology and supply exact lineage for genesis. Root initialization or another composition adapter materializes the records through Agent-owned contracts. PDS does not keep a shadow Agent record or evaluate the Agent maintained condition independently.

The unresolved cardinality question is real. One product could create one multi-concern Agent, several specialized Agents, or a declared topology. Future contracts must represent the selected answer without making PDS the runtime coordinator of their cognition.

## Truth Five: Maintained Conditions Are The Runtime Form Of Standing Intent

Standing stewardship is not a durable Goal and not a PDS episode state.

Current [`AgentMaintainedCondition`](../../../../../crates/meld-world-model/src/agent/maintained_condition.rs) proves the lower ownership boundary. It is reusable desired state that remains authoritative across transient Goal lifecycles and is installed by exact revision on an Agent.

PDS may expose a principal-facing responsibility such as maintaining documentation freshness. Its compiler must resolve that selection to one or more owner-validated maintained-condition revisions and bind them through Agent genesis.

Agent evaluates the installed conditions and creates transient Goals when current knowledge shows a breach. Agent continues to own Goal authority and reconciliation.

Any PDS stewardship episode is therefore a product-facing projection or lifecycle correlation unless a later coordination need proves independent authoritative state. It cannot become a second owner of breach, Goal, Plan, or restoration truth.

## Truth Six: Strategy Plan, EpiOp, And Task Are Runtime Products

PDS may supply stable semantic theory needed to construct Plans. It cannot package a situated Plan.

Strategy receives the current Goal, a frozen knowledge cut, current active Capability offers, separately admitted reusable Strategy knowledge, construction policy, search controls, and current authority context. It constructs or reconstructs a Plan against those inputs.

The Plan may contain bounded Epistemic Operations and Tasks. PDS may supply stable vocabulary, proof meaning, maintained conditions, Curation operation declarations, and Strategy semantic constraints. It does not choose the current EpiOp, Task, causal topology, or schedule.

Legacy Workflow has no role in the canonical architecture. A compatibility wrapper may preserve old behavior during migration, but a PDS package must not become a serialized Workflow or a fixed action chain.

## Truth Seven: Curation Becomes A First-Class Compilation Target

World Model Reconciliation makes Curation theory a necessary PDS concept.

Standing Curation applies installed Agent specifications before Goal creation. Planned Curation realizes bounded EpiOps selected inside a Strategy Plan. Both use exact rule or operation revisions, Agent perspective, bounded knowledge cuts, admissible output vocabulary, terminal meaning, and provenance.

PDS does not own Curation execution or Curation state. It must be able to link stable, owner-validated Curation declarations into the product and bind the selected revisions to the materialized Agent or Strategy environment.

Current `agent-curation-rule` routing is an implementation anchor, not the final Curation contract. Current Agent Curation is Goal curation and does not yet implement the proposed epistemic edge worker. Later contracts should preserve the owner route while allowing the new Curation domain to own its grammar and realization.

## Truth Eight: Traversal And Events Remain Product Blind

PDS does not define another graph or event system.

Packages may carry domain-owned vocabulary, relation meaning, publication rules, evidence classes, and proof requirements. Those components are validated by their semantic owners. Curation and product domains publish situated records through Events. Traversal materializes admitted nodes and edges and supports bounded reads.

Events does not interpret PDS package bodies. Traversal does not inspect product profiles. Neither needs a PDS lifecycle gate for ordinary runtime progress.

This prevents PDS from becoming a universal ontology, graph mutation API, event grammar, or reconciliation bus.

## Truth Nine: Capability Availability Is Activation Data

PDS semantic packages must not define the exact active Capability catalog or a Strategy shortlist.

A distribution may co-ship Capability contracts or implementation contributors. The semantic package may declare activation requirements, governance classifications, outcome meaning, and compatibility expectations. Activation binds the complete exact offers available to an assignment generation.

Strategy reasons over that live immutable snapshot. Agent judges the resulting product. Execution validates only its own executable and authority invariants.

Current Docs Freshness and Dependency Security packages route exact Capability components through the semantic package. That is useful compatibility and installation evidence, but it conflicts with the target boundary where availability comes from activation. Future contracts must separate co-distribution from semantic ownership.

## Truth Ten: Authority Is A Chain, Not A Package Field

PDS can represent requested autonomy, governance classifications, approval posture, and grant lineage. It cannot grant effective authority by declaration.

The stable authority chain is:

```text
product-supported authority classes
+ principal request
+ principal or organization grant
+ current restrictions
+ Agent judgment
+ Execution enforcement
-> effective realization authority
```

Assignment is the durable place to bind request and grant lineage. Activation may bind credentials and physical enforcement resources. Agent and Execution retain their own current decisions and enforcement.

The exact approval, delegation, budget, expiry, and revocation contracts remain open. Their separation from package semantics does not.

## Truth Eleven: Proof Obligations Belong In Theory, Restoration Belongs At Runtime

PDS may carry stable verification profiles, evidence classes, settlement obligations, outcome interpretation, and proof requirements.

Product domains own substantive outcome meaning. Belief owns evidence admission. Agent owns whether current admitted knowledge restores a maintained condition.

```text
Task completion
!= product outcome
!= admitted evidence
!= maintained-condition restoration
```

PDS may correlate these runtime products for inspection. It cannot turn correlation into an authoritative PDS outcome engine.

## Truth Twelve: Exact Lineage Is Part Of Product Meaning

A runtime generation must be explainable through exact product lineage.

At minimum, the explanation must be able to identify the installed package receipt, principal-facing product revision, assignment identity, activation identity or generation, Agent identity, and every owner-installed theory revision that materially affected meaning.

Current package receipts and content hashes prove exact semantic lineage. Current assignment identity includes the exact package receipt. Current activation identity includes the assignment and physical selections. Current Agent bindings retain exact maintained-condition and Curation revisions.

The transport shape remains open. Future records may carry compact lineage identities and resolve them through PDS rather than embedding a large structure everywhere. What is fixed is that runtime products cannot become historically uninterpretable after a package change.

## Truth Thirteen: Upgrade Creates A New Semantic Position

An upgrade cannot mutate old theory in place or reinterpret historical runtime products under a new package.

New semantic content requires a new exact owner revision and linked receipt. Runtime activation must establish a visible generation boundary. Native domains own any local migration or reconciliation from the old position to the new one.

World Model Reconciliation supplies the correct post-upgrade behavior: Agent reassesses against the newly installed semantic position and current world state. It does not rewrite old beliefs, Plans, EpiOps, Tasks, or outcomes.

Whether an upgrade preserves assignment identity, creates a new assignment, runs old and new generations in parallel, or requires explicit incompatibility handling remains open.

## Truth Fourteen: Runtime Owners Continue Without PDS In The Loop

Once an exact product generation is installed, native domains communicate through their own public contracts.

PDS becomes active again when the principal inspects, revises, suspends, upgrades, rolls back, rebinds, or retires the product. It may also project current status from owner records and Events.

This means PDS lifecycle contracts should coordinate product generations and lineage. They should not become receiver-side semantic guards on every Event, belief, Plan, EpiOp, Task, or outcome.

## Contract Families That Can Now Be Framed

The following contract families are justified. Their exact types are not.

| Contract family | Stable responsibility | Must not contain |
| --- | --- | --- |
| principal declaration | record product intent, scope choices, requested autonomy, budgets, escalation, and verification posture | current beliefs, current Plans, exact active Capability catalog |
| product profile | select a curated public surface over reusable package semantics | raw runtime internals or universal domain meanings |
| semantic package | carry exact structural composition, imports, routed components, and package identity | owner semantic validation or situated cognition |
| owner component route | let a domain validate, install, resolve, and verify its own exact semantic revision | PDS-owned decoding of the domain body |
| linked installation receipt | prove which exact owner revisions formed one installed semantic program | live runtime state |
| assignment | bind product revision to principal, subject, perspective, branch, requested authority, and grant lineage | credentials, mutable observations, effective authority |
| activation | bind physical resources, implementation offers, placement, isolation, quotas, and generation identity | domain meaning or Strategy decisions |
| Agent genesis plan | describe which Agent identities and exact installed theory bindings must be materialized | Agent runtime state or cognitive progression |
| lifecycle coordination | establish current product generation, transition lineage, and principal control intent | duplicate domain operation state |
| product inspection | correlate exact declaration, package, assignment, activation, Agent, and owner-reported status | new authoritative cognitive conclusions |
| semantic diff | explain meaning, authority-request, scope, verification, and activation changes before transition | line-oriented syntax assumptions |

These families may be implemented in one crate or several. Their ownership and exclusions should survive either choice.

## Pass One Domain Sweep

The current domain universe was regenerated from root `src`, the four workspace crates, and the six current `meld-world-model` domains on 2026-08-20.

| Domain | Needed relationship | Current ground | Completeness | Non-integration or follow-up |
| --- | --- | --- | --- | --- |
| root theory | own | routed packages, owner handlers, receipts, exact resolution | partial | preserve structural linker ownership |
| root config | adapter | stewardship declaration, assignment, activation identities | partial | compatibility shape must not freeze final PDS model |
| root init | adapter | route catalog, theory install, Agent genesis | partial | adapt compiled product without semantic interpretation |
| root runtime | adapter and lifecycle owner | exact theory view, activation generations, assembly | partial | product generation lifecycle remains incomplete |
| root docs | publish | docs claim policy and package contribution | partial | publish corrected Curation and Plan theory |
| root dependency security | publish | security policy and package contribution | partial | publish durable semantic products and corrected operation theory |
| root capability | publish | exact contracts and contribution publication | partial | keep availability in activation |
| root provider | publish | provider profiles and physical runtime binding | partial | activation reference only |
| root branches | publish | branch identity and scoped state | complete for current assignment needs | no PDS semantic ownership |
| root workspace | publish | current software subject and observation source | partial | optional product-domain binding, not universal PDS scope |
| root harness | observe | package and flywheel characterization | partial | inspection evidence only |
| root telemetry | observe | runtime status and summaries | partial | never authoritative product state |
| root cli | adapter | configuration and tooling surfaces | partial | principal-facing product surface remains open |
| root serve | adapter | runtime service surface | partial | expose owner contracts only if needed |
| root agent | none | separate root Agent configuration tooling | not needed | world-model Agent owns stewardship cognition |
| root bin | none | executable entrypoints | not needed | adapter implementation detail |
| root context | none | immutable frames and context state | not needed directly | projection is derived by world-model owners |
| root control | none | orchestration records | not needed | PDS does not own runtime orchestration |
| root events | none | adapter around canonical Event contracts | not needed directly | native producers and consumers use Events |
| root execution | none | Execution adapters and Goal Set ports | not needed directly | PDS does not enter executable intake |
| root merkle traversal | none | workspace traversal implementation | not needed | no PDS semantic contract |
| root metadata | none | product metadata behavior | not needed | avoid hidden package registry |
| root prompt context | none | bounded prompt artifacts | not needed directly | selected by runtime context needs |
| root session | none | command lifecycle | not needed | product lifecycle is distinct |
| root store | none | persistence primitives | not needed | storage does not interpret PDS |
| root task | none | Task compatibility and product behavior | not needed | PDS does not construct current Tasks |
| root tree | none | tree utilities | not needed | implementation detail |
| root workflow | none | explicitly legacy workflow system | not needed | no canonical planning role |
| `meld-world-model` | consume | installed Belief, Agent, Strategy, and current Curation theory | partial | major reconciliation target |
| `meld-events` | none | durable neutral carriage and replay | complete reuse | remain package blind |
| `meld-execution` | none | executable realization and Task Network | complete reuse at PDS boundary | consume activated work, not PDS theory |
| `meld-lang` | none | permissive shared values and pure functions | complete reuse | no PDS grammar enforcement |

The frozen affected set is root theory, config, init, runtime, docs, dependency security, capability, provider, branches, workspace, harness, telemetry, CLI, serve, and `meld-world-model`.

## Pass Two Affected Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk |
| --- | --- | --- | --- | --- | --- |
| package materialization and identity | theory | exact bounded source and content hash | retain one small structural envelope | extend existing | package schema grows into ontology |
| route publication and validation | theory plus owner domains | validate, install, verify, validate links | route new owner components without decoding | extend existing | generic router interprets semantics |
| exact receipts and heads | theory | immutable receipts and selected head | preserve historical resolution across upgrades | extend existing | mutable heads reinterpret history |
| principal declaration | PDS product model through config adapter | minimal stewardship selection only | represent product intent above runtime details | new local behavior | compatibility config becomes canonical schema |
| product profile | PDS product model | absent | expose curated package choices | new local behavior | profile leaks runtime grammar |
| assignment identity | PDS product model | exact package, principal, Agent, subject, perspective, branch, grants | preserve conceptual identity and lineage | extend existing | assignment absorbs activation or cognition |
| activation identity | runtime composition | bindings, implementations, placement, limits | bind exact environment and generation | extend existing | placement implies false isolation guarantees |
| product generation lifecycle | runtime composition | startup and current activation generation | coordinate transition and readiness | extend existing | PDS becomes supervisor or operation engine |
| Agent genesis | Agent through init adapter | idempotent registration and exact theory binding | materialize declared Agent topology | extend existing | PDS shadows Agent state |
| maintained concern installation | Agent | exact maintained-condition revisions | lower profile intent into owner-validated conditions | extend existing | PDS evaluates breach itself |
| Belief theory installation | Belief | exact belief families and mappings | install package-selected stable epistemic semantics | reuse and extend data | PDS owns inference results |
| Strategy theory installation | Strategy | executable-only theory package | install stable heterogeneous Plan semantics | extend existing | package fixes current Plan or toolbox |
| Curation theory installation | future Curation owner | current Agent Curation rule compatibility route | install standing and planned operation semantics | new owner behavior | PDS or Agent owns epistemic realization |
| docs semantic contribution | docs | claim policy and executable-only package | contribute docs-owned observation, proof, Curation, and Plan meaning | extend existing | PDS centralizes docs correctness |
| security semantic contribution | dependency security | security policy and epistemic Capability shapes | contribute security-owned observation, assessment, Curation, and Plan meaning | extend existing | PDS centralizes security posture |
| exact Capability offers | capability and activation | contracts and package compatibility components | bind complete live catalog outside semantic package | extend existing seam | co-shipping becomes semantic shortlist |
| provider and resource binding | provider and activation | provider identities and physical refs | bind resources without changing product meaning | reuse and extend adapter | provider choice changes semantic package identity |
| branch and subject binding | branch and source owners | explicit branch and `DomainObjectRef` identities | preserve source authority and perspective | reuse unchanged | PDS invents universal subject ontology |
| product inspection | PDS projection over owner state | harness snapshots and runtime reports | correlate lineage and owner-reported status | new local behavior | projection becomes source truth |
| CLI and service presentation | root adapters | current stewardship config and runtime tooling | expose product operations and semantic diff | adapter only | adapters bypass owners |

## Current World Model Consumption Boundary

Inside `meld-world-model`, only four semantic owners require direct PDS compilation targets.

Agent consumes installed maintained-condition and Curation configuration and owns the resulting runtime actor, Goals, Plan progression, and judgment.

Belief consumes installed family and evidence-mapping revisions and owns admission, inference, contradiction, and revision state.

Strategy consumes stable planning semantics but constructs each situated Plan from live inputs.

Curation will consume stable operation and publication semantics but own each situated EpiOp and its terminal results.

Planner and world-state Traversal remain PDS blind. Their richer frozen-cut and graph-read contracts are derived from current runtime needs rather than declared as static PDS projections. Waiting has no PDS relationship.

## Reassessment Of Existing Open Decisions

World Model Reconciliation and current code allow several proposal questions to be narrowed.

| Existing decision | New discovery position |
| --- | --- |
| PDS location | remains open between root ownership and later extraction |
| package ownership model | structurally linked, owner-routed components are now the strongest supported framing |
| customer source representation | remains open |
| package, profile, assignment, activation | conceptual separation is solid; exact record and storage separation remains open |
| standing objective ownership | resolved at runtime to Agent maintained conditions; PDS owns principal declaration lineage |
| stewardship episode ownership | situated progression belongs to Agent; PDS may project or coordinate product lifecycle only |
| domain vocabulary | federated owner semantics under structural routes; no central ontology |
| facet connector scope | semantic validate, install, verify, and link is proven; broader activation and migration protocol remains open |
| context projection | fixed package projection is rejected; projection is derived from current Goal and knowledge needs |
| governance ownership | split request, grant, Agent judgment, and Execution enforcement is solid |
| outcome verification | owner result, Belief admission, and Agent restoration judgment is solid; PDS correlation is a projection |
| Workflow end state | Workflow is legacy and has no canonical PDS or Plan role |
| package scenarios | remains open |
| runtime actor instantiation | owner and adapter split is solid; actor multiplicity and placement remain open |
| lineage transport | exact lineage is required; compact reference versus embedded shape remains open |
| package upgrade | new semantic identity and visible reconciliation boundary are solid; assignment and coexistence choreography remain open |
| PDS crate timing | remains open |
| isolation encoding | placement must not imply guarantees; exact portable vocabulary remains open |

## Explicit Non-Integration Decisions

PDS contracts should not be added to `meld-lang`, `meld-events`, Task, Task Network, Execution planning, Traversal, Planner projections, context storage, Merkle traversal, sessions, or legacy Workflow merely because these domains participate in a complete product run.

PDS should not define a universal object taxonomy, relation enum, evidence engine, graph store, event grammar, Goal store, Plan runtime, EpiOp runtime, Task runtime, Capability catalog, provider supervisor, or authority engine.

PDS should not make root configuration or runtime assembly authoritative for product semantics. Those modules are adapters that resolve and supply exact inputs to owner contracts.

## Smallest Missing Connective Model

The smallest missing PDS model is not another runtime.

It is a principal-facing declaration and linker that can select reusable package semantics, compile them through owner routes, bind the resulting exact receipt to a subject and Agent topology, bind an environment activation, and materialize that product through native Agent and runtime contracts.

Current code proves the lower half from routed package through Agent genesis. World Model Reconciliation clarifies the corrected owner targets. The upper principal-facing product model and lifecycle transition semantics remain the main discovery gap.

## What Future Contract Work May Assume

Future contract design may assume that PDS is a declarative product model, package semantics are federated, compilation produces plural exact owner revisions, maintained conditions become Agent-owned runtime responsibility, Curation and Strategy are distinct theory consumers, active Capabilities come from activation, and situated cognition never becomes PDS state.

Future contract design may also assume exact semantic lineage, non-destructive history, a visible activation-generation boundary, owner-local semantic validation, package-blind runtime substrates, and principal-facing inspection as correlation rather than source truth.

It may not assume one Agent per product, a final profile syntax, a final PDS crate, one storage model, a universal facet protocol, a static projection declaration, an upgrade choreography, a process placement, or a PDS-owned episode engine.

## Confidence And Stop Point

Confidence is high on the product versus runtime split, federated semantic ownership, plural compilation target, Agent maintained-condition ownership, Curation and Strategy separation, Capability and authority separation, proof boundary, and exact lineage requirement.

Confidence is moderate on the final number of conceptual records and the need for a first-class PDS lifecycle source of truth beyond exact receipts and assignment generations. The roles are distinct, but current code does not yet prove their optimal storage decomposition.

Confidence is deliberately low on customer syntax, Agent topology, crate extraction, actor placement, generic facet lifecycle, and upgrade coexistence.

This is the correct discovery stop. The architecture framing is sufficiently stable to guide contract design, but this document does not authorize those contracts or decide the remaining product choices.
