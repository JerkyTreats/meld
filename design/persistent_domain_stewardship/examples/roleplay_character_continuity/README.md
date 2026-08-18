# Roleplay Character Continuity

Date: 2026-08-15
Status: discovery example
Scope: perspective-bound continuity for one persistent character in an evolving narrative

## Working Use Case

A character continuity steward maintains coherent character knowledge, relationships, commitments, secrets, scene state, and unresolved narrative threads across sessions.

The maintained condition is not merely that a plausible response was generated. It is that every accepted response and state transition remains compatible with the active canon revision, the character's own admissible knowledge, the principal's disclosure policy, and durable prior commitments.

```text
world or canon truth
!= character knowledge
!= player or narrator knowledge
!= material the character may reveal
```

This separation is the center of the example. If one shared narrative summary can safely replace it, the PDS form is unnecessary.

## Domain Boundary

The bounded subject is one character within one declared campaign, world, branch, or narrative project. The scope may include scenes, actors, locations, possessions, relationships, promises, secrets, accepted events, retcons, and open threads relevant to that character.

Token generation, model training, universal world simulation, and unrestricted canon mutation remain outside the boundary. Generated text does not become canon merely because the steward produced it.

The principal may be a player, narrator, game master, shared-world authority, or product policy. The assignment binds that principal, the active character perspective, the narrative branch, and the exact package receipt.

## Standing Condition

The candidate standing conditions are:

- keep canon truth distinct from character knowledge
- preserve active commitments and relationship consequences across sessions
- keep unresolved narrative threads addressable after prompt-window turnover
- prevent disclosure of protected facts that lack an admissible information path
- preserve corrections and retcons as supersession rather than historical erasure
- keep accepted responses traceable to the continuity evidence that constrained them

Ambiguity may remain unresolved. Coherence does not grant authority to invent a reconciliation.

## Observations And Evidence

Candidate observations include user utterances, character utterances, narrator events, accepted scene outcomes, canon assertions, explicit corrections, relationship changes, commitment changes, scene transitions, and external world updates.

Every promoted observation preserves speaker or source identity, narrative time, observation time, scene or session identity, authority class, exact source reference, branch, and perspective. It also preserves whether the content is an assertion, perception, belief, deception, speculation, or narrator-authoritative fact where known.

Observation actions may hydrate a bounded set of prior scenes, retrieve the provenance chain for a disputed fact, inspect a relationship transition, resolve active commitments, read the active canon revision, or ask the principal for clarification.

An utterance is evidence. It is not automatically canon. World truth is not automatically evidence that the character knows a proposition.

## Actions

Candidate actions include:

- produce a draft in-character response from a bounded continuity projection
- propose state changes implied by an accepted turn
- record an opened or resolved thread after owner admission
- record a supported relationship or commitment transition
- surface a continuity conflict
- propose a retcon without applying it
- defer mutation pending principal confirmation

Response generation and narrative state mutation are separate capabilities with separate authority.

## Outcomes

A completed response artifact is only a mechanical result. Stewardship success requires later evidence such as:

- no contradiction with the active high-authority canon projection
- no knowledge or secret leak across the active perspective boundary
- active commitments remain represented after the turn
- relationship changes trace to accepted events
- principal acceptance, correction, or explicit retcon
- equivalent grounding decisions when the same exact narrative state is replayed

Later correction is outcome evidence, not merely an error message. It may reopen a continuity concern and supersede a prior interpretation without deleting the earlier record.

## Authority And Disclosure

Reading accepted history and retrieving bounded context are observation authority. Producing a response may be draft or reversible execution authority. Committing state implied by a turn is policy-dependent. Ambiguous identity changes and established-canon retcons require approval by default.

The steward is prohibited from rewriting principal-authored source history or revealing protected material outside the active character's knowledge and disclosure grants.

Shared narrative storage does not imply shared perspective. Each assignment retains its own principal, character perspective, branch, grants, continuity state, and disclosure policy.

## Why PDS

The mandatory baseline is a persona prompt, bounded conversation history, deterministic session storage, and semantic retrieval over prior messages or lore.

PDS is justified only when persistent, selective, authority-aware maintenance materially improves cross-session contradiction rate, knowledge leakage, commitment recovery, retcon correctness, context cost, or the ability to explain why a fact constrained a response.

The strongest fit is a shared, mutable, multi-session world with several perspectives. A bounded single-session conversation remains a poor fit.

## Physical Runtime Variants

The same package meaning could be realized through:

- a trusted in-process continuity resolver and local generation adapter
- an owned subprocess with a bounded context and structured response contract
- a shared sidecar that isolates each assignment's perspective and context namespace
- a remote model service that receives only a bounded projection and returns an untrusted candidate artifact
- a persistent external narrative service that publishes authenticated source advances without owning Meld decisions

Placement does not change package identity or grant disclosure authority. Every result still passes narrative owner admission with exact assignment, perspective, branch, source, activation-generation, and participant-incarnation lineage.

## Explicit Non Commitments

This example does not commit Meld to a narrative-specific runtime, a universal story ontology, one model provider, one context store, autonomous canon mutation, or one final package schema.

It does not decide whether response evaluation is deterministic, model-assisted, principal-driven, or composed from all three. It does not make generated prose authoritative source truth.

## Discovery Questions

- Which owner contract defines disclosure eligibility without turning world-model perspective into an authorization system
- Which accepted event establishes that a candidate response may produce narrative state transitions
- How should simultaneous roleplay assignments over one character arbitrate conflicting proposed turns
- Which continuity checks can be deterministic and which remain evidence-producing capabilities
- What exact late-result policy applies when a remote response returns after the active scene or generation changes

## Source Material

- [Original Roleplay Character Example](../roleplay_character.md)
- [Expression Catalog](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [Lore And Canon Example](../lore_and_canon/README.md)
- [PDS Router Detailed Design](../../../plan/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [PDS Architectural Invariants](../../architectural_invariants.md)
- [Proposal Status](../../proposal_status.md)
