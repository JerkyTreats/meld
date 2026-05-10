# World Model Public Interface

Date: 2026-05-10
Status: active
Scope: common contract for world model operations invoked by capabilities and other domains

## Thesis

The world model exposes a public interface that capabilities can invoke without importing world model internals. This interface is the world model's equivalent of execution's Goal Set API — a narrow, stable contract that other domains consume.

The public interface defines the common contract. Implementation routes live in each owning domain: belief routes in belief, graph routes in graph, agent routes in agent. Each domain owns its operations and their semantics. The interface document defines what is available and which domain owns it.

If ECS substrate consolidation proceeds, some per-domain routes may converge into unified component queries. The public contract remains stable regardless — ECS changes the implementation, not the interface.

## Interface By Domain

### Graph

Owned by `world_model/graph`. These operations expose the bitemporal state graph for traversal and discovery.

```
// Bounded graph walk from an entity with direction, filter, and depth
walk(from: DomainObjectRef, filter: WalkFilter, depth: u32) -> WalkResult

// Current anchors for a subject, optionally scoped by perspective
query_anchors(subject: DomainObjectRef, perspective: Option<Perspective>) -> Vec<AnchorSummary>

// Lineage chain for an anchor (supersession history)
query_lineage(anchor: AnchorId) -> LineageChain

// Provenance bundle explaining why an anchor is current
query_provenance(anchor: AnchorId) -> ProvenanceBundle

// Object history by domain object reference
query_object_history(subject: DomainObjectRef) -> ObjectHistory

// Relation adjacency for an entity
query_relations(subject: DomainObjectRef, direction: Direction, filter: Option<RelationFilter>) -> Vec<Relation>
```

Graph operations are read-only from the public interface. Graph mutation happens through reducers processing spine events, not through the public API.

### Belief

Owned by `world_model/belief`. These operations expose belief state, evidence channels, and belief key management.

```
// All belief views for a subject, scoped by perspective
query_beliefs(subject: DomainObjectRef, perspective: Perspective) -> Vec<BeliefView>

// Single belief view by key
query_belief(key: BeliefKey) -> Option<BeliefView>

// Available evidence channels for a subject (what kinds of observations can feed beliefs)
query_evidence_channels(subject: DomainObjectRef) -> Vec<EvidenceChannel>

// Register a new belief key for a subject/dimension pair
// Returns existing key if one already exists for the pair
register_belief_key(subject: DomainObjectRef, dimension: BeliefDimension) -> BeliefKey

// Belief freshness summary for a subject (which beliefs are current, stale, or missing)
query_freshness(subject: DomainObjectRef, perspective: Perspective) -> FreshnessSummary
```

`register_belief_key` is the only write operation. It creates a belief key without settling a belief — the belief starts in an unassessed state. Evidence and comparator assessment produce the first revision.

### Agent

Owned by `world_model/agent`. These operations manage agent identity, perspective, and subscriptions.

```
// Register a new agent with perspective and observation scope
register_agent(perspective: Perspective, scope: ObservationScope) -> AgentId

// Subscribe an agent to belief revision events for a belief key
subscribe(agent_id: AgentId, belief_key: BeliefKey) -> SubscriptionId

// Remove a subscription
unsubscribe(agent_id: AgentId, subscription_id: SubscriptionId)

// List an agent's current subscriptions
list_subscriptions(agent_id: AgentId) -> Vec<Subscription>

// Query agent status (registered, bootstrapping, operational, suspended)
query_agent_status(agent_id: AgentId) -> AgentStatus
```

Agent registration creates the identity and perspective anchor. Subscription binding happens during the bootstrap lifecycle — either directly through subscription operations or through capabilities that invoke them.

### Planner

Owned by `world_model/planner`. These operations expose the action-relevant world model projection.

```
// Full world model view for a perspective (belief summaries, uncertainty, freshness, regime context)
query_world_model_view(perspective: Perspective) -> WorldModelView

// Observation opportunities for a subject (where expected information gain justifies observation cost)
query_observation_opportunities(subject: DomainObjectRef, perspective: Perspective) -> Vec<ObservationOpportunity>

// Execution preconditions for a subject (world-facing conditions that must hold for action)
query_preconditions(subject: DomainObjectRef) -> Vec<ExecutionPrecondition>
```

Planner operations are read-only projections over belief, causation, and regime state. They do not expose raw inference internals.

## Capability Invocation Pattern

Capabilities invoke the public interface through domain routes. The capability contract declares which interface operations it uses:

```
Capability: survey_beliefs_for_subject
  Interface: belief.query_beliefs, belief.query_evidence_channels
  Input: DomainObjectRef (the subject to survey)
  Output: BeliefSurveyArtifact (existing beliefs, available channels, gaps)

Capability: bind_agent_subscriptions
  Interface: agent.subscribe
  Input: AgentId, Vec<BeliefKey>
  Output: Vec<SubscriptionId>

Capability: register_missing_belief_keys
  Interface: belief.register_belief_key
  Input: DomainObjectRef, Vec<BeliefDimension>
  Output: Vec<BeliefKey>
```

The capability is the execution-side contract. The interface operation is the world-model-side contract. The capability invokes the operation. Neither side imports the other's internals.

## ECS Note

In an ECS substrate, many of these operations become component queries:

- "All beliefs for subject X" → entities with `BeliefView` component where subject matches
- "All subscriptions for agent A" → entities with `Subscription` component where agent_id matches
- "Walk from entity X" → relation component traversal

The per-domain route structure remains correct as the public contract. ECS may consolidate the implementation behind those routes, but the interface operations and their owning domains stay the same. Capabilities invoke domain routes, not raw ECS queries.

## Relationship to Execution's Goal Set API

The two public APIs form a symmetric pair:

| Execution's Goal Set API | World Model's Public Interface |
|---|---|
| Curated by world model agents | Invoked by execution capabilities |
| add, modify, remove, satisfy, suspend, resume, read | query, walk, subscribe, register |
| Owns goal lifecycle state | Owns belief/graph/subscription state |
| Narrow mutation contract | Read-heavy with selective writes |

Each domain exposes a public contract. Capabilities bridge them. The spine carries the results. Neither domain imports the other's internals.

## What This Interface Does Not Cover

### Belief mutation

Beliefs are not mutated through the public interface. Belief revision happens through the internal fact→evidence→comparator→revision pipeline. The public interface exposes belief state for reading and belief keys for registration, not belief settlement.

### Graph mutation

Graph state is not mutated through the public interface. Graph materialization happens through reducers processing spine events. The public interface exposes graph state for traversal and discovery.

### Event publication

Writing to the spine is not a world model operation. Capabilities that produce observations publish facts through the spine's own append contract. The world model reads those facts through its internal reducers.

## Read With

- [World Model Domain](README.md)
- [World Model Agent](agent/README.md)
- [Goal Curation](agent/goal_curation.md)
- [World Model Belief](belief/README.md)
- [World Model Graph](graph/README.md)
- [World Model Planner](planner/README.md)
- [Goals (Execution)](../execution/goals/README.md)
