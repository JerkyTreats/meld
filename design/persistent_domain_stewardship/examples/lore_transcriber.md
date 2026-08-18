# Lore Transcriber And Canon Steward

Date: 2026-08-08  
Status: illustrative  
Scope: a Persistent Domain Stewardship expression for extracting and maintaining durable definitions of people, places, things, events, concepts, and relationships from an evolving body of user-authored material

> This long-form source case predates the canonical cognition split. Use the normalized [use case](lore_and_canon/README.md), [router specification](lore_and_canon/router_spec.md), and [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) when ownership language conflicts.

## Purpose

This example tests whether PDS can support a persistent lore or knowledge transcriber without reducing the problem to entity extraction or retrieval.

The simplest version of the use case is:

```text
receive text
→ extract entities
→ write summaries
```

That is a transcription workflow, not Persistent Domain Stewardship.

The stewardship-shaped form is a **canon steward** responsible for maintaining a coherent, provenance-preserving entity and claim model while new material continually introduces aliases, corrections, contradictions, temporal changes, and incomplete definitions.

The mandate is not to manufacture a complete ontology. It is to keep the declared working corpus internally inspectable and progressively reconciled while preserving uncertainty and source authority.

## Qualification

| Property | Lore/canon stewardship |
|---|---|
| Bounded domain | A declared project, corpus, world, research notebook, or user workspace |
| Persistent mandate | Definitions remain maintained as new source material arrives |
| Independent change | New files, notes, conversations, imports, and user corrections may arrive without a prior steward action |
| Partial observability | Mentions may be ambiguous, contradictory, incomplete, or temporally scoped |
| Standing objective | Maintain stable identity, provenance, conflict visibility, and current derived definitions |
| Observation choice | Retrieve source passages, neighboring mentions, prior aliases, and candidate identity matches |
| Intervention choice | Create, link, merge, split, supersede, summarize, defer, or ask for clarification |
| Action economics | Full-corpus reprocessing is expensive; targeted reconciliation is cheaper |
| Feedback | Later references, user corrections, accepted merges, and contradiction resolution evaluate prior decisions |
| Authority boundary | Extraction does not imply authority to declare every inferred claim canonical |
| Temporal continuity | Entity identity and claim history matter across the lifetime of the corpus |
| Audit value | Users may need to know where a definition or relation came from and why it changed |

**Verdict:** conditional PDS fit. Raw extraction remains a workflow. Persistent canon maintenance with identity resolution, contradiction handling, active disambiguation, supersession, and bounded authority is stewardship-shaped.

## Domain Boundary

### In scope

- a bounded user or project corpus;
- people, characters, places, organizations, objects, events, concepts, terms, and other declared entity classes;
- aliases and identity candidates;
- source-backed claims and relationships;
- temporal validity;
- corrections, retractions, and supersession;
- generated definitions and summaries as derived artifacts;
- unresolved ambiguity and contradiction.

### Out of scope

- treating generated summaries as primary source truth;
- silently merging ambiguous identities;
- universal ontology design;
- unrestricted web-scale entity resolution unless explicitly activated as an external evidence source;
- modifying original user material merely to make the extracted model consistent;
- converting model inference into authoritative fact without declared evidence semantics.

### Principal

The principal is the user, project owner, or corpus authority that defines:

- source scope;
- entity classes of interest;
- authority ordering among sources;
- acceptable automatic reconciliation;
- when user confirmation is required;
- whether the steward maintains private notes, shared canon, or both.

## Descriptive Model

Representative subject types:

```text
lore.entity
lore.person
lore.character
lore.place
lore.organization
lore.object
lore.event
lore.concept
lore.term
lore.alias
lore.claim
lore.relationship
lore.source
lore.definition
lore.revision
```

Representative relations:

```text
alias names entity
claim concerns entity
claim supported_by source
claim contradicted_by claim
claim superseded_by claim
entity related_to entity
entity participated_in event
entity located_at place
entity member_of organization
definition summarizes entity
definition derived_from claim
entity possibly_same_as entity
entity split_from entity
entity merged_into entity
```

The graph may project current identity and relationships, but source claims and revision history remain durable evidence rather than being overwritten by the current summary.

## Identity And Supersession

Identity resolution is the central pressure point.

The steward must distinguish:

```text
new mention of known entity
new alias for known entity
possible identity match
confirmed merge
incorrect prior merge requiring split
same name, different entity
same entity, changed title or role
entity state changed over time
source correction
retroactive canon change
```

A high-confidence similarity score is not sufficient authority to destroy separate identities.

Ambiguous merges should remain explicit candidates until the configured confirmation threshold or principal approval is satisfied.

## Observation Model

Promoted observations may include:

```text
lore.source_added
lore.source_changed
lore.source_removed
lore.passage_observed
lore.entity_mention_observed
lore.claim_extracted
lore.alias_observed
lore.relationship_claim_observed
lore.user_correction_observed
lore.identity_confirmation_observed
lore.canon_revision_observed
```

Possible raw sources:

- chat turns;
- notes;
- manuscripts;
- design documents;
- transcripts;
- imported databases;
- structured forms;
- user corrections;
- external source systems when explicitly activated.

Every extracted claim should preserve a source reference and extraction lineage.

Silence is not negative evidence. Absence of a relation from one source does not imply that the relation is false.

## Operational Domain Theory

### Belief family: `lore.identity_match`

Question:

```text
How strongly does available evidence support that two mentions or entity records denote the same persistent subject?
```

Evidence may include:

- explicit aliases;
- names and titles;
- co-occurring relations;
- temporal compatibility;
- location and organization context;
- user confirmation;
- contradiction such as simultaneous incompatible identities.

The comparator must preserve a distinction between similarity and identity authority.

### Belief family: `lore.claim_support`

Question:

```text
How strongly does admissible evidence support the selected claim for the selected temporal interval and authority context?
```

The family can represent:

- supported;
- contradicted;
- superseded;
- source-disputed;
- temporally expired;
- unresolved.

### Belief family: `lore.definition_completeness`

Question:

```text
How strongly does the current derived definition cover the claim classes required by the active profile for this entity?
```

Completeness is profile-relative. A lightweight glossary entry and a full character dossier have different required fields.

### Belief family: `lore.definition_freshness`

Question:

```text
How strongly does evidence support that the current derived definition still reflects the active accepted claim set?
```

A source or canon revision may reduce freshness without implying that the definition is factually wrong in every respect.

### Belief family: `lore.conflict_state`

Question:

```text
Does the current claim set contain unresolved contradictions or incompatible identity assignments that materially affect the derived definition?
```

Conflict is first-class state. The steward should not hide disagreement by choosing whichever completion is easiest to summarize.

## Stewardship Charter

Representative standing conditions:

```text
maintain stable persistent identity for established entities
maintain provenance for every accepted claim
maintain unresolved identity ambiguity as explicit uncertainty
maintain contradictions as visible until resolved or superseded
maintain generated definitions against the active accepted claim set
refresh definitions when material source changes invalidate their evidence map
preserve revision history through merges, splits, corrections, and canon changes
```

A profile may choose different sensitivity for different classes. For example, a project may auto-accept harmless aliases while requiring approval for entity merges.

## Observation Actions

Candidate observation actions:

- retrieve exact source passages for a claim;
- retrieve all mentions of a candidate entity;
- retrieve aliases and relation neighborhoods;
- compare temporal constraints for possible identity matches;
- inspect prior merge or split rationale;
- inspect sources with higher declared authority;
- ask the principal a targeted identity or contradiction question;
- re-extract only the affected source region after an edit.

## Intervention Actions

Candidate interventions:

- create a provisional entity;
- add an alias;
- attach a source-backed claim;
- create a possible-identity relation;
- propose an entity merge;
- execute an approved merge;
- split an incorrectly merged entity while preserving lineage;
- mark a claim contradicted or superseded;
- update a derived definition;
- emit a conflict report;
- request clarification;
- regenerate downstream summaries affected by a changed claim.

These actions should remain distinct from source ingestion and from user-facing prose generation.

## Known Methods

### `lore.ingest_and_reconcile`

Illustrative composition:

```text
ingest changed source region
→ extract mentions and candidate claims
→ resolve high-confidence stable identities
→ preserve ambiguous matches as candidates
→ publish claims with provenance
→ ground affected belief families
→ identify stale definitions or material conflicts
→ update only affected derived artifacts
```

### `lore.resolve_identity_ambiguity`

Illustrative composition:

```text
collect candidate mentions and relation neighborhoods
→ compare temporal and relational compatibility
→ inspect high-authority source evidence
→ if confidence and policy permit, merge
→ otherwise request principal confirmation
→ preserve decision lineage
```

### `lore.refresh_definition`

Illustrative composition:

```text
resolve entity identity
→ collect active accepted claims
→ include unresolved conflicts explicitly where profile requires
→ generate bounded definition artifact
→ attach evidence map
→ verify coverage and freshness
```

## Outcome Model

Mechanical completion:

```text
entity record or definition artifact written
```

This does not prove successful stewardship.

Verification evidence may include:

- future mentions resolve to the intended persistent entity;
- source citations remain recoverable;
- a definition's evidence map covers its material claims;
- user correction can supersede an earlier claim without history loss;
- an approved merge does not introduce temporal or relational contradictions;
- a split restores formerly conflated identities;
- changed source material causes only affected definitions to become stale;
- replay under the same package revision yields equivalent identity and claim decisions.

Harmful outcomes include:

- merging two different people because their names are similar;
- duplicating one entity indefinitely because aliases are not reconciled;
- presenting model inference as user-authored canon;
- removing contradictory evidence from history;
- rewriting prior temporal facts using later state;
- failing to refresh a summary after a material accepted claim changes;
- propagating one incorrect merge across many downstream definitions.

## Governance

Illustrative default postures:

| Action | Default posture |
|---|---|
| read activated source corpus | Observe |
| extract provisional mentions and claims | Draft |
| create provisional entity | Draft |
| add low-risk alias with strong evidence | policy-dependent |
| update generated definition from accepted claims | Draft or ExecuteReversible |
| merge ambiguous persistent identities | approval required |
| split established identity | approval or elevated policy |
| mark source claim superseded from explicit user correction | policy-dependent |
| delete original source material | Prohibited |
| fabricate unsupported canon | Prohibited |

## Simpler Baseline

The required baseline is:

```text
named-entity recognition or LLM extraction
+ vector retrieval
+ deterministic document store
+ optional knowledge graph
+ generated summaries
```

PDS is justified only if it measurably improves outcomes such as:

- duplicate-entity rate;
- incorrect-merge rate;
- successful resolution of later references;
- provenance coverage;
- explicit contradiction coverage;
- correction and supersession correctness;
- incremental refresh cost versus full-corpus regeneration;
- ability to reconstruct why an identity or claim was accepted.

## Relationship To Roleplay Continuity

The lore transcriber and the roleplay character steward are separate PDS expressions even when they share the same narrative corpus.

The lore steward maintains descriptive canon and identity:

```text
Who or what exists?
What claims are supported?
Which records refer to the same entity?
What changed or was superseded?
```

The roleplay steward maintains perspective-bound interactive continuity:

```text
What does this character know?
What commitments and relationships constrain the next response?
What may this character reveal or act upon?
```

They may share source observations and graph identities while declaring different belief families, standing conditions, actions, and authority.

A useful composition is therefore:

```text
user material
→ lore/canon stewardship
→ stable entity and claim projection
→ character-specific grounding
→ roleplay continuity stewardship
→ interactive response
```

Neither steward should absorb the other's normative responsibilities merely because they use common evidence.

## What This Example Tests

This case stresses:

1. **Identity resolution as a persistent responsibility.** Extraction is episodic; identity stewardship survives every ingestion run.
2. **Claim provenance.** Derived definitions must remain traceable to source evidence.
3. **Correction without history destruction.** Supersession and split/merge lineage matter.
4. **Active uncertainty reduction.** The steward may selectively retrieve or ask rather than guessing.
5. **Derived-artifact maintenance.** Summaries are projections that become stale when underlying accepted claims change.
6. **Separation from knowledge-graph mechanics.** A graph represents entities and relations; stewardship decides how uncertain evidence becomes maintained domain meaning under authority constraints.

If these behaviors lower through existing Meld theory and runtime contracts without a bespoke lore runtime, the case provides evidence that PDS can host persistent semantic curation as well as operational maintenance.

The concrete external-product constraints, current runtime limitations, and acceptance criteria for using Meld as a freeform narrative database are specified in [Freeform Narrative Database Constraint Case](freeform_narrative_database.md).

## Read With

- [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md)
- [Use-Case Decomposition](../use_case_decomposition.md)
- [Roleplay Character Continuity Steward](roleplay_character.md)
- [Freeform Narrative Database Constraint Case](freeform_narrative_database.md)
- [Software Quality Stewardship Example](software_quality.md)
