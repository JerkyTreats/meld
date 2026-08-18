# Freeform Narrative Database Stewardship

Date: 2026-08-15
Status: discovery constraint example
Scope: ongoing stewardship of narrative structure and derived scene projections supplied by an external narrative product

## Working Use Case

An external product accepts user-authored stories, extracts narrative structure, lets a user select or describe a scene, and produces a render-ready narrative specification for a separate visual system.

Meld first serves as a durable narrative database. PDS is considered only for the later responsibility of maintaining identity, claim provenance, conflict visibility, accepted current projections, and derived-scene freshness as the story changes.

```text
persist and query narrative structure
!= continuously steward narrative structure
```

The database milestone is independently valuable and remains the mandatory baseline. This example fails if PDS is required merely to append, query, or replay narrative records.

## Domain Boundary

The bounded subject is one declared narrative corpus and its versioned structural products. These may include documents, passages, scenes, beats, entities, characters, places, objects, mentions, claims, events, qualified participants, relationship claims, state assertions, visual scene candidates, and revisions.

The external narrative product owns source documents, exact passage coordinates, extraction runs, mentions, identity candidates, scoped narrative claims, editorial acceptance, render requests, and render-ready narrative specifications.

Meld owns only the durable event acceptance, graph materialization, bounded traversal, current-anchor selection, replay, and stewardship state that is explicitly activated within Meld.

The external visual product owns appearance references, rendering style, model selection, image generation, image editing, pose control, and visual identity evaluation. Narrative and visual identity may refer to the same character without becoming one authority.

## Standing Condition

The candidate stewardship conditions are:

- preserve exact evidence recovery from accepted narrative products to source revision and passage
- keep mention identity provisional until editorial or policy authority resolves it
- preserve viewpoint, branch, polarity, temporal bounds, and narrative scope through projection
- retain contradictory claims and prior current selections through revision
- keep selected entity, claim, scene, and derived specification projections aligned with accepted source products
- mark and refresh only affected derived scene specifications after accepted changes

Database integrity and query availability are prerequisites. They are not themselves proof of a PDS standing condition.

## Observations And Evidence

The external product may publish source revisions, extraction runs, mentions, identity candidates, claims, editorial decisions, scene selections, render requests, and later corrections through served contracts.

Every product preserves producer identity, source revision, exact passage coordinates, extraction version, narrative scope, viewpoint, temporal bounds, and publication lineage where relevant.

Stewardship observation actions may hydrate the authoritative event behind a graph fact, inspect neighboring passages, retrieve identity and relation neighborhoods, compare accepted claim sets, identify affected derived projections, or request clarification from the external product or principal.

Graph proximity is not claim support. Extractor confidence is not editorial acceptance. A current anchor selects a projection without destroying earlier products.

## Actions

Candidate stewardship actions include:

- propose an identity merge or split
- propose claim supersession or conflict classification
- request editorial clarification
- mark affected scene projections stale
- refresh a render-ready narrative specification through an external capability
- verify that a refreshed specification covers the current accepted claim set
- preserve an incomplete publication as non-current until semantic revision closure

The database baseline separately supports append, replay, query, hydration, and deterministic current selection. Those mechanics do not require an Agent decision.

## Outcomes

Database success requires exact replay, graph query, event hydration, evidence recovery, scoped contradictory claims, merge and split lineage, bounded publication, and served access without private store reads.

Stewardship success additionally requires evidence that:

- identity and claim maintenance persisted across new story revisions
- selective observation avoided unnecessary full-corpus reprocessing
- interventions respected editorial authority
- an accepted change marked only affected projections stale
- a refreshed narrative specification covers the active accepted claim set
- historical decisions remain explainable from exact package, source, assignment, and activation lineage

An image being generated is not evidence that the narrative specification was current or well supported.

## Authority

The external narrative product remains source and editorial authority. Meld may observe its products, form declared stewardship beliefs, propose maintenance actions, and invoke explicitly granted refresh capabilities.

Meld does not gain authority to rewrite source documents, accept extracted claims, merge identities, or mutate render history through possession of a query or generation capability.

The external product must use versioned served contracts. Direct access to Meld storage internals and duplicate indexes over private store formats are rejected integration paths.

## Why PDS

The mandatory baseline is external extraction, source and span storage, entity and claim records, graph projection, deterministic current-revision selection, bounded query, and replay.

PDS is justified only when the system must continuously detect stale derived material, reconcile identity, expose contradiction, select targeted observations, propose bounded maintenance, and verify later results under persistent authority constraints.

If extraction plus graph storage performs equivalently on incorrect merges, duplicate entities, contradiction visibility, evidence recovery, correction correctness, and affected-only refresh cost, the database remains useful and PDS is not justified.

## Physical Runtime Variants

The same stewardship package could compose:

- an in-process event and graph client with deterministic projection checks
- an owned subprocess for bounded extraction or render-spec generation
- a shared local narrative index with exact tenant and revision namespaces
- a remote narrative extraction service
- a persistent external product that publishes authenticated source advances and consumes served graph queries

The external product may remain continuously running while Meld is idle. Its hidden state never replaces Meld package, assignment, operation, source, or admission lineage.

## Explicit Non Commitments

This example does not commit Meld to image storage, visual model management, latent identity, one narrative ontology, one extractor, a narrative-specific planner, or direct external storage access.

It does not assume that every extracted field becomes a belief. It does not collapse the external product's editorial truth into Meld stewardship judgment. It does not treat a database adoption milestone as proof of PDS value.

## Adjacent Visual Discovery

[Visual Concept Expansion For Render Hydration](visual_concept_expansion.md) records the bounded product feature in which stable visual concepts expand into model-specific prompt and workflow material.

[Activation-Bounded Lateral Lowering](../../../ideas/activation_bounded_lateral_lowering.md) preserves the broader non-authoritative riff about contextual salience, tangential expansion, vector candidate generation, research corpora, and possible Regime interaction. The bounded visual feature does not depend on that generalized idea.

## Discovery Questions

- Which owner admits a new source domain into generic graph materialization
- Whether event hydration is an event query, graph result, or root-composed served view
- How one semantic story revision closes across bounded idempotent publication batches
- Which current selections belong in generic anchors and which remain external product read models
- How qualified narrative participation should be represented without central narrative enums
- Which accepted external products may become belief evidence and under what mapping

## Source Material

- [Original Constraint Case](../freeform_narrative_database.md)
- [Lore And Canon Example](../lore_and_canon/README.md)
- [Roleplay Continuity Example](../roleplay_character_continuity/README.md)
- [Expression Catalog](../pds_expression_catalog.md)
- [PDS Router Detailed Design](../../../plan/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Semantic Unit Preservation Policy](../../../../governance/semantic_unit_preservation_policy.md)
- [Proposal Status](../../proposal_status.md)
