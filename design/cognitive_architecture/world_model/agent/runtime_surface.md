# Agent Runtime Surface

Date: 2026-05-28
Status: active
Scope: concrete runtime contracts for `world_model/agent`

## Thesis

The agent runtime surface is the first concrete implementation boundary for the agent domain.

It must make agent state durable, make runtime activation resumable, make subscription delivery idempotent, and make goal curation deterministic over explicit inputs.

The first slice does not need dynamic spawned agents. It does need the same runtime shape that spawned agents will later use.

## Owned Runtime Parts

`world_model/agent` exposes these first-slice parts:

- `AgentStore`
  durable writes for agent records, subscriptions, activation state, cursors, and curation decisions
- `AgentQuery`
  read facade for agent records, subscriptions, status, cursors, and recent curation decisions
- `AgentRuntime`
  process runner that activates existing records and dispatches watched belief revisions to curation
- `AgentCuration`
  pure decision module that turns one agent record, one belief view, projected world state, and active goal summary into goal commands
- `AgentRegistration`
  command surface for seed registration and later spawned agent registration
- `AgentSubscription`
  command surface for binding belief keys and advancing delivery cursors

Deferred runtime part:

- `AgentRuntime`
  process runner that activates existing records and dispatches watched belief revisions to curation

`AgentStore` owns durable agent state.
`AgentRuntime` owns only live process handles.
`AgentCuration` must be replayable without live handles.

## Durable Records

### AgentRecord

`AgentRecord` is the durable identity and policy record.

Required fields for the first slice:

- `agent_id`
- `perspective_key`
- `subject`
- `branch_scope`
- `observation_scope`
- `directive`
- `seed_provenance`
- `status`
- `created_at_seq`
- `updated_at_seq`

Deferred fields for the full design:

- `trust_policy_ref`
- `evidence_policy_ref`
- `spawn_policy_ref`
- `curator_agent_id`
- `parent_goal_id`
- `activation_policy`
- `calibration_profile_ref`

### AgentSubscriptionRecord

`AgentSubscriptionRecord` binds an agent to one watched belief key.

Required fields:

- `subscription_id`
- `agent_id`
- `belief_key`
- `status`
- `last_delivered_revision_id`
- `last_delivered_seq`
- `created_at_seq`
- `updated_at_seq`

The subscription is durable because restart activation must resume from the last delivered belief revision.

### AgentActivationRecord

`AgentActivationRecord` records runtime hydration attempts.

Required fields:

- `activation_id`
- `agent_id`
- `started_at_seq`
- `status`
- `last_error`
- `lease_id`

Activation records are diagnostic and replay aids. They are not the source of truth for agent existence.

### AgentCurationDecision

`AgentCurationDecision` records one deterministic curation evaluation.

Required fields:

- `decision_id`
- `agent_id`
- `subscription_id`
- `belief_revision_id`
- `belief_key`
- `projection_version`
- `decision`
- `goal_command_id`
- `dedupe_key`
- `input_refs`
- `created_at_seq`

The decision record prevents duplicate goal commands when a revision is redelivered after restart.

## Command Surface

Agent commands mutate durable agent state.

Commands should be idempotent by stable command id or natural key.

```rust
register_seed_agent(request: SeedAgentRegistration) -> AgentRecord
register_agent(request: AgentRegistrationRequest) -> AgentRecord
activate_agent(command: ActivateAgentCommand) -> AgentStatus
deactivate_agent(command: DeactivateAgentCommand) -> AgentStatus
subscribe(command: SubscribeAgentCommand) -> AgentSubscriptionRecord
unsubscribe(command: UnsubscribeAgentCommand) -> AgentSubscriptionRecord
advance_subscription(command: AdvanceSubscriptionCommand) -> AgentSubscriptionRecord
record_curation_decision(command: RecordCurationDecisionCommand) -> AgentCurationDecision
```

Seed registration is allowed without a parent curator.
Non seed registration requires curator provenance in the full design.

## Query Surface

Agent queries are read only.

```rust
agent(agent_id: AgentId) -> Option<AgentRecord>
agents_by_status(status: AgentStatus) -> Vec<AgentRecord>
subscriptions(agent_id: AgentId) -> Vec<AgentSubscriptionRecord>
subscription(subscription_id: SubscriptionId) -> Option<AgentSubscriptionRecord>
pending_subscriptions(agent_id: AgentId) -> Vec<AgentSubscriptionRecord>
recent_decisions(agent_id: AgentId, limit: usize) -> Vec<AgentCurationDecision>
decision_by_dedupe_key(dedupe_key: AgentCurationDedupeKey) -> Option<AgentCurationDecision>
```

Execution capabilities should use the public interface. Internal runtime code may use the query facade directly.

## Event Intake

The agent runtime consumes belief revision notifications.

The first implementation may poll durable belief views through `BeliefQuery`. Later implementations may subscribe to the event spine.

Each delivery must include:

- `agent_id`
- `subscription_id`
- `belief_key`
- `belief_revision_id`
- `revision_seq`

Delivery is at least once. The runtime must tolerate duplicate deliveries.

The handler must check the stored subscription cursor before running curation. If the revision was already delivered, the handler returns without creating another decision.

## Curation Runtime

The first curation rule is deterministic and receives its dimension, threshold, priority, desired summary, and source kind from runtime rule configuration.

Input:

- seed agent record
- watched belief view
- planner projection for the agent perspective
- active goal summary from execution

Rule:

```text
if configured belief confidence is below configured threshold
and no active matching goal exists
then emit a proposed goal command requiring configured dimension confidence above configured threshold
else absorb the revision
```

Output:

- `AgentCurationDecision`
- optional goal command for execution
- advanced subscription cursor

The curation rule must not write execution goal state directly. The first slice emits producer curation output, currently `AgentGoalCommand`, that maps into execution's neutral Goal Set API outside execution core.

## Goal Command Dedupe

The dedupe key for the first slice includes:

- `agent_id`
- `subject`
- `branch_scope`
- `dimension_id`
- `target_condition`
- `goal_source_kind`

For the first configured threshold rule, the key identifies one goal that requires configured confidence above the configured threshold for one subject and one agent perspective.

If execution already has an active matching goal, the agent records an absorbed decision and does not emit another command.

If a curation decision with the same dedupe key and belief revision already exists, the runtime does not emit another command.

## Replay Rules

Agent replay must be deterministic over stored inputs.

Replay order:

1. Load `AgentRecord`.
2. Load active `AgentSubscriptionRecord` values.
3. Read belief revisions after each subscription cursor.
4. Rebuild planner projection for each delivered revision.
5. Read active goal summary for the decision point.
6. Recompute `AgentCurationDecision`.
7. Compare with stored decision or store a missing decision.
8. Advance cursor only after decision persistence succeeds.

No live watcher handle may be required to replay curation.

## First Slice Requirements

The first slice implements:

- one seed agent registration path
- one durable agent record
- one subscription record
- one activation status
- one cursor over watched belief revisions
- one deterministic threshold curation rule supplied by runtime configuration
- one goal command dedupe key
- one decision record
- one query facade

It defers:

- spawned agent authorization
- multi agent conflict handling
- learned cost benefit comparator
- restart activation workers
- full `AgentRuntime` process worker
- event spine push delivery
- curation over multiple belief dimensions

## Read With

- [World Model Agent](README.md)
- [Agent Genesis And Activation](genesis_and_activation.md)
- [Goal Curation](goal_curation.md)
- [World Model Public Interface](../public_interface.md)
- [Goals](../../execution/goals/README.md)
