# World Model Agent Phased Implementation Plan

Date: 2026-05-30
Status: complete
Scope: first seed agent curation slice for `docs_freshness`

## Overview

This plan converts the Phase 5 agent assessment into one implementation order.

The first slice proves the first normative handoff after planner projection. It creates one seed agent from trusted init or configuration, binds one perspective and one watched belief key, reads the projected world state, and emits one `AgentGoalCommand` with a ground `meld-lang::Goal` when `docs_freshness` confidence is below `0.7`.

The order is dependency driven:

- Define the seed agent contract and runtime boundary
- Add an agent domain entry under `meld-world-model`
- Add durable agent, subscription, activation, and curation decision records
- Add `AgentStore` and `AgentQuery`
- Add seed registration and subscription binding commands
- Add a first curation rule for `docs_freshness`
- Add goal command dedupe by agent, subject, branch, dimension, condition, and source kind
- Add subscription cursor advancement after durable decision persistence
- Add a first execution goal command handoff contract
- Prove replay and duplicate delivery behavior through focused tests

Related specs:

- [World Model Domain](../../../cognitive_architecture/world_model/README.md)
- [World Model Agent](../../../cognitive_architecture/world_model/agent/README.md)
- [Agent Spec](../../../cognitive_architecture/world_model/agent/spec.md)
- [Agent Genesis And Activation](../../../cognitive_architecture/world_model/agent/genesis_and_activation.md)
- [Agent Runtime Surface](../../../cognitive_architecture/world_model/agent/runtime_surface.md)
- [Goal Curation](../../../cognitive_architecture/world_model/agent/goal_curation.md)
- [World Model Public Interface](../../../cognitive_architecture/world_model/public_interface.md)
- [World Model Planner](../../../cognitive_architecture/world_model/planner/README.md)
- [Execution Goals](../../../cognitive_architecture/execution/goals/README.md)
- [Meld Lang Goals And Methods](../../../cognitive_architecture/meld-lang/goals_and_methods.md)

Related plan docs:

- [Cognitive Architecture Implementation Plan](../../README.md)
- [Agent Readiness Assessment](assessment.md)
- [Planner Projection Assessment](../planner/assessment.md)
- [Belief Assessment](../belief/assessment.md)
- [Execution Goals Assessment](../../execution/goals/assessment.md)
- [Typed Loop Integration](../../integration/typed_loop.md)
- [Meld Lang Assessment](../../meld-lang/assessment.md)

## Implementation Status

Evidence date: 2026-05-30

The first agent curation slice is implemented.

Implemented runtime pieces:

- `world_model/agent` module entry and child modules under `crates/meld-world-model/src/agent/`
- seed agent registration command with `docs_freshness` supplied by test fixture configuration
- durable `AgentRecord`, `AgentSubscriptionRecord`, `AgentActivationRecord`, and `AgentCurationDecision`
- `AgentStore` over sled trees for agents, subscriptions, activations, decisions, and indexes
- `AgentQuery` read facade for agents, subscriptions, pending subscriptions, and recent decisions
- idempotent subscription binding by agent and belief key
- subscription cursor advancement with cursor regression rejection
- `AgentCuration` delivery handler using public `BeliefQuery` and `PlannerQuery`
- pure threshold curation rule over runtime `AgentCurationRuleConfig`
- `AgentGoalCommand` handoff with a ground `meld-lang::Goal`
- active goal summary duplicate suppression through configured dedupe keys
- decision dedupe by agent, subject, branch, dimension, target condition, source kind, and belief revision
- replayable deterministic curation over stored inputs
- fuzz target `fuzz_agent_contracts`

Available lower-layer inputs:

- `meld-lang` `Goal`, `GoalPriority`, `GoalSource`, `GoalLifecycle`, `Proposition`, `Condition`, `Term`, and `CostEstimate`
- `meld-lang` groundness and evaluation behavior
- `BeliefQuery` for current views, subject views, revision history, evidence hydration, provenance, and dirty keys
- First externally configured `docs_freshness` belief slice
- `PlannerQuery::project_current_world_state`
- Planner projection output with `WorldState`, source refs, hydration refs, warnings, and projection version
- Graph traversal query surfaces

Existing `meld-world-model` test harness:

- temp sled stores through `tempfile`
- reopen tests for durable graph, belief, world state, and planner query state
- serde round trip tests for public records
- deterministic ordering tests for query and projection output
- source scans for domain boundary and `mod.rs` rules
- proptest coverage for belief configuration and planner projection
- fuzz targets for belief config, graph walk, planner projection contracts, and agent contracts
- focused crate tests in `crates/meld-world-model/tests`
- workspace verification through check, test, clippy, and fuzz harness builds

Current crate verification evidence:

- `cargo check -p meld-world-model` passes.
- `cargo test -p meld-world-model agent` passes.
- `cargo test -p meld-world-model` passes with 127 total tests across unit tests, integration tests, and doc tests.
- `cargo clippy -p meld-world-model -- -D warnings` passes.
- `cargo check --manifest-path crates/meld-world-model/fuzz/Cargo.toml` passes.
- `cargo test --workspace` passes.
- `cargo clippy --workspace -- -D warnings` passes.
- `cargo +nightly fuzz run fuzz_agent_contracts -- -runs=1` passes.
- `cargo mutants --package meld-world-model --file crates/meld-world-model/src/agent.rs --file crates/meld-world-model/src/agent --test-tool cargo --timeout 120` found zero mutants under the requested filters.

Deferred:

- dynamic spawned agents through curated `CreateAgent` goals
- spawn authorization
- existing agent activation workers across process restart
- full `AgentRuntime` process worker
- multi agent conflict handling
- learned cost benefit comparator
- regime scoped prior switching
- curation over multiple belief dimensions
- event spine push delivery to agents

## Guiding Rules

| Rule | Statement |
|------|-----------|
| Seed first | The first agent is trusted seed state from init or configuration. |
| No self creation | The new agent is the output of initialization, not the actor that runs it. |
| Activation separate | Restart activation hydrates existing records and is not agent creation. |
| One curation rule | The first rule watches `docs_freshness` confidence only. |
| Ground goal | Every emitted goal target must be ground. |
| Public inputs only | Agent curation consumes `BeliefView`, `PlannerProjectionOutput`, agent records, and active goal summaries. |
| No belief internals | Agent code must not read raw evidence stores, leases, comparator drafts, or reducer state. |
| No execution ownership | Agent emits goal commands but does not own the goal store or lifecycle state. |
| Idempotent delivery | Duplicate belief revision delivery must not duplicate curation decisions or goal commands. |
| Cursor after decision | Advance subscription cursor only after decision persistence succeeds. |
| Replayable curation | Same stored inputs must reproduce the same curation decision. |
| Modern modules | Use `agent.rs` and `agent/*.rs`. Do not add `mod.rs`. |

## Quality Bar

Agent implementation must match the current `meld-world-model` crate bar.

Required verification families:

- Contract tests for serde round trip, required field validation, status validation, and dedupe key construction.
- Boundary tests proving agent code imports no execution internals and writes no goal lifecycle state directly.
- Source scans proving no `crates/meld-world-model/src/agent/mod.rs` exists and callers do not import private agent store modules.
- Grounding tests proving every emitted goal target passes `Proposition::is_ground`.
- Determinism tests proving identical curation inputs produce equal decisions, dedupe keys, goal commands, and cursor updates.
- Store tests using temp sled databases for agent records, subscriptions, activation records, curation decisions, and indexes.
- Query facade tests proving deterministic ordering for agent lists, subscription lists, pending deliveries, and recent decisions.
- Reopen tests proving agent records, subscription cursors, and curation decisions survive database close and reopen.
- Idempotency tests proving duplicate seed registration, duplicate subscription binding, duplicate revision delivery, and duplicate decision recording do not create duplicate records or goal commands.
- Cursor ordering tests proving cursor regression is rejected and cursor advancement happens only after decision persistence.
- Handoff tests proving low confidence `docs_freshness` emits one execution goal command and high confidence or matching active goal input emits an absorbed decision.
- Replay tests proving stored inputs can recompute the same curation decision without live watcher handles.
- Missing input tests proving absent belief or projection warnings produce typed absorbed or indeterminate decisions, not fabricated goals.
- Proptest coverage for seed ids, perspective ids, branch ids, dimension ids, confidence values around the threshold, duplicate delivery order, and dedupe key construction.
- Fuzz coverage for agent record JSON, subscription record JSON, curation decision JSON, and malformed dedupe key inputs.
- Mutation testing for the curation rule, dedupe logic, cursor advancement, and record validation before the first slice is considered complete.
- Full crate and workspace verification before the slice is considered complete.

Required verification commands:

```sh
cargo check -p meld-world-model
cargo test -p meld-world-model
cargo test --workspace
cargo clippy -p meld-world-model -- -D warnings
cargo clippy --workspace -- -D warnings
cargo check --manifest-path crates/meld-world-model/fuzz/Cargo.toml
cargo +nightly fuzz run fuzz_agent_contracts -- -runs=1
```

Mutation testing target:

```sh
cargo mutants --package meld-world-model --file crates/meld-world-model/src/agent.rs --file crates/meld-world-model/src/agent --test-tool cargo --timeout 120
```

## First Slice Contract

| Contract | First value |
|----------|-------------|
| Agent source | trusted seed config or init fixture |
| Agent id | runtime seed configuration value, with `seed.docs_freshness` used by tests |
| Subject | `DomainObjectRef` for a documented workspace node |
| Perspective | explicit perspective key carried through agent, belief, and planner |
| Branch scope | explicit branch scope with `main` as initial default |
| Watched belief | runtime configured belief key, with `docs_freshness` used by tests |
| Curation trigger | delivered belief revision for the watched key |
| Planner input | `PlannerQuery::project_current_world_state` |
| Active goal input | active goal summary filtered by agent, subject, branch, dimension, and target |
| Goal target | configured dimension confidence above configured threshold |
| Goal source | `GoalSource::BeliefDivergence` |
| Goal lifecycle | proposed command |
| Dedupe key | agent, subject, branch, dimension, target condition, source kind |
| Required persistence | agent record, subscription record, curation decision, cursor update |

## Development Phases

| Phase | Goal | Dependencies | Status |
|-------|------|--------------|--------|
| 0 | Scope lock and contract inventory | Planner, belief, execution goals, and meld-lang assessments | Complete |
| 1 | Module scaffold and public boundary | Phase 0 | Complete |
| 2 | Durable record contracts | Phase 1 | Complete |
| 3 | Store and query facade | Phase 2 | Complete |
| 4 | Seed registration and subscription binding | Phase 3 | Complete |
| 5 | Curation input assembly | Phase 4 | Complete |
| 6 | First curation rule and goal command | Phase 5 | Complete |
| 7 | Idempotency, cursor advancement, and replay | Phase 6 | Complete |
| 8 | Public interface integration | Phase 7 | Complete |
| 9 | End to end typed loop handoff tests | Phase 8 | Complete |

---

## Phase 0 -- Scope Lock And Contract Inventory

| Field | Value |
|-------|-------|
| Goal | Freeze the exact seed agent runtime surface for the first `docs_freshness` curation slice. |
| Dependencies | Planner projection complete, belief first slice complete, execution goals design ready, meld-lang complete |
| Docs | [Agent Runtime Surface](../../../cognitive_architecture/world_model/agent/runtime_surface.md), [Agent Genesis And Activation](../../../cognitive_architecture/world_model/agent/genesis_and_activation.md), [Agent Readiness Assessment](assessment.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Name the seed agent id, perspective key, subject input, branch scope, and watched dimension used by the first slice. | Complete |
| 2 | Define the seed configuration or init fixture shape. | Complete |
| 3 | Define the active goal summary input required for duplicate goal avoidance. | Complete |
| 4 | Define the exact execution goal command shape emitted by agent curation. | Complete |
| 5 | Confirm the curation rule emits only ground `Goal` values. | Complete |
| 6 | Record all deferred full design concerns before code starts. | Complete |

Test Suite Expansion:

- Add a contract test that names the seed agent fixture and confirms all required fields are present.
- Add a handoff fixture test that proves planner projection output can supply the goal target dimension.
- Add a source scan that fails if agent code imports execution internals instead of an execution goal command contract.
- Add a repository structure test that fails if `crates/meld-world-model/src/agent/mod.rs` exists.

| Exit Criterion | Status |
|----------------|--------|
| The seed agent identity and watched belief are concrete. | Complete |
| The execution goal command boundary is concrete enough to implement without execution internals. | Complete |
| Deferred concerns are explicit and do not leak into first-slice tasks. | Complete |

Verification:

- `cargo check -p meld-world-model`
- `cargo test -p meld-world-model`

Key files:

- `design/plan/world_model/agent/PLAN.md`
- `design/plan/world_model/agent/assessment.md`
- `design/cognitive_architecture/world_model/agent/runtime_surface.md`

---

## Phase 1 -- Module Scaffold And Public Boundary

| Field | Value |
|-------|-------|
| Goal | Add an agent domain under `meld-world-model` without disturbing graph, belief, or planner public imports. |
| Dependencies | Phase 0 |
| Docs | [World Model Agent](../../../cognitive_architecture/world_model/agent/README.md), [Agent Spec](../../../cognitive_architecture/world_model/agent/spec.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add `crates/meld-world-model/src/agent.rs` as the public domain entry. | Complete |
| 2 | Add `crates/meld-world-model/src/agent/contracts.rs` for serializable public record and command types. | Complete |
| 3 | Add `crates/meld-world-model/src/agent/store.rs` for durable records. | Complete |
| 4 | Add `crates/meld-world-model/src/agent/query.rs` for read facade behavior. | Complete |
| 5 | Add `crates/meld-world-model/src/agent/registration.rs` for seed and later spawned registration commands. | Complete |
| 6 | Add `crates/meld-world-model/src/agent/subscription.rs` for subscription binding and cursor rules. | Complete |
| 7 | Add `crates/meld-world-model/src/agent/curation.rs` for pure curation decisions. | Complete |
| 8 | Add `crates/meld-world-model/fuzz/fuzz_targets/fuzz_agent_contracts.rs` and manifest entry. | Complete |
| 9 | Re-export only public agent contracts from `crates/meld-world-model/src/lib.rs`. | Complete |

Test Suite Expansion:

- Add a compile-facing smoke test that imports public agent records and commands through `meld_world_model`.
- Add a module-boundary test that graph, belief, and planner public imports still compile.
- Add a no `mod.rs` structure test for the agent domain.
- Add a fuzz harness build check for the new agent contract target.

| Exit Criterion | Status |
|----------------|--------|
| `cargo check -p meld-world-model` succeeds with the scaffold. | Complete |
| No `mod.rs` file is added. | Complete |
| Existing world model public imports keep compiling. | Complete |

Verification:

- `cargo check -p meld-world-model`
- `cargo test -p meld-world-model`
- `cargo clippy -p meld-world-model -- -D warnings`

---

## Phase 2 -- Durable Record Contracts

| Field | Value |
|-------|-------|
| Goal | Define durable agent state without implementing curation behavior. |
| Dependencies | Phase 1 |
| Docs | [Agent Runtime Surface](../../../cognitive_architecture/world_model/agent/runtime_surface.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Define `AgentRecord` with seed id, perspective, subject, branch scope, observation scope, directive, provenance, status, and sequence fields. | Complete |
| 2 | Define `AgentSubscriptionRecord` with belief key, status, last delivered revision, last delivered sequence, and sequence fields. | Complete |
| 3 | Define `AgentActivationRecord` for diagnostic hydration attempts. | Complete |
| 4 | Define `AgentCurationDecision` with input refs, decision, goal command id, dedupe key, and projection version. | Complete |
| 5 | Define `AgentStatus`, `AgentSubscriptionStatus`, and `AgentDecisionKind`. | Complete |
| 6 | Add serde round trip tests and validation tests for required ids. | Complete |

Test Suite Expansion:

- Add round trip tests for every public record.
- Add validation tests for missing or malformed ids.
- Add status transition tests for first-slice allowed states.
- Add groundness tests for goal-bearing curation decisions.

| Exit Criterion | Status |
|----------------|--------|
| All durable record types are serializable and validated. | Complete |
| First-slice records can represent one seed agent, one subscription, and one curation decision. | Complete |

Verification:

- `cargo test -p meld-world-model agent`

---

## Phase 3 -- Store And Query Facade

| Field | Value |
|-------|-------|
| Goal | Persist and read durable agent records through a narrow facade. |
| Dependencies | Phase 2 |
| Docs | [Agent Runtime Surface](../../../cognitive_architecture/world_model/agent/runtime_surface.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Add `AgentStore` over the crate storage substrate used by graph and belief. | Complete |
| 2 | Store agent records by agent id. | Complete |
| 3 | Store subscriptions by subscription id and agent id index. | Complete |
| 4 | Store curation decisions by decision id and dedupe key index. | Complete |
| 5 | Store activation records by activation id and agent id index. | Complete |
| 6 | Add `AgentQuery` for records, subscriptions, pending subscriptions, recent decisions, and decision lookup by dedupe key. | Complete |
| 7 | Add reopen tests proving records survive database close and reopen. | Complete |

Test Suite Expansion:

- Add put and get tests for each record family.
- Add index tests for subscriptions and decisions.
- Add reopen tests for all first-slice records.
- Add deterministic ordering tests for query lists.

| Exit Criterion | Status |
|----------------|--------|
| Agent state has durable storage and read facades. | Complete |
| Query output order is deterministic. | Complete |
| Reopen tests pass. | Complete |

Verification:

- `cargo test -p meld-world-model agent_store`

---

## Phase 4 -- Seed Registration And Subscription Binding

| Field | Value |
|-------|-------|
| Goal | Create one seed agent and bind it to one watched belief key. |
| Dependencies | Phase 3 |
| Docs | [Agent Genesis And Activation](../../../cognitive_architecture/world_model/agent/genesis_and_activation.md), [World Model Public Interface](../../../cognitive_architecture/world_model/public_interface.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Define `SeedAgentRegistration` command with seed provenance. | Complete |
| 2 | Implement idempotent seed registration by agent id. | Complete |
| 3 | Define `SubscribeAgentCommand` with agent id and belief key. | Complete |
| 4 | Implement idempotent subscription binding by agent id and belief key. | Complete |
| 5 | Mark the seed agent operational only after required setup state exists. | Complete |
| 6 | Add tests for duplicate seed registration and duplicate subscription binding. | Complete |

Test Suite Expansion:

- Add seed registration success test.
- Add duplicate seed registration idempotency test.
- Add subscription binding success test.
- Add duplicate subscription idempotency test.
- Add missing agent rejection test for subscription binding.

| Exit Criterion | Status |
|----------------|--------|
| One seed agent can be registered durably. | Complete |
| One subscription can be bound durably. | Complete |
| Duplicate setup commands do not create duplicate records. | Complete |

Verification:

- `cargo test -p meld-world-model agent_registration`
- `cargo test -p meld-world-model agent_subscription`

---

## Phase 5 -- Curation Input Assembly

| Field | Value |
|-------|-------|
| Goal | Gather only public inputs required for the first curation rule. |
| Dependencies | Phase 4 |
| Docs | [World Model Planner](../../../cognitive_architecture/world_model/planner/README.md), [World Model Public Interface](../../../cognitive_architecture/world_model/public_interface.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Define `AgentCurationInput` with agent record, subscription record, belief view, planner projection, active goal summary, and input refs. | Complete |
| 2 | Read the current watched `BeliefView` through `BeliefQuery`. | Complete |
| 3 | Read the planner projection through `PlannerQuery`. | Complete |
| 4 | Define a minimal active goal summary contract supplied by execution. | Complete |
| 5 | Reject curation input when subject, perspective, or branch scope does not match the agent record. | Complete |
| 6 | Preserve projection version and source refs in input refs. | Complete |

Test Suite Expansion:

- Add input assembly test from seeded belief and planner stores.
- Add mismatch tests for subject, perspective, and branch scope.
- Add missing belief test that records an absorbed or indeterminate decision rather than fabricating a goal.

| Exit Criterion | Status |
|----------------|--------|
| Curation input can be assembled without reading lower domain internals. | Complete |
| Input refs are sufficient for replay and explanation. | Complete |

Verification:

- `cargo test -p meld-world-model agent_curation_input`

---

## Phase 6 -- First Curation Rule And Goal Command

| Field | Value |
|-------|-------|
| Goal | Emit a goal command when `docs_freshness` confidence is below `0.7`. |
| Dependencies | Phase 5 |
| Docs | [Goal Curation](../../../cognitive_architecture/world_model/agent/goal_curation.md), [Execution Goals](../../../cognitive_architecture/execution/goals/README.md), [Meld Lang Goals And Methods](../../../cognitive_architecture/meld-lang/goals_and_methods.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Define `AgentGoalCommand` as the execution-facing output of curation. | Complete |
| 2 | Define `AgentCurationDedupeKey` for first-slice duplicate avoidance. | Complete |
| 3 | Implement the pure configured threshold rule. | Complete |
| 4 | Construct a ground `Goal` target requiring confidence above `0.7`. | Complete |
| 5 | Set `GoalSource::BeliefDivergence` with dimension, observed, and desired summaries. | Complete |
| 6 | Set first-slice priority and cost ceiling from fixed seed policy. | Complete |
| 7 | Record absorbed decisions when confidence is sufficient or a matching active goal exists. | Complete |

Test Suite Expansion:

- Add low confidence emits goal command test.
- Add high confidence absorbed decision test.
- Add active matching goal absorbed decision test.
- Add ground target test for every emitted goal.
- Add source and priority tests for the emitted goal.

| Exit Criterion | Status |
|----------------|--------|
| The first curation rule emits one deterministic goal command when warranted. | Complete |
| The rule emits no duplicate command when an active matching goal exists. | Complete |

Verification:

- `cargo test -p meld-world-model agent_curation`

---

## Phase 7 -- Idempotency Cursor Advancement And Replay

| Field | Value |
|-------|-------|
| Goal | Make at least once belief delivery safe across duplicate delivery and restart. |
| Dependencies | Phase 6 |
| Docs | [Agent Runtime Surface](../../../cognitive_architecture/world_model/agent/runtime_surface.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Implement `record_curation_decision` as an idempotent durable write. | Complete |
| 2 | Implement decision lookup by dedupe key and belief revision id. | Complete |
| 3 | Implement `advance_subscription` after decision persistence. | Complete |
| 4 | Reject cursor regression. | Complete |
| 5 | Add duplicate delivery test that emits only one goal command. | Complete |
| 6 | Add replay test that recomputes the same curation decision from stored inputs. | Complete |
| 7 | Add reopen test that delivers the next revision after stored cursor. | Complete |

Test Suite Expansion:

- Add duplicate delivery idempotency test.
- Add cursor advancement ordering test.
- Add cursor regression rejection test.
- Add durable replay test.
- Add reopen resume test.

| Exit Criterion | Status |
|----------------|--------|
| Duplicate delivery cannot duplicate goal commands. | Complete |
| Cursor advances only after decision persistence. | Complete |
| Replay does not require live watcher handles. | Complete |

Verification:

- `cargo test -p meld-world-model agent_replay`

---

## Phase 8 -- Public Interface Integration

| Field | Value |
|-------|-------|
| Goal | Expose agent registration, subscription, cursor, status, and curation decision operations through the world model public interface. |
| Dependencies | Phase 7 |
| Docs | [World Model Public Interface](../../../cognitive_architecture/world_model/public_interface.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Wire seed registration through the public interface route. | Complete |
| 2 | Wire subscription binding through the public interface route. | Complete |
| 3 | Wire status query through the public interface route. | Complete |
| 4 | Wire curation decision recording through the public interface route. | Complete |
| 5 | Wire subscription cursor advancement through the public interface route. | Complete |
| 6 | Add boundary tests proving callers do not import agent store internals. | Complete |

Test Suite Expansion:

- Add public interface seed registration test.
- Add public interface subscription test.
- Add public interface status query test.
- Add public interface curation decision and cursor test.
- Add source scan for forbidden private module imports from callers.

| Exit Criterion | Status |
|----------------|--------|
| Execution-facing capabilities can use public interface routes for setup. | Complete |
| Private agent store types remain internal. | Complete |

Verification:

- `cargo test -p meld-world-model agent_public_interface`
- `cargo clippy -p meld-world-model -- -D warnings`

---

## Phase 9 -- End To End Typed Loop Handoff Tests

| Field | Value |
|-------|-------|
| Goal | Prove the seed agent creates the first ground goal from projected belief state. |
| Dependencies | Phase 8 |
| Docs | [Typed Loop Integration](../../integration/typed_loop.md), [Planner Projection Assessment](../planner/assessment.md) |
| Status | Complete |

| Order | Task | Status |
|-------|------|--------|
| 1 | Seed graph and belief stores with a docs subject and low confidence `docs_freshness` view. | Complete |
| 2 | Register the seed agent and bind its watched belief key. | Complete |
| 3 | Project current world state through planner query. | Complete |
| 4 | Run agent curation against an empty active goal summary. | Complete |
| 5 | Assert the emitted goal target evaluates as unsatisfied against the initial world state. | Complete |
| 6 | Apply a matching `meld-lang` effect and assert the goal evaluates as satisfied. | Complete |
| 7 | Re-deliver the same belief revision and assert no duplicate goal command is emitted. | Complete |

Test Suite Expansion:

- Add end to end seed agent curation test.
- Add duplicate delivery end to end test.
- Add indeterminate missing belief test.
- Add no `mod.rs` repository structure test.

| Exit Criterion | Status |
|----------------|--------|
| The seed agent produces one ground goal from low confidence belief. | Complete |
| The emitted goal works with `meld-lang::evaluate`. | Complete |
| Duplicate delivery remains idempotent. | Complete |

Verification:

- `cargo test -p meld-world-model`
- `cargo test -p meld-lang`
- `cargo test --workspace`
- `cargo clippy -p meld-world-model -- -D warnings`
- `cargo clippy --workspace -- -D warnings`

## Completion Criteria

The first agent slice is complete when:

- one seed agent is durably registered
- one watched belief key is durably subscribed
- one low confidence `docs_freshness` revision produces one curation decision
- one goal command with a ground target is emitted
- matching active goals suppress duplicate commands
- duplicate delivery suppresses duplicate commands
- subscription cursor advances only after decision persistence
- curation decisions replay deterministically
- public interface routes cover first-slice setup and state reads
- workspace tests and clippy pass

## Deferred Full Design Work

- `CreateAgent` goals curated by existing agents
- spawn authorization and delegation policy
- spawned agent satisfaction of its creation goal
- restart activation workers
- event spine push delivery
- multiple watched belief dimensions
- learned cost and value beliefs
- regime scoped cost benefit priors
- multi agent conflict handling
- broad goal set coordination
