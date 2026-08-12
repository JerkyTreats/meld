# World Model Public Interface

Date: 2026-07-23
Status: active
Scope: common contract for world model operations invoked by capabilities and other domains

## Thesis

The world model exposes a public interface that capabilities can invoke without importing world model internals. This interface is the world model's equivalent of execution's Goal Set API — a narrow, stable contract that other domains consume.

The public interface defines the common contract. Implementation routes live in each owning domain: belief routes in belief, graph routes in graph, agent routes in agent, planner routes in planner, and Strategy routes in Strategy. Each domain owns its operations and their semantics. The interface document defines what is available and which domain owns it.

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

// Register a new belief key for a subject and runtime dimension id
// Returns existing key if one already exists for the pair
register_belief_key(subject: DomainObjectRef, dimension_id: BeliefDimensionId) -> BeliefKey

// Belief freshness summary for a subject (which beliefs are current, stale, or missing)
query_freshness(subject: DomainObjectRef, perspective: Perspective) -> FreshnessSummary
```

`register_belief_key` is the only write operation. It creates a belief key without settling a belief — the belief starts in an unassessed state. Evidence and comparator assessment produce the first revision.
The dimension id must resolve through loaded runtime family configuration.
The public interface does not require Rust enum variants for belief families.

### Agent

Owned by `world_model/agent`. These operations manage durable agent identity, perspective, activation status, and subscriptions.

```
// Register a new agent with perspective, observation scope, and provenance
register_agent(request: AgentRegistrationRequest) -> AgentId

// Mark an existing durable agent as active after runtime hydration
activate_agent(agent_id: AgentId, activation: AgentActivationRecord) -> AgentStatus

// Mark an existing durable agent as inactive without deleting identity
deactivate_agent(agent_id: AgentId, reason: AgentDeactivationReason) -> AgentStatus

// Subscribe an agent to belief revision events for a belief key
subscribe(agent_id: AgentId, belief_key: BeliefKey) -> SubscriptionId

// Remove a subscription
unsubscribe(agent_id: AgentId, subscription_id: SubscriptionId)

// List an agent's current subscriptions
list_subscriptions(agent_id: AgentId) -> Vec<Subscription>

// Advance a subscription cursor after durable curation decision persistence
advance_subscription(agent_id: AgentId, subscription_id: SubscriptionId, revision_id: BeliefRevisionId) -> Subscription

// Record a deterministic curation decision for idempotency and replay
record_curation_decision(decision: AgentCurationDecision) -> AgentCurationDecision

// Ground one activated Directive into concrete belief questions over trusted scope
ground_directive(request: DirectiveGroundingRequest) -> EpistemicObligationSet

// Query agent status (registered, bootstrapping, operational, suspended)
query_agent_status(agent_id: AgentId) -> AgentStatus
```

Agent registration creates durable identity and perspective anchor state. Registration provenance may include trusted seed authority, curator authority, and the directive that seeded the agent.

Activation is process hydration for an existing durable agent record. It starts or resumes runtime watchers and subscription cursors. It does not create a new agent.

Subscription binding happens during the initialization workflow through execution capabilities that invoke these operations.

Directive grounding derives concrete belief questions from activated PDS theory and trusted graph scope. Belief retains authority for key registration, evidence admission, assessment, and revision.

The runtime surface also needs a durable curation decision record and subscription cursor advancement. These make at least once belief revision delivery safe across restart and replay.

### Planner

Owned by `world_model/planner`. These operations expose the action-relevant world model projection.

```
// Current world state for one subject and belief dimension
project_current_world_state(subject: DomainObjectRef, dimension_id: BeliefDimensionId, perspective: Option<Perspective>, branch_scope: Option<BranchScope>) -> PlannerProjectionOutput

// Full world model view for a scoped planning question
query_world_model_view(context: DecisionContext) -> WorldModelView

// Observation opportunities where expected information gain justifies observation cost
query_observation_opportunities(context: DecisionContext) -> Vec<ObservationOpportunityView>

// World-facing condition assessments for the scoped planning question
query_preconditions(context: DecisionContext) -> Vec<PreconditionAssessment>
```

The minimal planner route is `PlannerQuery::project_current_world_state`. It reads current belief views and current graph anchors, then returns a ground `meld-lang::WorldState` with provenance, hydration refs, and projection warnings.

Planner operations are read-only deterministic projections over graph and belief. Broad views may add causation and regime state.
They do not expose raw inference internals.
They must return view records with provenance and hydration handles, not free-form semantic summaries.

### Strategy

Owned by `world_model/strategy`. Strategy exposes bounded candidate search over an immutable problem, plus independent verification of any candidate before authorization.

```
search(request: StrategySearchRequest) -> StrategySearchResult
verify_candidate(problem: StrategyProblem, candidate: StrategyCandidate) -> StrategyVerification
```

The search input is a complete immutable `StrategyProblem` — goal, planner snapshot, theory snapshot, capability vocabulary, methods, and evaluation policy — with content-derived identity. Search is a pure function: no live world-model queries, live catalogs, ambient configuration, or unseeded randomness. The result carries honest completion — a bounded result may claim only strongest-found; only an exhaustive result with no recommendation may state that no eligible candidate exists.

Verification is independent of the discovering algorithm and accepts no search diagnostics as proof. Strategy construction consumes planner projections through their public contracts and does not expose or import lower inference internals.

The Goal-owning Agent authorizes a verified candidate before initial Goal admission. Persistence custody does not confer authority.

Execution consumes the admitted authorization through explicit contracts, then returns realization acceptance or rejection. Execution must not reinterpret the candidate.

The full semantic boundary lives in [Strategy Search](strategy/search.md) and [Strategy Boundary Contracts](strategy/contracts.md).

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
  Input: DomainObjectRef, Vec<BeliefDimensionId>
  Output: Vec<BeliefKey>
```

The capability is the execution-side contract. The interface operation is the world-model-side contract. The capability invokes the operation. Neither side imports the other's internals.

Multiple agents may query and project in parallel through agent scoped views.
They share lower domain state and write only agent scoped projections.
Shared graph, belief, causal, and regime writers remain domain owned.

## Relationship to Execution's Goal Set API

The two public APIs form a symmetric pair:

| Execution's Goal Set API | World Model's Public Interface |
|---|---|
| Initial admission requires Goal plus authorized Strategy inventory | Directive grounding constructs belief questions before Goal curation |
| Later lifecycle curated by world model Agents | Invoked by Execution capabilities and world-model domains |
| admit, modify, remove, satisfy, suspend, resume, read | query, walk, ground, subscribe, register, construct |
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
- [World Model Strategy](strategy/README.md)
- [Goals (Execution)](../execution/goals/README.md)
