# Activation-Bounded Lateral Lowering

Date: 2026-08-15
Status: non-authoritative discovery record
Scope: dynamic expansion from an active subject into contextually related entities, artifacts, and candidate corpora

## Discovery Posture

This document preserves an exploratory idea. It does not define a Meld requirement, accepted domain, runtime contract, PDS theory kind, or implementation plan.

The visual tag-cloud case motivated the idea but does not depend on it. The bounded visual feature is recorded separately in [Visual Concept Expansion For Render Hydration](../persistent_domain_stewardship/examples/freeform_narrative_database/visual_concept_expansion.md).

## Active Idea

Meld uses lowering in several places to translate a higher-level declaration or composition into a more concrete executable or graph-readable form. The idea explored here extends that intuition in a different direction.

A selected concept could dynamically elaborate into tangential but contextually useful entities. Each elaborated entity could become a new point of expansion. The resulting structure could support lateral discovery rather than only deterministic decomposition.

```text
celestial
→ moonlit grand ballroom
→ celestial ornamentation
→ lunar color palette
→ gala lighting references
→ adjacent visual concepts
```

This process can fractalize because every result may have its own relations and latent neighborhood. The proposed constraint is contextual activation. Only a bounded active neighborhood receives continued elaboration, refinement, and resource investment.

The term lowering is provisional. Existing Meld lowering generally translates declared meaning into a more concrete representation under deterministic contracts. This idea also discovers or proposes related material. It may ultimately be better described as lateral expansion, semantic elaboration, or activation-guided projection while retaining a lowering step only for the final admitted representation.

The parallel pressure test strengthens that boundary. Candidate discovery should not itself be called lowering. The broad interaction is better described as lateral elaboration, contextual selection, then deterministic lowering of one closed selection.

The current title remains a discovery-history label rather than a precise architectural name. Contextual frontier expansion or bounded associative projection more accurately names the surviving kernel because hard limits bound the work while activation only influences ordering.

## Context Activation

Context activation means the degree to which a concept is relevant to the current subject, scene, inquiry, or interaction segment. It is distinct from PDS activation, which binds an assignment to physical runtime realizations and authority fences.

An active concept could receive a positive numerical modifier as it is selected, reused, supported by nearby evidence, or judged useful. Unrelated or repeatedly unhelpful concepts could decay or receive negative pressure. Decay would lower current attention without deleting the concept or rewriting its historical value.

Activation is a prioritization signal rather than a termination mechanism. Hard depth, breadth, result-count, provider-call, and total-work limits remain necessary even when every candidate has a score. The safest initial reading keeps activation query-local. A durable learned prior would be a separately admitted and versioned product with declared outcome meaning.

An illustrative score could combine:

```text
context score
= direct user focus
+ semantic affinity
+ explicit graph relation support
+ evidence support
+ successful reuse
+ novelty within the active inquiry
- contextual divergence
- stale source pressure
- expansion cost
- repeated low-value retrieval
```

The score is not proposed as belief confidence, canon authority, truth probability, or PDS activation state. It would govern attention and expansion priority for one bounded context.

## Fractal Boundary

Activation alone may not be enough to bound recursive expansion. A useful context would likely combine several finite constraints:

- active subject and perspective scope
- maximum expansion depth and breadth
- time, token, storage, and provider-cost budgets
- minimum activation threshold
- cycle and duplicate suppression
- required provenance or source class
- novelty and saturation checks
- explicit pause, archive, or context-shift boundaries

The important behavior is selective refinement. The system invests in the concepts that remain live for the current context while preserving other candidates for later retrieval.

## Research Assistant Example

Consider a research assistant PDS assigned to a specific whitepaper. The paper has an admitted subject identity and a tag-like representation of its topics, methods, claims, citations, and adjacent concepts.

```text
selected whitepaper
→ active research representation
→ related methods and terminology
→ cited and citing work
→ competing results
→ neighboring research programs
→ candidate corpus for the active inquiry
```

If the user focuses on one subset, such as a method or disputed claim, that subset becomes more active. The assistant can deepen the corresponding neighborhood and materialize a bounded corpus for reading, comparison, or synthesis. Material outside the active subset remains discoverable but receives less expansion effort.

The corpus is a produced contextual artifact, not a declaration that every retrieved paper is authoritative or relevant. Exact sources, retrieval method, query context, scores, exclusions, and user selections would remain recoverable.

## Tags As Addresses Into Latent Projections

In this model, a tag is not merely a string label. It is a stable symbolic address that may participate in several changing semantic projections.

A vector representation is a plausible candidate mechanism because it can propose nearby concepts even when no explicit graph relation exists. A legitimate vector database could remain an external capability or domain-owned index. Meld need not own the vector store to consume attributable candidate results.

The useful separation is:

```text
durable concept identity and provenance
!= optional vector representation
!= similarity result
!= admitted relation or accepted meaning
```

Vector proximity could improve recall and lateral association. It should not establish narrative truth, research authority, causal relation, identity equivalence, or applicability by itself. An embedding model change also creates a new scoring context rather than silently reinterpreting historical results.

One symbolic address may have several model-specific vector projections. Corpus, chunking, language, embedding model, distance metric, access scope, and index revision all affect the resulting neighborhood. The vector is therefore neither concept identity nor durable semantic state.

The current world-model research already treats embedding models as optional scoring tools rather than truth storage. That boundary fits this idea well.

## Relationship To Regime

The Regime design is relevant because it asks whether new evidence still belongs to the same structural world. It owns changepoint inference, recurring context identity, continuation versus break comparison, archived priors, and mixture state when several regimes remain plausible.

A chapter transition away from the Moonbeam Gala illustrates two different cases.

When the new chapter boundary is explicit and authoritative, the narrative domain can close the old scene context and open the new one directly. Lateral expansion can reset, archive, or reweight its active neighborhood without waiting for Regime inference.

When the context shift is gradual, ambiguous, or inferred from changing evidence, Regime could supply a changepoint or mixture summary. That summary could warn consumers that the Gala-weighted neighborhood may no longer apply. Regime should not own concept activation scores or lateral candidate generation merely because it detects the structural change.

The generalized feature therefore does not necessarily require Regime for explicit context transitions. Regime becomes valuable when continuity versus break is itself uncertain, when old contexts recur, or when several contextual interpretations should remain weighted at once.

The Regime design is active architectural intent, but the corresponding runtime domain is not yet implemented in the current codebase. Any dependency on it would therefore place this exploration beyond the scoped visual-concept feature.

## Closest Meld Ground

Implemented primitives already demonstrate several narrower forms of lowering:

- execution compositions lower into deterministic task-network mutation proposals
- authored task packages lower into domain expansion templates
- stewardship configuration lowers into canonical named declarations
- graph source facts lower into replayable traversal and anchor state

Active architectural intent supplies adjacent mechanics:

- the graph provides identity, provenance, bounded walks, and current anchors
- belief keeps evidence settlement distinct from graph proximity
- Regime defines structural context shifts, recurrence, and mixture state
- operational domain theory supplies declared meaning without owning current state

The smallest inferred extension is not a universal vector database. It is a contract for producing a bounded, attributable expansion frontier from an active concept and context, followed by domain-owned validation and optional materialization.

Everything beyond that remains speculative, including learned activation updates, cross-domain latent spaces, autonomous expansion policy, generalized context memory, and Regime-conditioned attention.

## Parallel Pressure Test

Two independent discovery reviews pressed the idea in supportive and antagonist directions. They converged on a narrower kernel while disagreeing about whether it deserves a generalized Meld construct.

### Strongest Supportive Reading

The novel candidate is not graph traversal, vector retrieval, ranking, or lowering by itself. It is a reusable substrate for assembling a recursively managed, context-local frontier from several attributable candidate sources.

```text
stable subject and context
→ attributable candidate frontier
→ context-local prioritization
→ recursive selective expansion
→ domain-owned contextual artifact
→ deterministic lowering
```

Meld already provides much of the containment machinery. Domain object references and event relations provide stable addressing. Graph provides explicit relations, provenance, and bounded walks. Context provides immutable artifacts and deterministic truncation. Strategy provides a precedent for explicit search bounds, honest frontier completion, and independent validation. Capabilities provide exact external action contracts. Belief demonstrates that scoring input can remain separate from truth settlement.

The smallest supportive extension is a bounded expansion-frontier request and result contract plus a pure frontier reducer. Candidate providers may include graph traversal, lexical retrieval, vector search, domain indexes, or explicit user selection without surrendering semantic ownership to the reducer.

### Strongest Antagonist Reading

The broad abstraction fails if it claims to be a new kind of lowering, treats activation as the growth bound, or promotes retrieval scores into durable cross-domain relevance.

Existing lowering preserves already-declared meaning. Lateral discovery proposes uncertain meaning. A universal admission function would inflate authority because a candidate can become a domain entity, relation, belief, or accepted corpus member only through a named owner contract.

Activation only orders attention. Dense one-hop neighborhoods can still explode under a depth limit. Selection and reuse mostly measure exposure, so naive positive feedback amplifies popular hubs while negative pressure can bury rare but important material. A mutable vector index also cannot support replay merely by recording the query and score.

Several benefits may remain fully explained by simpler product mechanisms. Visual lock can use a curated expansion graph and deterministic prompt compilation. Research discovery can use citation walks, hybrid retrieval, reranking, top-k limits, and a saved corpus manifest. A general Meld construct is justified only if dissimilar products share a meaningful contract beyond ordinary retrieval infrastructure.

### Surviving Kernel

Both reviews support one narrow shape:

```text
exact context identity
+ seed concept references
+ source and index snapshot references
+ retrieval and ranking revision
+ hard structural and cost bounds
→ ordered candidate references
+ source provenance
+ derivation routes
+ score components
+ honest completion posture
+ exact result-manifest digest
```

The frontier contains proposals rather than facts. It preserves three states:

```text
proposed association
!= selected member of this contextual artifact
!= admitted domain relation or belief
```

Hard eligibility and soft activation remain separate. Authority, source class, access scope, provenance, and freshness may reject or quarantine a candidate before contextual ranking. Explicit graph relations and semantic candidates retain separate origins even when a consumer interleaves them.

Working activation may remain ephemeral. Materialized frontier snapshots are durable artifacts that preserve the exact candidates consumed by downstream work. Repeating a live vector query is new retrieval, not replay. Replay reuses the admitted manifest and its exact revisions.

Only after a domain selects and materializes a contextual artifact does deterministic lowering begin. This boundary is currently the strongest surviving model.

### Generality Falsifier

The generalized idea weakens or collapses into product-specific retrieval if visual expansion and research discovery cannot share all of these mechanics without sharing their semantic policies:

- exact context and seed identity
- several attributable candidate channels
- hard frontier and cost bounds
- cycle and duplicate suppression
- context-local ranking with inspectable score components
- honest bounded completion
- immutable result-manifest identity
- domain-owned selection and admission

If the common layer must understand atmosphere, prompt compatibility, citation quality, research disagreement, or narrative authority, the abstraction has crossed its legitimate boundary.

## Candidate Interaction Shape

One possible interaction remains deliberately abstract:

```text
active subject and context
→ retrieve explicit graph neighborhood
→ request optional semantic-neighbor candidates
→ apply hard source, scope, and authority eligibility
→ rank remaining proposals for this context
→ expand only the selected frontier
→ preserve the exact frontier manifest
→ let the owning domain select and admit a contextual artifact
→ lower the closed artifact through its domain contract
→ archive or shift when the context changes
```

This shape keeps vector retrieval, graph truth, contextual scoring, domain admission, and produced artifacts separate even if one product presents them as a fluid lateral-thinking experience.

## Unresolved Threads

- whether contextual activation belongs to world-model projection, Agent attention, Strategy search, context assembly, or a new domain-owned facet
- whether activation is an ephemeral working value, a durable learned prior, or both under separate identities
- what outcome can truthfully reinforce a tangential concept rather than merely record that it was selected
- how negative pressure avoids hiding rare but important relations
- how several simultaneous contexts compose without collapsing into one global score
- how contextual corpora remain reproducible across embedding and retrieval revisions
- whether explicit graph relations and vector neighbors share one frontier or remain separately ranked sources
- whether Regime summaries reset weights, select archived priors, or only warn downstream consumers
- where final semantic elaboration ends and deterministic lowering begins
- whether two dissimilar products can share the frontier contract without centralizing their selection meaning
- which exact result-affecting retrieval inputs must be captured before a frontier manifest is reproducible

## Explicit Non Commitments

This discovery does not commit Meld to embeddings, a vector database, reinforcement learning, one global latent space, spreading activation, autonomous research acquisition, a generic tag ontology, or a new central reasoning domain.

It does not reinterpret PDS activation as attention. It does not assign contextual scoring to Regime. It does not make similarity authoritative. It does not require the scoped visual concept feature to wait for the generalized construct.

## Grounding Sources

- [Visual Concept Expansion For Render Hydration](../persistent_domain_stewardship/examples/freeform_narrative_database/visual_concept_expansion.md)
- [Freeform Narrative Database Stewardship](../persistent_domain_stewardship/examples/freeform_narrative_database/README.md)
- [Graph](../cognitive_architecture/world_model/graph/README.md)
- [Regime Layer](../cognitive_architecture/world_model/regime/README.md)
- [Regime Requirements](../cognitive_architecture/world_model/regime/requirements.md)
- [Persistent Domain Stewardship](../cognitive_architecture/persistent_domain_stewardship.md)
- [Temporal Knowledge Graph Research Summary](../cognitive_architecture/research/world_model_architecture/summary/TEMPORAL_KG_GAP_SUMMARY.md)
- [Multi-Domain Event Spine](../cognitive_architecture/events/multi_domain_spine.md)
- [Strategy Search](../cognitive_architecture/world_model/strategy/search.md)
- [Strategy Contracts](../../crates/meld-world-model/src/strategy/contracts.rs)
- [Graph Walk Contracts](../../crates/meld-world-model/src/world_state/graph/contracts.rs)
- [Graph Walk Store](../../crates/meld-world-model/src/world_state/graph/store.rs)
- [Context Composition](../../src/context/query/composition.rs)
- [Prompt Context Contracts](../../src/prompt_context/contracts.rs)
- [Capability Contracts](../../crates/meld-execution/src/capability/contracts.rs)
- [Execution Composition Lowering](../../crates/meld-execution/src/planning/lowering.rs)
- [Task Package Lowering](../../crates/meld-execution/src/task/package/lower.rs)
