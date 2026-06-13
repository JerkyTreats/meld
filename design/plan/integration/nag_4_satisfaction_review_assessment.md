# NAG-4 Satisfaction Review Assessment

Date: 2026-06-09
Status: needs revision

## Correction Note

The later [goal ownership boundary audit](goal_ownership_boundary_audit.md) found that this assessment over-assigned satisfaction review ownership to execution. Treat this artifact as earlier gap evidence only until NAG-4 is reframed around world model agent satisfaction curation plus execution lifecycle command enforcement.

## Concern Definition

NAG-4 defines the domain contract that moves an execution goal from `Active` to `Satisfied` only after a current projected world state evaluates the goal target as satisfied through `meld_lang::evaluate`. The reviewer is a callable execution boundary after world model updates. It reads active execution goals, consumes caller supplied projected world state, evaluates each target mechanically, issues `SatisfyGoalCommand` only for `EvalResult::Satisfied`, and returns diagnostics for unsatisfied or indeterminate goals.

## In Scope

- Execution owned reviewer contract and result diagnostics.
- Active goal query through `GoalSetStore` and `PersistentGoalSetStore`.
- Goal target evaluation with `meld_lang::evaluate`.
- Lifecycle mutation through `SatisfyGoalCommand`.
- Idempotent command identity derived from goal id and review sequence.
- Reviewer input carrying a stable review `at_seq` derived from event cursor or world state cursor.

## Out Of Scope

- Runtime coordinator, worker loop, or flywheel service assembly.
- CLI command, API route, or user facing status formatting.
- Planner method selection or task network dispatch.
- World model ownership of execution lifecycle state.
- New belief semantics, projection field meanings, or graph reducers.
- NAG-5 failure outcome behavior.

## Domain Snapshot

Evidence date: 2026-06-09

Snapshot command:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

Snapshot output:

```text
agent
api
branches
capability
cli
compat
concurrency
config
context
control
error
events
execution
heads
ignore
init
lib
logging
merkle_traversal
metadata
prompt_context
provider
session
store
task
telemetry
types
views
workflow
workspace
world_state
```

## Domain Assessment

| Domain | Needed Integration | Current Integration | Completeness | Evidence | Non Integration Rationale | Follow Up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Agent curation emits goal commands and consumes active goal summaries, but it does not review satisfaction. | `not needed` | [agent curation plan](../world_model/agent/PLAN.md) | Satisfaction review is an execution lifecycle concern after world state projection. | none |
| `api` | `none` | No route is required for NAG-4. | `not needed` | [NAG requirements](non_assembly_gap_requirements.md) | The gap asks for a callable contract, not a route. | none |
| `branches` | `none` | Branch identity appears inside world model projection context. | `not needed` | [planner contracts](../../../crates/meld-world-model/src/planner/contracts.rs) | Branches do not decide goal lifecycle. | none |
| `capability` | `none` | Capability contracts are used by planning and task execution only. | `not needed` | [planning runtime](../../../crates/meld-execution/src/planning/runtime.rs) | A satisfied review path must not invoke capability work. | none |
| `cli` | `none` | No command surface exists for the reviewer. | `not needed` | [NAG requirements](non_assembly_gap_requirements.md) | CLI assembly is out of scope. | none |
| `compat` | `none` | No legacy compatibility path is involved. | `not needed` | [domain snapshot](#domain-snapshot) | NAG-4 adds a new contract and does not migrate old lifecycle behavior. | none |
| `concurrency` | `none` | No worker scheduling or shared limit is needed. | `not needed` | [NAG requirements](non_assembly_gap_requirements.md) | Runtime worker coordination is out of scope. | none |
| `config` | `none` | No loaded config contract is required by the first reviewer slice. | `not needed` | [goal contracts](../../../crates/meld-execution/src/goals/contracts.rs) | Review uses stored goals and supplied projection data. | none |
| `context` | `none` | Context frames can supply upstream facts but do not review goals. | `not needed` | [NAG requirements](non_assembly_gap_requirements.md) | The concern starts after world state projection. | none |
| `control` | `none` | Control node state is not the active goal lifecycle source. | `not needed` | [execution assessment](../execution/assessment.md) | Execution goal storage is authoritative for lifecycle. | none |
| `error` | `none` | Existing execution invariant errors cover goal command validation. | `not needed` | [goal store](../../../crates/meld-execution/src/goals/store.rs) | No top level error mapping is required until an adapter exposes the reviewer. | none |
| `events` | `none` | Event sequence is already available through publication and evidence paths. | `not needed` | [outcome evidence](../../../src/execution/outcome_evidence.rs), [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) | NAG-4 observes a caller supplied cursor but does not append or replay events. | none |
| `execution` | `own` | Goal storage can query active goals and apply `SatisfyGoalCommand`. Planning can report `PlanningResult::Satisfied`, but no reviewer converts evaluation into lifecycle mutation. | `partial` | [goal contracts](../../../crates/meld-execution/src/goals/contracts.rs), [goal store](../../../crates/meld-execution/src/goals/store.rs), [planning runtime](../../../crates/meld-execution/src/planning/runtime.rs), [goals test](../../../crates/meld-execution/tests/goals.rs) | not applicable | Add satisfaction reviewer contracts, implementation, and focused tests. |
| `heads` | `none` | Head index behavior is unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | Satisfaction does not touch legacy head state. | none |
| `ignore` | `none` | File selection policy is unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | Reviewer consumes projected facts, not filesystem ignore rules. | none |
| `init` | `none` | Bootstrap assets are unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | No init asset or template is required. | none |
| `lib` | `adapter` | Public exports expose execution, goal, and planning contracts, but no reviewer type exists yet. | `not started` | [execution root exports](../../../src/execution.rs), [execution crate root](../../../crates/meld-execution/src/lib.rs), [goals exports](../../../crates/meld-execution/src/goals.rs) | not applicable | Export reviewer request, result, and status types after implementation. |
| `logging` | `none` | Logging has no satisfaction contract. | `not needed` | [domain snapshot](#domain-snapshot) | Reviewer diagnostics are return values, not log authority. | none |
| `merkle_traversal` | `none` | Tree traversal is unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | Review uses projected world state rather than raw traversal. | none |
| `metadata` | `none` | Frame metadata is unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | Goal satisfaction lifecycle is not frame metadata. | none |
| `prompt_context` | `none` | Prompt lineage is unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | Reviewer should not inspect prompt artifacts. | none |
| `provider` | `none` | Provider execution is unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | Satisfied review must not call a provider. | none |
| `session` | `none` | Session lifecycle is not durable goal truth. | `not needed` | [domain snapshot](#domain-snapshot) | Goal lifecycle belongs to execution storage. | none |
| `store` | `none` | Storage primitives are already used by the execution goal store. | `not needed` | [persistent goal store](../../../crates/meld-execution/src/goals/persistent_store.rs) | No generic node store change is required. | none |
| `task` | `none` | Task outcomes are upstream of NAG-4 through prior gaps. | `not needed` | [NAG requirements](non_assembly_gap_requirements.md) | Task failure behavior is NAG-5. | none |
| `telemetry` | `none` | Telemetry is not required for the callable reviewer. | `not needed` | [NAG requirements](non_assembly_gap_requirements.md) | Diagnostics can be returned without observability integration. | none |
| `types` | `none` | Shared top level types are not needed. | `not needed` | [domain snapshot](#domain-snapshot) | Reviewer types should live in execution. | none |
| `views` | `none` | Presentation shaped output is unrelated. | `not needed` | [domain snapshot](#domain-snapshot) | No read model change is required. | none |
| `workflow` | `none` | Workflow gates evaluate workflow output only. | `not needed` | [workflow gates](../../../crates/meld-execution/src/workflow/gates.rs) | Workflow gate pass state is not execution goal satisfaction. | none |
| `workspace` | `none` | Workspace facts may feed evidence, but workspace does not decide lifecycle. | `not needed` | [NAG requirements](non_assembly_gap_requirements.md) | Review is downstream of projection. | none |
| `world_state` | `publish` | Planner query can project a `meld_lang::WorldState`; projection output carries source refs and hydration refs, but no explicit reviewer sequence field. | `partial` | [planner query](../../../crates/meld-world-model/src/planner/query.rs), [planner contracts](../../../crates/meld-world-model/src/planner/contracts.rs), [planner tests](../../../crates/meld-world-model/tests/planner.rs) | not applicable | Supply current projection plus stable review sequence to the execution reviewer. |

## Gaps And Follow Ups

| Gap | Owning Domain | Required Action | Proof |
| --- | --- | --- | --- |
| Missing satisfaction reviewer | `execution` | Add a reviewer that reads active goals, evaluates each goal target against supplied world state, and calls `satisfy_goal` only for `EvalResult::Satisfied`. | `satisfaction_reviewer_marks_goal_satisfied_only_after_world_state_match` |
| Missing reviewer result contract | `execution` | Define deterministic statuses for satisfied, unsatisfied, indeterminate, missing projection, and command outcome cases. | Contract round trip test |
| Missing stable review sequence contract | `execution`, `world_state` | Make reviewer input require `review_seq` or extend projection output with an equivalent high water cursor. | Idempotency test across repeated review at the same cursor |
| Missing public export | `lib` | Re export reviewer contracts from the execution crate and any root execution boundary after implementation. | Compile test through public import |

## Implementation Boundary

The narrow implementation should live under execution goal behavior, for example `crates/meld-execution/src/goals/review.rs`, because it owns lifecycle mutation and can reuse existing goal contracts. World model should continue to publish projected `meld_lang::WorldState` and provenance only. The reviewer should accept projections as input rather than reaching into world model internals.

The reviewer should derive command ids from goal id and review sequence, such as `satisfy:{goal_id}:{review_seq}`. Repeating the same review at the same cursor should replay the same goal command outcome. Unsatisfied and indeterminate evaluations should return diagnostics and leave the goal active.

## Evidence Summary

- `SatisfyGoalCommand` and `GoalLifecycle::Satisfied` already exist in [goal contracts](../../../crates/meld-execution/src/goals/contracts.rs).
- In memory and persistent stores already apply lifecycle commands idempotently by command id in [goal store](../../../crates/meld-execution/src/goals/store.rs) and [persistent goal store](../../../crates/meld-execution/src/goals/persistent_store.rs).
- Planning already uses `meld_lang::evaluate` and can detect already satisfied goals in [planning runtime](../../../crates/meld-execution/src/planning/runtime.rs).
- World model planner projection already produces `meld_lang::WorldState` in [planner query](../../../crates/meld-world-model/src/planner/query.rs).
- The current missing piece is a callable contract that ties these surfaces together without creating runtime assembly.
