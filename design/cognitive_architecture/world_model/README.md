# Meld World Model

Date: 2026-05-27
Status: active
Scope: durable world model shape across graph, belief, causation, regimes, planner-facing reads, Agent judgment, and Strategy

## Thesis

The world model starts from what Meld already has: a shared event spine, explicit graph attachment through `DomainObjectRef` and `EventRelation`, replayable graph materialization, current anchor selection, lineage, provenance, traversal indexes, and branch-scoped federation.

The research architecture remains the direction of travel, but the foundation is not speculative. `world_model/graph` is the substrate that other world model layers consume, and `world_model/belief` settles beliefs over that substrate. Planner-facing projection, causation, and regime layers build on graph and belief without re-reading raw events or importing source-domain internals.

The operating axiom remains "To Each Domain Be True." Each layer owns one kind of authority and crosses layer boundaries through explicit contracts.

For the longer-range framing and the preserved multi-agent vision language, see [World Model Vision](VISION.md).

World model layers:

1. bitemporal state graph
2. belief and generative inference
3. causal layer
4. regime layer
5. planner-facing world model projection
6. Agent-authorized Strategy

The event spine is upstream event authority, not a world model layer.

## Meld Foundation

The durable foundation the world model builds on is:

- `events` owns append, sequence, replay, idempotent record lookup, object refs, and relation edges.
- source domains publish promoted semantic facts without giving the world model direct access to their internals.
- `world_model/graph` materializes current anchors, lineage, provenance, adjacency, bounded walks, and branch-annotated traversal views.
- `world_model/belief` loads runtime family configuration, normalizes graph and promoted evidence, assesses configured belief keys, commits revisions, and projects planner-safe belief views.
- graph reducers publish derived anchor facts back through the spine, so graph state remains replayable.
- `meld-world-model` is the authority crate for graph materialization and belief assessment.

Within the world model itself, graph is the foundational durable layer and belief is the inference layer over it. Causation, regime, and planner-facing projection should consume graph and belief services rather than reconstructing their own view from raw event replay.

Graph and belief should therefore be read as substrate design, not as unbuilt research proposal.

## Multi-Agent Grounding

An agent in the current Meld context is a runtime-owned set of prompts, workflows, goals, and perspectives over specific `DomainObjectRef` values.

The grounded architectural requirement is shared substrate with perspective-scoped world model views.

That means:

- graph and events remain shared replayable foundations
- belief ownership may diverge by perspective, evidence policy, or branch scope
- regime interpretation may diverge across agents over the same underlying facts
- planner-facing projection must stay scoped to the consuming perspective

The maximalist multi-agent framing remains preserved in [World Model Vision](VISION.md).

## Graph Surface Consumed By Upper Layers

The graph layer publishes the functionality that higher world model layers should depend on:

- current anchor reads by subject, perspective, frame type, artifact type, or other graph-readable selector
- anchor lineage and supersession chains
- provenance bundles that explain which facts made an anchor current
- object history by `DomainObjectRef`
- relation adjacency and bounded graph walks with direction, filters, and current-only selection
- branch presence and branch-annotated federated reads
- replay cursors and source fact identifiers for durable rebuild and hydration

Belief should consume anchors, provenance, lineage, and object history as evidence inputs.
Causation should consume temporal state, intervention-shaped facts, outcome links, and measurement paths without treating anchor selection as proof of effect.
Regime should consume graph and belief signals that indicate structural change, including relation stability, observation cadence, calibration drift, and correlated failures.
The planner-facing projection should consume settled graph-derived views from belief, causation, and regime rather than raw reducer internals.
Strategy should consume typed planner projections and Agent authority to construct concrete theories of action without creating substitute epistemic verdicts.

## Upstream Dependency

### Event Spine

Job:
preserve what was observed, attempted, emitted, corrected, or superseded

Primary concepts:

- `ObservationFact`
- `ActionFact`
- `OutcomeFact`
- `CorrectionFact`
- `ProvenanceLink`
- `DomainObjectRef`
- `EventRelation`

Existing design:

- [Events Design](../events/README.md)
- [Multi-Domain Spine](../events/multi_domain_spine.md)
- [Sensory Domain](../sensory/README.md)

The spine is a dependency of the world model, not an internal world model layer.

## Layer Model

### 1. Bitemporal State Graph

Job:
materialize what holds for a reference time from what was known by a transaction time

Current role:
provide the graph services consumed by every higher world model layer

Functionality surfaced upward:

- current anchor reads by subject and perspective
- current frame head, snapshot, and artifact selectors where source domains publish graph-readable facts
- anchor lineage and supersession chains
- provenance bundles for why an anchor is current
- object history by `DomainObjectRef`
- relation adjacency and bounded graph walks
- branch presence and branch-annotated federated reads
- source fact identifiers and replay cursors for hydration and rebuild

Primary concepts:

- `TemporalEntity`
- `TemporalRelation`
- `TemporalAnchor`
- `ValidityInterval`
- `TransactionTime`
- `ReferenceTime`
- `AsOfView`
- `BranchScope`

Existing design:

- [World Model Crate](CRATE.md)
- [World Model Crate Migration](../../completed/world_model/MIGRATION.md)
- [World Model Graph](graph/README.md)

### 2. Belief And Generative Inference

Job:
maintain prior, posterior, uncertainty, surprise, freshness, contradiction, and latent hypotheses over the state graph

Consumes from graph:
current anchors, provenance bundles, lineage, object history, and graph walks that can become normalized evidence

Primary concepts:

- `BeliefKey`
- `EvidenceItem`
- `BeliefRevision`
- `PosteriorSummary`
- `BeliefView`
- `AssessmentLease`
- `ObservationOpportunity`

Deferred concepts:

- `BeliefPrior`
- `BeliefPosterior`
- `HypothesisSet`
- `FreshnessModel`
- `ObservationCoverage`

Existing design:

- [World Model Belief](belief/README.md)
- [Belief Microarchitecture](belief/microarchitecture.md)
- [Fact To Belief](belief/fact_to_belief.md)
- [Comparator Model](belief/comparator_model.md)
- [Belief Substrate](belief/substrate.md)

### 3. Causal Layer

Job:
separate evidence from mechanism and answer intervention and counterfactual questions without treating anchor selection as causal proof

Consumes from graph:
temporal state, relation paths, intervention/outcome object links, provenance, and measurement selection paths

Primary concepts:

- `CausalVariable`
- `StateVariable`
- `InterventionVariable`
- `OutcomeVariable`
- `SelectionVariable`
- `ConfounderHypothesis`
- `CausalClaim`
- `EffectEstimate`
- `CounterfactualQuery`

Design:

- [Causal Layer](causation/README.md)

### 4. Regime Layer

Job:
track structural change, recurring operating modes, and mixture prediction under uncertainty about which regime is active

Consumes from graph and belief:
relation stability, observation cadence, anchor churn, calibration drift, contradiction clusters, and effect shifts

Primary concepts:

- `RegimeId`
- `RegimePosterior`
- `ChangepointState`
- `RunLengthBelief`
- `MixturePrediction`
- `RegimeLibrary`
- `ContinuationModel`
- `BreakModel`
- `StressScenario`

Design:

- [Regime Layer](regime/README.md)

### 5. Planner

Job:
publish the action-relevant world model projection that execution reads without transferring execution policy into `world_model`

Consumes from graph indirectly:
planner views should receive graph-derived state through belief, causation, regime, and explicit hydration handles, not by scanning reducer internals

Primary concepts:

- `WorldModelView`
- `ActionableBeliefView`
- `DecisionContext`
- `ViewSnapshot`
- `ObservationOpportunityView`
- `AbstentionState`
- `CausalEffectSummary`
- `RiskEnvelope`
- `PreconditionAssessment`
- `ConflictSummary`
- `SensitivitySummary`
- `AssumptionSet`
- `HydrationHandle`

Design:

- [World Model Planner](planner/README.md)
- [Planner Spec](planner/spec.md)

### 6. Strategy

Job:
construct episode-specific semantic theories of action for Agent authorization

Consumes:
Agent authority, operational domain theory, trusted scope, typed planner projections, semantic action affordances, and operational projections

Primary concepts:

- proposed Goal
- Strategy candidate
- Agent authorization
- prospective evidence route
- settled operational replay

Agent curation creates the proposed Goal. Strategy creates a candidate course of action and gates initial Execution admission through Agent authorization. It does not settle beliefs, mutate admitted Goal lifecycle, resolve capabilities, or commit task-network work.

Design:

- [World Model Strategy](strategy/README.md)
- [Directive Grounding](agent/directive_grounding.md)
- [Strategy Boundary Contracts](strategy/contracts.md)
- [Docs Freshness Strategy](strategy/docs_freshness.md)

## Layer Rules

- The event spine remains upstream event authority, not a world model layer.
- The state graph owns current graph materialization, temporal truth maintenance, lineage, provenance, and traversal, not belief settlement.
- The belief layer owns uncertainty, freshness, and revision, not action dispatch.
- The causal layer owns mechanism claims, intervention semantics, and counterfactual answers.
- The regime layer owns structural change and recurring contexts.
- The planner-facing world model projection owns action-relevant reads, not execution policy or raw inference internals.
- The Agent owns Directive grounding, Goal drafts, and normative authorization for Goal admission and Strategy decisions.
- Strategy owns semantic candidate construction, not epistemic source truth or operational commitment.

## Public Interface

The world model exposes a public interface that capabilities can invoke without importing world model internals. Each domain owns its routes: graph owns traversal and discovery, belief owns belief queries and key registration, agent owns identity and subscription management, planner owns action-relevant projections.

This interface meets Execution at Goal admission and later lifecycle curation. Initial admission carries a Goal with a nonempty authorized Strategy inventory. World-model belief and graph remain available through domain-owned query routes.

See [World Model Public Interface](public_interface.md) for the full contract.

## Read Order

1. [Events Design](../events/README.md)
2. [Multi-Domain Spine](../events/multi_domain_spine.md)
3. [World Model Crate](CRATE.md)
4. [World Model Graph](graph/README.md)
5. [World Model Belief](belief/README.md)
6. [Fact To Belief](belief/fact_to_belief.md)
7. [Belief Families](belief/belief_families.md)
8. [Causal Layer](causation/README.md)
9. [Regime Layer](regime/README.md)
10. [World Model Planner](planner/README.md)
11. [World Model Agent](agent/README.md)
12. [Goal Curation](agent/goal_curation.md)
13. [Directive Grounding](agent/directive_grounding.md)
14. [World Model Strategy](strategy/README.md)
15. [World Model Public Interface](public_interface.md)

## Naming Rule

Use `world_model` for design, architecture, and public conceptual discussion.

Use `world_state` only when referring to compatibility surfaces that still exist in code, events, or migration notes.
