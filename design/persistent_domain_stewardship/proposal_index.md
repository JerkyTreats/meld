# Persistent Domain Stewardship Proposal Index

Date: 2026-07-16  
Status: proposed  
Scope: navigation and interpretation guide for the PDS proposal corpus

## Start Here

Persistent Domain Stewardship is a non-authoritative design proposal.

The documents explore possible application, package, profile, meta-domain, integration, migration, and implementation models. Concrete types and schemas remain candidate designs unless explicitly identified as existing Meld constraints.

Read [Proposal Status And Decision Semantics](proposal_status.md) before treating any proposed structure as a requirement.

## Current Thesis

```text
Meld cognitive runtime
    + stewardship package
    + customer profile
    + assignment
    + activation
    + persistent domain state
        ↓
configured persistent domain steward
```

The current preferred interpretation is:

- Meld supplies the cognitive data plane.
- PDS supplies the user-facing application and control-plane model.
- Participating domains retain authority for observation, belief, Agent judgment, execution, capabilities, and outcomes.
- PDS links domain contributions, coordinates activation, preserves lineage, and presents a unified stewardship projection.

This interpretation remains subject to the open decisions and experiments in this directory.

## Recommended Read Order

1. [Proposal Status And Decision Semantics](proposal_status.md)  
   Defines constraints, hypotheses, options, recommendations, and open decisions.

2. [Persistent Domain Stewardship Overview](README.md)  
   Introduces stewardship, standing objectives, episodes, package concepts, and the initial docs-freshness slice.

3. [PDS Meta-Domain](meta_domain.md)  
   Compares central schema, federated facets, root-only composition, and rejected runtime-orchestrator models.

4. [Assessment By Domain](assessment_by_domain.md)  
   Applies Meld's domain-isolation policy and identifies ownership, adapter, optional, and non-integration relationships.

5. [Steward Profile Abstraction](profile_abstraction.md)  
   Separates expert package mechanics from customer intent and evaluates YAML, DSL, SDK, GUI, and conversational interfaces.

6. [Stewardship Facet Protocol](facet_protocol.md)  
   Proposes one connector model for domain-owned package facets and multi-domain activation.

7. [Stewardship Package Model](package_model.md)  
   Explores the complete operational-domain-theory representation. Read its central schema as one option rather than a settled architecture.

8. [Runtime Anchor Map](runtime_anchor_map.md)  
   Maps PDS concepts to the implemented cognitive-runtime surfaces on `runtime-operator-visibility`.

9. [Candidate Implementation Requirements](candidate_implementation_requirements.md)  
   Extracts possible requirements while preserving their proposal status.

10. [Use-Case Decomposition](use_case_decomposition.md)  
    Defines stewardship qualification, mandatory baselines, decomposition, and falsification.

11. [Workflow Migration](workflow_migration.md)  
    Explores stewardship as application truth above existing workflow and execution mechanics.

12. [Open Decisions](open_decisions.md)  
    Records unresolved architecture choices and evidence required to resolve them.

13. [Evaluation Plan](evaluation_plan.md)  
    Defines experiments that may validate, narrow, or reject the current hypotheses.

## Examples

- [PDS Expression Catalog](examples/pds_expression_catalog.md)  
  Normalizes documentation freshness, CVE exposure, roleplay continuity, lore/canon stewardship, codebase quality, game faction strategy, service reliability, learner mastery, portfolio thesis/risk, and physical asset maintenance against one comparative expression surface.

- [Roleplay Character Continuity Steward](examples/roleplay_character.md)  
  Detailed decomposition of a persistent interactive character that separates canon truth, character knowledge, commitments, relationships, narrative threads, and response authority.

- [Lore Transcriber And Canon Steward](examples/lore_transcriber.md)  
  Detailed decomposition of persistent entity and claim curation, including alias resolution, merge/split lineage, provenance, contradiction, supersession, and derived-definition freshness.

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

- preserve PDS as a proposal until experiments resolve the central decisions;
- treat PDS as a control-plane meta-domain rather than a second cognitive runtime;
- explore federated domain-owned facets;
- keep package, profile, assignment, and activation conceptually distinct;
- use a small structured customer profile with generated visual and conversational editing;
- reuse existing cognitive-runtime contracts;
- treat workflows as compatibility methods where useful rather than deleting proven mechanics;
- start with documentation freshness, then test software performance and one non-software steward;
- use dissimilar expression cards to reject false-positive PDS use cases before freezing a package source schema.

## Current Non-Decisions

The proposal does not currently select:

- an implementation crate layout;
- a canonical source language;
- central schema or facets as the final package model;
- authoritative ownership of standing objectives or episodes;
- a context-projection owner;
- a final workflow migration endpoint;
- a package scenario language;
- a package upgrade protocol.

## Promotion Path

The proposal can produce an authoritative implementation plan only after:

- one package/profile lowers into existing runtime contracts;
- one dissimilar package proves the common form;
- customer profile authoring is tested;
- domain ownership is demonstrated;
- standing objective and episode ownership is resolved;
- authority and package upgrade paths are proven;
- workflow migration value is measured.

Until then, this directory records design options and evidence rather than final architecture.