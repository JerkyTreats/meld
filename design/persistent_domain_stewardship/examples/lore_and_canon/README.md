# Lore And Canon Stewardship

Date: 2026-08-15
Status: discovery example
Scope: identity, claim, provenance, conflict, and derived-definition maintenance for an evolving corpus

## Working Use Case

A lore and canon steward maintains durable entity identity, source-backed claims, visible contradiction, non-destructive supersession, and current derived definitions while new material arrives over time.

The maintained condition is not that text has been extracted. It is that the bounded corpus remains inspectable and progressively reconciled without hiding ambiguity or granting model inference the authority of source canon.

```text
source passage
!= extracted mention
!= identity hypothesis
!= source-backed claim
!= accepted current interpretation
!= generated definition
```

## Domain Boundary

The bounded subject is one declared project, corpus, world, research notebook, or user workspace. It may contain people, characters, places, organizations, objects, events, concepts, terms, aliases, relationships, claims, definitions, and revisions.

Universal ontology design, silent identity merge, unrestricted web-scale resolution, source rewriting, and automatic conversion of inference into canon remain outside the boundary.

The principal is the corpus authority. The principal defines source scope, authority ordering, entity classes of interest, acceptable automatic reconciliation, approval thresholds, and whether a projection is private notes, shared canon, or another named branch.

## Standing Condition

The candidate standing conditions are:

- keep established entity identity stable and resolvable across aliases and revisions
- preserve exact provenance for every accepted claim
- keep identity ambiguity and contradictory claims visible until resolved or superseded
- preserve merge, split, correction, and canon-revision lineage
- keep derived definitions aligned with the active accepted claim set
- refresh only affected derived artifacts when material evidence changes

Completeness remains profile-relative. A glossary entry and a character dossier need not require the same claim classes.

## Observations And Evidence

Candidate observations include source additions, changes, and removals; passages; mentions; extracted claims; aliases; relationship claims; user corrections; identity confirmations; and canon revisions.

Every extracted product preserves the source revision, exact passage coordinate, extractor identity, extraction lineage, narrative scope, viewpoint, temporal bounds, and authority class where applicable.

Observation actions may retrieve exact supporting passages, collect all mentions of a candidate entity, inspect alias and relationship neighborhoods, compare temporal constraints, inspect prior merge rationale, query higher-authority sources, re-extract one affected region, or ask a targeted clarification question.

Silence is not negative evidence. Similarity is not identity authority. An extraction score is not belief confidence or editorial acceptance.

## Actions

Candidate actions include:

- create a provisional entity or claim
- add a strongly supported low-risk alias
- preserve a possible-identity relation
- propose, approve, or execute an entity merge under policy
- split an incorrect merge while preserving lineage
- mark a claim contradicted or superseded
- update a derived definition with an exact evidence map
- emit a conflict report or request clarification
- refresh downstream projections affected by an accepted change

Source ingestion, stewardship judgment, and prose generation remain separate actions.

## Outcomes

Writing an entity record or definition artifact is mechanical completion. Stewardship success requires evidence such as:

- later mentions resolve to the intended persistent entity
- every material definition claim remains traceable to source passages
- approved merges introduce no unresolved temporal or relational incompatibility
- a split restores formerly conflated identities
- corrections supersede prior claims without destroying history
- only affected definitions become stale after a material source change
- replay under the same exact package and source revisions yields equivalent identity and claim decisions

User correction and later references can revise confidence in earlier reconciliation decisions. They do not erase the earlier evidence or authority context.

## Authority

Reading the activated corpus and retrieving evidence are observation authority. Extraction and provisional records are draft authority. Low-risk aliases and reversible definition refresh may be allowed by policy. Ambiguous merge, established identity split, and source-authority changes require approval by default.

The steward is prohibited from deleting original source material, fabricating unsupported canon, or rewriting a principal-authored record merely to make the graph consistent.

Different assignments may share source records while using distinct authority profiles, current projections, private annotations, and acceptance thresholds. Sharing storage does not collapse their normative state.

## Why PDS

The mandatory baseline is extraction, vector retrieval, deterministic document and claim storage, an optional knowledge graph, and generated summaries.

PDS is justified only when continuous selective reconciliation materially improves duplicate rate, incorrect merge rate, provenance coverage, contradiction visibility, supersession correctness, affected-only refresh cost, or reconstruction of why an identity or claim became current.

Raw transcription and static graph storage remain useful without PDS. Stewardship begins where identity, conflict, freshness, and bounded intervention persist across ingestion episodes.

## Physical Runtime Variants

The same package meaning could be realized through:

- trusted in-process parsing and deterministic graph queries
- an owned extraction subprocess over one admitted source batch
- a shared local index with assignment-safe namespaces and exact source revisions
- a remote extraction or identity-resolution service returning untrusted candidates
- a persistent external source watcher publishing authenticated revision advances

No placement becomes canon authority. External outputs pass lore owner admission before they become claims, identity candidates, or stewardship evidence.

## Explicit Non Commitments

This example does not commit Meld to one ontology, one extraction model, one graph implementation, one source format, automatic canon authority, or a lore-specific planner and runtime.

It does not decide whether editorial acceptance lives wholly in an external product or partly in a future narrative owner domain. It does not assume every extracted field becomes a Meld belief.

## Discovery Questions

- Which owner holds canonical entity and claim products when the source corpus lives in an external product
- Which identity decisions are editorial authority and which are evidence-grounded stewardship judgments
- Which current selections deserve generic graph anchors and which remain lore-domain projections
- How should exact source hydration cross a served boundary without duplicating producer truth
- What proof is sufficient before an automatic alias, merge, or definition refresh may execute

## Source Material

- [Original Lore Transcriber Example](../lore_transcriber.md)
- [Freeform Narrative Database Example](../freeform_narrative_database/README.md)
- [Expression Catalog](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [PDS Router Detailed Design](../../../completed/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Use Case Decomposition](../../use_case_decomposition.md)
- [Proposal Status](../../proposal_status.md)
