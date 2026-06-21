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

The Agent bridges the two domains through the shared typed language [`meld-lang`](../../meld-lang/README.md) and integration mapping into execution's neutral Goal Set API:

- the Agent reads its perspective-scoped belief views (world model authority)
- the Agent evaluates beliefs through cost-benefit comparators — combining state beliefs, cost beliefs (learned from execution outcomes), and value beliefs (learned from downstream outcome correlation) into act/tolerate decisions
- the Agent constructs `Goal` values using `meld-lang` types: the desired state is a `Proposition`, the priority is a `GoalPriority` with cost ceiling, the source records provenance as `GoalSource`
- the Agent emits producer curation output that integration maps into execution's Goal Set API: add, modify, remove, satisfy, suspend, resume
- execution evaluates goals mechanically against `WorldState` — it never interprets semantic intent

The normative framework reduces to: which belief keys the agent watches (subscription filter), and what regime-scoped priors it carries for the cost-benefit comparison on each concern class. See [Goal Curation](goal_curation.md) for the full mechanism.

The boundary is:

- `world_model/agent` decides what should be true (normative judgment over belief), expressed as `Proposition` targets
- `execution` decides how to make it true (planning, task decomposition, dispatch), evaluated mechanically against `WorldState`

The shared language eliminates the need for execution to interpret belief semantics. The Agent constructs a `Proposition::Holds { subject, dimension, condition }` and execution evaluates it with `evaluate(world_state, goal.target)`. The three-valued result (Satisfied, Unsatisfied, Indeterminate) drives planning decisions without any interpretation of what the dimension means. See [World State and Evaluation](../../meld-lang/world_state.md).

The Agent also has read access to the active goal set. This is epistemically valuable: knowledge of active goals enables prediction (what evidence to expect), anomaly detection (goals without progress), and avoidance of redundant goal generation.

See [Goals](../../execution/goals/README.md) for the full ownership split and curation API contract.
See [Goals and Methods](../../meld-lang/goals_and_methods.md) for the concrete `Goal` type definition and construction examples.

The Agent should consume the heavier pipelines of the other world model domains and assemble them into one perspective-scoped curation output, rather than re-owning their internal logic.

See [Agent Spec](spec.md) for domain types, data model, and pipelines.
See [Agent Runtime Surface](runtime_surface.md) for store, query, activation, subscription, curation, idempotency, and replay contracts.

## The Agent Meta-Layer

A directive is durable, user-originated intent: the persisted answer to why an agent exists. It belongs to the agent meta-layer, the layer that turns user intent into one or more agents handling related but executionally-distinct concerns.

The meta-layer extends the watching and reducing pattern one level above the agent:

| Layer | Watches | Produces | Decomposes | Question |
|---|---|---|---|---|
| meta-layer | user intent | agents | intent into an agent set | why |
| agent | belief revisions | goals | belief into a goal set | what |
| execution | goal set | tasks | goal into a task network | how |

Where execution decomposes a goal into tasks, the meta-layer decomposes intent into agents. The agent is the unit it produces.

### Durable Contract

Only the durable nouns that outlive the deferral window need to be settled now. The meta-layer's logic is runtime that persists nothing, so it can land later without migration. The settled shell is:

- a directive has independent identity, separate from any agent: `Directive { id, text }`, with an optional lifecycle status
- an agent may be attributed to a directive by reference, and not one-to-one: `AgentRecord.directive_id`, so one directive may be served by many agents
- goal lineage cites the directive through `GoalSource::UserDirected` in [`meld-lang`](../../meld-lang/README.md)

This shape stays neutral on the choices the meta-layer will make, so picking either later is additive:

- cardinality of intent to agent is reserved as many and never asserted as one
- coordination across agents that serve one directive is the deferred multi-agent goal coordination concern, anchored at the shared graph, goal set, and task network, not at a directive subsystem
- agent-creation authority is unaffected, because attribution through `directive_id` is orthogonal to authority through `seed_provenance` and curator provenance

### Deferred

The meta-layer's verbs are deferred and have a reserved home in the contract above:

- the translation runtime that turns intent into a chosen agent set
- decomposition records of that agent set and re-decomposition when intent changes
- a serialized goalset-template vocabulary that expands one directive into a goal DAG using existing `meld-lang` propositions and compositions, sibling to the method library
- natural-language interpretation of arbitrary user intent, realized as a planning capability that emits `CreateAgent` goals

In the first slice exactly one seed agent serves one directive, supplied as trusted seed configuration. See [Agent Genesis And Activation](genesis_and_activation.md).

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

The main payoff of the agent shape is here:
many Agents can share one identity and provenance foundation while carrying sparse, divergent, mutable world-model state without forcing one rigid record for every perspective.

## Agent Lifecycle

Agent creation and activation are separate.

Seed agents are created from trusted init or configuration state. This is genesis authority, not goal curation, because no prior agent exists to curate the first agent creation goal.

Existing agents are activated when the process starts by hydrating durable agent records into runtime watchers and subscriptions. Activation does not create a new agent and does not require a `CreateAgent` goal.

After seed agents exist, new agent creation is normal goal set curation. An authorized existing agent may add a `CreateAgent` goal for a separate concern. Execution turns the curated agent responsibility into an operational agent through the same goal to plan to task network to capability pipeline that handles all execution.

See [Agent Genesis And Activation](genesis_and_activation.md) for the durable state, runtime state, and authority paths.

### Bootstrap

```
1. Init        Seed config or curated CreateAgent goal supplies
                 the directive and agent responsibility
2. Resolve     Seed config supplies candidate belief dimensions
                 and subject scope
3. Survey      Capabilities invoke world model public interface:
                 - graph.walk to discover subject's entity neighborhood
                 - belief.query_beliefs to find existing beliefs
                 - belief.query_evidence_channels to find available observations
4. Bind        Capabilities invoke world model public interface:
                 - agent.register_agent to create identity and perspective
                 - belief.register_belief_key for missing dimensions
                 - agent.subscribe for each belief key
5. Observe     Execution requests first observation work for dimensions
                 where belief keys exist but no belief revision yet
6. Arrive      Agent processes first belief revision event through
                 its cost-benefit comparator — creation goal satisfied
```

Every step is a capability in the task network. Observable through the spine. Cost-tracked. Retryable.

The initialization workflow is run by execution. The new agent is the output of that workflow, not the actor that runs it.

The satisfaction criterion for a spawned agent creation goal: the agent has bound subscriptions and has processed at least one readiness signal through its cost-benefit evaluation. The newly arrived agent may then satisfy or provide satisfaction evidence for the `CreateAgent` goal that requested it.

For a seed agent, the same arrival criterion enables normal goal curation, but there is no prior `CreateAgent` goal to satisfy.

### Steady state

After bootstrap, the agent operates through the watching pattern described in [Goal Curation](goal_curation.md). Belief revision events arrive on subscribed keys. The cost-benefit comparator evaluates. Goal mutations emit when warranted.

### Re-survey

When the capability catalog changes or the subject scope expands, the agent re-surveys. This is a partial re-bootstrap: new evidence channels may be discoverable, new belief dimensions may be relevant. Re-survey can be triggered by spine events indicating capability registration or subject scope changes.

### Shutdown

Agent shutdown is also a goal. The agent's subscriptions are unbound. Active goals curated by this agent are evaluated for transfer to another agent or abandonment. Cleanup runs through the normal task network.

## First Slice

The implemented first slice remains narrow.

It defines:

- one seed Agent identity from trusted init or configuration
- one Agent identity anchored to `DomainObjectRef`
- one explicit perspective key
- one evidence and trust policy surface
- one branch and observation scope surface
- one path from belief views to planner-facing projection for that Agent through `BeliefQuery` and `PlannerQuery`
- seed registration through the world model agent command surface
- durable runtime records for registration, subscription, cursor, and curation decision
- one proposed `AgentGoalCommand` with a ground `meld-lang::Goal` built from runtime rule configuration and mapped into execution's neutral Goal Set API
- duplicate suppression through active goal summary input and curation decision dedupe

It defers:

- dynamic spawned Agent creation through curated `CreateAgent` goals
- existing Agent activation across process restart
- full `AgentRuntime` process workers
- full multi-Agent synchronization strategy
- shared planning between Agents
- multi-agent goal coordination protocol
- any requirement that other crates adopt world model implementation vocabulary

## Read With

- [World Model Domain](../README.md)
- [World Model Vision](../VISION.md)
- [World Model Planner](../planner/README.md)
- [Agent Spec](spec.md)
- [World Model Belief](../belief/README.md)
- [Belief Microarchitecture](../belief/microarchitecture.md)
- [Agent Spec](spec.md)
- [Agent Genesis And Activation](genesis_and_activation.md)
- [Agent Runtime Surface](runtime_surface.md)
- [Goal Curation](goal_curation.md)
- [World Model Public Interface](../public_interface.md)
- [Lang Domain](../../meld-lang/README.md)
- [Lang Goals and Methods](../../meld-lang/goals_and_methods.md)
- [Lang World State and Evaluation](../../meld-lang/world_state.md)
- [Execution Domain](../../execution/README.md)
- [Goals](../../execution/goals/README.md)
