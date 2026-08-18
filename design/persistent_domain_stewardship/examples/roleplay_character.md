# Roleplay Character Continuity Steward

Date: 2026-08-08  
Status: illustrative  
Scope: a Persistent Domain Stewardship expression for maintaining character and narrative continuity across interactive roleplay

> This long-form source case predates the canonical cognition split. Use the normalized [use case](roleplay_character_continuity/README.md), [router specification](roleplay_character_continuity/router_spec.md), and [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) when ownership language conflicts.

## Purpose

This example tests PDS against an interactive conversational domain rather than an operational software domain.

The candidate is not "an LLM that plays a character." A persona prompt, long context window, or retrieval-backed chatbot is the mandatory baseline.

The stewardship-shaped candidate is a **character continuity steward**: a persistent controller that maintains coherent character knowledge, relationships, commitments, narrative state, and canon boundaries across an indefinitely evolving interaction history.

The important test is whether Meld can preserve the separation between:

```text
world or canon truth
!= character belief and knowledge
!= narrator or player belief
!= what a response is allowed to reveal
```

If those distinctions are unnecessary, PDS is not justified.

## Qualification

| Property | Roleplay continuity |
|---|---|
| Bounded domain | One character, campaign, world, or explicitly scoped narrative |
| Persistent mandate | Continuity remains active after every turn and session |
| Independent change | Strong in shared worlds, multi-character systems, and externally updated canon; weak in isolated single-user chat |
| Partial observability | Characters may have incomplete, stale, false, or conflicting knowledge |
| Standing objective | Preserve coherent character/world continuity while the narrative evolves |
| Observation choice | Selectively hydrate prior episodes, canon, relationships, and unresolved threads |
| Intervention choice | Respond, defer, ask, investigate, tolerate ambiguity, or surface a continuity conflict |
| Action economics | Context hydration, generation, retrieval, and correction have variable cost |
| Feedback | Later user corrections, accepted turns, world events, and replay checks evaluate continuity |
| Authority boundary | Character output does not imply authority to rewrite canon or user declarations |
| Temporal continuity | Prior facts, commitments, secrets, and relationship changes remain relevant |
| Audit value | It can matter why a fact was treated as known, unknown, superseded, or disputed |

**Verdict:** conditional PDS fit. Strong for persistent, mutable, multi-session or multi-perspective narrative systems. Weak for a single bounded conversation where prompt plus retrieval is sufficient.

## Domain Boundary

### In scope

- one or more persistent roleplay characters
- character identity and persona constraints
- character knowledge and uncertainty
- relationships and social commitments
- locations, possessions, affiliations, and status relevant to the character
- accepted narrative events
- open narrative threads and unresolved commitments
- secrets and information-access boundaries
- explicit retcons, corrections, and supersession
- bounded interaction history used as evidence

### Out of scope

- token-level generation mechanics
- model training or fine-tuning
- unrestricted rewriting of user-authored canon
- treating generated text as canon merely because it was generated
- omniscient disclosure when the active character lacks the corresponding knowledge
- universal world simulation unless separately assigned

### Principal

The principal may be:

- the player or user;
- a game master or narrator;
- a shared-world authority;
- a product-defined narrative policy.

The principal grants the scope within which the steward may create provisional narrative state, commit state transitions, or request clarification.

## Descriptive Model

Representative subject types:

```text
narrative.character
narrative.actor
narrative.location
narrative.organization
narrative.object
narrative.event
narrative.relationship
narrative.commitment
narrative.secret
narrative.fact
narrative.thread
narrative.scene
narrative.utterance
narrative.canon_revision
```

Representative relations:

```text
character located_at location
character member_of organization
character possesses object
character knows fact
character believes fact
character suspects fact
character committed_to commitment
character related_to character
fact supported_by event
fact contradicted_by fact
fact superseded_by fact
thread opened_by event
thread concerns character
utterance occurred_in scene
```

Identity must survive aliases, title changes, disguises, renamed locations, transformed objects, and explicit retcons.

## Observation Model

Promoted observations may include:

```text
narrative.user_utterance_observed
narrative.character_utterance_observed
narrative.narrator_event_observed
narrative.canon_assertion_observed
narrative.canon_correction_observed
narrative.relationship_change_observed
narrative.commitment_change_observed
narrative.scene_transition_observed
narrative.external_world_update_observed
```

An utterance is evidence. It is not automatically authoritative canon.

Each promoted observation should preserve:

- speaker or source identity;
- narrative time and observation time;
- scene or session identity;
- authority class;
- subject extraction;
- exact source reference;
- whether the content is assertion, perception, belief, deception, speculation, or narrator truth where known.

## Operational Domain Theory

The PDS expression lowers into the canonical theory kinds defined by Persistent Domain Stewardship. It does not add a narrative-specific runtime.

### Belief family: `narrative.canon_support`

Question:

```text
How strongly does current admissible evidence support that this proposition is accepted world or campaign canon at the active narrative revision?
```

Evidence may include:

- principal-authored assertions;
- narrator-authoritative events;
- accepted prior scene outcomes;
- explicit corrections and retcons;
- imported canon with declared authority.

Contradiction is preserved rather than silently collapsed.

### Belief family: `character.knowledge`

Question:

```text
How strongly does evidence support that the active character knows the selected proposition at the current narrative time?
```

The family distinguishes:

```text
world truth
character observation
character testimony
information received from another actor
inference available to the character
out-of-character user knowledge
```

Planner projection must not turn world truth into character knowledge without an admissible information path.

### Belief family: `character.relationship_state`

Question:

```text
What current relationship state is supported between the active character and another actor, with what confidence and provenance?
```

Possible dimensions include trust, obligation, hostility, affection, fear, debt, authority, and affiliation. Package semantics determine which dimensions exist.

### Belief family: `character.commitment_state`

Question:

```text
What promises, obligations, intentions, and unresolved commitments remain active for the character?
```

Evidence includes explicit promises, accepted plans, completed actions, abandonment, release, contradiction, and supersession.

### Belief family: `narrative.thread_state`

Question:

```text
Which narrative threads are open, resolved, abandoned, contradicted, or awaiting evidence?
```

This family prevents unresolved material from disappearing merely because it left the current prompt window.

### Belief family: `character.identity_consistency`

Question:

```text
How strongly does the proposed response remain compatible with established persona, knowledge, commitments, voice constraints, and current narrative state?
```

This is not a generic style score. Its evidence must be grounded in declared character constraints and accepted history.

## Stewardship Charter

Representative standing conditions:

```text
maintain separation between canon truth and character knowledge
maintain active character commitments until resolved or superseded
maintain relationship state across sessions
maintain unresolved narrative threads as addressable state
avoid assertions that contradict high-authority canon without surfacing the conflict
avoid revealing information the active character lacks authority or knowledge to reveal
```

The steward may tolerate ambiguity. It should not invent a reconciliation solely to remove uncertainty.

## Observation Actions

Candidate observation actions:

- hydrate the most relevant prior scenes;
- retrieve the provenance chain for a disputed fact;
- inspect prior relationship-changing events;
- inspect commitments involving the active actor;
- retrieve the active canon revision;
- query another declared narrative source;
- request principal clarification when ambiguity blocks a coherent response.

Observation selection is part of the PDS value proposition: the system need not place the entire narrative history in every model context.

## Intervention Actions

Candidate interventions:

- generate an in-character response under a hydrated continuity projection;
- create provisional state changes implied by an accepted turn;
- record a newly opened or resolved narrative thread;
- record a relationship or commitment transition when supported by accepted evidence;
- surface a continuity conflict;
- propose a retcon or reconciliation without applying it;
- defer a state mutation until principal confirmation.

The character-response capability and the state-transition capability are distinct. Generating text does not itself grant authority to mutate canon.

## Known Methods

### `narrative.respond_with_continuity`

Illustrative composition:

```text
resolve active character and scene
→ ground relevant belief families
→ hydrate bounded continuity context
→ identify knowledge and authority constraints
→ generate candidate response
→ evaluate continuity constraints
→ publish response artifact
→ observe principal acceptance, correction, or subsequent narrative event
```

### `narrative.resolve_continuity_conflict`

Illustrative composition:

```text
materialize conflicting claims and provenance
→ rank source authority without discarding minority evidence
→ determine whether one claim supersedes another
→ if unresolved, ask or escalate
→ publish accepted correction or preserve explicit conflict
```

## Outcome Model

Mechanical completion:

```text
response artifact produced
```

This is not sufficient evidence of successful stewardship.

Verification evidence may include:

- no contradiction against the active high-authority canon projection;
- no unsupported knowledge leak from world truth into character knowledge;
- active commitments remain represented after the turn;
- relationship transitions are traceable to accepted events;
- subsequent principal acceptance or correction;
- replay of the same narrative state produces compatible grounding decisions;
- explicit retcons supersede prior claims without destroying history.

Harmful outcomes include:

- revealing a secret the character did not know;
- forgetting an active promise that should constrain behavior;
- silently changing a relationship state;
- converting a generated embellishment into authoritative canon;
- using a later retcon as if the character knew it earlier;
- resolving contradictory canon by arbitrary model preference.

## Governance

Illustrative authority classes:

| Action | Default posture |
|---|---|
| read accepted narrative history | Observe |
| retrieve bounded prior context | Observe |
| generate character response | Draft or ExecuteReversible |
| record provisional extracted state | Draft |
| commit low-risk state implied by accepted turn | policy-dependent |
| merge or split character identity | approval for ambiguous cases |
| retcon established canon | approval required |
| reveal protected secret outside character knowledge | Prohibited |
| rewrite principal-authored source history | Prohibited |

## Simpler Baseline

The required baseline is:

```text
persona system prompt
+ bounded conversation history
+ semantic retrieval over prior messages or lore
+ deterministic conversation/session storage
```

PDS is justified only if it produces measurable improvement in one or more of:

- cross-session contradiction rate;
- character-knowledge leakage rate;
- recovery of active commitments and relationships;
- correctness under retcon and supersession;
- context-token cost for equivalent continuity;
- explainability of why a fact constrained a response;
- multi-character or shared-world consistency.

## What This Example Tests

This example stresses PDS in ways documentation and vulnerability stewardship do not:

1. **Truth is perspective-relative.** The same proposition may be world truth while remaining unknown or disbelieved by a character.
2. **The main intervention is expressive.** The steward often acts by producing a response rather than mutating an external operational system.
3. **Continuity depends on selective context hydration.** Relevant history is unbounded while active model context remains bounded.
4. **Authority is narrative.** The system must distinguish permission to speak, permission to propose state, and permission to change canon.
5. **Corrections are temporal.** Retcons and supersession must preserve what was believed or known at earlier narrative times.

If these requirements can be expressed through existing event, graph, belief, Agent, planning, execution, and outcome contracts without narrative-specific runtime branches, the case provides evidence that the PDS application model generalizes beyond operational automation.

## Read With

- [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md)
- [Use-Case Decomposition](../use_case_decomposition.md)
- [Software Quality Stewardship Example](software_quality.md)
