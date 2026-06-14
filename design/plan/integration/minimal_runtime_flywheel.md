# Minimal Runtime Flywheel

Date: 2026-06-06
Status: assessed
Scope: focused runtime wiring plan for one `docs_writer` cognitive flywheel turn

## Purpose

This document defines the exact method for proving that the isolated domains created for the cognitive architecture are wired together correctly.

It is not a full architecture plan. It is a focused integration artifact for the minimum runtime path that lets one `docs_writer` flywheel turn move through seeded or observed evidence, world model projection, goal curation, execution planning, task network dispatch, outcome publication, world model update, and goal satisfaction review.

## Related Requirements

The non assembly blockers from this assessment are broken out in [Non Assembly Gap Fix Requirements](non_assembly_gap_requirements.md). That document owns the pre assembly requirements for goal acceptance, publication, evidence mapping, satisfaction review, and failure behavior.

The durable runtime assembly shape is defined in [Durable Runtime First Slice](durable_runtime_first_slice.md). This document defines the target loop. The durable slice defines how to host it without centralizing domain semantics.

## Concern Definition

The concern is one minimal runtime flywheel for the `docs_freshness` scenario over a `docs_writer` turn, starting from seeded or observed evidence for one workspace node, projecting a planner facing `WorldState`, curating an agent goal, storing and planning that goal, lowering the resulting execution composition into task network commands, executing the docs writer task path, publishing the task outcome into the event spine, replaying world state, and checking satisfaction. A completed flywheel turn means a low confidence docs freshness belief becomes a durable active execution goal, execution runs one docs writer task graph, the outcome is published as canonical execution facts, world state projection changes, and the goal is marked satisfied or explicitly not satisfied.

## In Scope

- one `docs_freshness` subject backed by a `workspace_fs` node reference
- seeded evidence or the smallest observed evidence source needed to start the loop
- current event spine append and replay contracts
- graph and belief projection into `meld_lang::WorldState`
- one threshold based agent curation rule that emits `AgentGoalCommand`
- one execution goal acceptance from producer curation output to active goal
- one planning runtime pass from active goal to execution composition
- one execution composition lowering pass into task network commands
- one task network worker path through claim, task execution, outcome, and publication state
- one outcome event handoff back into the event spine
- one world model update pass after outcome publication
- one goal satisfaction review pass
- focused characterization tests for the cross domain handoffs

## Out Of Scope

- recursive sub-goal lowering
- plan diffing
- switching cost model
- graph repair mutations
- shared task reuse across goals
- conditional edge execution unless required by `docs_writer`
- broad workflow migration
- full sensory runtime
- causation and regime inference
- multi-agent coordination
- provider capacity scheduling
- durable task executor resume beyond task network replay

## Minimal Loop Boundary

The assessed target is one complete runtime loop:

```text
seeded evidence or observed fact
-> event spine append
-> world model graph and belief update
-> planner-facing WorldState projection
-> agent goal command
-> neutral goal acceptance
-> execution goal store
-> planning runtime
-> task network mutation command
-> task network worker execution
-> task outcome publication
-> event spine append
-> world model consumption
-> updated WorldState projection
-> goal satisfaction check
```

For the first implementation, seeded evidence is acceptable. Full sensory observation remains deferred because there is no current top level `src/sensory.rs` domain and the implementation plan marks sensory as deferred.

## Required Domain Snapshot

Snapshot date: `2026-06-06`

Snapshot source: current repository command output.

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

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

## Domain Assessment Matrix

| Domain | Needed Integration | Current Integration | Required For Minimal Loop | Completeness | Evidence | Non Integration Rationale | Follow Up |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `agent` | `consume` | Legacy agent config and registry feed existing `docs_writer` workflow identity. Cognitive seed agent records live under `world_state`. | Map the curated goal agent id to the runtime agent identity used by provider and context writes. | `partial` | [agent root](../../../src/agent.rs), [runtime assembly](../../../src/cli/runtime_assembly.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | Define the seed agent identity bridge or keep cognitive agent identity explicitly separate from legacy writer config. |
| `api` | `adapter` | `ContextApi` wires world model queries and implements execution publication and world model query ports. | Provide the narrow runtime adapter for event publication, context reads, context writes, provider calls, and world model artifact lookup. | `partial` | [API world model wiring](../../../src/api.rs), [execution port impls](../../../src/execution/ports.rs) | none | Add the minimal flywheel host entrypoint or route that composes the existing ports. |
| `branches` | `none` | Branch migration runtime uses graph replay, but the flywheel can use a fixed branch scope as world model data. | No branch registration, federation, or migration behavior is required for one default branch loop. | `not needed` | [assessment policy](../../../governance/assessment_by_domain_policy.md), [plan phase list](../README.md) | The first loop only needs a branch id inside world model belief and planner contracts. | none |
| `capability` | `consume` | Capability catalog and invocation payloads are already the task facing execution surface. | Planning lowering must resolve operators to capability contracts, and task execution must invoke registered capabilities. | `complete` | [capability root](../../../src/capability.rs), [planning runtime test](../../../crates/meld-execution/tests/planning_runtime.rs), [task bridge test](../../../crates/meld-execution/tests/task_network_execution_bridge.rs) | none | none |
| `cli` | `adapter` | CLI runtime assembly creates `ContextApi`, `ProgressRuntime`, `GraphRuntime`, workflow registry, agent registry, and provider registry. | A user facing command is optional for the first proof, but CLI is the likely host for the runtime assembly. | `partial` | [CLI assembly](../../../src/cli/runtime_assembly.rs), [workflow CLI compatibility test](../../../tests/integration/workflow_task_compatibility.rs) | none | Add a focused command only after the library runtime path is proven. |
| `compat` | `none` | Compatibility reexports expose graph and world state store helpers for legacy tests. | No new compatibility wrapper is required for the first flywheel. | `not needed` | [compat root](../../../src/compat.rs) | The flywheel should use explicit domain contracts instead of adding shim behavior. | none |
| `concurrency` | `none` | Context writes use node locks, and task network dispatch owns fenced claims. | No extra shared concurrency policy is needed beyond existing context locks and task network claim checks. | `not needed` | [Context API write path](../../../src/api.rs), [task network dispatch](../../../crates/meld-execution/src/task_network/dispatch.rs) | The task network domain owns dispatch fencing for this concern. | none |
| `config` | `consume` | Config loads storage roots, workflow locations, agent configs, and provider configs. | Runtime assembly needs store roots, workflow assets, curation threshold config or fixture data, provider binding, and agent identity. | `partial` | [config root](../../../src/config.rs), [runtime assembly](../../../src/cli/runtime_assembly.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | Decide whether the first proof uses a fixed fixture config or a persisted flywheel config surface. |
| `context` | `publish` | Context owns frame writes, head selection events, graph backed head reads, and task facing context capabilities. | The docs writer task must read context and write a final frame while emitting context graph facts. | `complete` | [Context API put frame](../../../src/api.rs), [context head contracts](../../../src/context/head.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | none |
| `control` | `none` | Legacy control events can produce world state generation claims, but task network outcome is the selected path for this flywheel. | No control projection is needed if task outcome publication is canonical. | `not needed` | [world state reducer](../../../crates/meld-world-model/src/world_state/reducer.rs), [event spine test](../../../tests/integration/event_spine.rs) | Routing the new loop through legacy node control would make control authoritative for task network outcome. | none |
| `error` | `observe` | Store, API, execution, and world model errors already have stable mappings through domain error types. | The host needs to propagate or summarize failures without changing domain truth. | `partial` | [execution ports](../../../crates/meld-execution/src/execution/ports.rs), [task network store error tests](../../../crates/meld-execution/tests/task_network_store.rs) | none | Define the failure outcome contract for the end to end flywheel test. |
| `events` | `own` | `meld-events` owns envelopes, object refs, event relations, sequence allocation, idempotent append, and replay reads. | All durable cross domain facts must enter through the event spine. | `complete` | [events lib](../../../crates/meld-events/src/lib.rs), [event store](../../../crates/meld-events/src/events/store.rs), [event spine tests](../../../tests/integration/event_spine.rs) | none | none |
| `execution` | `own` | Goals, planning, lowering, task network contracts, task network store, task runtime bridge, outcome records, goal acceptance, and publication bridge exist. | Glue is still needed from durable goal query to planning, from planning to task network submission, and from updated world state to satisfaction. | `partial` | [goals](../../../crates/meld-execution/src/goals.rs), [planning](../../../crates/meld-execution/src/planning.rs), [lowering](../../../crates/meld-execution/src/planning/lowering.rs), [task network](../../../crates/meld-execution/src/task_network.rs), [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs), [task bridge test](../../../crates/meld-execution/tests/task_network_execution_bridge.rs) | none | Host the existing execution runtime pieces inside the durable first slice. |
| `heads` | `none` | Legacy head index remains as compatibility fallback behind graph backed head reads. | The flywheel should read current heads through world model graph when configured. | `not needed` | [head read adapter](../../../src/api.rs), [context head backfill](../../../src/context/head.rs) | The context domain already owns the compatibility fallback. | none |
| `ignore` | `none` | Ignore policy affects workspace scans. | The flywheel does not change ignored path selection. | `not needed` | [ignore root](../../../src/ignore.rs), [workspace scan usage](../../../src/workspace.rs) | Workspace scan can keep its existing ignore behavior. | none |
| `init` | `adapter` | Workflow initialization installs built in workflow assets used by docs writer tests. | The proof needs docs writer workflow and task package assets available before runtime. | `partial` | [init root](../../../src/init.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | Add seed data initialization for flywheel stores only if the runtime command needs it. |
| `lib` | `adapter` | Public crate surface exports current domains but does not expose a single flywheel runtime. | Public export is only needed if the first proof is hosted outside internal tests. | `partial` | [lib root](../../../src/lib.rs) | none | Export the host only after its owner is chosen. |
| `logging` | `none` | Logging config and output do not carry durable flywheel facts. | No logging integration is required for correctness. | `not needed` | [logging root](../../../src/logging.rs) | Durable evidence lives in events and world state, not process logs. | none |
| `merkle_traversal` | `consume` | The docs writer task uses traversal capability during existing task execution. | The task path must collect workspace context through existing traversal capability registration. | `complete` | [docs writer task capability registration](../../../tests/integration/docs_writer_task.rs), [task runtime](../../../crates/meld-execution/src/task/runtime.rs) | none | none |
| `metadata` | `publish` | Generated frame metadata is built and validated for docs writer output frames. | The final context frame must carry generated metadata and prompt provenance expected by context writes. | `complete` | [metadata use in docs writer test](../../../tests/integration/docs_writer_task.rs), [workflow executor](../../../src/workflow/executor.rs) | none | none |
| `prompt_context` | `consume` | Prompt context artifact storage is wired into `ContextApi` and task generation paths. | Provider prompts and prompt lineage must remain available to docs writer task execution. | `complete` | [API storage wiring](../../../src/api.rs), [runtime assembly](../../../src/cli/runtime_assembly.rs) | none | none |
| `provider` | `consume` | Provider registry and provider execution capability support docs writer provider calls. | The task runtime needs a provider binding for the docs writer capability chain. | `complete` | [provider root](../../../src/provider.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | none |
| `session` | `observe` | Session ids are used by progress and execution event contexts. | One event session id is required to group flywheel events. | `complete` | [execution event context](../../../crates/meld-execution/src/execution/ports.rs), [event spine tests](../../../tests/integration/event_spine.rs) | none | none |
| `store` | `consume` | Node store, event store, graph runtime, and task network stores all use sled, but goal, belief, and task network stores are not assembled under one flywheel root. | The proof needs deterministic storage roots for spine, graph, belief, agent, goal, task network, and context frame state. | `partial` | [runtime assembly](../../../src/cli/runtime_assembly.rs), [task network sled store](../../../crates/meld-execution/src/task_network/store/sled.rs), [goal store test](../../../crates/meld-execution/tests/goals.rs) | none | Define storage layout for the flywheel runtime fixture. |
| `task` | `own` | Task package loading, compilation, expansion, executor, and task event emission already run the docs writer task path. | The task domain must execute the selected task graph and emit task events. | `complete` | [task root](../../../src/task.rs), [task runtime](../../../crates/meld-execution/src/task/runtime.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | none |
| `telemetry` | `adapter` | `ProgressRuntime` wraps event append for CLI and tests while reexporting event identity contracts. | Telemetry may host session scoped event emission, but events remain authoritative. | `complete` | [telemetry root](../../../src/telemetry.rs), [event spine tests](../../../tests/integration/event_spine.rs) | none | Keep telemetry downstream and adapter scoped. |
| `types` | `none` | Shared `NodeID` and `FrameID` are used by context and workspace internals. | No new shared type is needed for the flywheel. | `not needed` | [types root](../../../src/types.rs), [event identity contract](../../../crates/meld-events/src/lib.rs) | Cross domain identity already uses `DomainObjectRef`. | none |
| `views` | `consume` | Context views shape selected frames for docs writer task inputs. | The proof can use existing context view selectors. | `complete` | [API get node](../../../src/api.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | none |
| `workflow` | `adapter` | Existing docs writer workflow can execute through the task package path and emit workflow telemetry. | The cognitive flywheel may reuse docs writer assets, but execution goal and task network should not depend on workflow as authority. | `partial` | [workflow root](../../../src/workflow.rs), [workflow executor](../../../src/workflow/executor.rs), [workflow task compatibility test](../../../tests/integration/workflow_task_compatibility.rs) | none | Decide whether the first flywheel invokes task package assets directly or through workflow adapter. |
| `workspace` | `consume` | Workspace scan and node resolution provide stable node subjects for docs writer tests. | The proof needs one target workspace node mapped to `DomainObjectRef` domain `workspace_fs`. | `complete` | [workspace root](../../../src/workspace.rs), [context node ref](../../../src/context/head.rs), [docs writer task test](../../../tests/integration/docs_writer_task.rs) | none | none |
| `world_state` | `own` | Graph runtime, claim reducer, belief runtime, planner projection, agent store, and curation contracts exist under `meld-world-model`. | The loop needs outcome facts to update docs freshness belief, project a changed `WorldState`, and feed satisfaction review. | `partial` | [world model lib](../../../crates/meld-world-model/src/lib.rs), [graph runtime](../../../crates/meld-world-model/src/world_state/graph/runtime.rs), [world state reducer](../../../crates/meld-world-model/src/world_state/reducer.rs), [planner projection](../../../crates/meld-world-model/src/planner/projection.rs), [agent curation](../../../crates/meld-world-model/src/agent/curation.rs) | none | Connect task outcome evidence to belief reassessment and expose the post outcome projection for satisfaction. |

## Wiring Steps

| Step | Source Domain | Target Domain | Data Contract | Current Path | Required Path | Test Proof | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `workspace` or seeded fixture | `events` | `EventEnvelope` with `DomainObjectRef` subject | Context and task events already append envelopes | Select the first input source for `docs_freshness` and append it through the spine | [event spine tests](../../../tests/integration/event_spine.rs) | `partial` |
| 2 | `events` | `world_state` | sequenced event records | `GraphRuntime::catch_up` replays spine into traversal indexes | Same path | [traversal graph tests](../../../tests/integration/traversal_graph.rs), [workflow task compatibility test](../../../tests/integration/workflow_task_compatibility.rs) | `complete` |
| 3 | `world_state` graph and belief | `world_state` planner | `PlannerProjectionInput` to `PlannerProjectionOutput` with `WorldState` | `project_world_state` and `PlannerQuery` exist | Same path | [planner tests](../../../crates/meld-world-model/tests/planner.rs) | `complete` |
| 4 | `world_state` planner and belief | `world_state` agent | `AgentDelivery` to `AgentGoalCommand` | `AgentCuration` emits ground proposed goals | Same path | [agent tests](../../../crates/meld-world-model/tests/agent.rs) | `complete` |
| 5 | `world_state` agent | `execution` goals | `AgentGoalCommand` to `GoalAcceptanceRequest` to `AddGoalCommand` | [goal API](../../../crates/meld-execution/src/goals/api.rs) validates neutral requests, activates, stores, and dedupes by source identity | Same path | [goal acceptance test](../../../tests/integration/goal_acceptance.rs) | `complete` |
| 6 | `execution` goals | `execution` planning | active `Goal` plus `WorldState` | `PlanningRuntime::plan_goal` returns composed result | Assemble from durable goal query and current projection | [planning runtime tests](../../../crates/meld-execution/tests/planning_runtime.rs) | `partial` |
| 7 | `execution` planning | `execution` task network | `ExecutionComposition` to mutation `Set` | `ExecutionCompositionLowerer` emits task network mutations | Submit through command store | [composition lowering tests](../../../crates/meld-execution/tests/composition_lowering.rs) | `partial` |
| 8 | `execution` task network | `task` runtime | dispatch `Claim`, materialized init payload, `TaskExecutor` | Bridge test claims, executes, records outcome | Same path inside runtime host | [task network bridge test](../../../crates/meld-execution/tests/task_network_execution_bridge.rs) | `complete` |
| 9 | `execution` task network | `events` | pending `Publication` or task event envelopes | [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) appends retryable task outcome publications and marks publication state after append success | Same path | [publication bridge test](../../../crates/meld-execution/tests/task_network_publication_bridge.rs) | `complete` |
| 10 | `events` | `world_state` | execution outcome facts | Reducer consumes selected execution task and control events | Consume the chosen docs writer outcome fact as docs freshness evidence | [world state graph tests](../../../tests/integration/world_state_graph.rs) | `partial` |
| 11 | `world_state` planner | `execution` goals | post outcome `WorldState` plus active goal | Agent satisfaction curation and goal mutation adapter exist | Assemble after outcome replay and call execution satisfy API | [goal acceptance test](../../../tests/integration/goal_acceptance.rs) | `partial` |

## Required Runtime Assembly

| Runtime Object | Owner | Construction Site | Storage Root | Inputs | Outputs | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `ProgressRuntime` and `EventStore` | `events` with telemetry adapter | `CliRuntimeAssembly` today | configured sled store | event envelopes | sequenced event records | `complete` |
| `GraphRuntime` | `world_state` | `CliRuntimeAssembly` today | same sled store as event spine | sequenced event records | traversal indexes | `complete` |
| `WorldStateStore` | `world_state` | tests today | sled store | execution events | claims and evidence records | `partial` |
| `BeliefRuntime` and `BeliefStore` | `world_state` | tests today | sled store | graph anchors and promoted evidence | belief views | `partial` |
| `AgentStore` and `AgentCuration` | `world_state` | tests today | sled store | belief delivery and planner projection | curation decision and goal command | `partial` |
| `PersistentGoalSetStore` | `execution` | tests today | sled store | `AddGoalCommand` | active and satisfied goals | `partial` |
| `PlanningRuntime` | `execution` | tests today | in memory method library and catalog | goal and `WorldState` | `PlanningResult` | `partial` |
| `ExecutionCompositionLowerer` | `execution` | tests today | none | `ExecutionComposition` | task network mutation set | `partial` |
| `SledTaskNetworkStore` | `execution` | tests today | sled store | task network commands | reduced task graph and journal | `partial` |
| `TaskExecutor` | `task` | workflow and bridge tests | task artifact repo id | materialized task init | artifacts and task events | `complete` |
| `ContextApi` | `api` and `context` | `CliRuntimeAssembly` today | node, frame, artifact, and event stores | context and execution port calls | frame writes, event publication, graph reads | `complete` |
| publication bridge | `execution` and `events` | [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) | event spine plus task network store | pending or failed publications | appended event facts and marked publications | `complete` |
| agent satisfaction curation boundary | `world_state` and execution adapter | tests today | goal store plus planner projection | mutation command plus satisfy command | `partial` |

## Persistence And Replay

| State | Owner | Existing Store | Required Store | Replay Requirement | Gap |
| --- | --- | --- | --- | --- | --- |
| event spine | `events` | `EventStore` | same | read all events after cursor in sequence order | none |
| graph traversal | `world_state` | `TraversalStore` through `GraphRuntime` | same | catch up from last reduced seq | none |
| world state claims | `world_state` | `WorldStateStore` | same, assembled in runtime | replay execution facts into claims and evidence | not assembled in flywheel |
| belief views | `world_state` | `BeliefStore` | same, assembled in runtime | reassess subject after new evidence | not assembled in host |
| agent decisions | `world_state` | `AgentStore` | same, assembled in runtime | dedupe curation by decision key and subscription cursor | not assembled in host |
| execution goals | `execution` | `PersistentGoalSetStore` | same, assembled in runtime | reopen active and satisfied lifecycle state | not assembled in host |
| task network | `execution` | `SledTaskNetworkStore` | same, assembled in runtime | replay journal into identical reduced state | runtime assembly missing |
| context frames | `context` | `FrameStorage` and node store | same | graph backed head reads match replay | none |

## Test Plan

| Test | Purpose | Fixture | Expected Proof | Status |
| --- | --- | --- | --- | --- |
| `full_docs_freshness_loop` | prove typed goal satisfaction mechanics | `meld-lang` docs freshness loop | goal starts unsatisfied and projected effects satisfy it | `existing` |
| `planner_projection_shape` | prove world model projects belief and graph into `WorldState` | world model planner fixture | confidence, stale, observation needed, and accessible propositions exist | `existing` |
| `agent_low_confidence_emits_ground_goal_command` | prove curation emits a ground goal | low confidence belief view | proposed goal command evaluates unsatisfied before update and satisfied after update | `existing` |
| `persistent_goal_store_reopens_records_and_command_outcomes` | prove durable execution goal state | sled goal store | active goal and command idempotency survive reopen | `existing` |
| `low_confidence_docs_goal_returns_composed` | prove planning turns the goal into composition | docs freshness planning fixture | `PlanningResult::Composed` with projected effect | `existing` |
| `composition_lowering_emits_all_operator_steps_in_order` | prove execution composition lowers to task graph | phase 8 docs composition | task network mutations for prepare, collect, and write steps | `existing` |
| `phase8_task_network_slice_runs_and_survives_reopen` | prove task network plus task runtime bridge | phase 8 task network fixture | claims execute, outcome records, publication mark, and reopen replay match | `existing` |
| `docs_writer_task_runs_to_completion` | prove existing docs writer task path | local provider fixture | final docs writer context frame exists | `existing` |
| `minimal_runtime_flywheel_turn_persists_and_satisfies_goal` | prove the actual flywheel | one seeded low confidence docs node | curation, goal store, planning, task network, event publication, world model update, and satisfaction happen in one test | `missing` |

## Acceptance Criteria

The minimal runtime flywheel is complete when all statements below are true.

- one first evidence source is accepted into the loop
- world model emits a planner facing `WorldState`
- agent curation emits a durable goal command
- execution stores the goal and plans a matching method
- execution lowers the composition into task network commands
- task network executes the required `docs_writer` work
- task outcome publication reaches the chosen event handoff
- world model consumes the outcome fact
- projected `WorldState` changes in the expected direction
- goal satisfaction is evaluated after outcome consumption
- the loop has an end to end test with stable fixtures

## Non Goals

These items must not be pulled into the minimal runtime flywheel unless a future domain assessment proves they are blockers.

- full autonomous sensory worker lifecycle
- conditional branch semantics
- task network graph repair
- shared task equivalence
- multi-agent coordination
- provider capacity scheduling
- durable task executor resume
- workflow migration beyond the exact `docs_writer` bridge needed for the slice

## Gaps And Follow Ups

The flywheel is `partial`, not complete.

The major gap is durable runtime hosting. Code exists for events, graph replay, planner projection, curation, goals, planning, lowering, task network execution, publication, evidence ingestion, and satisfaction curation, but no root runtime host currently ticks those domain runtimes through one durable turn.

Producer-neutral goal acceptance is implemented. `AgentCuration` emits `AgentGoalCommand`, integration maps it to `GoalAcceptanceRequest`, and [goal API](../../../crates/meld-execution/src/goals/api.rs) stores an active execution goal through `AddGoalCommand`.

Task outcome publication is implemented as a callable bridge. [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) appends task network outcome facts into the event spine and marks publication state after append success.

The post outcome world model update is implemented. [promoted evidence ingestion](../../../crates/meld-world-model/src/belief/ingestion.rs) accepts generic promoted records, and [outcome evidence mapping](../../../src/execution/outcome_evidence.rs) ties docs task success to configured belief evidence without moving task semantics into world model.

Satisfaction review is implemented as an agent curation boundary plus root integration adapter. The remaining work is to host it after outcome evidence ingestion and planner projection refresh.

The durable host must follow [Durable Runtime First Slice](durable_runtime_first_slice.md): one product process may tick every actor, but all semantic progress must be recorded through domain stores, event append, or command APIs.

## Work Extraction

| Work Item | Owning Domain | Blocking Domain | Evidence Link | Verification Command | Priority |
| --- | --- | --- | --- | --- | --- |
| Select the first evidence source and fixture for `docs_freshness` | `world_state` | `workspace` | [plan phase list](../README.md) | `cargo test -p meld-world-model --test belief` | `P0` |
| Implement producer-neutral goal acceptance | `execution` | `world_state` | [goal API](../../../crates/meld-execution/src/goals/api.rs), [goal acceptance test](../../../tests/integration/goal_acceptance.rs) | `cargo test --test integration_tests producer_neutral_goal_acceptance_stores_active_plannable_goal` | `done` |
| Define durable runtime host shape | `integration` | `events`, `world_state`, `execution` | [durable runtime first slice](durable_runtime_first_slice.md) | doc review | `done` |
| Add durable runtime host assembly | `meld` | `events`, `world_state`, `execution`, `api`, `store` | [runtime assembly](../../../src/cli/runtime_assembly.rs), [execution ports](../../../crates/meld-execution/src/execution/ports.rs) | `cargo test minimal_runtime_flywheel_turn_persists_and_satisfies_goal` | `P0` |
| Add task outcome publication bridge | `execution` | `events` | [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs), [publication bridge test](../../../crates/meld-execution/tests/task_network_publication_bridge.rs) | `cargo test -p meld-execution --test task_network_publication_bridge` | `done` |
| Connect docs writer outcome to docs freshness belief update | `world_state` | `events`, `execution` | [promoted evidence ingestion](../../../crates/meld-world-model/src/belief/ingestion.rs), [outcome evidence mapper](../../../src/execution/outcome_evidence.rs), [outcome evidence test](../../../tests/integration/outcome_evidence.rs) | `cargo test --test integration_tests docs_writer_success_promotes_configured_freshness_evidence` | `done` |
| Add satisfaction curation host tick | `meld` | `world_state`, `execution` | [goal mutation adapter](../../../src/execution/goal_mutation.rs), [goal acceptance test](../../../tests/integration/goal_acceptance.rs) | `cargo test --test integration_tests agent_satisfaction_curation_marks_goal_satisfied_only_after_world_state_match` | `P0` |
| Add end to end minimal flywheel test | `execution` | all participating domains | [task bridge test](../../../crates/meld-execution/tests/task_network_execution_bridge.rs) | `cargo test minimal_runtime_flywheel_turn_persists_and_satisfies_goal` | `P0` |

## Verification Commands

Record of commands that should prove the existing slices and the final flywheel.

```sh
cargo test -p meld-lang --test evaluation_loop full_docs_freshness_loop
cargo test -p meld-world-model --test planner planner_projection_shape
cargo test -p meld-world-model --test agent agent_low_confidence_emits_ground_goal_command
cargo test -p meld-execution --test goals persistent_goal_store_reopens_records_and_command_outcomes
cargo test -p meld-execution --test planning_runtime low_confidence_docs_goal_returns_composed
cargo test -p meld-execution --test composition_lowering composition_lowering_emits_all_operator_steps_in_order
cargo test -p meld-execution --test task_network_execution_bridge phase8_task_network_slice_runs_and_survives_reopen
cargo test --test integration_tests docs_writer_task_runs_to_completion
cargo test minimal_runtime_flywheel_turn_persists_and_satisfies_goal
```

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |
| `2026-06-06` | Domain snapshot command | `31` current top level source domains | Includes `compat`, which is not listed in the advisory policy snapshot. |
| `2026-06-06` | [event store](../../../crates/meld-events/src/events/store.rs) and [event spine tests](../../../tests/integration/event_spine.rs) | Event append and replay are complete | Event spine is not the missing flywheel piece. |
| `2026-06-06` | [planner tests](../../../crates/meld-world-model/tests/planner.rs) and [agent tests](../../../crates/meld-world-model/tests/agent.rs) | Projection and curation are complete for first slice contracts | Runtime assembly remains missing. |
| `2026-06-06` | [planning runtime tests](../../../crates/meld-execution/tests/planning_runtime.rs), [composition lowering tests](../../../crates/meld-execution/tests/composition_lowering.rs), and [task bridge test](../../../crates/meld-execution/tests/task_network_execution_bridge.rs) | Execution contracts exist through task network bridge | Later NAG work added publication and satisfaction boundaries. |
| `2026-06-06` | [docs writer task test](../../../tests/integration/docs_writer_task.rs) and [workflow task compatibility test](../../../tests/integration/workflow_task_compatibility.rs) | Existing docs writer task path works | Existing workflow path is not yet the cognitive flywheel. |
| `2026-06-08` | [goal API](../../../crates/meld-execution/src/goals/api.rs) and [goal acceptance test](../../../tests/integration/goal_acceptance.rs) | Producer-neutral goal acceptance is complete | NAG-2 through NAG-5 were open at that point. |
| `2026-06-08` | [publication bridge](../../../crates/meld-execution/src/task_network/publication.rs) and [publication bridge test](../../../crates/meld-execution/tests/task_network_publication_bridge.rs) | Task outcome publication bridge is complete | NAG-3 through NAG-5 were open at that point. |
| `2026-06-09` | [promoted evidence ingestion](../../../crates/meld-world-model/src/belief/ingestion.rs), [outcome evidence mapper](../../../src/execution/outcome_evidence.rs), and [outcome evidence test](../../../tests/integration/outcome_evidence.rs) | Outcome fact to belief evidence is complete | NAG-4 and NAG-5 remain open. |
