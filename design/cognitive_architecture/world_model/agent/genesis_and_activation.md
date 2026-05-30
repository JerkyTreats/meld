# Agent Genesis And Activation

Date: 2026-05-28
Status: active
Scope: seed agent authority, runtime activation, and execution capability work for agent initialization

## Thesis

Agent creation and agent activation are separate concerns.

Creation establishes durable agent identity, perspective, policy, scope, and subscriptions.
Activation starts or resumes the runtime workers that watch belief revisions and curate goals for an existing durable agent.

The first operational agents are seed agents. They are created from trusted init or configuration state, not from a curated goal, because no prior agent is available to curate that goal.

After seed agents exist, new agent creation is normal goal set curation. An authorized existing agent may add a goal that requests another agent for a separate concern. Execution then runs the initialization workflow through tasks and capabilities.

## Authority Paths

### Seed Agent Creation

Seed agents are genesis state.

They are created by trusted product initialization or loaded trusted configuration. This path exists only to create the first curators that can operate the rest of the system.

Seed creation must record provenance:

- seed agent id
- perspective key
- subject scope
- configured directive
- trust policy reference
- source config or init source
- creation sequence when available

Seed agents should be minimal. They need enough authority to curate initial operational goals and spawn more specialized agents. They do not need special authority after normal agent curation is available.

### Existing Agent Activation

Activation is process hydration.

When the Meld process starts, the runtime loads durable agent records whose lifecycle says they should be active. It reconstructs runtime state for each record:

- subscription cursors
- watcher registrations
- branch scope
- observation scope
- last processed belief revision
- current planner projection handle
- health and lease state

Activation is not a `CreateAgent` goal. No new identity is created. The runtime is only restarting the workers for existing durable state.

If activation fails, the system may surface a repair goal through an existing seed or operational agent. The failure does not become implicit agent creation.

### Spawned Agent Creation

After seed agents exist, a new agent is created by goal curation.

An authorized existing agent adds a `CreateAgent` goal when its cost benefit evaluation says a separate concern deserves its own perspective, policy, and subscriptions.

Execution owns the initialization workflow. The new agent is the output of that workflow, not the actor that runs it.

The spawned agent may satisfy the `CreateAgent` goal after it reaches operational status and processes its first readiness signal.

## Durable And Runtime State

### AgentRecord

`AgentRecord` is durable world model state.

It should include:

- agent id
- perspective key
- subject scope
- observation scope
- branch scope
- trust policy reference
- evidence policy reference
- subscription refs
- directive provenance
- curator provenance for spawned agents
- lifecycle status
- activation policy

`AgentRecord` is the source of truth for whether an agent exists.

### AgentRuntime

`AgentRuntime` is ephemeral process state.

It should include:

- running watcher handles
- current subscription cursors
- active leases
- last processed revision markers
- runtime health
- restart attempt state

`AgentRuntime` can be dropped and rebuilt without changing agent identity.

The concrete store, query, cursor, curation decision, and replay contracts are defined in [Agent Runtime Surface](runtime_surface.md).

## Initialization Workflow

Agent initialization is an execution workflow driven by tasks and capabilities.

The workflow starts from either trusted seed configuration or a curated `CreateAgent` goal.

Required capability work:

- decompose directive into candidate belief dimensions
- walk the world model graph for the subject scope
- query existing belief views for the perspective
- query available evidence channels
- register missing belief keys
- register the agent record and perspective
- bind subscriptions to relevant belief keys
- create or request first observation work for missing beliefs
- project the first planner facing world state for the perspective
- mark the agent operational when readiness criteria pass

The workflow emits normal execution outcomes and world model facts through the spine.

## Readiness Criteria

An agent has arrived when all required setup state exists and one readiness signal has been processed.

Minimum readiness for the first slice:

- durable agent record exists
- perspective key exists
- at least one belief key is subscribed
- branch scope and observation scope are bound
- the first watched belief view is readable
- first planner projection for that perspective succeeds
- the agent can construct a ground `Goal`

For a spawned agent, arrival allows it to satisfy or provide satisfaction evidence for the `CreateAgent` goal that requested it.

For a seed agent, arrival allows it to begin normal goal curation.

## Minimal Slice

The minimal slice implements seed agent creation only.

It does not require dynamic spawned agents.

It provides:

- one seed agent identity
- one perspective key
- one configured subject scope
- one configured watched belief dimension
- one threshold curation rule configured for the watched belief dimension
- one deterministic goal construction path
- one durable subscription cursor
- one curation decision record
- one goal command handoff boundary

Dynamic `CreateAgent` goals, inter agent spawn policy, and spawned agent satisfaction of initialization goals are full design concerns deferred past the minimal slice.

## Full Design

The full design should support all three paths:

- seed agent creation from trusted init or configuration
- existing agent activation on process start
- spawned agent creation through curated `CreateAgent` goals

The full design must also define spawn authorization:

- which agents may create other agents
- which scopes they may delegate
- which trust policies they may assign
- how duplicate spawn goals are deduped
- how failed initialization is repaired or abandoned

## Read With

- [World Model Agent](README.md)
- [Agent Runtime Surface](runtime_surface.md)
- [Goal Curation](goal_curation.md)
- [World Model Public Interface](../public_interface.md)
- [Goals](../../execution/goals/README.md)
- [Execution Domain](../../execution/README.md)
- [Task Network](../../execution/task_network.md)
