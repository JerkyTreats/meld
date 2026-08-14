# Freeform Narrative Database Constraint Case

Date: 2026-08-13
Status: illustrative constraint case
Scope: a concrete external-product use case used to expose Meld and PDS requirements for freeform narrative structure

## Purpose

This case grounds the lore and roleplay candidates in a product that is being designed now.

The product accepts user-authored stories, extracts durable narrative structure, lets a user select a scene, and sends a render-ready scene description to a separate visual-generation system. Character appearance and visual coherence are maintained by that external product. Meld is a candidate durable narrative database, not the image renderer or character-image authority.

The case is deliberately more specific than the general lore example. Its job is to reveal the minimum contracts Meld would need before an external product could truthfully adopt it as the narrative database.

It also separates two claims that should not be conflated:

```text
Meld can persist and query narrative structure

Meld can operate as a persistent narrative steward
```

The first claim is a database and graph-integration milestone. The second additionally requires continuous reconciliation, selective observation, bounded intervention, verification, and authority-aware canon maintenance.

## Current Product Use Case

The user flow is:

```text
submit a story
→ extract characters, descriptions, actions, scenes, and relationships
→ preserve exact supporting passages and unresolved ambiguity
→ select or describe a scene to visualize
→ hydrate current character and scene state
→ produce a render-ready scene specification
→ send that specification to the external visual system
```

Representative interactions include:

```text
Show me Kelsey in eveningwear
Have her lounging on a couch
Make the dress green
```

Those turns require two cooperating but independently useful systems:

- the narrative system resolves which character, state, scene, action, and source claims are relevant;
- the visual system owns appearance references, render style, image generation, image editing, pose control, wardrobe rendering, and visual identity evaluation.

The narrative system may describe a character's physical appearance and clothing claims. It does not own trained adapters, model weights, latent identity, image pixels, or render history merely because those assets refer to the same character.

## Freeform Narrative Structure

Freeform does not mean untyped storage. It means that the narrative vocabulary cannot be frozen to one story format, genre, or extraction schema.

A useful first model needs stable structural classes such as:

```text
corpus
document
chapter
passage
scene
beat
entity
character
place
object
mention
claim
event
event participant
relationship claim
state assertion
visual scene candidate
revision
```

It must also permit domain-owned open vocabulary beneath those classes, including new event types, character attributes, relationship meanings, narrative roles, and scene properties.

Examples include:

```text
character performs action
character participates in event with role
event occurs in scene
scene follows scene
event causes event
claim describes character
claim has narrative scope
claim has viewpoint
claim has polarity
claim is supported by passage
claim supersedes claim
mention may denote entity
visual scene candidate derives from scene and accepted claims
```

The model must preserve distinction among:

- a source passage;
- an extracted mention;
- a hypothesis that a mention denotes an entity;
- a source-backed claim;
- an accepted current interpretation;
- a generated summary or render specification.

Collapsing these into one entity record would destroy the evidence and revision semantics needed for later correction.

## Narrative Database Baseline

The required simpler baseline is:

```text
external extraction
+ source and span store
+ entity and claim records
+ graph projection
+ deterministic current-revision selection
+ bounded query API
```

This baseline should be implemented or simulated before claiming PDS value.

It is sufficient when the system only needs to ingest, retrieve, edit, and project narrative structure. PDS becomes justified when the system must maintain the structure over time by detecting stale derived material, reconciling identity, exposing contradictions, selecting targeted observations, proposing merge or split operations, and verifying later outcomes.

## Established Source Authority

The external narrative product owns the canonical narrative products it creates:

- source documents and exact passage coordinates;
- extraction runs and extractor versions;
- mentions and identity candidates;
- narrative claims and their scope;
- editorial acceptance or rejection;
- render requests and render-ready narrative specifications.

Meld owns durable event acceptance, graph materialization, traversal, current-anchor selection, replay, and later stewardship state that is explicitly activated in Meld.

Meld must carry producer-owned narrative products intact through event wrappers. It must not copy selected narrative fields into sibling wrapper fields and accidentally create a second authority.

The external product must use a served contract. It must not open Meld's storage engine directly.

## Current Meld Ground And Limitations

The table describes the repository state observed while defining this case. It is evidence for future planning, not a permanent architectural claim.

| Capability | Current ground | Limitation for this case | Required extension |
|---|---|---|---|
| Durable ingress | The event authority accepts producer-owned JSON payloads, object references, relations, idempotency, and replay | Narrative events can be appended but that alone does not make them queryable narrative graph facts | Preserve generic event ingress and define a narrative producer contract over it |
| Graph materialization | The traversal reducer materializes events only from `workspace_fs`, `context`, and `execution` | A `visual_narrative` event remains in the ledger but is skipped by graph reduction | Add a generic domain projection registration boundary or equivalent domain-owned reducer integration |
| Current anchors | Current anchor contracts are generic, while intent extraction recognizes a fixed set of workspace, context, and execution event types | Narrative current revision, accepted claim, entity resolution, and scene selection cannot declare anchor intent without root code changes | Allow domain-owned anchor intent registration using generic graph contracts |
| Graph fact shape | Traversal facts retain event type, object references, relations, and ledger sequence | They do not retain the producer payload or source occurrence time needed to reconstruct exact evidence spans and rich narrative state | Provide fact-to-event hydration through the authoritative event service or preserve an explicit payload reference |
| Relation shape | Event relations provide a type and two endpoints | Qualified participant roles, temporal bounds, confidence, narrative scope, and viewpoint cannot live on an edge | Represent qualified relations as domain-owned objects or add a generic relation-fact contract without central narrative semantics |
| Graph queries | In-process query contracts support object lookup, neighbors, bounded walks, current anchors, history, and provenance | The served `/v1` surface does not expose graph or belief reads | Publish bounded graph, anchor, provenance, and event-hydration routes through existing contracts |
| Request sizing | The loopback server accepts request bodies up to one MiB | A full story plus extraction payload may exceed one request, while one semantic revision may span several records | Define bounded batches and a resumable revision publication protocol that preserves semantic-unit identity |
| Belief semantics | Belief families provide durable, evidence-grounded inferred state | Extraction confidence, editorial acceptance, and canon truth are different concepts and cannot be mapped to belief confidence by convenience | Keep extractor confidence in producer records, editorial state in narrative authority, and Meld belief only for declared epistemic questions |
| External consumption | The loopback service exposes event and diagnostic routes | There is no complete external narrative append-and-query client contract | Validate one client outside the Meld process against only served APIs |
| PDS activation | PDS package, profile, assignment, activation, and facet models remain proposals | A narrative graph cannot yet activate a lore steward through accepted PDS contracts | Defer PDS activation claims until the proposal promotion gates are met |

## Constraint Set

### Evidence must remain recoverable

Every extracted narrative claim must resolve to exact source evidence. A graph neighbor result without a path back to the producer payload and passage coordinates is insufficient.

### Mention identity must remain provisional

A mention, a candidate identity, and a canonical entity are different products. Similarity may propose a merge but cannot silently establish one.

### Narrative scope must survive projection

Claims may be scoped to narrator, viewpoint character, dream, hypothetical branch, disputed account, retrospective account, or a bounded interval. Projection must not turn a scoped claim into unqualified world truth.

### Revision must not erase history

Editorial correction, retcon, merge, split, and supersession select current state while preserving prior claims, evidence, and decision lineage.

### Open vocabulary must remain domain-owned

New narrative event and relation types must not require expression-name branches in the Meld root, event ledger, graph store, or PDS core.

### Rich relationships need explicit identity

When a relation needs role, time, confidence, order, manner, or source support, it should become a domain-owned relationship or participation object. The generic binary edge remains an indexable connection, not the full semantic product.

### Database truth and stewardship judgment must remain separate

The narrative database may store contradictory claims. A steward may judge whether evidence supports a merge, refresh, or clarification action. The judgment must cite the stored products and must not replace them.

### Rendering remains an external capability

Meld may provide a scene and character-state projection to the visual product. It does not select checkpoints, manage image-generation memory, or judge visual identity unless a later explicit capability adapter supplies those results as evidence.

### Local service boundaries remain authoritative

An external product uses the loopback service and public contracts. Direct sled access, duplicated local indexes over private store formats, and imports of internal reducer modules are rejected integration paths.

## Acceptance Criteria For Meld As Narrative Database

These criteria define the minimum credible database milestone. They do not by themselves prove PDS.

### NDB-A01 Generic narrative event materialization

Given a durable event from a registered narrative source domain, replay materializes exactly one graph fact with all referenced objects and relations. No narrative-domain branch is added to the root route table or generic graph store.

### NDB-A02 Served graph reads

An external client can query an object, current anchor, anchor history, neighbors, and a bounded walk through the served API. The client never opens Meld storage directly.

### NDB-A03 Event hydration

Every returned traversal fact can be hydrated to the authoritative event record, including producer payload, occurrence time, ledger identity, and sequence.

### NDB-A04 Exact evidence recovery

Starting from an accepted character, action, relationship, or scene claim, the client can recover the exact source document revision and passage coordinates that support it.

### NDB-A05 Qualified participation

One scene event can represent multiple participants with distinct roles, temporal order, and source support without encoding role semantics into a central Meld enum.

### NDB-A06 Scoped and contradictory claims

Two incompatible claims can coexist with separate evidence, viewpoint, narrative scope, polarity, and temporal bounds. Selecting a current editorial interpretation does not delete or rewrite either claim.

### NDB-A07 Identity merge and split lineage

The client can propose, approve, and later reverse an entity merge while preserving mention assignments, source claims, former current selections, and the reason for each decision.

### NDB-A08 Current revision anchors

The narrative domain can select current document, extraction, entity resolution, accepted claim set, and scene projection revisions through generic anchor contracts and domain-owned intent mapping.

### NDB-A09 Incremental publication

A story larger than the served request limit can be published in bounded batches. Interrupted publication resumes idempotently, and readers can distinguish an incomplete revision from an accepted complete revision.

### NDB-A10 Replay equivalence

Replaying the same ledger reconstructs equivalent narrative graph facts, current anchors, and lineage without duplicate facts or lost prior revisions.

### NDB-A11 Render hydration

Given a selected scene and character identity, the client can retrieve the current accepted physical-description claims, wardrobe state, location, participating actions, and exact evidence needed to build a render-ready scene specification.

Meld returns narrative state and provenance. The external visual product converts that state into model prompts and image operations.

### NDB-A12 External consumer proof

A client owned outside the Meld repository completes append, replay, query, hydration, correction, and render-hydration scenarios using only versioned served contracts.

## Acceptance Criteria For Narrative Stewardship

These criteria begin only after the narrative database milestone works.

### NPS-A01 Persistent objective

An activated lore steward maintains explicit conditions for identity stability, claim provenance, conflict visibility, and derived-scene freshness across process restarts and new story revisions.

### NPS-A02 Selective observation

When a new mention is ambiguous, the steward can retrieve bounded neighboring passages, prior aliases, temporal evidence, and relation neighborhoods instead of reprocessing the entire corpus.

### NPS-A03 Bounded intervention

The steward can draft a merge, split, supersession, clarification request, or derived-scene refresh. Effective authority determines whether each action may execute or requires approval.

### NPS-A04 Verified refresh

When an accepted character or scene claim changes, only affected derived scene specifications become stale. A successful regeneration is followed by evidence that the new specification covers the active claim set.

### NPS-A05 Belief separation

Identity support and definition freshness may be Meld belief families. Extractor scores and editorial acceptance remain their owning domain's records and enter belief only through declared evidence mappings.

### NPS-A06 Generic PDS lowering

The narrative package lowers through the same package, profile, assignment, activation, facet, Agent, goal, execution, and verification contracts used by a dissimilar steward. It adds no narrative-specific field or lifecycle to PDS core.

### NPS-A07 Simpler-baseline comparison

Evaluation shows a material improvement over extraction plus graph storage in at least one of these measures:

- incorrect merge rate;
- duplicate entity rate;
- unresolved contradiction visibility;
- exact evidence recovery;
- correction and supersession correctness;
- affected-only refresh cost;
- successful reconstruction of why a claim or identity became current.

If no material improvement appears, the narrative database remains useful and PDS is not justified for this application.

## Acceptance Scenario

One end-to-end scenario should exercise the requirements together.

```text
1. Publish a story revision containing Kelsey and an exact physical description.
2. Extract two mentions, one character candidate, one appearance claim, one scene, and one action.
3. Query Kelsey and recover both mentions and exact source passages.
4. Publish a second chapter containing an ambiguous same-name character.
5. Preserve a possible identity match without merging automatically.
6. Approve the correct identity decision and retain its evidence and lineage.
7. Request the current scene state for Kelsey lounging on a couch in eveningwear.
8. Produce a render-ready narrative specification outside Meld.
9. Correct the dress color in the accepted narrative state.
10. Mark only affected scene projections stale and refresh them.
11. Replay from the ledger and recover the same current state plus the complete prior history.
```

The scenario fails if a client must read a private Meld store, if exact evidence cannot be recovered, if ambiguity is silently collapsed, or if a correction destroys the earlier record.

## Assessment By Domain

### Regenerated domain snapshot

The current domains relevant to this concern are events, world-model graph, belief, Agent, goals, planning, task execution, capability, PDS control plane, root service adapters, and the external narrative and visual products.

Storage primitives, logging, telemetry, workspace traversal, branch state, and legacy heads remain available infrastructure but do not acquire narrative meaning.

### Pass one domain sweep

| Domain | Level | Reason |
|---|---|---|
| External narrative product | own | Owns source text, spans, extraction products, editorial state, and render-ready narrative specifications |
| Events | publish and consume substrate | Accepts and replays intact domain products without interpreting narrative meaning |
| World-model graph | consume facet | Materializes registered narrative events, anchors, traversal indexes, and provenance links |
| Belief | optional consume facet | Needed only for declared stewardship questions such as identity support or definition freshness |
| Agent | optional consume facet | Needed only when narrative maintenance becomes normative stewardship |
| Goals and execution | optional publish and consume | Needed only for observation, reconciliation, refresh, and verification actions |
| Capability | optional adapter | Binds extractors, source readers, render-spec builders, and later visual evaluation without owning their semantics |
| PDS control plane | optional own | Owns package and activation lineage only after the stewardship milestone begins |
| Root and served API | adapter | Binds public contracts and exposes graph and hydration reads without semantic branching |
| External visual product | adapter peer | Consumes narrative projections and owns character appearance and rendering |
| Workspace and branches | none | This use case is not inherently repository-backed |
| Context and prompt context | optional adapter | May hydrate bounded model input but must not become narrative authority |
| Store, logging, telemetry, heads, Merkle traversal | none | Infrastructure or unrelated domains must not learn narrative semantics |

### Frozen affected set

The required database set is:

```text
external narrative product
events
world-model graph
root served API
```

The stewardship extension set is:

```text
belief
Agent
goals and execution
capability
PDS control plane
```

The external visual product is a peer consumer. It is not part of Meld's narrative authority.

### Pass two decomposition

| Affected domain | Contract needed | Owner | Verification |
|---|---|---|---|
| External narrative product | versioned narrative event and query adapter | external product | contract tests against a running loopback service |
| Events | intact append, idempotency, replay, event hydration | events | append and replay conformance plus fact-to-event lookup |
| World-model graph | domain projection registration, facts, current anchors, lineage, bounded reads | graph domain with narrative facet | replay and traversal tests using narrative events |
| Root served API | versioned graph, anchor, provenance, and hydration routes | root adapter | route parity with in-process public contracts |
| Belief | narrative evidence mappings and family revisions | belief domain | contradictory and stale-evidence scenarios |
| Agent | narrative concern bindings and bounded decisions | Agent domain | tolerate, observe, draft, approve, and restore scenarios |
| Goals and execution | reconciliation and refresh actions | execution domains | durable action and verification lineage |
| Capability | external extractor and render-spec bindings | capability owner plus root adapter | availability and authority remain independently enforced |
| PDS control plane | package, profile, assignment, activation, and projection | PDS candidate owner | generic lowering shared with a dissimilar steward |

## Ownership And Boundary Synthesis

The smallest viable integration is an external adapter with two conceptual operations:

```text
publish narrative revision
query narrative graph and hydrate evidence
```

Meld should not gain a central narrative schema merely to support those operations. The narrative domain owns schema evolution and validation. Meld graph contracts own generic projection and traversal. Root owns served transport. PDS later links the narrative facet to existing cognitive domains.

This boundary lets the narrative product use Meld incrementally. It can adopt event and graph persistence before PDS packages, standing objectives, or Agents are mature.

## Explicit Non-Integration

This case does not require:

- image storage in Meld;
- model checkpoint or adapter management;
- latent or embedding ownership in the graph;
- 3D scene or character models;
- ComfyUI workflow storage;
- a universal narrative ontology in PDS core;
- direct access to Meld persistence internals;
- automatic canon authority for model-generated extractions;
- belief creation for every extracted field;
- a narrative-specific planner or task runtime.

## Unresolved Questions

1. Which domain owns the registration contract that admits a new source domain into graph materialization?
2. Should event hydration be a graph query result, an event-authority lookup, or a composed root view?
3. What is the smallest publication protocol that preserves one narrative revision across bounded requests without inventing a distributed transaction?
4. Which current selections deserve graph anchors, and which remain narrative-domain read models?
5. Should qualified relationship objects use one general narrative participation shape or several domain-specific products?
6. Which editorial decisions are source authority, and which identity or freshness judgments are Meld beliefs?
7. What query latency and corpus size are sufficient for interactive scene hydration?
8. Which dissimilar PDS package should share the generality experiment with this case?

## Evidence Basis

- [Event contracts](../../../crates/meld-events/src/events/contracts.rs)
- [Event records](../../../crates/meld-events/src/events.rs)
- [Graph contracts](../../../crates/meld-world-model/src/world_state/graph/contracts.rs)
- [Graph reducer](../../../crates/meld-world-model/src/world_state/graph/reducer.rs)
- [Graph source intent](../../../crates/meld-world-model/src/world_state/graph/source_intent.rs)
- [Served routes](../../../src/serve/routes.rs)
- [Served listener](../../../src/serve/listener.rs)
- [Semantic Unit Preservation Policy](../../../governance/semantic_unit_preservation_policy.md)
- [Lore Transcriber And Canon Steward](lore_transcriber.md)
- [Roleplay Character Continuity Steward](roleplay_character.md)

## Decision Use

This case should be used in three ways:

1. as an acceptance contract for any proposal that calls Meld a freeform narrative database;
2. as a falsification case for generic graph projection and served query design;
3. as a later non-software PDS generality test after the database baseline works.

It should not displace documentation freshness as the first PDS implementation slice. It is a close candidate because it exposes high-value, concrete requirements that the software-oriented examples do not.
