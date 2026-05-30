# Agent Readiness Assessment

Status: implemented
Depends on: `design/plan/world_model/belief/assessment.md`, `design/plan/world_model/planner/assessment.md`, `design/plan/execution/goals/assessment.md`, `design/plan/meld-lang/assessment.md`
Design source: `design/cognitive_architecture/world_model/agent/README.md`, `design/cognitive_architecture/world_model/agent/goal_curation.md`, `design/cognitive_architecture/world_model/agent/spec.md`, `design/cognitive_architecture/world_model/public_interface.md`, `design/cognitive_architecture/execution/goals/README.md`, `design/cognitive_architecture/meld-lang/goals_and_methods.md`
Evidence date: 2026-05-30

## Verdict Summary

Agent design is implemented for one seed agent, one perspective, one watched belief family, and one curation rule.

The first curation output is one ground `meld-lang::Goal`.

Runtime world-model agent curation is implemented in `meld-world-model`.

Seed agent creation is now explicit as trusted init or configuration state. Dynamic spawned agent creation through curated `CreateAgent` goals remains deferred.

## Conceptual Correctness

Agent solves normative judgment over belief without moving goal storage or task execution into world model.

The boundary is correct: agent decides what should become true, expressed as a `Goal`; execution decides how to pursue it.

## Completeness

The first watched belief is supplied by runtime configuration. Tests use the externally configured `docs_freshness` dimension.

The implemented first curation rule is:

- if configured belief confidence is below the configured threshold, emit one proposed goal command requiring confidence above that threshold

The goal target must be ground. It must not contain `Term::Variable`.

## Boundary Clarity

Agent owns perspective, evidence policy, trust profile, observation scope, branch scope, regime sensitivity, normative judgment, planner-facing view assembly, and goal construction.

Agent does not own the goal set store, goal lifecycle state machine, task graphs, task decomposition, dispatch, continuation state, provider execution, or event append.

Execution owns the initialization workflow that builds an agent record, binds policy, registers belief keys, binds subscriptions, requests first observations, and verifies readiness.

## Dependency Readiness

Belief has landed the externally configured `docs_freshness` slice.

Planner projection is implemented for one `WorldState`.

Execution goal types are implemented in `meld-lang`.

`meld-lang` is implemented for `Goal` construction.

## First-Slice Feasibility

Agent supports the typed-loop design path by constructing one proposed goal command from projected belief state loaded from runtime configuration.

No multi-agent coordination or learned policy is required.

No dynamic agent spawning is required for the first slice.

## Current Implementation Evidence

- `design/cognitive_architecture/world_model/agent/README.md`
- `design/cognitive_architecture/world_model/agent/genesis_and_activation.md`
- `design/cognitive_architecture/world_model/agent/runtime_surface.md`
- `design/cognitive_architecture/world_model/agent/goal_curation.md`
- `design/cognitive_architecture/world_model/agent/spec.md`
- `design/cognitive_architecture/world_model/public_interface.md`
- `crates/meld-world-model/src/agent.rs`
- `crates/meld-world-model/src/agent/contracts.rs`
- `crates/meld-world-model/src/agent/store.rs`
- `crates/meld-world-model/src/agent/query.rs`
- `crates/meld-world-model/src/agent/registration.rs`
- `crates/meld-world-model/src/agent/subscription.rs`
- `crates/meld-world-model/src/agent/curation.rs`
- `crates/meld-world-model/tests/agent.rs`
- `crates/meld-world-model/fuzz/fuzz_targets/fuzz_agent_contracts.rs`
- `crates/meld-world-model/src/belief.rs`
- `crates/meld-world-model/src/belief/`
- `design/cognitive_architecture/execution/goals/README.md`
- `design/cognitive_architecture/meld-lang/goals_and_methods.md`

## Verification Evidence

- `cargo check -p meld-world-model` passes.
- `cargo test -p meld-world-model agent` passes.
- `cargo test -p meld-world-model` passes.
- `cargo clippy -p meld-world-model -- -D warnings` passes.
- `cargo check --manifest-path crates/meld-world-model/fuzz/Cargo.toml` passes.
- `cargo test --workspace` passes.
- `cargo clippy --workspace -- -D warnings` passes.
- `cargo +nightly fuzz run fuzz_agent_contracts -- -runs=1` passes.

## Remaining Gaps

- Execution goal store integration is still a handoff boundary. Agent emits `AgentGoalCommand` and does not write execution lifecycle state.
- Agent initialization capability chain does not exist yet.
- Production seed configuration source is still outside this slice.
- Existing agent runtime activation on process restart is deferred.
- Full `AgentRuntime` process worker is deferred.
- Cost-benefit comparator factors remain outside the first slice.
- Learned normative policy remains outside the first slice.
- Subscription filter evolution beyond one watched belief key remains deferred.
- Multi-agent goal coordination is deferred.
- Dynamic `CreateAgent` goal curation and spawn authorization are deferred.

## Open Questions

- Which cost ceiling applies to the first `docs_freshness` goal.
- Which seed config source should own production seed agent directive and scope beyond the test fixture.

## Recommendation

Proceed to execution planning runtime integration. Keep the agent slice narrow until the first end-to-end flywheel turn is complete.
