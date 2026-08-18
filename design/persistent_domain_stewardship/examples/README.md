# PDS Example Router Corpus

Date: 2026-08-18
Status: aligned validation corpus
Scope: normalized use-case examples and their `theory::router` attachment designs

## Purpose

Each PDS example now has two entry points:

- `README.md` solidifies the stewardship use case and its domain boundary
- `router_spec.md` lowers the use case through domain-owned theory routes and records what it teaches the shared theory set

The folders are design probes. They do not approve implementation, freeze a package schema, or make every candidate a valid stewardship application.

Every example follows the semantic boundary in [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md). Router components describe stable PDS meaning. Exact capabilities, Methods, construction policy, search controls, projection requests, and effective authority are shown as separate runtime inputs.

Existing long-form files in this directory remain source material. The folders provide a common comparison surface without erasing details that are useful only to one domain.

## Canonical Example Set

| Example | Use case and router | Distinctive router pressure |
| --- | --- | --- |
| documentation freshness | [README](documentation_freshness/README.md) and [router spec](documentation_freshness/router_spec.md) | exact local theory, selective provider binding, bounded workspace effects |
| dependency security | [README](dependency_security/README.md) and [router spec](dependency_security/router_spec.md) | external runtime, passive source advance, bounded negative evidence |
| roleplay character continuity | [README](roleplay_character_continuity/README.md) and [router spec](roleplay_character_continuity/router_spec.md) | perspective, disclosure authority, bounded context hydration |
| lore and canon | [README](lore_and_canon/README.md) and [router spec](lore_and_canon/router_spec.md) | identity reconciliation, claim provenance, merge and split authority |
| freeform narrative database | [README](freeform_narrative_database/README.md) and [router spec](freeform_narrative_database/router_spec.md) | falsification boundary between durable database and stewardship |
| codebase quality | [README](codebase_quality/README.md) and [router spec](codebase_quality/router_spec.md) | several concern families over one subject with competing protected floors |
| game faction strategy | [README](game_faction_strategy/README.md) and [router spec](game_faction_strategy/router_spec.md) | perspective-bound uncertainty and simulated strategic authority |
| service reliability | [README](service_reliability/README.md) and [router spec](service_reliability/router_spec.md) | high-rate observation, risky actuation, readiness and safe-point lifecycle |
| learner mastery | [README](learner_mastery/README.md) and [router spec](learner_mastery/router_spec.md) | active diagnosis, delayed outcomes, protected instructional authority |
| portfolio thesis and risk | [README](portfolio_thesis_and_risk/README.md) and [router spec](portfolio_thesis_and_risk/router_spec.md) | source-of-record separation and strict research versus execution authority |
| physical asset maintenance | [README](physical_asset_maintenance/README.md) and [router spec](physical_asset_maintenance/router_spec.md) | physical evidence, scheduling versus actuation, safety governance |

Software performance and reliability remain concern families within codebase quality rather than duplicate top-level examples.

## Reading The Router Specs

Every router spec should answer the same questions:

1. Which package components enter through shared or domain-owned routes?
2. Which owner interprets and installs each body?
3. Which cross-component relationships are structural and which remain owner semantic checks?
4. What belongs to assignment, activation, activation generation, participant incarnation, or external source state rather than package theory?
5. Which activation capabilities are selected and which authority gates remain external to PDS semantics?
6. How are observations or passive deliveries admitted as canonical domain products?
7. Which isolation and lifecycle properties vary by physical implementation?
8. What common theory mechanism does the example support, and what meaning must remain domain-owned?
9. What missing bridge or false abstraction does the example expose?

## Shared Synthesis

[Theory Router Synthesis Across PDS Examples](theory_router_synthesis.md) collects the common route spine, optional route families, domain-specific routes, and unresolved theory bridges revealed by the fanout.

## Source Corpus

- [PDS Expression Catalog](pds_expression_catalog.md)
- [PDS Examples Across Compilation Layers](compilation_layer_span.md)
- [Use-Case Decomposition](../use_case_decomposition.md)
- [PDS Router Detailed Design Specification](../../plan/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../isolation_and_runtime_portability.md)
