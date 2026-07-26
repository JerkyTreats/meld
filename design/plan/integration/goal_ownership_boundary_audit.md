# Goal Ownership Boundary Audit

Date: 2026-06-13
Status: assessed; code follow-ups closed 2026-07-25

Closure note 2026-07-25, from an assembly code survey: every code gap this audit named is now closed in the audit's recommended direction. The boundary command exists as `AgentGoalMutationCommand` with satisfy and reopen kinds; only the owning agent's goals are curated for satisfaction; `review_seq` is first-class with a durable satisfaction checkpoint; the adapter maps the agent command into execution's satisfy API; and planning reads goals through the active-goal query only, reporting `PlanningResult::Satisfied` as a diagnostic without mutating lifecycle. The remaining follow-ups are documentation debts in the older design texts that still describe planning-loop satisfaction authority.

## Audit Question

Does the design space firmly define goal ownership as a world model agent concern, and does it clearly express how that ownership crosses the boundary into execution goals.

## Verdict

The design intent is mostly firm but not cleanly enforced across documents.

The strongest design sources define the world model agent as the owner of normative goal intent and goal set curation. Execution owns the goal set store, lifecycle command enforcement, planning, task decomposition, dispatch, and cleanup. The implemented add-goal path expresses this boundary through `AgentGoalCommand`, integration mapping, and producer-neutral `GoalSetApi.accept_goal`.

The satisfaction path is not yet firm. Several documents still say the planning loop sets `Satisfied` directly when `meld_lang::evaluate` returns satisfied. That conflicts with the stronger ownership model where the world model agent detects satisfaction from belief revision and calls execution's satisfy API.

## Ownership Model

| Concern | Owner | Evidence | Assessment |
| --- | --- | --- | --- |
| Normative judgment over belief | `world_model/agent` | [World Model Agent](../../cognitive_architecture/world_model/agent/README.md), [Goal Curation](../../cognitive_architecture/world_model/agent/goal_curation.md) | firm |
| Goal construction | `world_model/agent` | [agent curation implementation](../../../crates/meld-world-model/src/agent/curation.rs), [agent tests](../../../crates/meld-world-model/tests/agent.rs) | implemented for first slice |
| Execution goal set storage | `execution` | [Goals](../../cognitive_architecture/execution/goals/README.md), [goal contracts](../../../crates/meld-execution/src/goals/contracts.rs), [goal store](../../../crates/meld-execution/src/goals/store.rs) | firm |
| Add-goal boundary | integration boundary and `execution` goal store | [goal API](../../../crates/meld-execution/src/goals/api.rs), [goal acceptance test](../../../tests/integration/goal_acceptance.rs) | implemented |
| Satisfaction decision | intended owner is `world_model/agent` | [Goals](../../cognitive_architecture/execution/goals/README.md), [World Model Agent](../../cognitive_architecture/world_model/agent/README.md) | conceptually present, inconsistently documented |
| Satisfaction storage transition | `execution` | [goal contracts](../../../crates/meld-execution/src/goals/contracts.rs), [persistent goal store](../../../crates/meld-execution/src/goals/persistent_store.rs) | implemented as a raw command |
| Satisfaction boundary from agent to execution | shared boundary | [NAG requirements](non_assembly_gap_requirements.md) | missing |

## Firm Boundary Evidence

The world model agent design says the agent owns perspective and normative judgment, constructs `Goal` values, and curates execution's goal set through public API operations including add, modify, remove, satisfy, suspend, and resume.

The execution goals design says execution owns the goal set as data, but does not decide which goals should exist. It also says the world model agent is the active entity in the belief to goal bridge and the only entity that needs to span belief and execution goal curation.

The implemented first slice follows that model for add-goal:

- `world_model/agent` emits `AgentGoalCommand` with a proposed ground `Goal`.
- `AgentGoalCommand::validate` enforces producer dedupe invariants.
- Integration maps producer output to `GoalAcceptanceRequest`.
- `GoalSetApi.accept_goal` validates the neutral request and converts `GoalLifecycle::Proposed` to `GoalLifecycle::Active`.
- Execution persists the goal through `AddGoalCommand`.
- The integration test proves replay and duplicate delivery are idempotent.

That is a clear crossing from world model agent ownership of intent to execution ownership of operational state.

## Inconsistent Boundary Evidence

The satisfaction path is split across two incompatible claims.

One claim says the world model agent satisfies goals. The execution goals design says the agent detects satisfaction through belief revision and calls the satisfy API. The belief family trace also ends with agent satisfaction.

The other claim says the planning loop satisfies goals. The execution goals lifecycle section says the planning loop sets `Satisfied` after `evaluate`. The goal curation design says the planning loop transitions lifecycle to `Satisfied`. The meld language design rules repeat that framing. The execution planning flow diagram routes a planning evaluation directly to mark goal satisfied.

The current NAG-4 assessment follows the second claim and assigns satisfaction review ownership to execution. That is the wrong owner if the agent is the owner of goal intent and satisfaction judgment.

## Boundary Gaps

The add-goal path has a concrete boundary object. The satisfaction path does not.

Current execution has `SatisfyGoalCommand`, but this is a raw execution store command. There is no agent-authored boundary object equivalent to `AgentGoalCommand`.

Missing boundary pieces:

- agent curation output for satisfaction, such as `AgentGoalSatisfactionCommand` or a generic `AgentGoalMutationCommand`
- adapter from agent satisfaction command to `SatisfyGoalCommand`
- validation that the command comes from the owning agent or an authorized agent
- stable idempotency identity derived from goal id and reviewed cursor
- evidence or projection provenance carried across the boundary
- focused test proving that the agent detects satisfaction and execution only records it

## Recommended Interpretation

Goal ownership should be stated as three separate authorities.

The world model agent owns goal intent, satisfaction judgment, and curation decisions.

Execution owns the goal set as durable operational state and enforces lifecycle command validity.

`meld-lang` owns the shared representation and pure evaluation function. It does not own lifecycle transitions.

Under this interpretation, the NAG-4 entity is not an execution-owned satisfaction reviewer. It is a world model agent satisfaction curation path that evaluates projected world state or belief view state, then calls execution's public satisfy API. Execution validates and persists the lifecycle transition.

The planning loop may still evaluate active goals mechanically so it can avoid unnecessary work and clean up task network state. It should not be the authoritative satisfaction decision-maker. If planning observes a satisfied goal before the agent command arrives, it should report or expose that fact rather than directly mutating lifecycle.

## Required Follow Ups

| Follow Up | Owner | Purpose |
| --- | --- | --- |
| Correct NAG-4 ownership | integration docs | Reframe NAG-4 around agent satisfaction curation plus execution lifecycle command enforcement |
| Add satisfaction boundary command | `world_model/agent` and `execution` | Make satisfaction crossing as explicit as `AgentGoalCommand` |
| Update execution goals design | design docs | Remove direct planning-loop satisfaction authority and make planning cleanup reactive to goal set changes |
| Update goal curation design | design docs | Say satisfaction checking follows the agent watching pattern and uses execution's public satisfy API |
| Update meld language design rules | design docs | Keep `evaluate` pure and remove lifecycle authority from language docs |
| Add focused proof | integration tests | Prove agent-detected satisfaction issues one idempotent execution satisfy command |

## Bottom Line

The architecture already contains the intended ownership model, but it is not yet firm enough for implementation. Add-goal ownership and crossing are clear and implemented. Satisfaction ownership and crossing need design cleanup and a boundary command before NAG-4 should be implemented.
