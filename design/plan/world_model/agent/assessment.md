# Agent Readiness Assessment

Status: conditionally ready
Depends on: `design/plan/world_model/belief/assessment.md`, `design/plan/world_model/planner/assessment.md`, `design/plan/execution/goals/assessment.md`, `design/plan/meld-lang/assessment.md`
Design source: `design/cognitive_architecture/world_model/agent/README.md`, `design/cognitive_architecture/world_model/agent/goal_curation.md`, `design/cognitive_architecture/world_model/agent/ECS.md`, `design/cognitive_architecture/world_model/public_interface.md`, `design/cognitive_architecture/execution/goals/README.md`, `design/cognitive_architecture/meld-lang/goals_and_methods.md`
Evidence date: 2026-05-21

## Verdict Summary

Agent is conditionally ready for one agent, one perspective, one watched belief family, and one curation rule.

The first curation output is one ground `meld-lang::Goal`.

## Conceptual Correctness

Agent solves normative judgment over belief without moving goal storage or task execution into world model.

The boundary is correct: agent decides what should become true, expressed as a `Goal`; execution decides how to pursue it.

## Completeness

The first watched belief is `docs_freshness`.

The first curation rule is:

- if `docs_freshness` confidence is below `0.7`, create an active goal requiring confidence above `0.7`

The goal target must be ground. It must not contain `Term::Variable`.

## Boundary Clarity

Agent owns perspective, evidence policy, trust profile, observation scope, branch scope, regime sensitivity, normative judgment, planner-facing view assembly, and goal construction.

Agent does not own the goal set store, goal lifecycle state machine, task graphs, task decomposition, dispatch, continuation state, provider execution, or event append.

## Dependency Readiness

Belief is conditionally ready for `docs_freshness`.

Planner projection is conditionally ready for one `WorldState`.

Execution goals are conditionally ready for one goal set.

`meld-lang` is ready for `Goal` construction.

## First-Slice Feasibility

Agent supports the typed loop by constructing one active goal from the projected `docs_freshness` belief state.

No multi-agent coordination or learned policy is required.

## Current Implementation Evidence

- `src/agent.rs`
- `tests/integration/agent_cli.rs`
- `tests/integration/agent_authorization.rs`
- `design/cognitive_architecture/world_model/agent/README.md`
- `design/cognitive_architecture/world_model/agent/goal_curation.md`
- `design/cognitive_architecture/world_model/public_interface.md`
- `design/cognitive_architecture/execution/goals/README.md`
- `design/cognitive_architecture/meld-lang/goals_and_methods.md`

## Gaps

- Cost-benefit comparator factors remain outside the first slice.
- Learned normative policy remains outside the first slice.
- Subscription filter design needs concrete implementation shape.
- Multi-agent goal coordination is deferred.

## Open Questions

- Which agent identity names the first docs freshness curator.
- Which cost ceiling applies to the first `docs_freshness` goal.

## Recommendation

Proceed with one `docs_freshness` curation rule.
