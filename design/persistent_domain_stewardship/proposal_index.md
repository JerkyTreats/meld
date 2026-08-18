# Persistent Domain Stewardship Proposal Index

Date: 2026-08-18
Status: active proposal index under a canonical semantic boundary
Scope: navigation and interpretation guide for the PDS proposal corpus

## Start Here

Persistent Domain Stewardship now has a canonical semantic boundary and a still-proposed upper design surface.

Read [Canonical Persistent Domain Stewardship](../cognitive_architecture/persistent_domain_stewardship.md) first. It fixes what PDS may declare, what Meld cognition must produce, and how Strategy inputs remain independently owned. The documents here explore application, package, profile, meta-domain, integration, migration, and implementation representations within that boundary. Concrete types and schemas remain candidates unless explicitly accepted elsewhere.

Read [Proposal Status And Decision Semantics](proposal_status.md) before treating any proposed structure as a requirement.

## Current Thesis

```text
Meld cognitive runtime
    + PDS semantic package
    + customer profile
    + assignment
    + activation
    + Strategy runtime inputs
    + persistent domain state
        ↓
configured persistent domain steward
```

The current preferred interpretation is:

- Meld supplies the cognitive data plane.
- PDS supplies stable domain semantics and the user-facing application model.
- Participating domains retain authority for observation, belief, Agent judgment, execution, capabilities, and outcomes.
- PDS routing links semantic contributions and preserves exact lineage.
- Activation supplies exact capabilities and physical bindings.
- Agent supplies situated Goals, construction policy, and candidate judgment. Strategy supplies projection needs, separately admitted Methods, and candidates. Agent and execution establish and enforce authority decisions.

The upper authoring, profile, lifecycle, and projection representations remain subject to the open decisions and experiments in this directory.

## Recommended Read Order

1. [Canonical Persistent Domain Stewardship](../cognitive_architecture/persistent_domain_stewardship.md)
   Fixes the semantic and cognition boundary that every proposal must preserve.

2. [Proposal Status And Decision Semantics](proposal_status.md)
   Defines constraints, hypotheses, options, recommendations, and open decisions.

3. [Persistent Domain Stewardship Overview](README.md)
   Introduces stewardship, standing objectives, episodes, package concepts, and the initial docs-freshness slice.

4. [PDS Code Expression Map](code_expression_map.md)
   Maps the current PDS expression all the way to Rust/runtime contracts and distinguishes enforced types, typed-but-injected theory, docs-specific compatibility forms, reusable substrate, and design-only gaps.

5. [Persistent Domain Stewardship Architectural Invariants](architectural_invariants.md)
   Extracts the cross-expression architecture constraints that should remain stable across documentation, security, reliability, roleplay, lore, learner, portfolio, simulation, and physical-maintenance stewards.

6. [PDS Isolation And Runtime Portability](isolation_and_runtime_portability.md)
   Separates semantic, assignment, binding, state, failure, resource, effect, admission, and replay isolation across linked, subprocess, sidecar, remote-service, and persistent-controller placements.

   Read with the canonical [Runtime Lifecycle And Quiescence](../cognitive_architecture/runtime_lifecycle_and_quiescence.md) contract.

7. [PDS Examples Across Compilation Layers](examples/compilation_layer_span.md)
   Reinterprets the example corpus as a compiler pipeline from user intent through an approvable canonical declaration into compiled semantic IR and domain-owned runtime state.

8. [PDS Cognition Boundary Assessment](../plan/integration/pds_cognition_boundary_domain_assessment.md)
   Records the breadth-first domain assessment and frozen ownership map behind the canonical decision.

9. [PDS Meta-Domain](meta_domain.md)
   Compares central schema, federated facets, root-only composition, and rejected runtime-orchestrator models.

10. [PDS Authorized Design Delivery Program](../plan/integration/pds_authorized_design_delivery_program.md)
   Records the completed accepted design packets, delivery stages, review contract, and implementation handoff for `W00` through `W06`.

11. [PDS Authorized Implementation Workstreams](../plan/integration/pds_implementation_workstreams.md)
   Defines the authorized delivery workstreams through `W06`, with owners, dependencies, tasks, gates, code regions, and verification strategy.

12. [PDS Design-Gated Continuations](../plan/integration/pds_design_gated_continuations.md)
   Isolates the non-authorized dependency-security completion, Strategy replay, upper-layer schema, projection, and upgrade concepts.

13. [Steward Profile Abstraction](profile_abstraction.md)
   Separates expert package mechanics from customer intent and evaluates YAML, DSL, SDK, GUI, and conversational interfaces.

14. [Stewardship Facet Protocol](facet_protocol.md)
   Proposes one connector model for domain-owned package facets and multi-domain activation.

15. [Stewardship Package Model](package_model.md)
   Explores the complete operational-domain-theory representation. Read its central schema as one option rather than a settled architecture.

16. [Runtime Anchor Map](runtime_anchor_map.md)
    Maps PDS concepts to the implemented cognitive-runtime surfaces on `runtime-operator-visibility`.

17. [Candidate Implementation Requirements](candidate_implementation_requirements.md)
    Extracts possible requirements while preserving their proposal status.

18. [Use-Case Decomposition](use_case_decomposition.md)
    Defines stewardship qualification, mandatory baselines, decomposition, and falsification.

19. [Workflow Migration](workflow_migration.md)
    Explores stewardship as application truth above existing workflow and execution mechanics.

20. [Open Decisions](open_decisions.md)
    Records unresolved architecture choices and evidence required to resolve them.

21. [Evaluation Plan](evaluation_plan.md)
    Defines experiments that may validate, narrow, or reject the current hypotheses.

## Examples

- [PDS Example Router Corpus](examples/README.md)
  Indexes one folder per use case with a normalized README and `theory::router` specification.

- [Theory Router Synthesis Across PDS Examples](examples/theory_router_synthesis.md)
  Collects the common route spine, owner-specific meanings, falsification cases, and unresolved shared theory bridges revealed by the example fanout.

- [PDS Examples Across Compilation Layers](examples/compilation_layer_span.md)  
  Spans documentation freshness, CVE exposure, roleplay, lore, code quality, game strategy, service reliability, learner mastery, portfolio thesis/risk, and physical maintenance across user intent, canonical declaration, compiler target, and runtime state.

- [PDS Expression Catalog](examples/pds_expression_catalog.md)  
  Normalizes documentation freshness, CVE exposure, roleplay continuity, lore/canon stewardship, codebase quality, game faction strategy, service reliability, learner mastery, portfolio thesis/risk, and physical asset maintenance against one comparative expression surface.

- [Roleplay Character Continuity Steward](examples/roleplay_character.md)  
  Detailed decomposition of a persistent interactive character that separates canon truth, character knowledge, commitments, relationships, narrative threads, and response authority.

- [Lore Transcriber And Canon Steward](examples/lore_transcriber.md)  
  Detailed decomposition of persistent entity and claim curation, including alias resolution, merge/split lineage, provenance, contradiction, supersession, and derived-definition freshness.

- [Freeform Narrative Database Constraint Case](examples/freeform_narrative_database.md)
  Grounds the lore candidate in an external story-to-visual product and defines current Meld limitations, database acceptance criteria, later stewardship criteria, domain boundaries, and explicit non-integration.

- [Software Quality Stewardship](examples/software_quality.md)  
  Full expert decomposition across reliability, persistence, performance, usability, maintainability, and documentation.

- [Software Quality Steward Profile](examples/software_quality_profile.md)  
  Customer-facing profile over the same package, including assignment, activation, semantic diff, and refinement.

## Proposal Layers

```text
Customer interaction
    profile language / forms / conversational editor
        ↓
PDS control plane
    package, profile, assignment, activation, linking, lineage
        ↓
Domain facet boundary
    domain-owned validation, registration, runtime behavior
        ↓
Cognitive runtime
    sensory, events, graph, belief, Agent, goals, planning, tasks, capabilities
        ↓
PDS projection
    user-facing stewardship status and history
```

## Current Recommendations

The current recommendations are:

- preserve the canonical semantic boundary while testing upper-layer representations;
- treat PDS as a semantic and application layer rather than a second cognitive runtime;
- explore federated domain-owned facets;
- keep package, profile, assignment, and activation conceptually distinct;
- use a small structured customer profile with generated visual and conversational editing;
- reuse existing cognitive-runtime contracts;
- treat workflows as separately admitted compatibility Methods where useful rather than PDS theory;
- preserve documentation freshness as the parity consumer, use the bounded dependency-security slice as the next proof, and keep full completion plus any non-software steward design-gated;
- use dissimilar expression cards to reject false-positive PDS use cases before freezing a package source schema;
- require a new expression to lower through generic domain contracts rather than adding expression-name semantic branches to root runtime code;
- distinguish a typed Rust shape from complete PDS implementation by checking generality, durability, revision lineage, and authority ownership separately;
- treat current detailed PDS examples as semantic compiler targets plus explicit runtime-assembly inputs, and require future examples to keep that split visible;
- require every canonical example to include a router-oriented use-case entry and per-example route specification before it informs the common theory set;
- preserve separate durable identities for the principal-approved canonical declaration and the compiled stewardship image that the runtime executed;
- treat runtime placement as an activation concern and isolation as a vector of explicit guarantees rather than a package property or one process-mode flag;
- carry assignment, activation generation, stable operation, attempt, and passive-delivery lineage through every external result admission path.

## Current Non-Decisions

The proposal does not currently select:

- an implementation crate layout;
- a canonical source language;
- central schema or facets as the final package model;
- principal-facing objective projection and stewardship episode ownership;
- the concrete context-projection execution contract beyond Goal-derived request ownership;
- a final workflow migration endpoint;
- a package scenario language;
- a package upgrade protocol;
- a universal process topology or sandbox implementation.

## Promotion Path

The proposal can promote its upper declaration and profile layers only after:

- documentation freshness migrates through the generic package and activation path with parity;
- dependency security proves the common form;
- customer profile authoring is tested;
- domain ownership is demonstrated;
- principal-facing objective and episode projection needs are resolved;
- authority and package upgrade paths are proven;
- workflow migration value is measured.

The current authorized dependency order and evidence gates through `W06` are recorded in [PDS Authorized Implementation Workstreams](../plan/integration/pds_implementation_workstreams.md). Later work remains subject to the design and reauthorization rules in [PDS Design-Gated Continuations](../plan/integration/pds_design_gated_continuations.md).

Until those gates close, this directory records upper-layer design options and evidence under the canonical semantic boundary.
