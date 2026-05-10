# World Model Agent

Date: 2026-05-01
Status: active
Scope: perspective-scoped agent identity and world-model ownership above belief and below execution commitment

## Thesis

`world_model/agent` defines the Agent as a world-model concern before it becomes an execution concern.

The Agent is the fulcrum from belief to action.
It does not dispatch tasks or own runtime control.
It owns the perspective that determines which facts are trusted, which uncertainty matters, which regime concerns are relevant, and which planner-facing world-model view should be published for that perspective.

This area exists because shared graph truth is not the same as shared belief.
Many Agents may consume one shared event and graph substrate while producing different belief views, regime sensitivities, and action-relevant projections.

## Boundary

`world_model/agent` owns:

- agent identity as a world-model perspective anchor
- perspective-scoped belief ownership
- evidence policy and trust profile
- observation scope and branch scope
- regime sensitivity profile
- normative framework — what belief states the agent cares about and what thresholds trigger action
- planner-facing world-model view assembly for one perspective
- goal set curation — evaluating beliefs through cost-benefit comparators and curating execution's goal set through its public API
- active goal awareness — reading the goal set for prediction, redundancy avoidance, and normative evaluation
- cost-benefit evaluation — deciding when belief divergence warrants action based on learned cost and value beliefs

`world_model/agent` does not own:

- the goal set itself (owned by execution)
- goal lifecycle state machine (owned by execution)
- task graphs, task decomposition, or dispatch
- continuation or runtime control state
- provider execution
- canonical event append

## Relationship To Other World Model Domains

`graph` is shared substrate.
It does not become agent-private because one Agent distrusts or ignores part of it.

`belief` produces perspective-scoped posterior state.
`agent` defines which perspective is being served and which belief views should be assembled for it.

`causation` may expose effect summaries that differ by the intervention and measurement assumptions relevant to the Agent.

`regime` may expose structural uncertainty that one Agent treats as central and another treats as tolerable.

`planner` remains the final projection layer that turns these concerns into action-relevant world-model reads.

## Relationship To Execution

The Agent is not the execution runtime.

Within `world_model`, the Agent owns epistemic perspective and normative judgment.
Within `execution`, the Agent's goals are data — the planning loop reads them and the task network works toward them.

The Agent bridges the two domains through execution's public Goal Set API:

- the Agent reads its perspective-scoped belief views (world model authority)
- the Agent evaluates beliefs through cost-benefit comparators — combining state beliefs, cost beliefs (learned from execution outcomes), and value beliefs (learned from downstream outcome correlation) into act/tolerate decisions
- the Agent curates execution's goal set through the API: add, modify, remove, satisfy, suspend, resume
- execution reacts to the current goal set without understanding why it changed

The normative framework reduces to: which belief keys the agent watches (subscription filter), and what regime-scoped priors it carries for the cost-benefit comparison on each concern class. See [Goal Curation](goal_curation.md) for the full mechanism.

The boundary is:

- `world_model/agent` decides what should be true (normative judgment over belief)
- `execution` decides how to make it true (planning, task decomposition, dispatch)

The Agent also has read access to the active goal set. This is epistemically valuable: knowledge of active goals enables prediction (what evidence to expect), anomaly detection (goals without progress), and avoidance of redundant goal generation.

See [Goals](../../execution/goals/README.md) for the full ownership split and curation API contract.

## ECS Note

The ECS interpretation for this domain lives in [Agent ECS](ECS.md).

The important point is that Agent should consume the heavier systems of the other world model domains and assemble them into one perspective-scoped handoff, rather than re-owning their internal logic.

## Core Design Rule

The Agent should be the owner of perspective, not the owner of truth.

That means:

- graph truth stays shared and replayable
- belief may diverge by perspective
- regime sensitivity may diverge by perspective
- planner-facing world-model views may diverge by perspective
- execution still receives one shaped view per consuming perspective

## Multi-Agent Requirement

This domain is where the world model satisfies the `En Masse and At Will` requirement.

The architecture should support:

- one shared event and graph substrate
- many Agent entities over that substrate
- sparse and divergent attached belief state
- independent planner-facing projections
- stable replay and audit across all Agents

The main payoff of an ECS-shaped internal substrate is here:
many Agents can share one identity and provenance foundation while carrying sparse, divergent, mutable world-model state without forcing one rigid record for every perspective.

## Agent Lifecycle

Agent creation is an execution goal. A directive ("ensure code quality for module X") is turned into an operational agent through the same goal → plan → task network → capability pipeline that handles all execution.

### Bootstrap

```
1. Init        Goal generated: "create agent with directive D for subject S"
2. Decompose   Execution decomposes directive into candidate belief dimensions
                 (capability: semantic analysis of directive against subject)
3. Survey      Capabilities invoke world model public interface:
                 - graph.walk to discover subject's entity neighborhood
                 - belief.query_beliefs to find existing beliefs
                 - belief.query_evidence_channels to find available observations
4. Bind        Capabilities invoke world model public interface:
                 - agent.register_agent to create identity and perspective
                 - belief.register_belief_key for missing dimensions
                 - agent.subscribe for each belief key
5. Observe     Agent generates observation goals for dimensions
                 where belief keys exist but no belief revision yet
6. Arrive      Agent processes first belief revision event through
                 its cost-benefit comparator — creation goal satisfied
```

Every step is a capability in the task network. Observable through the spine. Cost-tracked. Retryable.

The satisfaction criterion for the creation goal: the agent has bound subscriptions and has processed at least one belief revision event through its cost-benefit evaluation. "I have arrived" — the agent can curate goals.

### Steady state

After bootstrap, the agent operates through the watching pattern described in [Goal Curation](goal_curation.md). Belief revision events arrive on subscribed keys. The cost-benefit comparator evaluates. Goal mutations emit when warranted.

### Re-survey

When the capability catalog changes or the subject scope expands, the agent re-surveys. This is a partial re-bootstrap: new evidence channels may be discoverable, new belief dimensions may be relevant. Re-survey can be triggered by spine events indicating capability registration or subject scope changes.

### Shutdown

Agent shutdown is also a goal. The agent's subscriptions are unbound. Active goals curated by this agent are evaluated for transfer to another agent or abandonment. Cleanup runs through the normal task network.

## First Slice

The first slice should remain narrow.

It should define:

- one Agent identity anchored to `DomainObjectRef`
- one explicit perspective key
- one evidence and trust policy surface
- one branch and observation scope surface
- one path from belief views to planner-facing projection for that Agent
- bootstrap lifecycle through execution goal decomposition

It should defer:

- full multi-Agent synchronization strategy
- shared planning between Agents
- multi-agent goal coordination protocol
- any requirement that other crates adopt ECS vocabulary

## Read With

- [World Model Domain](../README.md)
- [World Model Vision](../VISION.md)
- [World Model Planner](../planner/README.md)
- [Agent ECS](ECS.md)
- [World Model Belief](../belief/README.md)
- [Belief Microarchitecture](../belief/microarchitecture.md)
- [Knowledge Graph ECS Decision Memo](../belief/knowledge_graph_ecs_decision_memo.md)
- [Goal Curation](goal_curation.md)
- [World Model Public Interface](../public_interface.md)
- [Execution Domain](../../execution/README.md)
- [Goals](../../execution/goals/README.md)
