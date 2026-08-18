# Game Faction Strategy Stewardship

Date: 2026-08-15
Status: discovery example
Scope: persistent strategic stewardship of one faction under partial and perspective-dependent information

## Working Use Case

A faction strategy steward maintains survival, protected territory, critical resources, relationships, commitments, and long-horizon objectives while the simulated world changes independently.

The steward acts at strategic timescales. It chooses whether to observe, negotiate, allocate, plan, tolerate uncertainty, or initiate an operation. Deterministic simulation systems continue to own per-frame control and world-state transition.

```text
objective simulation state
!= faction observation
!= faction belief
!= player or rival knowledge
!= action authority
```

The example is especially useful because complete ground truth may exist for evaluation while remaining unavailable to the active faction perspective.

## Domain Boundary

The bounded subject is one faction within one declared scenario, campaign, branch, or simulation world. Scope may include controlled actors, territory, resources, threats, diplomatic relationships, promises, treaties, strategic assets, operations, and objectives.

Per-frame aiming, steering, navigation, collision, animation, combat resolution, and authoritative world simulation remain outside the steward. The faction steward issues bounded strategic intents through capabilities published by those owners.

The principal may be a player, scenario author, game policy, or simulation experiment. The principal defines faction scope, protected conditions, risk posture, treaty constraints, resource authority, and escalation rules.

## Standing Condition

Candidate standing conditions include:

- preserve faction viability and protected critical resources
- keep strategic awareness above declared uncertainty limits where observation is affordable
- preserve treaty, promise, and relationship consequences across episodes
- avoid irreversible escalation without sufficient evidence and authority
- keep active plans viable as resources, threats, and world conditions change
- distinguish tolerated uncertainty from silently forgotten risk

A temporary goal such as secure one food route may close while the standing responsibility for food security remains active.

## Observations And Evidence

Candidate observations include sightings, scout reports, testimony, resource updates, diplomatic messages, treaty changes, combat outcomes, operation outcomes, territorial changes, and public world events.

Every admitted observation preserves source identity, faction perspective, branch, simulation time, observation time, provenance, reliability class, disclosure scope, and exact world event reference where available.

Observation actions include scouting, inspecting controlled assets, querying an ally under treaty permissions, interrogating a source, requesting reconnaissance, waiting for a lower-risk information opportunity, or running a bounded counterfactual simulation.

Objective ground truth used by an evaluation harness does not automatically enter the faction's knowledge. Test visibility and steward visibility remain separate.

## Actions

Candidate interventions include:

- allocate controlled resources
- reposition strategic assets through an external operation contract
- negotiate or propose a treaty
- form, honor, or break a commitment under policy
- initiate a bounded strategic operation
- suspend or repair a plan after material world change
- tolerate uncertainty while retaining an explicit risk state
- escalate a decision outside faction authority to the principal

The steward issues strategic intent. Simulation owners validate legality, resolve mechanics, and publish world outcomes.

## Outcomes

Issuing an operation or receiving simulation success is not by itself stewardship success. Verification may use later evidence about:

- faction survival and protected resource floors
- territorial or supply effects
- casualties and opportunity cost
- treaty and relationship consequences
- whether a threat belief was supported or falsified
- whether a repaired plan remained viable
- whether the same faction perspective reconstructs the decision rationale under replay

The evaluation harness may compare steward beliefs with hidden world truth after an episode. That comparison is outcome evidence and must not leak back into the historical faction perspective.

## Authority And Disclosure

The assignment grants control only over declared faction resources and strategic action classes. Observation may also be constrained by faction ownership, treaties, reconnaissance capability, and scenario disclosure rules.

The steward may draft negotiation or operation intents without authority to apply world transitions. The simulation owner remains authoritative for world legality and resulting state.

Several faction stewards may share one world process while retaining distinct principals, perspectives, secrets, beliefs, capability catalogs, and grants. One faction's source report cannot enter another faction's evidence without an admitted disclosure path.

## Why PDS

The mandatory baseline is a behavior tree, utility system, goal-oriented action planner, blackboard, or scripted simulation policy.

PDS is justified only if persistent divergent beliefs, provenance-sensitive testimony, active scouting, long-term commitment memory, strategic plan repair, verified outcomes, or counterfactual explanation materially outperform the simpler controller.

The fit is strongest as a research case with cheap replay and available hidden ground truth. A deterministic tactical policy with fully observed state remains a poor PDS fit.

## Physical Runtime Variants

The same package meaning could run as:

- a trusted in-process strategic controller inside the simulation host
- an owned subprocess receiving bounded faction projections and returning intents
- a shared sidecar serving several isolated faction assignments
- a remote turn-based strategy service
- a persistent external campaign controller consuming authenticated world advances

Placement never grants access to full simulation truth. Every projection and result remains fenced by faction, perspective, branch, source revision, activation generation, and authority.

## Explicit Non Commitments

This example does not commit Meld to a game engine, universal faction ontology, real-time control loop, one planning algorithm, automatic diplomacy authority, or one simulation transport.

It does not assume that counterfactual simulation results are facts about the live world. It does not make hidden evaluator truth visible to the steward. It does not treat all strategic action as safe merely because the environment is simulated.

## Discovery Questions

- Which domain owns faction policy, treaty meaning, and strategic action legality
- How a world owner supplies a bounded perspective projection without leaking hidden state
- Whether counterfactual runs are observation evidence, planning estimates, or both under separate products
- How simultaneous faction actions acquire effect ownership and deterministic resolution order
- Which delayed world outcomes are sufficient to verify a maintained strategic condition
- How shared simulation infrastructure proves assignment-safe state and disclosure isolation

## Source Material

- [Expression Catalog](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [Use Case Decomposition](../../use_case_decomposition.md)
- [Roleplay Continuity Example](../roleplay_character_continuity/README.md)
- [PDS Router Detailed Design](../../../plan/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [PDS Architectural Invariants](../../architectural_invariants.md)
- [Proposal Status](../../proposal_status.md)
