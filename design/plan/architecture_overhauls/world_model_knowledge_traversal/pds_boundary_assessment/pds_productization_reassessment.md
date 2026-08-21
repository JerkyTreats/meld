# PDS Productization Through World Model Reconciliation

Date: 2026-08-20

Status: corrected discovery synthesis, implementation not authorized

## Correction

The first assessment answered the wrong question.

It asked whether Persistent Domain Stewardship was required inside World Model Reconciliation. The answer is no. Product domains, Traversal, Curation, Belief, Agent, Strategy, Events, and Execution already divide the situated reconciliation loop among the correct runtime owners.

That does not make PDS optional administration. PDS was designed to answer a different question:

> How does a principal turn Meld into a durable, named, configurable software product whose Agents continuously steward selected domain concerns?

The canonical architecture already answers that question directly. [Persistent Domain Stewardship](../../../../cognitive_architecture/persistent_domain_stewardship.md) defines PDS as the semantic program by which durable domain meaning reaches Meld as declarations. It explicitly distinguishes that semantic program from Meld cognition.

The corrected architecture is:

```text
principal product intent
-> PDS declaration and semantic packages
-> owner-validated operational theory revisions
-> assignment and activation
-> configured Agents
-> World Model Reconciliation
```

PDS productizes Meld. World Model Reconciliation operates the product.

## Sharp Thesis

PDS is the declarative product and lifecycle model for configured stewardship. World Model Reconciliation is the native runtime that grounds that product against a current subject, perspective, world state, capability catalog, and authority context.

An Endless Horizon software developer is therefore naturally a PDS product expression. It may combine documentation freshness, dependency security, commenting governance, test thoroughness, and other maintained concerns into a versioned semantic program. The program causes one or more Agents to exist with exact domain theory, bounded subjects, perspectives, grants, and physical runtime bindings.

The product expression is not the cognitive loop. It does not contain current observations, beliefs, Goals, Plans, Epistemic Operations, Tasks, or outcomes. Those are produced after the semantic program is installed and grounded by Meld.

## What Data Flies Through The Runtime

The statement that PDS is the data flying through the runtime is directionally correct, with one necessary boundary.

PDS supplies stable, state-free semantic data and exact lineage. This includes domain vocabulary, relevant event and fact kinds, evidence and proof semantics, maintained conditions, normative policy, settlement obligations, outcome interpretation, governance classifications, and activation requirements. Packages carry those declarations to their owning domains. Assignments bind them to a principal, Agent, subject, perspective, branch, and grant lineage. Activations bind the assignment to physical resources and implementations.

Situated state is not PDS data. Current observations, graph products, admitted evidence, beliefs, Goals, Plans, Epistemic Operations, Tasks, execution outcomes, and reconciliation decisions remain owned by the native runtime domains that create them.

This gives the boundary a useful formulation:

```text
PDS carries what the product means and how it is instantiated.
World Model Reconciliation determines what is true now and what must happen next.
```

## Agent Directives And Compilation

The proposal that PDS packages reduce to Agent directives is also directionally correct, but too narrow as a compilation contract.

The Agent directive is the organizing statement of responsibility. A package compiler must fan that product intent into several independently owned runtime inputs rather than one opaque directive blob. Current Docs Freshness already selects a belief family, outcome mapping, curation rule, maintained condition, Strategy theory, authority policy, and docs claim policy through [`TheorySelection`](../../../../../src/config/stewardship/selection.rs). Its installed package routes those components to world-model, docs, and Execution owners through [`pds-package.json`](../../../../../theory/docs_freshness/pds-package.json).

The target compilation model is therefore:

```text
product expression
-> Agent directive and maintained concerns
-> owner-installed belief and evidence theory
-> owner-installed Curation operation theory
-> owner-installed Strategy Plan theory
-> product-owned policy and outcome interpretation
-> assignment and activation requirements
```

The exact active Capability catalog remains an activation and runtime input. It must not be frozen into the semantic body as the only toolbox Strategy may consider. Effective authority also remains a situated decision rather than package truth.

## Current Code Confirmation

The current repository contains a substantial partial expression of this architecture.

[`PdsPackageManifestV1`](../../../../../src/theory/package.rs) gives a package an exact identity, exact imports, routed components, owner identities, schema revisions, bounded content references, and structural requirements. [`TheoryRouter`](../../../../../src/theory/router.rs) validates package structure, delegates semantic validation and installation to owner handlers, preserves exact owner revision references, validates semantic links, and records an installation receipt. This is a structural linker and installer rather than a semantic super-domain.

[`StewardshipDeclaration`](../../../../../src/config/stewardship/selection.rs) currently selects an expression, target, subject, Agent, principal, provider, and theory identities. Its own documentation says it is smaller than the eventual principal-facing declaration and deliberately avoids embedding owner-controlled theory bodies.

[`StewardshipAssignmentV1`](../../../../../src/config/stewardship/assignment.rs) binds an exact package receipt to a principal, Agent, subject, perspective, branch, requested authority, and grant lineage. [`StewardshipActivationV1`](../../../../../src/config/stewardship/activation.rs) separately binds that assignment to physical resources, selected implementations, placement, isolation, and operating limits.

The world initialization pipeline then registers the Agent, binds exact Curation and maintained-condition revisions, creates its belief subscription, and marks it operational in [`pipeline.rs`](../../../../../src/init/world/pipeline.rs). Runtime activation records an assignment generation and resolves stable runtime references for the Agent, belief scope, Capability catalog, Execution scope, and admission in [`activation.rs`](../../../../../src/runtime/activation.rs).

These are not merely names left over from a discarded theory. Together they already express the intended productization sequence:

```text
package receipt
+ assignment
+ activation
-> operational Agent and installed domain theory
```

The implementation is partial and some current component choices conflict with the emerging reconciliation architecture. The existence of those gaps does not weaken the PDS framing. It identifies the exact integration work that later requirements must assess.

## Endless Horizon Software Developer

The proposed product is a strong generality test.

An Endless Horizon software developer is not one giant Agent prompt and not one universal policy owner. It is a principal-facing PDS expression that selects and composes several domain-owned stewardship concerns for a repository subject.

Documentation owns documentation requirement, claim, coverage, materiality, and correctness semantics. Dependency Security owns dependency identity, inventory completeness, advisory provenance, applicability, policy, posture, finding, and verification semantics. Governance domains own commenting and testing policy. World-model owners install belief, maintained-condition, Curation, and Strategy theory that uses those meanings. PDS links the exact revisions into an auditable product and binds that product to configured Agents.

One product expression may materialize one Agent with several maintained concerns or several specialized Agents coordinated around the same principal and subject. Current evidence does not decide that cardinality. PDS must own the declarative and lifecycle answer once the product model chooses it. Agent continues to own each materialized runtime identity and its situated state.

## Reassessment Of Docs Freshness

The docs freshness evidence proves that Docs owns correctness semantics and that World Model Reconciliation needs an epistemic-only path before Strategy constructs executable work. It also exposes the current defect where an unchanged README can settle only after the complete executable chain has run.

Its no-PDS conclusion does not survive the corrected question. The report showed that Docs plus root composition plus World Model Reconciliation can operate one already-configured concern. It did not explain how a principal declares a durable docs stewardship product, selects exact semantic revisions, assigns the product to an Agent and repository subject, activates its resources, upgrades it, or inspects which product definition is operating.

Current Docs Freshness supplies direct productization evidence. Its package contains owner-routed belief, outcome, Curation, maintained-condition, Strategy, authority, claim-policy, and Capability components. Its characterization fixture records input selection, exact installed revisions, activation, flywheel behavior, lifecycle, and reopen identity in [`expected_semantic_snapshot.json`](../../../../../tests/fixtures/pds/docs_characterization/v1/provider_free/expected_semantic_snapshot.json).

The specialist report therefore tells PDS what the docs product must package without transferring docs semantics to PDS. The reconciliation redesign changes some package targets. Curation must become genuine bounded epistemic work. Strategy theory must construct heterogeneous Plans. Capability availability must come from activation rather than a semantic shortlist. Those are PDS integration requirements to derive later, not reasons to remove PDS.

## Reassessment Of Dependency Security

The dependency security evidence proves that current admission is narrow and in memory, semantic results are not durably published, declared security Event types are not appended, and all four current Capability-shaped operations are epistemic in meaning. Those findings identify real World Model Reconciliation gaps.

They also strengthen the PDS product case. Dependency Security and Docs Freshness have sharply different product semantics but already fit the same package shape. The security package routes security policy to its product owner and routes belief, outcome, Curation, maintained-condition, Strategy, authority, and current Capability declarations to their respective owners in [`pds-package.json`](../../../../../theory/dependency_security/pds-package.json).

The target package should retain stable security meaning and the theory required to initialize its Agents. Inventory observation, advisory acquisition, assessment, and verification should be reconsidered as Curation operations where their end product is shared knowledge. Genuine repository or environment interventions remain executable capabilities supplied through activation and handled by Execution.

The specialist report therefore does not show that PDS is unnecessary. It shows that current PDS component routing must be reconciled with the EpiOp and Task distinction.

## Reassessment Of The Use Case Sweep

The use-case sweep established two valuable facts. Product semantics remain federated, and every situated cognitive loop can be expressed by its product owners plus World Model Reconciliation. Both findings remain valid.

The mistake was using those facts to demand that PDS own some residual runtime behavior before it could exist. PDS should not own such behavior. Its repeated cross-product function is declarative assembly and lifecycle:

```text
name a stewardship product
select exact owner theory
compose compatible concerns
bind principal and subject
materialize and manage Agents
bind physical activation
preserve version and lineage
inspect and upgrade the installed semantic program
```

The variety in the eleven cases is evidence for routed, owner-validated package components rather than a central PDS ontology. The recurrence of package, assignment, activation, Agent genesis, lineage, and upgrade pressure is evidence for a PDS product domain rather than eleven unrelated root configuration branches.

The mandatory simpler baseline still applies. A fixed scanner, controller, workflow, or policy engine should remain available when it solves the product. PDS earns its complexity when a product needs persistent, epistemically aware, dynamically planned reconciliation across changing evidence and capabilities.

## Correct Domain Boundary

PDS owns the principal-facing declaration of a stewardship product. It owns package identity, exact imports, structural component routing, exact installation receipts, product expressions and profiles, assignment identity, activation requirements, semantic-program lineage, and the management semantics for installation, upgrade, rollback, suspension, retirement, and inspection.

PDS coordinates compilation. It does not validate the meaning of owner components. Each product or runtime domain owns its component schema, semantic validation, installed revision, and interpretation.

PDS may declare that an Agent must maintain selected conditions under selected domain theory. Agent owns the resulting runtime record, current directive, Goal authority, Plan progression, and reconciliation judgment.

PDS may declare physical requirements and bind an activation to exact implementation offers. Capability owners define executable contracts. Activation establishes availability. Agent and Execution establish effective authority and safe realization.

PDS may carry stable graph vocabulary, fact kinds, relation kinds, admission policy references, and proof requirements through owner routes. Traversal owns graph materialization and reads. Curation owns situated epistemic authorship. Belief owns selective evidence admission.

## Domain Impact Map

The corrected concern changes the earlier impact classification.

| Domain | Relationship to PDS productization | Current evidence | Likely target pressure |
| --- | --- | --- | --- |
| theory | direct structural owner | package manifest, routes, owner handlers, receipts, exact imports | preserve linker role and add only proven component routes |
| config stewardship | current declaration adapter and identity model | expression selection, assignment, activation | evolve toward principal-facing product declaration and exact lifecycle contracts |
| init world | installation and Agent genesis adapter | installed theory binding, Agent registration, subscriptions | materialize the compiled product without interpreting product semantics |
| runtime | activation and lifecycle adapter | generation identity, catalogs, scopes, readiness | manage installed assignment generations and product upgrades |
| docs | semantic component owner | claim policy, observations, verification, executable capabilities | publish docs-owned Curation and Plan theory for package linkage |
| dependency security | semantic component owner | policy, inventories, advisories, assessments, verification | publish durable products and corrected Curation theory |
| world-model Agent | runtime consumer and owner | maintained conditions, Curation binding, subscriptions, Goals | accept installed theory and own all situated progression |
| world-model Strategy | runtime consumer and owner | current Strategy theory installation | consume owner-installed Plan theory without becoming package-aware |
| world-model Curation | future runtime consumer and owner | absent as a first-class domain | publish a route for stable operation theory and realize situated EpiOps |
| world-model Belief | runtime consumer and owner | belief families and outcome mappings | retain semantic validation and exact revision ownership |
| Traversal | reuse with owner-published semantics | generic graph reduction and reads | remain package-ignorant |
| Events | reuse unchanged | durable append, replay, cursors | remain package-ignorant |
| Capability and Execution | activation participants | exact contracts, implementations, Task realization | remove semantic-package toolbox coupling where necessary |
| meld-lang | reuse unchanged | permissive shared language | no PDS grammar enforcement |

This is an architecture impact map, not authorization for those writes. The exact affected set depends on the final PDS source model, Agent cardinality, upgrade semantics, and World Model Reconciliation contracts.

## What The Prior Reports Still Prove

The old synthesis correctly protected the sacred seams. PDS does not sit between Events and Traversal, Traversal and Curation, Curation and Agent, Agent and Strategy, or Agent and Execution. It does not own product vocabularies, situated evidence, beliefs, Plans, EpiOps, Tasks, Task Network, or outcome truth. It does not become a semantic super-domain merely because it links packages.

The old synthesis also correctly showed that runtime domains can operate without consulting PDS on every transition. Once an exact product generation has been installed, native owners communicate through their existing contracts. PDS becomes relevant again for lifecycle, inspection, upgrade, rollback, and lineage, not as an in-loop gate.

What is superseded is the conclusion that direct product registration through root composition is an adequate product model. That approach hardcodes each product into application assembly and recreates the exact productization problem PDS exists to solve.

## Open Design Questions

The high-level framing is confirmed. Several upper contracts remain open.

The product model must decide whether one expression materializes one multi-concern Agent, several specialized Agents, or a declared topology of Agents. It must define how principal-facing responsibilities compile into maintained conditions and directives without centralizing their semantics.

The package model must decide how new Curation operation theory and heterogeneous Strategy Plan theory are routed and linked. It must remove or reinterpret current semantic dependencies on exact Capability components while preserving activation requirements and co-shipped implementation discovery.

The lifecycle model must define semantic upgrade, rollback, Agent state continuity, reconciliation after theory revision, and historical interpretation against exact receipts. Current assignment and activation identities are strong implementation anchors, not yet accepted final contracts.

The product surface must define how a principal creates, inspects, revises, suspends, and retires an Endless Horizon software developer without exposing the full internal theory graph.

These are requirements inputs for a later specification. They are not settled by this reassessment.

## Final Finding

The user framing is confirmed with one refinement.

PDS is the answer to how Meld becomes a product through Domain Theory. It is the stable semantic program, declarative application model, and lifecycle control plane that turns principal intent into exact owner-installed theory, Agent assignments, and physical activations.

World Model Reconciliation is the runtime mechanics that ground and continuously reconcile that program against a changing world.

PDS is therefore not merely data passing through a runtime, and it is not merely thin administration. It is the versioned product definition that the runtime consumes. The runtime owns every situated cognitive and executable product derived from it.

The earlier no-PDS conclusion is refused. Its runtime ownership evidence remains valid and now supports a sharper architecture: federated semantic ownership, one productization model, and one native reconciliation runtime.
