# Use Case Catalog

Date: 2026-07-24
Status: active
Scope: candidate stewardship use cases as design instruments, separated into stable descriptions, fluid Meld conversions, and required semantics

## Purpose

Each use case in this catalog is a design instrument. A use case stresses specific axes of the Strategy schema and the stewardship model, and the catalog exists so those axes are chosen deliberately rather than inherited from whichever example came first.

Every use-case document keeps three concerns separate:

1. **Use case.** A pure description of the problem in domain language, with no Meld vocabulary. This section is stable and survives any redesign of the runtime.
2. **Meld semantics conversion.** The proposed translation into subjects, relations, maintained conditions, belief questions, evidence, affordances, and derived topology. This section is fluid and non-authoritative. The transliteration into Meld changes as the schema matures, and disagreement between two conversions is a schema finding, not an error in either document.
3. **Required semantics.** The claims that must be true of Meld for the use case to be usable even on paper. These are the testable preconditions: if Meld cannot express one of them, the use case fails before any implementation question arises. Each entry names where the claim currently stands.

Worked Strategy derivations, when a use case has been proven at that depth, live under [World Model Strategy](../cognitive_architecture/world_model/strategy/README.md) as canonical documents. This catalog carries what canonical files cannot: history, status, motivation, and open questions.

## Relationship to other corpora

[Persistent Domain Stewardship](../persistent_domain_stewardship/use_case_decomposition.md) owns the qualification rubric: whether a use case is stewardship-shaped at all and whether a simpler deterministic baseline suffices. This catalog assumes that rubric and applies it per case.

[Strategy Requirements](../cognitive_architecture/world_model/strategy/requirements.md) STR-075 requires that the schema derive structurally distinct topologies across domains. The catalog is where candidate domains for that proof are compared before one is worked at full depth.

## Index

| Use case | Maintained condition | Axes stressed | Status |
|---|---|---|---|
| [Docs freshness](docs_freshness.md) | folder READMEs correct against code | containment obligations, bounded context, produced-artifact evaluation, bottom-up compression | worked canonical example; historical first proof |
| [CVE freshness](cve_freshness.md) | dependencies free of admitted advisories within policy | acquisition settlement, scoped negatives, constraint coupling, shared-artifact scoping, external drift | worked canonical example; STR-075 pair |
| [Test flakiness](test_flakiness.md) | per-test flakiness below threshold at confidence | statistical settlement, repeated observation, divergent-value alternatives, convergence in place of conditionals | proposed |
| [Commenting style](commenting_style.md) | code files comply with commenting policy | flat trivial topology as control, policy-revision mass invalidation, judgment evaluators | proposed control |
| [Podcast series](podcast_series.md) | a coherent teachable series exists for a topic | created subjects, staged grounding, sequence obligations, top-down constraint artifacts | frontier |

## Selection guidance

A new worked example should break the pattern of the existing ones on as many axes as possible. A use case that resembles a worked example validates little; one that bends the schema in a named place is doing its job. Findings that force schema amendments are the desired output, not a failure of the use case.

## Read with

- [World Model Strategy](../cognitive_architecture/world_model/strategy/README.md)
- [Strategy Ground Map](../plan/world_model/strategy/ground_map.md)
- [Persistent Domain Stewardship](../persistent_domain_stewardship/README.md)
