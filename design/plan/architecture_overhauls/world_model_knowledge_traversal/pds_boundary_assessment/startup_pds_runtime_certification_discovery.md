# Startup PDS And Runtime Flywheel Nonce

Date: 2026-08-22

Status: discovery record, not part of the approval candidate

Implementation authorization: none

## Active Idea

A Startup PDS is a small system product assigned to the Meld runtime itself. It causes genesis of one Agent whose standing concern is a self-fulfilling startup Event round trip for the current runtime generation.

The maintained condition is intentionally small. For startup nonce N under activation generation G, the Startup Goal was triggered and the linked Startup Event was received back through the declared epistemic path.

This product uses ordinary PDS and World Model Reconciliation behavior. PDS supplies the Agent identity, directive, maintained condition, Curation meaning, Strategy knowledge, Capability expectation, observation routes, and activation lineage. Native owners perform the runtime work. Agent owns Goal inception when admitted evidence shows that the prophecy is not fulfilled.

## Three Different Readiness Claims

| Claim | Meaning | Owner |
| --- | --- | --- |
| operational health | processes, leases, heartbeats, bounded actor steps, and restart policy are functioning | supervisor and operational runtime owners |
| structural readiness | the exact activation generation has its required participants, owner readiness receipts, current head, and open admission epoch | root lifecycle structure plus native readiness owners |
| flywheel nonce satisfied | one generation-scoped Startup Event progressed through Task, Capability, Events, epistemic ingestion, and returned to the originating Agent | native reconciliation owners, with final Goal satisfaction owned by Agent |

No claim substitutes for another. A structurally ready runtime may still have a broken producer-consumer connection. A running process may still host a stalled program. A completed flywheel nonce proves one exact path, not the absence of every later fault.

## Two-Barrier Startup

The Startup PDS cannot prove the structural readiness required for its own Agent to run. Making its certification a prerequisite for its own generation would create a circular startup dependency.

The coherent model therefore has two barriers.

```text
native preparation and structural readiness
-> Startup PDS generation becomes current
-> Startup Agent runs one Event nonce round trip
-> generation-scoped nonce satisfaction becomes visible
-> dependent PDS product control may request publication of later product generations
```

The first barrier establishes enough runtime to perform cognition. The second barrier proves that the admitted runtime can actually carry one bounded cognitive obligation through the flywheel.

Compilation and inert preparation of other products need not wait. Only their live activation request or current-generation publication need be withheld if the principal selects fail-closed startup certification.

## Self-Fulfilling Event Prophecy

The Startup PDS declares two linked nonce-scoped propositions for the current generation:

```text
Startup Goal N was triggered for generation G
Startup Event N was received by Startup Agent A
```

Curation authors the expected Startup Event entity and its required relationship to Agent A, Goal N, and generation G. The first bounded assessment finds no received Event. Agent accepts that mismatch and incepts Goal N with the desired outcome that the Startup Event be fired and received.

Strategy constructs one complete Task. Agent authorizes it. Execution admits and lowers it. A deliberately small Startup Event Capability appends the exact custom Event with the nonce, Startup Agent, Goal, activation generation, and required lineage. The Event then returns through Graph or configured Belief and Agent accepts the declared milestone.

The first proposition records that ordinary Agent mismatch handling successfully incepted the Goal. The second proposition closes only after the custom Event returns through epistemics. Together they prove the intended flywheel without introducing a separate startup command path or a semantic diagnostics workflow.

## Proposed Round Trip

The canary begins with owner-issued structural and operational observations for one exact generation. Curation evaluates whether the expected custom Startup Event has been received. If it has not, Agent establishes a bounded Goal. Strategy constructs a Plan with the Task needed to fire the Event. Agent authorizes the complete Task.

```text
Startup PDS installation and assignment
-> Startup Agent genesis
-> current bootstrap generation and open admission epoch
-> lifecycle and startup expectation observations
-> Event publication
-> Graph and Traversal visibility
-> configured Belief revision
-> Curation authors expected Startup Event N and assesses it as not received
-> Agent mismatch judgment and Goal
-> Strategy Plan
-> Agent Task authorization
-> Execution Goal Set admission
-> unified Task Network lowering and dispatch
-> Startup Event Capability appends custom Event N
-> Event authority append and replay
-> Graph or configured Belief visibility
-> epistemic propositions Goal N was triggered and Event N was received
-> Agent milestone absorption
-> Goal satisfaction and maintained-condition reconciliation
-> generation-scoped flywheel nonce satisfied
```

The initial Epistemic Operation should therefore mean assess whether Startup Event N was received for Agent A and generation G. Its result identifies the missing Event evidence that justifies Goal inception.

The executable Task is intentionally simple. The Startup Event Capability has one required effect: append the uniquely identified custom Startup Event. It may attach bounded diagnostic metadata, but that data is optional decoration and does not change Goal satisfaction. Execution owns admission, lowering, dispatch, and its operational outcome. Event authority owns append and replay. Curation, Graph, Belief, and Agent own the returning epistemic meaning.

## Why The Custom Event Is Enough

The custom Event is the exact Task effect, so the minimal canary does not need a second runtime-diagnostics observation product. Event append still does not by itself satisfy the Goal. The Plan declares the downstream epistemic milestone that must return to Agent.

The custom Event binds Domain Object references or equivalent exact identities for nonce N, Startup Agent A, Goal N, and generation G. Curation may author the relations that say the Goal was triggered and the Event was received. A configured Belief route may then produce the exact revision Agent consumes.

Current runtime self-observation Events do not satisfy that path. The current Graph reducer admits only workspace, context, and Execution source domains. The narrow missing bridge is admission of the owner-issued Startup Event into the existing epistemic substrate. It does not require a second Event system, a universal health ontology, a runtime diagnostics domain, or semantic interpretation by root runtime.

## Failure Evidence Is The Primary Product

A nonce timeout should not collapse to Meld unhealthy. The inspection surface should show the deepest durable position reached:

```text
prepared
ready
Agent created
initial evidence published
Graph visible
Belief settled
Goal created
Plan judged
Task admitted
operational node claimed
custom Startup Event durable
Startup Event epistemically visible
Agent milestone absorbed
nonce satisfied
```

The first absent or conflicted successor position identifies the broken handoff without asking one coordinator to reinterpret every domain. This is the strongest operational value of the proposal.

One startup nonce account can project this chain as the primary answer to Is Meld running correctly. The account is a read-only correlation over native owner products. It shows the nonce, Agent, Goal, generation, custom Event, current status, deepest durable position, first unresolved successor, and exact evidence lineage. It does not create health truth, advance the Agent, or replace the underlying stores.

Receipt of the custom Event closes the Startup Agent Goal and permits that Agent to publish a complete wait for a generation change or explicit new nonce. Whole-runtime quiescence still requires every realized participant to report no eligible work, complete waits, and viable wakes. Startup nonce satisfaction is strong evidence within that aggregate, not a replacement for it.

## PDS And Lifecycle Ownership

The Startup PDS is product data, not the startup coordinator. It declares a system Agent and its theory like any other product. PDS product control may treat the resulting Agent-owned nonce-satisfaction receipt as a prerequisite for requesting activation of dependent products.

Root lifecycle should not read an Agent belief and decide what it means. It should continue to consume structurally valid lifecycle intents and readiness receipts. Product control owns the policy that a dependent activation request is not issued until the nonce-satisfaction product is current. Root remains semantically blind.

Core diagnostic, lifecycle, Event, storage, inspection, and operator surfaces must remain available when the nonce fails. The bootstrap generation cannot be gated by its own canary, and a failure must not hide the evidence needed to diagnose it.

## Current Grounding

Current Meld already has substantial local primitives.

The [PDS framing](pds_design_framing_after_world_model_reconciliation.md) establishes owner-routed theory, assignment, Agent genesis, activation, and maintained conditions. The [product compilation design](../detailed_design/product_compilation_and_agent_genesis.md) establishes finite Agent topology and an inert prepared closure. The [activation design](../detailed_design/activation_generation_and_lifecycle_closure.md) establishes structural readiness, current publication, owner waits, restart, replacement, and retirement.

The [world initialization pipeline](../../../../../src/init/world/pipeline.rs) can install theory, create an Agent and subscription, mark it operational, and append an epistemic seed Event. Today the operational label requires only one subscription and precedes complete activation readiness.

The [startup activation store](../../../../../src/runtime/activation.rs) can represent a first assignment generation and supplied readiness references, but no production caller connects it to the prepared closure and supervisor.

The [runtime self-observation watcher](../../../../../src/runtime/self_observation.rs) emits durable runtime failure-threshold Events for consumer lag, retention gaps, ingest drops, and restart storms. It emits no positive nonce round trip, keeps crossing state in process memory, and its Event path is observational rather than startup-gating.

The [current Graph reducer](../../../../../crates/meld-world-model/src/world_state/graph/reducer.rs) does not admit runtime-domain Events. A full canary therefore needs the owner-publication bridge already anticipated by the graph-addressed World Model Reconciliation architecture.

## Relationship To The Approved Design Program

The non-gating canary does not require a new architectural owner or a material rewrite of the seven detailed-design slices. It is a new integrated product proof that exercises their existing boundaries from PDS compilation through lifecycle, Curation, Strategy, Agent, Execution, Events, Graph, Belief, and inspection.

The stronger policy that blocks dependent product activation is a separate refinement. It remains compatible with current ownership only if PDS product control owns the dependency and root lifecycle remains semantically blind. Making root interpret the nonce result or making the Startup Agent satisfy its own structural prerequisite would materially violate the accepted design.

If promoted, the smallest program insertion is an early integration slice after the minimal owner-publication, Agent, Strategy, Execution, and activation verticals exist. It can become the first complete runtime product proof before Docs Freshness and Dependency Security are activated. It does not need a new Task Network, Event authority, graph substrate, or runtime supervisor.

## Open Threads

The product name, custom Event type, nonce identity, maintained-condition vocabulary, Event object and relation decoration, optional diagnostic fields, configured epistemic return route, startup account projection, retry cadence, timeout policy, and dependent-product activation expression remain open.

It also remains open whether the Startup Agent fires one nonce per process start, one per activation generation, or additional bounded nonces on demand. Any recurring policy must avoid Event storms and preserve exact generation lineage.

## Non Commitments

This discovery does not amend the approval candidate, add a delivery gate, authorize implementation, select schemas or stores, make the startup nonce globally mandatory, or claim that one canary proves complete system health.
